use std::ffi::c_void;
use std::sync::mpsc::{self, Sender};
use std::time::Duration;

use serde::Serialize;
use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
    SeekDirection,
};
use tauri::{AppHandle, Emitter};

const SEEK_STEP_MS: i64 = 10_000;

pub enum MediaUpdate {
    Metadata {
        title: String,
        artist: String,
        album: String,
        cover_url: Option<String>,
        duration_ms: u32,
    },
    Playing { position_ms: u32 },
    Paused { position_ms: u32 },
    Stopped,
}

#[derive(Clone, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum MediaCommand {
    Play,
    Pause,
    Toggle,
    Next,
    Previous,
    SeekBy { offset_ms: i64 },
    SetPosition { position_ms: u64 },
}

fn map_event(event: MediaControlEvent) -> Option<MediaCommand> {
    let signed = |direction: SeekDirection, ms: i64| match direction {
        SeekDirection::Forward => ms,
        SeekDirection::Backward => -ms,
    };
    Some(match event {
        MediaControlEvent::Play => MediaCommand::Play,
        MediaControlEvent::Pause | MediaControlEvent::Stop => MediaCommand::Pause,
        MediaControlEvent::Toggle => MediaCommand::Toggle,
        MediaControlEvent::Next => MediaCommand::Next,
        MediaControlEvent::Previous => MediaCommand::Previous,
        MediaControlEvent::Seek(direction) => MediaCommand::SeekBy {
            offset_ms: signed(direction, SEEK_STEP_MS),
        },
        MediaControlEvent::SeekBy(direction, amount) => MediaCommand::SeekBy {
            offset_ms: signed(direction, amount.as_millis() as i64),
        },
        MediaControlEvent::SetPosition(MediaPosition(position)) => MediaCommand::SetPosition {
            position_ms: position.as_millis() as u64,
        },
        _ => return None,
    })
}

fn position(ms: u32) -> Option<MediaPosition> {
    Some(MediaPosition(Duration::from_millis(ms as u64)))
}

pub fn spawn(app: AppHandle, hwnd: Option<isize>) -> Sender<MediaUpdate> {
    let (tx, rx) = mpsc::channel::<MediaUpdate>();
    std::thread::spawn(move || {
        let config = PlatformConfig {
            display_name: "SpotiLarp",
            dbus_name: "spotilarp",
            hwnd: hwnd.map(|handle| handle as *mut c_void),
        };
        let Ok(mut controls) = MediaControls::new(config) else {
            return;
        };
        let attached = controls.attach(move |event| {
            if let Some(command) = map_event(event) {
                let _ = app.emit("media-control", command);
            }
        });
        if attached.is_err() {
            return;
        }

        while let Ok(update) = rx.recv() {
            let _ = match update {
                MediaUpdate::Metadata {
                    title,
                    artist,
                    album,
                    cover_url,
                    duration_ms,
                } => controls.set_metadata(MediaMetadata {
                    title: Some(&title),
                    artist: Some(&artist),
                    album: Some(&album),
                    cover_url: cover_url.as_deref(),
                    duration: Some(Duration::from_millis(duration_ms as u64)),
                }),
                MediaUpdate::Playing { position_ms } => controls.set_playback(MediaPlayback::Playing {
                    progress: position(position_ms),
                }),
                MediaUpdate::Paused { position_ms } => controls.set_playback(MediaPlayback::Paused {
                    progress: position(position_ms),
                }),
                MediaUpdate::Stopped => controls.set_playback(MediaPlayback::Stopped),
            };
        }
    });
    tx
}
