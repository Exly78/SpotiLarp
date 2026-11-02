//! Connecting a Discord account: Discord's "SpotiLarp wants to access your
//! account" screen, signed in through the browser like Spotify. The Discord
//! app is a public client, so there's no client secret: PKCE proves it's the
//! same app asking. For now it's who you are (name, @username, picture); later
//! features ask for more.

use keyring::Entry;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use url::Url;

use crate::auth::{loopback, pkce, REDIRECT_PORT};
use crate::config;
use crate::discord::SPOTILARP_APP_ID;

// SpotiLarp's Discord app (discord.rs) needs, in its OAuth2 settings, Public
// Client on and http://127.0.0.1:8898/discord as a redirect.
const AUTHORIZE_URL: &str = "https://discord.com/oauth2/authorize";
const TOKEN_URL: &str = "https://discord.com/api/oauth2/token";
const REVOKE_URL: &str = "https://discord.com/api/oauth2/token/revoke";
const ME_URL: &str = "https://discord.com/api/v10/users/@me";
const AVATAR_BASE: &str = "https://cdn.discordapp.com";
const REDIRECT_PATH: &str = "/discord";
const SCOPES: &str = "identify";
const KEYRING_SERVICE: &str = "SpotiLarp-discord";
const KEYRING_USER: &str = "refresh_token";

const NOT_SET_UP: &str = "Discord turned the login down. In the Discord app's OAuth2 settings, turn on \
     Public Client and add http://127.0.0.1:8898/discord as a redirect";

/// Who's connected, kept in the config so it shows without asking Discord.
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct DiscordProfile {
    pub id: String,
    pub username: String,
    /// Their display name, or the username without one.
    pub name: String,
    pub avatar_url: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    global_name: Option<String>,
    avatar: Option<String>,
}

enum TokenError {
    /// The connection was revoked (from Discord's Authorized Apps, say).
    Revoked,
    Failed(String),
}

/// The connected account, as last seen.
pub fn account() -> Option<DiscordProfile> {
    load_refresh_token().and(config::load_discord_account())
}

/// Signs in through the browser and remembers who it was.
pub async fn connect(app: &AppHandle) -> Result<DiscordProfile, String> {
    let client_id = SPOTILARP_APP_ID;
    let pkce_pair = pkce::generate();
    let csrf_state = pkce::generate_state();
    let redirect_uri = format!("http://127.0.0.1:{REDIRECT_PORT}{REDIRECT_PATH}");

    let mut url = Url::parse(AUTHORIZE_URL).expect("static authorize URL is valid");
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("scope", SCOPES)
        .append_pair("state", &csrf_state)
        .append_pair("code_challenge", &pkce_pair.challenge)
        .append_pair("code_challenge_method", "S256");

    let listener = loopback::listen(REDIRECT_PORT, REDIRECT_PATH, "Discord").await?;
    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|e| format!("Couldn't open the browser: {e}"))?;
    let callback = tokio::task::spawn_blocking(move || listener.wait())
        .await
        .map_err(|e| e.to_string())??;
    if callback.state != csrf_state {
        return Err("The Discord login didn't match the one SpotiLarp started, please try again".into());
    }

    let http = crate::http::client();
    let params = [
        ("grant_type", "authorization_code"),
        ("code", callback.code.as_str()),
        ("redirect_uri", redirect_uri.as_str()),
        ("client_id", client_id),
        ("code_verifier", pkce_pair.verifier.as_str()),
    ];
    let tokens = token_request(&http, &params).await.map_err(|e| match e {
        TokenError::Revoked => NOT_SET_UP.to_string(),
        TokenError::Failed(e) => e,
    })?;
    let profile = fetch_profile(&http, &tokens.access_token).await?;
    if let Some(refresh_token) = &tokens.refresh_token {
        save_refresh_token(refresh_token)?;
    }
    config::save_discord_account(Some(profile.clone()))?;
    Ok(profile)
}

/// Asks Discord again, for a changed name or picture. A connection that was
/// revoked is forgotten.
pub async fn refresh() -> Result<Option<DiscordProfile>, String> {
    let Some(refresh_token) = load_refresh_token() else {
        return Ok(None);
    };
    let http = crate::http::client();
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token.as_str()),
        ("client_id", SPOTILARP_APP_ID),
    ];
    let tokens = match token_request(&http, &params).await {
        Ok(tokens) => tokens,
        Err(TokenError::Revoked) => {
            forget()?;
            return Ok(None);
        }
        Err(TokenError::Failed(e)) => return Err(e),
    };
    if let Some(refresh_token) = &tokens.refresh_token {
        save_refresh_token(refresh_token)?;
    }
    let profile = fetch_profile(&http, &tokens.access_token).await?;
    config::save_discord_account(Some(profile.clone()))?;
    Ok(Some(profile))
}

