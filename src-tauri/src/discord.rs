use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};

// Litterbox, catbox.moe's temporary host: Discord shows permanent catbox.moe
// links as a "?", but loads these. Uploads are deleted after UPLOAD_KEPT.
const COVER_HOST: &str = "https://litterbox.catbox.moe/resources/internals/api.php";
const COVER_LINK_PREFIX: &str = "https://litter.catbox.moe/";
const UPLOAD_KEPT: &str = "72h";
const LINK_LIFETIME: Duration = Duration::from_secs(70 * 60 * 60);
const UPLOAD_RETRY: Duration = Duration::from_secs(120);
/// SpotiLarp's Discord app: the status says "Listening to SpotiLarp", and
/// Connect Discord signs in through it. An app ID is public, not a secret.
pub const SPOTILARP_APP_ID: &str = "1539761294515773562";
const SPOTIFY_WEB: &str = "https://open.spotify.com/";
/// Where the "Get SpotiLarp" button leads: the newest installer.
const DOWNLOAD_URL: &str = "https://github.com/Exly78/SpotiLarp/releases/latest";

static FAILED_UPLOADS: Mutex<Vec<(PathBuf, Instant)>> = Mutex::new(Vec::new());

#[derive(Clone)]
pub enum PresenceUpdate {
    Playing {
        name: String,
        artists: String,
        album: String,
        cover_url: Option<String>,
        position_ms: u32,
        duration_ms: u32,
        track_url: Option<String>,
    },
    Paused {
        name: String,
        artists: String,
        album: String,
        cover_url: Option<String>,
        track_url: Option<String>,
    },
    Cleared,
    /// Where friends listen along (jam.rs), or `None` to drop the button.
    ListenLink(Option<String>),

    Shutdown,
}

pub fn spawn(client_id: String) -> Sender<PresenceUpdate> {
    let (tx, rx) = mpsc::channel::<PresenceUpdate>();
    std::thread::spawn(move || {
        let mut client = DiscordIpcClient::new(&client_id);
        let mut connected = client.connect().is_ok();
        let mut listen_link: Option<String> = None;
        // What's showing, and since when, to show again with the button changed.
        let mut last: Option<(PresenceUpdate, Instant)> = None;
        while let Ok(update) = rx.recv() {
            let update = match update {
                PresenceUpdate::Shutdown => {
                    if connected {
                        let _ = client.close();
                    }
                    return;
                }
                PresenceUpdate::ListenLink(link) => {
                    listen_link = link;
                    match &last {
                        Some((shown, since)) => moved_on(shown, since.elapsed()),
                        None => continue,
                    }
                }
                update => {
                    last = Some((update.clone(), Instant::now()));
                    update
                }
            };
            if !connected {
                connected = client.connect().is_ok();
                if !connected {
                    continue;
                }
            }
            let link = listen_link.as_deref();
            if apply(&mut client, &update, link).is_err() {
                let _ = client.close();
                connected = client.connect().is_ok() && apply(&mut client, &update, link).is_ok();
            }
        }
    });
    tx
}

/// A status as it is `elapsed` after it was set: a playing song has moved on.
fn moved_on(update: &PresenceUpdate, elapsed: Duration) -> PresenceUpdate {
    let mut update = update.clone();
    if let PresenceUpdate::Playing { position_ms, duration_ms, .. } = &mut update {
        let elapsed = u32::try_from(elapsed.as_millis()).unwrap_or(u32::MAX);
        *position_ms = position_ms.saturating_add(elapsed).min(*duration_ms);
    }
    update
}

fn discord_text(text: &str) -> String {
    let mut text: String = text.chars().take(128).collect();
    while text.chars().count() < 2 {
        text.push('\u{2800}');
    }
    text
}

/// Discord fetches covers itself, so a local file's cover is uploaded and the link
/// kept beside it until it expires. Blocks while uploading, which only this thread can afford.
fn public_cover(url: &str) -> Option<String> {
    if url.starts_with("https://") {
        return Some(url.to_string());
    }
    let file = crate::local_files::cover_file(url)?;
    let link_file = file.with_extension("link");
    let saved_recently = fs::metadata(&link_file)
        .and_then(|meta| meta.modified())
        .is_ok_and(|saved| saved.elapsed().is_ok_and(|age| age < LINK_LIFETIME));
    let saved_link = fs::read_to_string(&link_file).ok().filter(|link| link.starts_with(COVER_LINK_PREFIX));
    if let Some(link) = saved_link.filter(|_| saved_recently) {
        return Some(link);
    }
    {
        let mut failed = FAILED_UPLOADS.lock().ok()?;
        failed.retain(|(_, at)| at.elapsed() < UPLOAD_RETRY);
        if failed.iter().any(|(path, _)| *path == file) {
            return None;
        }
    }
    match upload_cover(&file) {
        Ok(link) => {
            let _ = fs::write(&link_file, &link);
            Some(link)
        }
        Err(e) => {
            eprintln!("Couldn't upload a local cover for Discord: {e}");
            if let Ok(mut failed) = FAILED_UPLOADS.lock() {
                failed.push((file, Instant::now()));
            }
            None
        }
    }
}

