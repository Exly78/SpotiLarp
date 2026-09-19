use std::sync::mpsc::Sender as StdSender;
use std::sync::Arc;

use librespot_core::session::Session;
use librespot_playback::mixer::Mixer;
use librespot_playback::player::Player;
use tokio::sync::Mutex;

use crate::discord::PresenceUpdate;
use crate::spotify_api::client::SpotifyClient;

pub struct PlaybackHandle {
    pub session: Session,
    pub player: Arc<Player>,
    pub mixer: Arc<dyn Mixer>,
}

pub struct AppState {
    pub spotify: Mutex<SpotifyClient>,
    pub playback: Mutex<Option<PlaybackHandle>>,
    
    pub discord: Mutex<Option<StdSender<PresenceUpdate>>>,
}
