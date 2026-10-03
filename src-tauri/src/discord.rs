use std::sync::mpsc::{self, Sender};
use std::time::{SystemTime, UNIX_EPOCH};

use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};

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

    Shutdown,
}

pub fn spawn(client_id: String) -> Sender<PresenceUpdate> {
    let (tx, rx) = mpsc::channel::<PresenceUpdate>();
    std::thread::spawn(move || {
        let mut client = DiscordIpcClient::new(&client_id);
        let mut connected = client.connect().is_ok();
        while let Ok(update) = rx.recv() {
            if matches!(update, PresenceUpdate::Shutdown) {
                if connected {
                    let _ = client.close();
                }
                return;
            }
            if !connected {
                connected = client.connect().is_ok();
                if !connected {
                    continue;
                }
            }
            if apply(&mut client, &update).is_err() {
                let _ = client.close();
                connected = client.connect().is_ok() && apply(&mut client, &update).is_ok();
            }
        }
    });
    tx
}

fn discord_text(text: &str) -> String {
    let mut text: String = text.chars().take(128).collect();
    while text.chars().count() < 2 {
        text.push('\u{2800}');
    }
    text
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

fn listen_buttons(track_url: &Option<String>) -> Vec<activity::Button<'_>> {
    track_url
        .iter()
        .map(|url| activity::Button::new("Listen on Spotify", url))
        .collect()
}

fn apply(client: &mut DiscordIpcClient, update: &PresenceUpdate) -> Result<(), Box<dyn std::error::Error>> {
    match update {
        PresenceUpdate::Cleared | PresenceUpdate::Shutdown => client.clear_activity()?,
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
            let assets = cover_assets(&album, cover_url);
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
                .buttons(listen_buttons(track_url));
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
            let assets = cover_assets(&album, cover_url);
            let name = discord_text(name);
            let state = discord_text(&format!("Paused - {artists}"));
            let payload = activity::Activity::new()
                .activity_type(activity::ActivityType::Listening)
                .details(&name)
                .state(&state)
                .assets(assets)
                .buttons(listen_buttons(track_url));
            client.set_activity(payload)?
        }
    }
    Ok(())
}
