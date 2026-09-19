use librespot_metadata::audio::UniqueFields;
use librespot_playback::player::{Player, PlayerEvent};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::discord::PresenceUpdate;
use crate::state::AppState;

#[derive(Clone, Serialize)]
#[serde(tag = "type")]
pub enum PlaybackEvent {
    Playing { position_ms: u32 },
    Paused { position_ms: u32 },
    Stopped,
    EndOfTrack,
    PositionChanged { position_ms: u32 },
    Seeked { position_ms: u32 },
    VolumeChanged { volume: u16 },
    TrackChanged {
        name: String,
        artists: String,
        
        primary_artist_id: Option<String>,
        
        track_id: Option<String>,
        album: String,
        duration_ms: u32,
        cover_url: Option<String>,
    },
}

fn map_event(event: PlayerEvent) -> Option<PlaybackEvent> {
    match event {
        PlayerEvent::Playing { position_ms, .. } => Some(PlaybackEvent::Playing { position_ms }),
        PlayerEvent::Paused { position_ms, .. } => Some(PlaybackEvent::Paused { position_ms }),
        PlayerEvent::Stopped { .. } => Some(PlaybackEvent::Stopped),
        PlayerEvent::EndOfTrack { .. } => Some(PlaybackEvent::EndOfTrack),
        PlayerEvent::PositionChanged { position_ms, .. } => {
            Some(PlaybackEvent::PositionChanged { position_ms })
        }
        PlayerEvent::Seeked { position_ms, .. } => Some(PlaybackEvent::Seeked { position_ms }),
        PlayerEvent::VolumeChanged { volume } => Some(PlaybackEvent::VolumeChanged { volume }),
        PlayerEvent::TrackChanged { audio_item } => {
            let (artists, primary_artist_id, album, track_id) = match &audio_item.unique_fields {
                UniqueFields::Track { artists, album, .. } => (
                    artists
                        .iter()
                        .map(|a| a.name.clone())
                        .collect::<Vec<_>>()
                        .join(", "),
                    artists.first().and_then(|a| a.id.to_id().ok()),
                    album.clone(),
                    audio_item.track_id.to_id().ok(),
                ),
                UniqueFields::Local {
                    artists, album, ..
                } => (
                    artists.clone().unwrap_or_default(),
                    None,
                    album.clone().unwrap_or_default(),
                    None,
                ),
                UniqueFields::Episode { show_name, .. } => {
                    (String::new(), None, show_name.clone(), None)
                }
            };
            let cover_url = audio_item.covers.first().map(|c| c.url.clone());
            Some(PlaybackEvent::TrackChanged {
                name: audio_item.name.clone(),
                artists,
                primary_artist_id,
                track_id,
                album,
                duration_ms: audio_item.duration_ms,
                cover_url,
            })
        }
        _ => None,
    }
}

struct CurrentTrack {
    name: String,
    artists: String,
    album: String,
    cover_url: Option<String>,
    duration_ms: u32,
}

async fn update_discord_presence(app: &AppHandle, update: PresenceUpdate) {
    let state = app.state::<AppState>();
    let discord = state.discord.lock().await;
    if let Some(sender) = discord.as_ref() {
        let _ = sender.send(update);
    }
}

pub fn spawn_forwarder(app: AppHandle, player: &Player) {
    let mut channel = player.get_player_event_channel();
    tauri::async_runtime::spawn(async move {
        let mut current_track: Option<CurrentTrack> = None;
        while let Some(event) = channel.recv().await {
            if let Some(mapped) = map_event(event) {
                match &mapped {
                    PlaybackEvent::TrackChanged {
                        name,
                        artists,
                        album,
                        duration_ms,
                        cover_url,
                        ..
                    } => {
                        current_track = Some(CurrentTrack {
                            name: name.clone(),
                            artists: artists.clone(),
                            album: album.clone(),
                            cover_url: cover_url.clone(),
                            duration_ms: *duration_ms,
                        });
                    }
                    PlaybackEvent::Playing { position_ms } | PlaybackEvent::Seeked { position_ms } => {
                        if let Some(t) = &current_track {
                            update_discord_presence(
                                &app,
                                PresenceUpdate::Playing {
                                    name: t.name.clone(),
                                    artists: t.artists.clone(),
                                    album: t.album.clone(),
                                    cover_url: t.cover_url.clone(),
                                    position_ms: *position_ms,
                                    duration_ms: t.duration_ms,
                                },
                            )
                            .await;
                        }
                    }
                    PlaybackEvent::Paused { .. } => {
                        if let Some(t) = &current_track {
                            update_discord_presence(
                                &app,
                                PresenceUpdate::Paused {
                                    name: t.name.clone(),
                                    artists: t.artists.clone(),
                                    album: t.album.clone(),
                                    cover_url: t.cover_url.clone(),
                                },
                            )
                            .await;
                        }
                    }
                    PlaybackEvent::Stopped | PlaybackEvent::EndOfTrack => {
                        update_discord_presence(&app, PresenceUpdate::Cleared).await;
                    }
                    _ => {}
                }
                let _ = app.emit("player-event", mapped);
            }
        }
    });
}
