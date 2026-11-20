//! Scrobbling to Last.fm. Last.fm only lets an API account sign its calls with
//! that account's shared secret, and SpotiLarp has no server to keep one on,
//! so you bring your own API account (like the Spotify Client ID). The key,
//! secret and session stay in the OS keyring.
//!
//! Signing in is Last.fm's desktop flow: get a token, approve it in the
//! browser, then trade it for a session key. Scrobbles that can't be sent
//! (offline, Last.fm down) wait in a small file and go with the next ones.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use keyring::Entry;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use url::Url;

const API_URL: &str = "https://ws.audioscrobbler.com/2.0/";
const AUTH_URL: &str = "https://www.last.fm/api/auth/";
const KEYRING_SERVICE: &str = "SpotiLarp-lastfm";
const KEYRING_KEYS: &str = "api_account";
const KEYRING_SESSION: &str = "session";
/// The API takes up to this many scrobbles per call.
const BATCH: usize = 50;
/// Last.fm turns down scrobbles older than two weeks.
const MAX_AGE_SECS: i64 = 14 * 24 * 60 * 60;
/// So a long time offline can't grow the file without end.
const MAX_QUEUED: usize = 2000;
const POLL_EVERY: Duration = Duration::from_secs(2);
const POLL_FOR: Duration = Duration::from_secs(120);
/// What the page shows for a cancelled sign-in (api.js LOGIN_CANCELLED), so it says nothing.
const CANCELLED: &str = "Login cancelled";

/// Bumped by a cancel, so a sign-in waiting on the browser stops.
static CONNECT_ATTEMPT: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize, Deserialize, Clone)]
struct ApiAccount {
    api_key: String,
    secret: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Session {
    key: String,
    name: String,
}

#[derive(Serialize)]
pub struct Status {
    connected: bool,
    username: Option<String>,
    has_keys: bool,
    /// Scrobbles waiting to be sent.
    queued: usize,
}

/// A song as Last.fm wants it: `duration` in seconds.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TrackInfo {
    artist: String,
    track: String,
    #[serde(default)]
    album: Option<String>,
    #[serde(default)]
    duration: Option<u32>,
}

/// `timestamp` is when the song started, in seconds since 1970 (UTC).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Scrobble {
    #[serde(flatten)]
    track: TrackInfo,
    timestamp: i64,
}

/// How a call to Last.fm went wrong.
#[derive(Debug, PartialEq)]
enum CallError {
    /// Worth trying again later: no connection, Last.fm down or busy, or a
    /// login problem that reconnecting fixes. A scrobble waits in the queue.
    Retry(String),
    /// Last.fm said no to the request itself; sending it again won't help.
    Rejected(String),
    /// The session was revoked (from Last.fm's settings, say).
    SessionGone,
}

impl CallError {
    fn message(&self) -> String {
        match self {
            CallError::Retry(m) | CallError::Rejected(m) => m.clone(),
            CallError::SessionGone => "Last.fm no longer accepts this login, connect again".to_string(),
        }
    }
}

// ---- Commands ----

#[tauri::command]
pub async fn lastfm_status() -> Status {
    let session = load_session();
    let queued = read_queue().len();
    // Anything left over from last time goes now, without holding up the answer.
    if session.is_some() && queued > 0 {
        tauri::async_runtime::spawn(async {
            let _ = flush(Vec::new()).await;
        });
    }
    Status {
        connected: session.is_some(),
        username: session.map(|s| s.name),
        has_keys: load_keys().is_some(),
        queued,
    }
}

/// Saves your API account. A session belongs to the API account that made
/// it, so a different key needs connecting again.
#[tauri::command]
pub fn lastfm_set_keys(api_key: String, secret: String) -> Result<(), String> {
    let api_key = api_key.trim().to_string();
    let secret = secret.trim().to_string();
    if api_key.is_empty() || secret.is_empty() {
        return Err("Paste both the API key and the shared secret".to_string());
    }
    if load_keys().is_some_and(|keys| keys.api_key != api_key) {
        forget_session()?;
    }
    save_json(KEYRING_KEYS, &ApiAccount { api_key, secret })
}

