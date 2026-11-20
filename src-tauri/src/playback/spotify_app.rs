//! The Spotify app on this computer, played through by a small Spicetify
//! extension SpotiLarp writes for it.
//!
//! Spotify only takes remote orders (Spotify Connect) for Premium accounts,
//! but its desktop app plays any song on a free one. So on its own computer
//! SpotiLarp tells the app what to play directly: the extension connects to
//! SpotiLarp over a WebSocket on this computer only, with a key only the two
//! of them know, takes "play this song from here", "pause" and the like, and
//! reports what the app is doing. To the rest of SpotiLarp it's one more
//! device to play on (connect.rs), so Jams and listening along work through it.

use std::hash::{BuildHasher, Hasher};
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::Message;

use super::connect::{self, Snapshot};
use super::events::{ArtistRef, PlaybackEvent};
use crate::config;

/// How the rest of SpotiLarp knows it among the devices it can play on.
pub const DEVICE_ID: &str = "spotilarp:spotify-app";
pub const DEVICE_NAME: &str = "Spotify app";
const PORT: u16 = 8897;
const EXTENSION_FILE: &str = "spotilarp.js";
const STATUS_EVENT: &str = "spotify-app";
const EXTENSION: &str = include_str!("spotify_app_extension.js");

/// The extension connected now, if any: which connection it is, and where
/// messages for it go.
static LINK: Mutex<Option<(u64, UnboundedSender<String>)>> = Mutex::new(None);
static CONNECTIONS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Serialize)]
pub struct Status {
    connected: bool,
    /// Whether Spicetify is installed, for setting the extension up.
    spicetify: bool,
}

pub fn connected() -> bool {
    LINK.lock().is_ok_and(|link| link.is_some())
}

pub fn status() -> Status {
    Status {
        connected: connected(),
        spicetify: find_spicetify().is_some(),
    }
}

/// The key the extension shows to connect, made once and kept.
fn token() -> String {
    if let Some(token) = config::load_spotify_app_token() {
        return token;
    }
    // Two of the standard library's randomly seeded hashes: 128 random bits.
    let random = || std::collections::hash_map::RandomState::new().build_hasher().finish();
    let token = format!("{:016x}{:016x}", random(), random());
    let _ = config::save_spotify_app_token(&token);
    token
}

/// Listens for the extension, on this computer only.
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let listener = match tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, PORT)).await {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("The Spotify app's link couldn't open port {PORT}: {e}");
                return;
            }
        };
        let token = token();
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                tauri::async_runtime::spawn(serve(app.clone(), stream, token.clone()));
            }
        }
    });
}

