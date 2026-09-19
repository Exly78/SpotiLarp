use serde::Deserialize;
use std::time::{Duration, Instant};
use url::Url;

use crate::auth::{token_store, AUTH_SCOPES, REDIRECT_URI};

const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const AUTHORIZE_URL: &str = "https://accounts.spotify.com/authorize";
const API_BASE: &str = "https://api.spotify.com/v1";

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
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
}

impl SpotifyClient {
    pub fn new(client_id: Option<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            client_id,
            access_token: None,
            access_token_expires_at: None,
            refresh_token: token_store::load_refresh_token(),
        }
    }

    pub fn has_refresh_token(&self) -> bool {
        self.refresh_token.is_some()
    }

    pub fn has_client_id(&self) -> bool {
        self.client_id.is_some()
    }

    pub fn set_client_id(&mut self, client_id: String) {
        self.client_id = Some(client_id);
    }

    fn require_client_id(&self) -> Result<&str, String> {
        self.client_id
            .as_deref()
            .ok_or_else(|| "Spotify Client ID not configured".to_string())
    }

    pub fn logout(&mut self) -> Result<(), String> {
        self.access_token = None;
        self.access_token_expires_at = None;
        self.refresh_token = None;
        token_store::clear_refresh_token()
    }

    pub fn build_authorize_url(&self, challenge: &str, state: &str) -> Result<String, String> {
        let client_id = self.require_client_id()?;
        let mut url = Url::parse(AUTHORIZE_URL).expect("static authorize URL is valid");
        url.query_pairs_mut()
            .append_pair("client_id", client_id)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", REDIRECT_URI)
            .append_pair("code_challenge_method", "S256")
            .append_pair("code_challenge", challenge)
            .append_pair("scope", AUTH_SCOPES)
            .append_pair("state", state)
            
            .append_pair("show_dialog", "true");
        Ok(url.to_string())
    }

    pub async fn exchange_code(&mut self, code: &str, verifier: &str) -> Result<(), String> {
        let client_id = self.require_client_id()?.to_string();
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", REDIRECT_URI),
            ("client_id", &client_id),
            ("code_verifier", verifier),
        ];
        let resp = self
            .http
            .post(TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        self.store_token_response(resp).await
    }

    async fn refresh(&mut self) -> Result<(), String> {
        let client_id = self.require_client_id()?.to_string();
        let refresh_token = self
            .refresh_token
            .clone()
            .ok_or_else(|| "no refresh token available, log in first".to_string())?;
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
            ("client_id", client_id.as_str()),
        ];
        let resp = self
            .http
            .post(TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        self.store_token_response(resp).await
    }

    async fn store_token_response(&mut self, resp: reqwest::Response) -> Result<(), String> {
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Spotify token request failed ({status}): {body}"));
        }
        let token: TokenResponse = resp.json().await.map_err(|e| e.to_string())?;
        self.access_token = Some(token.access_token);
        self.access_token_expires_at =
            Some(Instant::now() + Duration::from_secs(token.expires_in.saturating_sub(30)));
        if let Some(refresh_token) = token.refresh_token {
            let _ = token_store::save_refresh_token(&refresh_token);
            self.refresh_token = Some(refresh_token);
        }
        Ok(())
    }

    async fn ensure_valid_token(&mut self) -> Result<String, String> {
        let needs_refresh = match (&self.access_token, &self.access_token_expires_at) {
            (Some(_), Some(expiry)) => Instant::now() >= *expiry,
            _ => true,
        };
        if needs_refresh {
            self.refresh().await?;
        }
        self.access_token
            .clone()
            .ok_or_else(|| "not logged in".to_string())
    }

    pub async fn get_me(&mut self) -> Result<Me, String> {
        self.get_json("/me", &[]).await
    }

    pub async fn get_json<T: serde::de::DeserializeOwned>(
        &mut self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, String> {
        let token = self.ensure_valid_token().await?;
        let resp = self
            .http
            .get(format!("{API_BASE}{path}"))
            .query(query)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = resp.status();
        let body = resp.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("GET {path} failed ({status}): {body}"));
        }
        serde_json::from_str(&body)
            .map_err(|e| format!("GET {path} decode error: {e}\nbody: {body}"))
    }

    pub async fn send_empty(
        &mut self,
        method: reqwest::Method,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<(), String> {
        let token = self.ensure_valid_token().await?;
        let resp = self
            .http
            .request(method.clone(), format!("{API_BASE}{path}"))
            .query(query)
            
            .header(reqwest::header::CONTENT_LENGTH, "0")
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("{method} {path} failed ({status}): {body}"));
        }
        Ok(())
    }
}