/// Opens Last.fm's "allow access" page and waits for you to approve it.
#[tauri::command]
pub async fn lastfm_connect(app: AppHandle) -> Result<String, String> {
    let keys = load_keys().ok_or("Add your Last.fm API key and shared secret first")?;
    let attempt = CONNECT_ATTEMPT.fetch_add(1, Ordering::SeqCst) + 1;
    let http = crate::http::client();

    let token = call(&http, &keys, None, "auth.getToken", Vec::new(), false)
        .await
        .map_err(|e| e.message())?
        .get("token")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or("Last.fm didn't send a sign-in token")?;

    let mut url = Url::parse(AUTH_URL).expect("static auth URL is valid");
    url.query_pairs_mut().append_pair("api_key", &keys.api_key).append_pair("token", &token);
    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|e| format!("Couldn't open the browser: {e}"))?;

    let started = std::time::Instant::now();
    loop {
        tokio::time::sleep(POLL_EVERY).await;
        if CONNECT_ATTEMPT.load(Ordering::SeqCst) != attempt {
            return Err(CANCELLED.to_string());
        }
        let params = vec![("token".to_string(), token.clone())];
        match call(&http, &keys, None, "auth.getSession", params, false).await {
            Ok(body) => {
                let session = body.get("session").ok_or("Last.fm didn't send a session")?;
                let session = Session {
                    key: text(session, "key").ok_or("Last.fm didn't send a session key")?,
                    name: text(session, "name").unwrap_or_default(),
                };
                save_json(KEYRING_SESSION, &session)?;
                // Scrobbles from before the session went missing can go now.
                tauri::async_runtime::spawn(async {
                    let _ = flush(Vec::new()).await;
                });
                return Ok(session.name);
            }
            // 14 is "not approved yet", and a dropped connection keeps waiting too;
            // a bad token or API account ends it.
            Err(CallError::Retry(message)) => {
                if matches!(code_of(&message), "4" | "10" | "13" | "15" | "26") {
                    return Err(friendly(&message));
                }
            }
            Err(e) => return Err(friendly(&e.message())),
        }
        if started.elapsed() > POLL_FOR {
            return Err("Timed out waiting for you to allow access on Last.fm, try again".to_string());
        }
    }
}

#[tauri::command]
pub fn lastfm_cancel_connect() {
    CONNECT_ATTEMPT.fetch_add(1, Ordering::SeqCst);
}

/// Forgets the session and what was waiting to be sent; the API account stays.
#[tauri::command]
pub fn lastfm_disconnect() -> Result<(), String> {
    forget_session()?;
    write_queue(&[]);
    Ok(())
}

/// Shows the song on your profile as playing now. Best effort: a miss doesn't matter.
#[tauri::command]
pub async fn lastfm_now_playing(track: TrackInfo) -> Result<(), String> {
    let (Some(keys), Some(session)) = (load_keys(), load_session()) else {
        return Ok(());
    };
    let mut params = vec![
        ("artist".to_string(), track.artist),
        ("track".to_string(), track.track),
    ];
    if let Some(album) = track.album.filter(|a| !a.is_empty()) {
        params.push(("album".to_string(), album));
    }
    if let Some(duration) = track.duration {
        params.push(("duration".to_string(), duration.to_string()));
    }
    match call(&crate::http::client(), &keys, Some(&session), "track.updateNowPlaying", params, true).await {
        Ok(_) => Ok(()),
        Err(CallError::SessionGone) => {
            forget_session()?;
            Err(CallError::SessionGone.message())
        }
        Err(e) => Err(e.message()),
    }
}

/// Sends the scrobbles, with any still waiting from before. What can't be
/// sent now is kept for next time, so this only fails for a missing login.
#[tauri::command]
pub async fn lastfm_scrobble(scrobbles: Vec<Scrobble>) -> Result<usize, String> {
    flush(scrobbles).await
}

// ---- Sending ----

fn flush_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

/// Sends the queue plus `fresh` in batches; returns how many are still waiting.
async fn flush(fresh: Vec<Scrobble>) -> Result<usize, String> {
    // One at a time, so two flushes can't send the same queue twice.
    let _guard = flush_lock().lock().await;
    let mut pending = read_queue();
    pending.extend(fresh);
    let now = unix_now();
    pending.retain(|s| now - s.timestamp < MAX_AGE_SECS);

    let (Some(keys), Some(session)) = (load_keys(), load_session()) else {
        write_queue(&pending);
        return Err("Not connected to Last.fm".to_string());
    };
    let http = crate::http::client();
    let mut waiting = Vec::new();
    let mut batches = pending.chunks(BATCH);
    for batch in batches.by_ref() {
        match call(&http, &keys, Some(&session), "track.scrobble", scrobble_params(batch), true).await {
            // Accepted, or ignored by Last.fm (it says why per scrobble): either way they're done.
            Ok(_) | Err(CallError::Rejected(_)) => {}
            Err(CallError::SessionGone) => {
                forget_session()?;
                waiting.extend_from_slice(batch);
                break;
            }
            Err(CallError::Retry(_)) => {
                waiting.extend_from_slice(batch);
                break;
            }
        }
    }
    // After a failure the rest wait too, rather than hammering Last.fm.
    for batch in batches {
        waiting.extend_from_slice(batch);
    }
    write_queue(&waiting);
    Ok(waiting.len())
}