async fn serve(app: AppHandle, stream: TcpStream, token: String) {
    let key = format!("token={token}");
    let check = |request: &Request, response: Response| -> Result<Response, ErrorResponse> {
        let allowed = request
            .uri()
            .query()
            .is_some_and(|query| query.split('&').any(|pair| pair == key));
        if allowed {
            Ok(response)
        } else {
            let mut refused = ErrorResponse::new(Some("Not SpotiLarp's extension".to_string()));
            *refused.status_mut() = StatusCode::FORBIDDEN;
            Err(refused)
        }
    };
    let Ok(socket) = tokio_tungstenite::accept_hdr_async(stream, check).await else {
        return;
    };
    let (mut outgoing, mut incoming) = socket.split();
    let (sender, mut queued) = mpsc::unbounded_channel::<String>();
    let id = CONNECTIONS.fetch_add(1, Ordering::SeqCst) + 1;
    if let Ok(mut link) = LINK.lock() {
        // A newer connection (Spotify restarted, say) replaces the last.
        *link = Some((id, sender));
    }
    let _ = app.emit(STATUS_EVENT, status());
    let writer = tauri::async_runtime::spawn(async move {
        while let Some(text) = queued.recv().await {
            if outgoing.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });
    while let Some(Ok(message)) = incoming.next().await {
        if let Message::Text(text) = message {
            match serde_json::from_str::<Report>(text.as_str()) {
                // Playing, but no song: a moment where Spotify's state hasn't
                // caught up. Taken as "stopped", it would make SpotiLarp let go.
                Ok(report) if report.uri.is_empty() && report.playing => {}
                Ok(report) => connect::report(&app, report.into_snapshot()),
                Err(_) => {}
            }
        }
    }
    writer.abort();
    let was_current = LINK
        .lock()
        .map(|mut link| {
            let current = link.as_ref().is_some_and(|(current, _)| *current == id);
            if current {
                *link = None;
            }
            current
        })
        .unwrap_or(false);
    if was_current {
        let _ = app.emit(STATUS_EVENT, status());
        connect::device_gone(&app, DEVICE_ID, "The Spotify app closed");
    }
}

fn send(message: serde_json::Value) -> Result<(), String> {
    let link = LINK.lock().map_err(|e| e.to_string())?;
    let (_, sender) = link
        .as_ref()
        .ok_or_else(|| "The Spotify app isn't connected. Is it open?".to_string())?;
    sender
        .send(message.to_string())
        .map_err(|_| "The Spotify app isn't connected. Is it open?".to_string())
}

/// A player command, as connect.rs sends them to devices, for the extension.
pub fn command(command: &serde_json::Value) -> Result<(), String> {
    match command["endpoint"].as_str().unwrap_or_default() {
        "play" => send(json!({
            "type": "play",
            "uri": command["context"]["uri"],
            "position": command["options"]["seek_to"],
            "paused": command["options"]["initially_paused"],
        })),
        "pause" => send(json!({ "type": "pause" })),
        "resume" => send(json!({ "type": "resume" })),
        "seek_to" => send(json!({ "type": "seek", "position": command["value"] })),
        other => Err(format!("The Spotify app can't do \"{other}\" from SpotiLarp")),
    }
}

/// 0 to 65535, as SpotiLarp's own volume.
pub fn set_volume(volume: u16) -> Result<(), String> {
    send(json!({ "type": "volume", "volume": f64::from(volume) / f64::from(u16::MAX) }))
}

/// What the extension says the app is doing.
#[derive(Deserialize)]
struct Report {
    #[serde(default)]
    uri: String,
    /// An ad, on a free account.
    #[serde(default)]
    ad: bool,
    #[serde(default)]
    playing: bool,
    #[serde(default)]
    position: f64,
    #[serde(default)]
    duration: f64,
    /// 0 to 1.
    volume: Option<f64>,
    #[serde(default)]
    name: String,
    #[serde(default)]
    artists: Vec<ReportArtist>,
    #[serde(default)]
    album: String,
    cover: Option<String>,
}

#[derive(Deserialize)]
struct ReportArtist {
    #[serde(default)]
    name: String,
    uri: Option<String>,
}

/// The ID in a `spotify:<kind>:<id>` URI.
fn id_of(uri: &str, kind: &str) -> Option<String> {
    uri.strip_prefix(&format!("spotify:{kind}:"))
        .filter(|id| !id.is_empty())
        .map(str::to_string)
}

impl Report {
    fn into_snapshot(self) -> Snapshot {
        let millis = |value: f64| value.clamp(0.0, f64::from(u32::MAX)) as u32;
        let duration_ms = millis(self.duration);
        let artist_list: Vec<ArtistRef> = self
            .artists
            .iter()
            .filter(|artist| !artist.name.is_empty())
            .map(|artist| ArtistRef {
                id: artist.uri.as_deref().and_then(|uri| id_of(uri, "artist")),
                name: artist.name.clone(),
            })
            .collect();
        let track = (!self.uri.is_empty() && !self.ad).then(|| PlaybackEvent::TrackChanged {
            name: self.name.clone(),
            artists: artist_list.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "),
            primary_artist_id: artist_list.first().and_then(|a| a.id.clone()),
            artist_list,
            track_id: id_of(&self.uri, "track"),
            uri: self.uri.clone(),
            album: self.album.clone(),
            duration_ms,
            cover_url: self.cover.clone().filter(|url| url.starts_with("https://")),
        });
        Snapshot {
            active_device_id: if self.uri.is_empty() { String::new() } else { DEVICE_ID.to_string() },
            uri: self.uri,
            playing: self.playing,
            position_ms: millis(self.position),
            volumes: self
                .volume
                .map(|volume| vec![(DEVICE_ID.to_string(), (volume.clamp(0.0, 1.0) * f64::from(u16::MAX)) as u16)])
                .unwrap_or_default(),
            ad: self.ad,
            duration_ms,
            track,
            from_app: true,
        }
    }
}

/// Where Spicetify's program is: on the PATH, or where its installer puts it.
fn find_spicetify() -> Option<PathBuf> {
    let name = if cfg!(windows) { "spicetify.exe" } else { "spicetify" };
    let on_path = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).map(|dir| dir.join(name)).collect::<Vec<_>>())
        .unwrap_or_default();
    let installed = [
        dirs::data_local_dir().map(|dir| dir.join("spicetify").join(name)),
        dirs::home_dir().map(|dir| dir.join(".spicetify").join(name)),
    ];
    on_path
        .into_iter()
        .chain(installed.into_iter().flatten())
        .find(|candidate| candidate.is_file())
}

