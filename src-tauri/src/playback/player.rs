use std::sync::Arc;
use std::time::Duration;

use librespot_core::session::Session;
use librespot_core::SpotifyUri;
use librespot_playback::config::{AudioFormat, PlayerConfig};
use librespot_playback::mixer::{self, Mixer, MixerConfig};
use librespot_playback::player::Player;

use super::sink;

pub struct PlayerHandle {
    pub player: Arc<Player>,
    pub mixer: Arc<dyn Mixer>,
}

pub fn start(session: Session) -> PlayerHandle {
    let format = AudioFormat::default();

    let mixer_fn = mixer::find(None).expect("no mixer compiled in");
    let mixer = mixer_fn(MixerConfig::default()).expect("failed to open mixer");
    let volume_getter = mixer.get_soft_volume();

    let config = PlayerConfig {
        
        position_update_interval: Some(Duration::from_millis(1000)),
        ..PlayerConfig::default()
    };

    let player = Player::new(config, session, volume_getter, move || sink::open(None, format));

    PlayerHandle { player, mixer }
}

pub fn play_uri(player: &Player, uri: &str) -> Result<(), String> {
    let spotify_uri = SpotifyUri::from_uri(uri).map_err(|e| e.to_string())?;
    player.load(spotify_uri, true, 0);
    Ok(())
}
