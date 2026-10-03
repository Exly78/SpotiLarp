use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use super::models::Page;
use crate::auth::oauth::{self, AuthorizationCode, RefreshError, TokenResponse, LOGIN_EXPIRED};
use crate::auth::{token_store, AUTH_SCOPES};

const API_BASE: &str = "https://api.spotify.com/v1";

const MAX_CONCURRENT_REQUESTS: usize = 6;
const MAX_RATE_LIMIT_RETRIES: u32 = 3;
const MAX_RETRY_WAIT: Duration = Duration::from_secs(30);
const MAX_ERROR_BODY_CHARS: usize = 300;

enum RequestError {
    Unauthorized,
    Other(String),
}

impl From<RequestError> for String {
    fn from(error: RequestError) -> Self {
        match error {
            RequestError::Unauthorized => "Spotify didn't accept the login (401 Unauthorized)".to_string(),
            RequestError::Other(message) => message,
        }
    }
}

#[derive(Deserialize)]
pub struct Me {
    pub display_name: Option<String>,
    pub id: String,
}

pub struct SpotifyClient {
    http: reqwest::Client,
    client_id: Option<String>,
    access_token: Option<String>,
    access_token_expires_at: Option<Instant>,
    refresh_token: Option<String>,
    granted_scopes: Option<String>,
    user_id: Option<String>,
}

impl SpotifyClient {
    pub fn new(client_id: Option<String>) -> Self {
        Self {
            http: crate::http::client(),
            client_id,
            access_token: None,
            access_token_expires_at: None,
            refresh_token: token_store::load_refresh_token(),
            granted_scopes: None,
            user_id: None,
        }
    }

    pub fn has_refresh_token(&self) -> bool {
        self.refresh_token.is_some()
    }

    pub fn has_client_id(&self) -> bool {
        self.client_id.is_some()
    }

    pub fn set_client_id(&mut self, client_id: String) -> Result<(), String> {
        let changed = self
            .client_id
            .as_deref()
            .is_some_and(|current| current != client_id);
        self.client_id = Some(client_id);
        if changed {
            self.logout()?;
        }
        Ok(())
    }

    pub fn require_client_id(&self) -> Result<&str, String> {
        self.client_id
            .as_deref()
            .ok_or_else(|| "Spotify Client ID not configured".to_string())
    }

    pub fn logout(&mut self) -> Result<(), String> {
        self.access_token = None;
        self.access_token_expires_at = None;
        self.refresh_token = None;
        self.granted_scopes = None;
        self.user_id = None;
        token_store::clear_refresh_token()
    }

    pub async fn exchange_code(&mut self, code: &AuthorizationCode) -> Result<(), String> {
        let client_id = self.require_client_id()?.to_string();
        let token = oauth::exchange_code(&self.http, &client_id, code).await?;
        self.store_token(token);
        Ok(())
    }

    async fn refresh(&mut self) -> Result<String, String> {
        let client_id = self.require_client_id()?.to_string();
        let refresh_token = self
            .refresh_token
            .clone()
            .ok_or_else(|| "no refresh token available, log in first".to_string())?;
        match oauth::refresh(&self.http, &client_id, &refresh_token).await {
            Ok(token) => Ok(self.store_token(token)),
            Err(RefreshError::Revoked) => {
                let _ = self.logout();
                Err(LOGIN_EXPIRED.to_string())
            }
            Err(RefreshError::Failed(e)) => Err(e),
        }
    }

    fn store_token(&mut self, token: TokenResponse) -> String {
        self.access_token_expires_at =
            Some(Instant::now() + Duration::from_secs(token.expires_in.saturating_sub(30)));
        if token.scope.is_some() {
            self.granted_scopes = token.scope;
        }
        if let Some(refresh_token) = token.refresh_token {
            let _ = token_store::save_refresh_token(&refresh_token);
            self.refresh_token = Some(refresh_token);
        }
        self.access_token = Some(token.access_token.clone());
        token.access_token
    }

    async fn ensure_valid_token(&mut self) -> Result<String, String> {
        match (&self.access_token, &self.access_token_expires_at) {
            (Some(token), Some(expiry)) if Instant::now() < *expiry => Ok(token.clone()),
            _ => self.refresh().await,
        }
    }

    async fn with_token<T, F, Fut>(&mut self, request: F) -> Result<T, String>
    where
        F: Fn(reqwest::Client, String) -> Fut,
        Fut: Future<Output = Result<T, RequestError>>,
    {
        let token = self.ensure_valid_token().await?;
        match request(self.http.clone(), token).await {
            Err(RequestError::Unauthorized) => {
                let token = self.refresh().await?;
                request(self.http.clone(), token).await.map_err(String::from)
            }
            result => result.map_err(String::from),
        }
    }

    pub async fn get_me(&mut self) -> Result<Me, String> {
        let me: Me = self.get_json("/me", &[]).await?;
        self.user_id = Some(me.id.clone());
        Ok(me)
    }

    pub async fn user_id(&mut self) -> Result<String, String> {
        match &self.user_id {
            Some(id) => Ok(id.clone()),
            None => Ok(self.get_me().await?.id),
        }
    }

    pub async fn missing_scopes(&mut self) -> Result<Vec<String>, String> {
        self.ensure_valid_token().await?;
        let Some(granted) = &self.granted_scopes else {
            return Ok(Vec::new());
        };
        let granted: Vec<&str> = granted.split_whitespace().collect();
        Ok(AUTH_SCOPES
            .split_whitespace()
            .filter(|scope| !granted.contains(scope))
            .map(str::to_string)
            .collect())
    }