fn run(spicetify: &Path, args: &[&str]) -> Result<String, String> {
    let mut command = std::process::Command::new(spicetify);
    command.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // No console window flashing up.
        command.creation_flags(0x0800_0000);
    }
    let output = command
        .output()
        .map_err(|e| format!("Couldn't run Spicetify: {e}"))?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if output.status.success() {
        Ok(text)
    } else {
        Err(format!("Spicetify said: {}", text.trim()))
    }
}

/// Writes the extension into Spicetify's Extensions folder, turns it on and
/// applies it (which restarts Spotify).
pub async fn setup() -> Result<String, String> {
    tokio::task::spawn_blocking(|| {
        let spicetify = find_spicetify().ok_or_else(|| {
            "Spicetify isn't installed yet. Install it from spicetify.app, then press Set up again.".to_string()
        })?;
        let config_file = run(&spicetify, &["-c"])?
            .lines()
            .map(str::trim)
            .find(|line| line.ends_with(".ini"))
            .map(PathBuf::from)
            .ok_or_else(|| "Couldn't find Spicetify's settings".to_string())?;
        let extensions = config_file
            .parent()
            .ok_or_else(|| "Couldn't find Spicetify's settings".to_string())?
            .join("Extensions");
        std::fs::create_dir_all(&extensions).map_err(|e| format!("Couldn't make Spicetify's Extensions folder: {e}"))?;
        let source = EXTENSION
            .replace("__SPOTILARP_PORT__", &PORT.to_string())
            .replace("__SPOTILARP_TOKEN__", &token());
        std::fs::write(extensions.join(EXTENSION_FILE), source).map_err(|e| format!("Couldn't write the extension: {e}"))?;
        run(&spicetify, &["config", "extensions", EXTENSION_FILE])?;
        // A first apply on a fresh Spicetify needs Spotify backed up first.
        if run(&spicetify, &["apply"]).is_err() {
            run(&spicetify, &["backup", "apply"])?;
        }
        Ok("Spotify restarts with SpotiLarp's extension, and connects a few seconds after it opens.".to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::{id_of, Report, DEVICE_ID, EXTENSION};
    use crate::playback::events::PlaybackEvent;

    #[test]
    fn reads_the_extensions_report() {
        let report: Report = serde_json::from_str(
            r#"{"uri":"spotify:track:abc","playing":true,"position":1234.6,"duration":200000,"volume":0.5,
                "name":"Song","artists":[{"name":"A","uri":"spotify:artist:a1"},{"name":"B"}],"album":"LP",
                "cover":"https://i.scdn.co/image/x"}"#,
        )
        .unwrap();
        let snapshot = report.into_snapshot();
        assert_eq!(snapshot.active_device_id, DEVICE_ID);
        assert_eq!(snapshot.position_ms, 1234);
        assert_eq!(snapshot.volumes, vec![(DEVICE_ID.to_string(), 32767)]);
        match snapshot.track {
            Some(PlaybackEvent::TrackChanged { artists, primary_artist_id, track_id, .. }) => {
                assert_eq!(artists, "A, B");
                assert_eq!(primary_artist_id.as_deref(), Some("a1"));
                assert_eq!(track_id.as_deref(), Some("abc"));
            }
            _ => panic!("no track"),
        }
    }

    #[test]
    fn an_ad_is_no_song() {
        let report: Report = serde_json::from_str(r#"{"uri":"spotify:ad:x","ad":true,"playing":true}"#).unwrap();
        let snapshot = report.into_snapshot();
        assert!(snapshot.ad);
        assert!(snapshot.track.is_none());
    }

    #[test]
    fn reads_ids() {
        assert_eq!(id_of("spotify:track:abc", "track").as_deref(), Some("abc"));
        assert_eq!(id_of("spotify:episode:abc", "track"), None);
        assert_eq!(id_of("spotify:track:", "track"), None);
    }

    #[test]
    fn the_extension_has_its_placeholders() {
        assert!(EXTENSION.contains("__SPOTILARP_PORT__"));
        assert!(EXTENSION.contains("__SPOTILARP_TOKEN__"));
    }
}
