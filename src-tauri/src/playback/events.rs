use librespot_metadata::audio::UniqueFields;
use librespot_playback::player::{Player, PlayerEvent};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::discord::PresenceUpdate;
use crate::media::MediaUpdate;
use crate::state::AppState;

#[derive(Clone, Serialize)]
pub struct ArtistRef {
    pub id: Option<String>,
    pub name: String,
}

#[derive(Clone, Serialize)]
#[serde(tag = "type")]
pub enum PlaybackEvent {
    Playing { position_ms: u32 },
    Paused { position_ms: u32 },
    Stopped,
    EndOfTrack,
    Unavailable,
    PreloadNext,
    PositionChanged { position_ms: u32 },
    Seeked { position_ms: u32 },
    VolumeChanged { volume: u16 },
    TrackChanged {
        name: String,
        artists: String,
        
        primary_artist_id: Option<String>,
        artist_list: Vec<ArtistRef>,
        
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
        PlayerEvent::Unavailable { .. } => Some(PlaybackEvent::Unavailable),
        PlayerEvent::TimeToPreloadNextTrack { .. } => Some(PlaybackEvent::PreloadNext),
        PlayerEvent::PositionChanged { position_ms, .. } => {
            Some(PlaybackEvent::PositionChanged { position_ms })
        }
        PlayerEvent::Seeked { position_ms, .. } => Some(PlaybackEvent::Seeked { position_ms }),
        PlayerEvent::VolumeChanged { volume } => Some(PlaybackEvent::VolumeChanged { volume }),
        PlayerEvent::TrackChanged { audio_item } => {
            let (artists, primary_artist_id, artist_list, album, track_id) = match &audio_item.unique_fields {
                UniqueFields::Track { artists, album, .. } => (
                    artists
                        .iter()
                        .map(|a| a.name.clone())
                        .collect::<Vec<_>>()
                        .join(", "),
                    artists.first().and_then(|a| a.id.to_id().ok()),
                    artists
                        .iter()
                        .map(|a| ArtistRef {
                            id: a.id.to_id().ok(),
                            name: a.name.clone(),
                        })
                        .collect(),
                    album.clone(),
                    audio_item.track_id.to_id().ok(),
                ),
                UniqueFields::Local {
                    artists, album, ..
                } => (
                    artists.clone().unwrap_or_default(),
                    None,
                    Vec::new(),
                    album.clone().unwrap_or_default(),
                    None,
                ),
                UniqueFields::Episode { show_name, .. } => {
                    (String::new(), None, Vec::new(), show_name.clone(), None)
                }
            };
            let cover_url = audio_item.covers.first().map(|c| c.url.clone());
            Some(PlaybackEvent::TrackChanged {
                name: audio_item.name.clone(),
                artists,
                primary_artist_id,
                artist_list,
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
    track_url: Option<String>,
}

fn announce_track(app: &AppHandle, name: &str, artists: &str) {
    crate::tray::set_tooltip(app, &format!("{name} - {artists}"));
    let focused = app
        .get_webview_window("main")
        .and_then(|window| window.is_focused().ok())
        .unwrap_or(false);
    if !focused && app.state::<AppState>().settings().notifications {
        let _ = app.notification().builder().title(name).body(artists).show();
    }
}

async fn update_discord_presence(app: &AppHandle, update: PresenceUpdate) {
    let state = app.state::<AppState>();
    let discord = state.discord.lock().await;
    if let Some(sender) = discord.as_ref() {
        let _ = sender.send(update);
    }
}

fn update_media(app: &AppHandle, update: MediaUpdate) {
    if let Some(sender) = app.state::<AppState>().media.get() {
        let _ = sender.send(update);
    }
}

pub fn spawn_forwarder(app: AppHandle, player: &Player) {
    let mut channel = player.get_player_event_channel();
    tauri::async_runtime::spawn(async move {
        let mut current_track: Option<CurrentTrack> = None;
        let mut is_playing = false;
        while let Some(event) = channel.recv().await {
            if let Some(mapped) = map_event(event) {
                match &mapped {
                    PlaybackEvent::TrackChanged {
                        name,
                        artists,
                        album,
                        duration_ms,
                        cover_url,
                        track_id,
                        ..
                    } => {
                        announce_track(&app, name, artists);
                        current_track = Some(CurrentTrack {
                            name: name.clone(),
                            artists: artists.clone(),
                            album: album.clone(),
                            cover_url: cover_url.clone(),
                            duration_ms: *duration_ms,
                            track_url: track_id
                                .as_ref()
                                .map(|id| format!("https://open.spotify.com/track/{id}")),
                        });
                    }
                    PlaybackEvent::Playing { position_ms } | PlaybackEvent::Seeked { position_ms } => {
                        if matches!(mapped, PlaybackEvent::Playing { .. }) {
                            is_playing = true;
                        }
                        if let Some(t) = current_track.as_ref().filter(|_| is_playing) {
                            update_discord_presence(
                                &app,
                                PresenceUpdate::Playing {
                                    name: t.name.clone(),
                                    artists: t.artists.clone(),
                                    album: t.album.clone(),
                                    cover_url: t.cover_url.clone(),
                                    position_ms: *position_ms,
                                    duration_ms: t.duration_ms,
                                    track_url: t.track_url.clone(),
                                },
                            )
                            .await;
                        }
                    }
                    PlaybackEvent::Paused { .. } => {
                        is_playing = false;
                        if let Some(t) = &current_track {
                            update_discord_presence(
                                &app,
                                PresenceUpdate::Paused {
                                    name: t.name.clone(),
                                    artists: t.artists.clone(),
                                    album: t.album.clone(),
                                    cover_url: t.cover_url.clone(),
                                    track_url: t.track_url.clone(),
                                },
                            )
                            .await;
                        }
                    }
                    PlaybackEvent::Stopped | PlaybackEvent::EndOfTrack | PlaybackEvent::Unavailable => {
                        is_playing = false;
                        update_discord_presence(&app, PresenceUpdate::Cleared).await;
                    }
                    _ => {}
                }

                let media_update = match &mapped {
                    PlaybackEvent::TrackChanged {
                        name,
                        artists,
                        album,
                        duration_ms,
                        cover_url,
                        ..
                    } => Some(MediaUpdate::Metadata {
                        title: name.clone(),
                        artist: artists.clone(),
                        album: album.clone(),
                        cover_url: cover_url.clone(),
                        duration_ms: *duration_ms,
                    }),
                    PlaybackEvent::Playing { position_ms } => Some(MediaUpdate::Playing {
                        position_ms: *position_ms,
                    }),
                    PlaybackEvent::PositionChanged { position_ms } | PlaybackEvent::Seeked { position_ms } => {
                        Some(if is_playing {
                            MediaUpdate::Playing { position_ms: *position_ms }
                        } else {
                            MediaUpdate::Paused { position_ms: *position_ms }
                        })
                    }
                    PlaybackEvent::Paused { position_ms } => Some(MediaUpdate::Paused {
                        position_ms: *position_ms,
                    }),
                    PlaybackEvent::EndOfTrack => Some(MediaUpdate::Paused {
                        position_ms: current_track.as_ref().map_or(0, |t| t.duration_ms),
                    }),
                    PlaybackEvent::Stopped | PlaybackEvent::Unavailable => Some(MediaUpdate::Stopped),
                    _ => None,
                };
                if let Some(update) = media_update {
                    update_media(&app, update);
                }

                let _ = app.emit("player-event", mapped);
            }
        }
    });
}