/// Disconnects: Discord is told to drop SpotiLarp's access, and it's forgotten here.
pub async fn disconnect() -> Result<(), String> {
    if let Some(token) = load_refresh_token() {
        let params = [
            ("token", token.as_str()),
            ("token_type_hint", "refresh_token"),
            ("client_id", SPOTILARP_APP_ID),
        ];
        // Forgotten here whether or not Discord heard.
        let _ = crate::http::client().post(REVOKE_URL).form(&params).send().await;
    }
    forget()
}

fn forget() -> Result<(), String> {
    clear_refresh_token()?;
    config::save_discord_account(None)
}

async fn token_request(http: &reqwest::Client, params: &[(&str, &str)]) -> Result<TokenResponse, TokenError> {
    let response = http
        .post(TOKEN_URL)
        .form(params)
        .send()
        .await
        .map_err(|e| TokenError::Failed(format!("Couldn't reach Discord: {e}")))?;
    let status = response.status();
    let body = response.text().await.map_err(|e| TokenError::Failed(e.to_string()))?;
    if status.is_success() {
        return serde_json::from_str(&body).map_err(|e| TokenError::Failed(e.to_string()));
    }
    #[derive(Deserialize)]
    struct ErrorBody {
        error: String,
    }
    let error = serde_json::from_str::<ErrorBody>(&body).map(|body| body.error).unwrap_or_default();
    if status == StatusCode::BAD_REQUEST && matches!(error.as_str(), "invalid_grant" | "invalid_client") {
        return Err(TokenError::Revoked);
    }
    if status == StatusCode::UNAUTHORIZED {
        return Err(TokenError::Revoked);
    }
    Err(TokenError::Failed(format!("Discord said no ({status}): {error}")))
}

async fn fetch_profile(http: &reqwest::Client, access_token: &str) -> Result<DiscordProfile, String> {
    let user: DiscordUser = http
        .get(ME_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Couldn't reach Discord: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Discord didn't say who you are: {e}"))?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(profile(user))
}

fn profile(user: DiscordUser) -> DiscordProfile {
    let avatar_url = match &user.avatar {
        Some(hash) => format!("{AVATAR_BASE}/avatars/{}/{hash}.png?size=128", user.id),
        // Discord's own default pictures, picked from the ID.
        None => {
            let index = user.id.parse::<u64>().map(|id| (id >> 22) % 6).unwrap_or(0);
            format!("{AVATAR_BASE}/embed/avatars/{index}.png")
        }
    };
    DiscordProfile {
        name: user.global_name.filter(|name| !name.trim().is_empty()).unwrap_or_else(|| user.username.clone()),
        username: user.username,
        id: user.id,
        avatar_url,
    }
}

fn save_refresh_token(token: &str) -> Result<(), String> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|e| e.to_string())?;
    entry.set_password(token).map_err(|e| e.to_string())
}

fn load_refresh_token() -> Option<String> {
    Entry::new(KEYRING_SERVICE, KEYRING_USER).ok()?.get_password().ok()
}

fn clear_refresh_token() -> Result<(), String> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|e| e.to_string())?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(avatar: Option<&str>, global_name: Option<&str>) -> DiscordUser {
        DiscordUser {
            id: "80351110224678912".into(),
            username: "nelly".into(),
            global_name: global_name.map(str::to_string),
            avatar: avatar.map(str::to_string),
        }
    }

    #[test]
    fn profiles_use_the_display_name_and_picture() {
        let p = profile(user(Some("8342729096ea3675442027381ff50dfe"), Some("Nelly")));
        assert_eq!(p.name, "Nelly");
        assert_eq!(p.username, "nelly");
        assert_eq!(
            p.avatar_url,
            "https://cdn.discordapp.com/avatars/80351110224678912/8342729096ea3675442027381ff50dfe.png?size=128"
        );
    }

    #[test]
    fn no_picture_or_display_name_falls_back() {
        let p = profile(user(None, Some("  ")));
        assert_eq!(p.name, "nelly");
        // (80351110224678912 >> 22) % 6
        assert_eq!(p.avatar_url, "https://cdn.discordapp.com/embed/avatars/5.png");
    }
}
