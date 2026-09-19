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
    },
    Paused {
        name: String,
        artists: String,
        album: String,
        cover_url: Option<String>,
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
            let result = apply(&mut client, update);
            if result.is_err() {
                connected = false;
            }
        }
    });
    tx
}

fn apply(client: &mut DiscordIpcClient, update: PresenceUpdate) -> Result<(), Box<dyn std::error::Error>> {
    match update {
        PresenceUpdate::Cleared | PresenceUpdate::Shutdown => client.clear_activity()?,
        PresenceUpdate::Playing {
            name,
            artists,
            album,
            cover_url,
            position_ms,
            duration_ms,
        } => {
            let mut assets = activity::Assets::new().large_text(&album);
            if let Some(url) = &cover_url {
                assets = assets.large_image(url);
            }
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let start = now - (position_ms as i64 / 1000);
            let end = start + (duration_ms as i64 / 1000);
            let payload = activity::Activity::new()
                .activity_type(activity::ActivityType::Listening)
                .details(&name)
                .state(&artists)
                .assets(assets)
                .timestamps(activity::Timestamps::new().start(start).end(end));
            client.set_activity(payload)?
        }
        PresenceUpdate::Paused {
            name,
            artists,
            album,
            cover_url,
        } => {
            let mut assets = activity::Assets::new().large_text(&album);
            if let Some(url) = &cover_url {
                assets = assets.large_image(url);
            }
            let state = format!("Paused - {artists}");
            let payload = activity::Activity::new()
                .details(&name)
                .state(&state)
                .assets(assets);
            client.set_activity(payload)?
        }
    }
    Ok(())
}
