use std::sync::atomic::{AtomicBool, AtomicU16};
use std::sync::mpsc::Sender as StdSender;
use std::sync::{Arc, OnceLock};

use librespot_core::session::Session;
use librespot_playback::mixer::Mixer;
use librespot_playback::player::Player;
use tauri::PhysicalSize;
use tokio::sync::Mutex;

use crate::config::Settings;
use crate::discord::PresenceUpdate;
use crate::local_files::LocalIndex;
use crate::media::MediaUpdate;
use crate::playback::events::EventSender;
use crate::playback::local::LocalPlayer;
use crate::remote::RemoteCommand;
use crate::spotify_api::client::SpotifyClient;

pub struct PlaybackHandle {
    pub session: Session,
    pub player: Arc<Player>,
    pub mixer: Arc<dyn Mixer>,
}

pub struct MiniRestore {
    pub size: PhysicalSize<u32>,
    pub maximized: bool,
}

pub struct AppState {
    pub spotify: Mutex<SpotifyClient>,
    pub playback: Mutex<Option<PlaybackHandle>>,
    pub playback_connect: Mutex<()>,
    pub volume: AtomicU16,
    pub media: OnceLock<StdSender<MediaUpdate>>,
    pub settings: std::sync::Mutex<Settings>,
    pub mini_restore: std::sync::Mutex<Option<MiniRestore>>,

    pub discord: Mutex<Option<StdSender<PresenceUpdate>>>,
    pub local_files: Mutex<Option<LocalIndex>>,
    pub local_player: OnceLock<LocalPlayer>,
    /// Whether the local player (rather than librespot) owns playback.
    pub local_active: AtomicBool,
    pub playback_events: OnceLock<EventSender>,
    /// Commands this launch was started with (`spotilarp <link>`), until the UI takes them.
    pub pending_commands: std::sync::Mutex<Vec<RemoteCommand>>,
    /// Whether the main window is minimized or in the tray (background.rs).
    pub background: AtomicBool,
    /// Which run of the page's clock is current (background.rs).
    pub clock_generation: std::sync::atomic::AtomicU64,
}

impl AppState {
    pub fn settings(&self) -> Settings {
        self.settings.lock().map(|s| s.clone()).unwrap_or_default()
    }
}