/// The batch form of track.scrobble: `artist[0]`, `track[0]`, `timestamp[0]`...
fn scrobble_params(scrobbles: &[Scrobble]) -> Vec<(String, String)> {
    let mut params = Vec::new();
    for (i, s) in scrobbles.iter().enumerate() {
        params.push((format!("artist[{i}]"), s.track.artist.clone()));
        params.push((format!("track[{i}]"), s.track.track.clone()));
        params.push((format!("timestamp[{i}]"), s.timestamp.to_string()));
        if let Some(album) = s.track.album.as_ref().filter(|a| !a.is_empty()) {
            params.push((format!("album[{i}]"), album.clone()));
        }
        if let Some(duration) = s.track.duration {
            params.push((format!("duration[{i}]"), duration.to_string()));
        }
    }
    params
}

/// A signed call. `post` is for the calls that change something (scrobbling).
async fn call(
    http: &reqwest::Client,
    keys: &ApiAccount,
    session: Option<&Session>,
    method: &str,
    mut params: Vec<(String, String)>,
    post: bool,
) -> Result<Value, CallError> {
    params.push(("method".to_string(), method.to_string()));
    params.push(("api_key".to_string(), keys.api_key.clone()));
    if let Some(session) = session {
        params.push(("sk".to_string(), session.key.clone()));
    }
    let signature = sign(&params, &keys.secret);
    params.push(("api_sig".to_string(), signature));
    params.push(("format".to_string(), "json".to_string()));

    let request = if post { http.post(API_URL).form(&params) } else { http.get(API_URL).query(&params) };
    let response = request
        .send()
        .await
        .map_err(|e| CallError::Retry(format!("Couldn't reach Last.fm: {e}")))?;
    let status = response.status();
    let body: Value = match response.json().await {
        Ok(body) => body,
        Err(_) if status.is_server_error() => return Err(CallError::Retry(format!("Last.fm is having trouble ({status})"))),
        Err(e) => return Err(CallError::Retry(format!("Last.fm sent something odd: {e}"))),
    };
    match body.get("error").and_then(Value::as_u64) {
        None if status.is_success() => Ok(body),
        None => Err(CallError::Retry(format!("Last.fm is having trouble ({status})"))),
        Some(code) => Err(classify(code, text(&body, "message").unwrap_or_default())),
    }
}

/// Last.fm's error codes: https://www.last.fm/api/errorcodes
fn classify(code: u64, message: String) -> CallError {
    let message = format!("{code}: {message}");
    match code {
        9 => CallError::SessionGone,
        // Offline, temporarily unavailable, operation failed, rate limited, and the
        // key/signature/token problems a fixed API account or a new login cure
        // (14 is "not approved yet" while signing in).
        8 | 11 | 16 | 29 | 4 | 10 | 13 | 14 | 15 | 26 => CallError::Retry(message),
        _ => CallError::Rejected(message),
    }
}

/// The error code at the start of a message from `classify`, or "" for a connection problem.
fn code_of(message: &str) -> &str {
    message.split_once(':').map(|(code, _)| code).filter(|code| code.parse::<u64>().is_ok()).unwrap_or("")
}

/// What a failed sign-in says on screen.
fn friendly(message: &str) -> String {
    match code_of(message) {
        "10" | "26" => "Last.fm doesn't recognize that API key, check it in Settings".to_string(),
        "13" => "Last.fm says the shared secret doesn't match the API key, check it in Settings".to_string(),
        "15" => "The sign-in expired before it was approved, try again".to_string(),
        _ => format!("Last.fm said no: {message}"),
    }
}

/// api_sig: every parameter but `format` and `callback`, sorted by name,
/// written name-then-value with nothing between, then the secret, as MD5.
fn sign(params: &[(String, String)], secret: &str) -> String {
    let mut sorted: Vec<&(String, String)> = params
        .iter()
        .filter(|(name, _)| name != "format" && name != "callback")
        .collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut text = String::new();
    for (name, value) in sorted {
        text.push_str(name);
        text.push_str(value);
    }
    text.push_str(secret);
    format!("{:x}", md5::compute(text.as_bytes()))
}