fn upload_cover(file: &Path) -> Result<String, String> {
    let bytes = fs::read(file).map_err(|e| e.to_string())?;
    let name = file
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let content_type = crate::local_files::cover_content_type(&name).unwrap_or("application/octet-stream");
    tauri::async_runtime::block_on(async move {
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(name)
            .mime_str(content_type)
            .map_err(|e| e.to_string())?;
        let form = reqwest::multipart::Form::new()
            .text("reqtype", "fileupload")
            .text("time", UPLOAD_KEPT)
            .part("fileToUpload", part);
        let client = crate::http::client_builder()
            .user_agent(concat!(
                "SpotiLarp/",
                env!("CARGO_PKG_VERSION"),
                " (https://github.com/Exly78/SpotiLarp)"
            ))
            .build()
            .map_err(|e| e.to_string())?;
        let response = client
            .post(COVER_HOST)
            .multipart(form)
            .send()
            .await
            .and_then(|response| response.error_for_status())
            .map_err(|e| e.to_string())?;
        let link = response.text().await.map_err(|e| e.to_string())?;
        let link = link.trim();
        if link.starts_with(COVER_LINK_PREFIX) {
            Ok(link.to_string())
        } else {
            Err(link.to_string())
        }
    })
}

fn cover_assets<'a>(album: &'a Option<String>, cover_url: &'a Option<String>) -> activity::Assets<'a> {
    let mut assets = activity::Assets::new();
    if let Some(album) = album {
        assets = assets.large_text(album);
    }
    if let Some(url) = cover_url {
        assets = assets.large_image(url);
    }
    assets
}

/// A Spotify song (one with a link) gets a button to listen: with a
/// listen-along session open it opens SpotiLarp instead of Spotify, and the
/// page it lands on (docs/listen) still offers the song on Spotify to anyone
/// without SpotiLarp. "Get SpotiLarp" comes second, always.
fn listen_buttons<'a>(track_url: &'a Option<String>, listen_link: Option<&'a str>) -> Vec<activity::Button<'a>> {
    let listen = track_url.as_deref().map(|track_url| match listen_link {
        Some(link) => activity::Button::new("Listen along on SpotiLarp", listen_along_url(link, track_url)),
        None => activity::Button::new("Listen on Spotify", track_url),
    });
    listen
        .into_iter()
        .chain(std::iter::once(activity::Button::new("Get SpotiLarp", DOWNLOAD_URL)))
        .collect()
}

/// The listen-along link with the song after the code: "...#<code>/track/<id>".
fn listen_along_url(link: &str, track_url: &str) -> String {
    match track_url.strip_prefix(SPOTIFY_WEB) {
        Some(song) => format!("{link}/{song}"),
        None => link.to_string(),
    }
}

fn apply(
    client: &mut DiscordIpcClient,
    update: &PresenceUpdate,
    listen_link: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    match update {
        PresenceUpdate::Cleared | PresenceUpdate::ListenLink(_) | PresenceUpdate::Shutdown => client.clear_activity()?,
        PresenceUpdate::Playing {
            name,
            artists,
            album,
            cover_url,
            position_ms,
            duration_ms,
            track_url,
        } => {
            let album = (!album.trim().is_empty()).then(|| discord_text(album));
            let cover = cover_url.as_deref().and_then(public_cover);
            let assets = cover_assets(&album, &cover);
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let start = now - (*position_ms as i64 / 1000);
            let end = start + (*duration_ms as i64 / 1000);
            let name = discord_text(name);
            let artists = discord_text(artists);
            let payload = activity::Activity::new()
                .activity_type(activity::ActivityType::Listening)
                .details(&name)
                .state(&artists)
                .assets(assets)
                .timestamps(activity::Timestamps::new().start(start).end(end))
                .buttons(listen_buttons(track_url, listen_link));
            client.set_activity(payload)?
        }
        PresenceUpdate::Paused {
            name,
            artists,
            album,
            cover_url,
            track_url,
        } => {
            let album = (!album.trim().is_empty()).then(|| discord_text(album));
            let cover = cover_url.as_deref().and_then(public_cover);
            let assets = cover_assets(&album, &cover);
            let name = discord_text(name);
            let state = discord_text(&format!("Paused - {artists}"));
            let payload = activity::Activity::new()
                .activity_type(activity::ActivityType::Listening)
                .details(&name)
                .state(&state)
                .assets(assets)
                .buttons(listen_buttons(track_url, listen_link));
            client.set_activity(payload)?
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listen_along_links_carry_the_song() {
        let link = "https://exly78.github.io/SpotiLarp/listen/#abc";
        assert_eq!(
            listen_along_url(link, "https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC"),
            "https://exly78.github.io/SpotiLarp/listen/#abc/track/4uLU6hMCjMI75M1A2tKUQC"
        );
        assert_eq!(listen_along_url(link, "https://example.com/x"), link);
    }
}
