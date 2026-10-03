use std::path::PathBuf;

use librespot_core::authentication::Credentials;
use librespot_core::cache::Cache;
use librespot_core::config::SessionConfig;
use librespot_core::session::Session;
use tauri::AppHandle;

use super::token_store;
use super::{CONNECT_OAUTH_REDIRECT_PATH, CONNECT_OAUTH_REDIRECT_PORT, CONNECT_OAUTH_SCOPES};
use crate::auth::oauth::{self, RefreshError, TokenResponse};

const NOT_CONNECTED: &str = "Playback isn't connected, click \"Connect playback\"";

pub fn has_cached_credentials() -> bool {
    token_store::load_refresh_token().is_some()
}

pub fn audio_cache_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|base| base.join("SpotiLarp").join("audio"))
}

fn remove_legacy_credentials() {
    if let Some(base) = dirs::config_dir() {
        let _ = std::fs::remove_file(base.join("SpotiLarp").join("librespot-cache").join("credentials.json"));
    }
}

pub fn forget_credentials() -> Result<(), String> {
    remove_legacy_credentials();
    token_store::clear_refresh_token()
}

fn into_credentials(token: TokenResponse) -> Credentials {
    if let Some(refresh_token) = token.refresh_token.as_deref().filter(|t| !t.is_empty()) {
        let _ = token_store::save_refresh_token(refresh_token);
    }
    Credentials::with_access_token(token.access_token)
}

async fn obtain_credentials(app: &AppHandle, client_id: &str, interactive: bool) -> Result<Credentials, String> {
    let http = crate::http::client();
    if let Some(refresh_token) = token_store::load_refresh_token() {
        match oauth::refresh(&http, client_id, &refresh_token).await {
            Ok(token) => return Ok(into_credentials(token)),
            Err(RefreshError::Revoked) => {
                let _ = token_store::clear_refresh_token();
            }
            Err(RefreshError::Failed(e)) => return Err(e),
        }
    }
    if !interactive {
        return Err(NOT_CONNECTED.to_string());
    }
    let scopes = CONNECT_OAUTH_SCOPES.join(" ");
    let code = oauth::browser_login(
        app,
        client_id,
        &scopes,
        CONNECT_OAUTH_REDIRECT_PORT,
        CONNECT_OAUTH_REDIRECT_PATH,
        false,
    )
    .await?;
    Ok(into_credentials(oauth::exchange_code(&http, client_id, &code).await?))
}

pub async fn connect(app: &AppHandle, cache_limit_mb: u64, interactive: bool) -> Result<Session, String> {
    remove_legacy_credentials();
    let (audio_dir, size_limit) = match audio_cache_dir() {
        Some(dir) if cache_limit_mb > 0 => (Some(dir), Some(cache_limit_mb * 1024 * 1024)),
        _ => (None, None),
    };
    let cache = Cache::new(None::<PathBuf>, None, audio_dir, size_limit).map_err(|e| e.to_string())?;

    let config = SessionConfig::default();
    let credentials = obtain_credentials(app, &config.client_id, interactive).await?;

    let session = Session::new(config, Some(cache));
    session
        .connect(credentials, false)
        .await
        .map_err(|e| format!("failed to connect Spotify Connect session: {e}"))?;
    Ok(session)
}
