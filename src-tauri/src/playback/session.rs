use std::path::PathBuf;

use librespot_core::authentication::Credentials;
use librespot_core::cache::Cache;
use librespot_core::config::SessionConfig;
use librespot_core::session::Session;
use librespot_oauth::{OAuthClient, OAuthClientBuilder};

use super::token_store;
use super::{CONNECT_OAUTH_REDIRECT_PORT, CONNECT_OAUTH_SCOPES};

pub fn has_cached_credentials() -> bool {
    token_store::load_refresh_token().is_some()
}

fn cache_dir() -> PathBuf {
    let base = dirs::config_dir().expect("could not determine OS config directory");
    base.join("SpotiLarp")
        .join("librespot-cache")
}

fn build_oauth_client(client_id: &str) -> Result<OAuthClient, String> {
    let redirect_uri = format!("http://127.0.0.1:{CONNECT_OAUTH_REDIRECT_PORT}/login");
    OAuthClientBuilder::new(client_id, &redirect_uri, CONNECT_OAUTH_SCOPES.to_vec())
        .open_in_browser()
        .build()
        .map_err(|e| format!("failed to build librespot OAuth client: {e}"))
}

fn obtain_credentials_interactive(client_id: &str) -> Result<Credentials, String> {
    let oauth_client = build_oauth_client(client_id)?;
    let token = oauth_client
        .get_access_token()
        .map_err(|e| format!("librespot OAuth login failed: {e}"))?;
    let _ = token_store::save_refresh_token(&token.refresh_token);
    Ok(Credentials::with_access_token(token.access_token))
}

fn obtain_credentials(client_id: &str) -> Result<Credentials, String> {
    if let Some(refresh_token) = token_store::load_refresh_token() {
        let oauth_client = build_oauth_client(client_id)?;
        if let Ok(token) = oauth_client.refresh_token(&refresh_token) {
            let _ = token_store::save_refresh_token(&token.refresh_token);
            return Ok(Credentials::with_access_token(token.access_token));
        }
    }
    obtain_credentials_interactive(client_id)
}

pub async fn connect() -> Result<Session, String> {
    let dir = cache_dir();
    let cache = Cache::new(Some(&dir), None, None, None).map_err(|e| e.to_string())?;

    let config = SessionConfig::default();
    let client_id = config.client_id.clone();

    let credentials = tokio::task::spawn_blocking(move || obtain_credentials(&client_id))
        .await
        .map_err(|e| format!("librespot OAuth task panicked: {e}"))??;

    let session = Session::new(config, Some(cache));
    session
        .connect(credentials, true)
        .await
        .map_err(|e| format!("failed to connect Spotify Connect session: {e}"))?;
    Ok(session)
}