    pub async fn get_json<T: DeserializeOwned>(
        &mut self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, String> {
        self.with_token(|http, token| async move { Self::get_json_with(&http, &token, path, query).await })
            .await
    }

    async fn get_json_with<T: DeserializeOwned>(
        http: &reqwest::Client,
        token: &str,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, RequestError> {
        let request = http
            .get(format!("{API_BASE}{path}"))
            .query(query)
            .bearer_auth(token);
        let resp = send_with_retry(request).await.map_err(RequestError::Other)?;
        let body = read_body(resp, &format!("GET {path}")).await?;
        serde_json::from_str(&body).map_err(|e| {
            RequestError::Other(format!("GET {path} decode error: {e}\nbody: {}", truncate(&body)))
        })
    }

    pub async fn get_json_concurrent<T>(
        &mut self,
        requests: Vec<(String, Vec<(&'static str, String)>)>,
    ) -> Result<Vec<T>, String>
    where
        T: DeserializeOwned + Send + 'static,
    {
        if requests.is_empty() {
            return Ok(Vec::new());
        }
        self.with_token(|http, token| Self::fetch_concurrent(http, token, requests.clone()))
            .await
    }

    async fn fetch_concurrent<T>(
        http: reqwest::Client,
        token: String,
        requests: Vec<(String, Vec<(&'static str, String)>)>,
    ) -> Result<Vec<T>, RequestError>
    where
        T: DeserializeOwned + Send + 'static,
    {
        let limiter = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));
        let count = requests.len();
        let mut tasks = JoinSet::new();
        for (index, (path, query)) in requests.into_iter().enumerate() {
            let http = http.clone();
            let token = token.clone();
            let limiter = limiter.clone();
            tasks.spawn(async move {
                let _permit = limiter
                    .acquire_owned()
                    .await
                    .map_err(|e| RequestError::Other(e.to_string()))?;
                let query: Vec<(&str, &str)> = query.iter().map(|(k, v)| (*k, v.as_str())).collect();
                let result = Self::get_json_with::<T>(&http, &token, &path, &query).await?;
                Ok::<_, RequestError>((index, result))
            });
        }

        let mut results: Vec<Option<T>> = (0..count).map(|_| None).collect();
        while let Some(joined) = tasks.join_next().await {
            let (index, result) = joined.map_err(|e| RequestError::Other(e.to_string()))??;
            results[index] = Some(result);
        }
        Ok(results.into_iter().flatten().collect())
    }

    pub async fn get_all_pages<T>(&mut self, path: &str, page_size: u32) -> Result<Vec<T>, String>
    where
        T: DeserializeOwned + Send + 'static,
    {
        let limit = page_size.to_string();
        let first: Page<T> = self
            .get_json(path, &[("limit", &limit), ("offset", "0")])
            .await?;
        let mut items = first.items;

        let requests = (page_size..first.total)
            .step_by(page_size as usize)
            .map(|offset| {
                (
                    path.to_string(),
                    vec![("limit", limit.clone()), ("offset", offset.to_string())],
                )
            })
            .collect();
        for page in self.get_json_concurrent::<Page<T>>(requests).await? {
            items.extend(page.items);
        }
        Ok(items)
    }

    pub async fn send_json<T: DeserializeOwned>(
        &mut self,
        method: reqwest::Method,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T, String> {
        self.with_token(|http, token| {
            let method = method.clone();
            async move {
                let request = http
                    .request(method.clone(), format!("{API_BASE}{path}"))
                    .json(body)
                    .bearer_auth(token);
                let resp = send_with_retry(request).await.map_err(RequestError::Other)?;
                let text = read_body(resp, &format!("{method} {path}")).await?;
                let json = if text.trim().is_empty() { "null" } else { text.as_str() };
                serde_json::from_str(json).map_err(|e| {
                    RequestError::Other(format!("{method} {path} decode error: {e}\nbody: {}", truncate(&text)))
                })
            }
        })
        .await
    }

    pub async fn send_empty(
        &mut self,
        method: reqwest::Method,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<(), String> {
        self.with_token(|http, token| {
            let method = method.clone();
            async move {
                let request = http
                    .request(method.clone(), format!("{API_BASE}{path}"))
                    .query(query)
                    .header(reqwest::header::CONTENT_LENGTH, "0")
                    .bearer_auth(token);
                let resp = send_with_retry(request).await.map_err(RequestError::Other)?;
                read_body(resp, &format!("{method} {path}")).await.map(|_| ())
            }
        })
        .await
    }
}

async fn send_with_retry(request: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
    let mut attempt = 0;
    loop {
        let resp = request
            .try_clone()
            .ok_or_else(|| "request can't be retried".to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if resp.status() == StatusCode::TOO_MANY_REQUESTS && attempt < MAX_RATE_LIMIT_RETRIES {
            let wait = retry_after(&resp).unwrap_or_else(|| Duration::from_secs(1 << attempt));
            if wait <= MAX_RETRY_WAIT {
                attempt += 1;
                tokio::time::sleep(wait).await;
                continue;
            }
        }
        return Ok(resp);
    }
}

async fn read_body(resp: reqwest::Response, what: &str) -> Result<String, RequestError> {
    let status = resp.status();
    if status == StatusCode::UNAUTHORIZED {
        return Err(RequestError::Unauthorized);
    }
    let body = resp.text().await.map_err(|e| RequestError::Other(e.to_string()))?;
    if !status.is_success() {
        return Err(RequestError::Other(format!("{what} failed ({status}): {}", truncate(&body))));
    }
    Ok(body)
}

fn retry_after(resp: &reqwest::Response) -> Option<Duration> {
    let secs = resp
        .headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()?;
    Some(Duration::from_secs(secs))
}

pub(crate) fn truncate(body: &str) -> String {
    match body.char_indices().nth(MAX_ERROR_BODY_CHARS) {
        Some((cut, _)) => format!("{}...", &body[..cut]),
        None => body.to_string(),
    }
}
