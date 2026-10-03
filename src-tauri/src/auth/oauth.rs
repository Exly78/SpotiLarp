use reqwest::StatusCode;
use serde::Deserialize;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use url::Url;

use super::{loopback, pkce};
use crate::spotify_api::client::truncate;

const AUTHORIZE_URL: &str = "https://accounts.spotify.com/authorize";
const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";

pub const LOGIN_EXPIRED: &str = "Your Spotify login has expired, please log in again";

#[derive(Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

pub enum RefreshError {
    Revoked,
    Failed(String),
}

pub struct AuthorizationCode {
    pub code: String,
    pub verifier: String,
    pub redirect_uri: String,
}

pub async fn browser_login(
    app: &AppHandle,
    client_id: &str,
    scopes: &str,
    port: u16,
    path: &'static str,
    show_dialog: bool,
) -> Result<AuthorizationCode, String> {
    let pkce_pair = pkce::generate();
    let csrf_state = pkce::generate_state();
    let redirect_uri = format!("http://127.0.0.1:{port}{path}");

    let mut url = Url::parse(AUTHORIZE_URL).expect("static authorize URL is valid");
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("code_challenge_method", "S256")
        .append_pair("code_challenge", &pkce_pair.challenge)
        .append_pair("scope", scopes)
        .append_pair("state", &csrf_state);
    if show_dialog {
        url.query_pairs_mut().append_pair("show_dialog", "true");
    }

    let listener = loopback::listen(port, path).await?;
    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|e| format!("failed to open browser: {e}"))?;
    let callback = tokio::task::spawn_blocking(move || listener.wait())
        .await
        .map_err(|e| format!("loopback listener task panicked: {e}"))??;

    if callback.state != csrf_state {
        return Err("OAuth state mismatch, possible CSRF, aborting login".to_string());
    }
    Ok(AuthorizationCode {
        code: callback.code,
        verifier: pkce_pair.verifier,
        redirect_uri,
    })
}

pub async fn exchange_code(
    http: &reqwest::Client,
    client_id: &str,
    code: &AuthorizationCode,
) -> Result<TokenResponse, String> {
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code.code.as_str()),
        ("redirect_uri", code.redirect_uri.as_str()),
        ("client_id", client_id),
        ("code_verifier", code.verifier.as_str()),
    ];
    let (status, body) = token_request(http, &params).await?;
    if !status.is_success() {
        return Err(format!("Spotify token request failed ({status}): {}", truncate(&body)));
    }
    serde_json::from_str(&body).map_err(|e| e.to_string())
}

pub async fn refresh(
    http: &reqwest::Client,
    client_id: &str,
    refresh_token: &str,
) -> Result<TokenResponse, RefreshError> {
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", client_id),
    ];
    let (status, body) = token_request(http, &params).await.map_err(RefreshError::Failed)?;
    if status.is_success() {
        return serde_json::from_str(&body).map_err(|e| RefreshError::Failed(e.to_string()));
    }
    if is_invalid_grant(status, &body) {
        return Err(RefreshError::Revoked);
    }
    Err(RefreshError::Failed(format!(
        "Spotify token request failed ({status}): {}",
        truncate(&body)
    )))
}

async fn token_request(http: &reqwest::Client, params: &[(&str, &str)]) -> Result<(StatusCode, String), String> {
    let resp = http
        .post(TOKEN_URL)
        .form(params)
        .send()
        .await
        .map_err(|e| format!("Couldn't reach Spotify: {e}"))?;
    let status = resp.status();
    let body = resp.text().await.map_err(|e| e.to_string())?;
    Ok((status, body))
}

fn is_invalid_grant(status: StatusCode, body: &str) -> bool {
    #[derive(Deserialize)]
    struct ErrorBody {
        error: String,
    }
    status == StatusCode::BAD_REQUEST
        && serde_json::from_str::<ErrorBody>(body).is_ok_and(|body| body.error == "invalid_grant")
}
