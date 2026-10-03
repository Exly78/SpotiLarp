use std::sync::Arc;
use std::time::Duration;

use librespot_core::session::Session;
use librespot_core::SpotifyUri;
use librespot_playback::config::{AudioFormat, Bitrate, PlayerConfig};
use librespot_playback::mixer::{self, Mixer, MixerConfig};
use librespot_playback::player::Player;

use super::sink;
use crate::config::{AudioQuality, Settings};

pub struct PlayerHandle {
    pub player: Arc<Player>,
    pub mixer: Arc<dyn Mixer>,
}

pub fn start(session: Session, settings: &Settings) -> PlayerHandle {
    let format = AudioFormat::default();

    let mixer_fn = mixer::find(None).expect("no mixer compiled in");
    let mixer = mixer_fn(MixerConfig::default()).expect("failed to open mixer");
    let volume_getter = mixer.get_soft_volume();

    let config = PlayerConfig {
        bitrate: match settings.audio_quality {
            AudioQuality::Low => Bitrate::Bitrate96,
            AudioQuality::Normal => Bitrate::Bitrate160,
            AudioQuality::High => Bitrate::Bitrate320,
        },
        normalisation: settings.normalize_volume,

        position_update_interval: Some(Duration::from_millis(1000)),
        ..PlayerConfig::default()
    };

    let player = Player::new(config, session, volume_getter, move || sink::open(None, format));

    PlayerHandle { player, mixer }
}

pub fn play_uri(player: &Player, uri: &str, position_ms: u32) -> Result<(), String> {
    let spotify_uri = SpotifyUri::from_uri(uri).map_err(|e| e.to_string())?;
    player.load(spotify_uri, true, position_ms);
    Ok(())
}

pub fn preload_uri(player: &Player, uri: &str) -> Result<(), String> {
    let spotify_uri = SpotifyUri::from_uri(uri).map_err(|e| e.to_string())?;
    player.preload(spotify_uri);
    Ok(())
}