// ---- Storage ----

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn load_keys() -> Option<ApiAccount> {
    load_json(KEYRING_KEYS)
}

fn load_session() -> Option<Session> {
    load_json(KEYRING_SESSION)
}

fn forget_session() -> Result<(), String> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_SESSION).map_err(|e| e.to_string())?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn load_json<T: for<'de> Deserialize<'de>>(user: &str) -> Option<T> {
    let saved = Entry::new(KEYRING_SERVICE, user).ok()?.get_password().ok()?;
    serde_json::from_str(&saved).ok()
}

fn save_json<T: Serialize>(user: &str, value: &T) -> Result<(), String> {
    let entry = Entry::new(KEYRING_SERVICE, user).map_err(|e| e.to_string())?;
    let json = serde_json::to_string(value).map_err(|e| e.to_string())?;
    entry.set_password(&json).map_err(|e| e.to_string())
}

fn queue_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("SpotiLarp")
        .join("lastfm_queue.json")
}

fn read_queue() -> Vec<Scrobble> {
    std::fs::read_to_string(queue_path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_queue(scrobbles: &[Scrobble]) {
    let path = queue_path();
    if scrobbles.is_empty() {
        let _ = std::fs::remove_file(path);
        return;
    }
    // The newest are kept if it's grown too long.
    let kept = &scrobbles[scrobbles.len().saturating_sub(MAX_QUEUED)..];
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(kept) {
        let _ = std::fs::write(path, json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn signs_like_lastfm_documents() {
        // Last.fm's own example; the MD5 was worked out separately (Node's crypto)
        // from "api_keyxxxxxxxxmethodauth.getSessiontokenyyyyyyilovecher".
        let params = pairs(&[("token", "yyyyyy"), ("method", "auth.getSession"), ("api_key", "xxxxxxxx"), ("format", "json")]);
        assert_eq!(sign(&params, "ilovecher"), "1333ebf6f7dec747486b6ce965cca66b");
    }

    #[test]
    fn signs_a_scrobble_batch_sorted_by_name() {
        let scrobble = Scrobble {
            track: TrackInfo {
                artist: "Artist & Co".into(),
                track: "Ünïcode Song".into(),
                album: Some("Album".into()),
                duration: Some(200),
            },
            timestamp: 1_700_000_000,
        };
        let mut params = scrobble_params(&[scrobble]);
        params.push(("method".into(), "track.scrobble".into()));
        params.push(("api_key".into(), "KEY".into()));
        params.push(("sk".into(), "SESSION".into()));
        // From "album[0]Albumapi_keyKEYartist[0]Artist & Coduration[0]200methodtrack.scrobble
        // skSESSIONtimestamp[0]1700000000track[0]Ünïcode Songsecret" (UTF-8), also via Node.
        assert_eq!(sign(&params, "secret"), "fb4e3e7c4c47d24cdea793b0d596e377");
    }

    #[test]
    fn batch_params_skip_missing_album_and_duration() {
        let scrobble = Scrobble {
            track: TrackInfo { artist: "A".into(), track: "T".into(), album: Some(String::new()), duration: None },
            timestamp: 5,
        };
        assert_eq!(
            scrobble_params(&[scrobble.clone(), scrobble]),
            pairs(&[
                ("artist[0]", "A"),
                ("track[0]", "T"),
                ("timestamp[0]", "5"),
                ("artist[1]", "A"),
                ("track[1]", "T"),
                ("timestamp[1]", "5"),
            ])
        );
    }

    #[test]
    fn errors_sort_into_retry_rejected_and_revoked() {
        assert_eq!(classify(9, "Invalid session key".into()), CallError::SessionGone);
        assert!(matches!(classify(29, "Rate limit".into()), CallError::Retry(_)));
        assert!(matches!(classify(11, "Offline".into()), CallError::Retry(_)));
        assert!(matches!(classify(6, "Invalid parameters".into()), CallError::Rejected(_)));
    }

    #[test]
    fn scrobbles_read_back_from_the_page_shape() {
        let json = r#"{"artist":"A","track":"T","album":"Al","duration":180,"timestamp":1700000000}"#;
        let scrobble: Scrobble = serde_json::from_str(json).unwrap();
        assert_eq!(scrobble.track.album.as_deref(), Some("Al"));
        assert_eq!(scrobble.timestamp, 1_700_000_000);
    }
}
