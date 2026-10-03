use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("SpotiLarp")
        .join("config.json")
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioQuality {
    Low,
    #[default]
    Normal,
    High,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub audio_quality: AudioQuality,
    pub normalize_volume: bool,
    pub cache_limit_mb: u64,
    pub close_to_tray: bool,
    pub notifications: bool,
    pub eq_enabled: bool,
    pub eq_gains: Vec<f32>,
    pub eq_preset: String,
    pub output_device: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            audio_quality: AudioQuality::Normal,
            normalize_volume: false,
            cache_limit_mb: 1024,
            close_to_tray: false,
            notifications: false,
            eq_enabled: false,
            eq_gains: vec![0.0; 10],
            eq_preset: "flat".to_string(),
            output_device: None,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

#[derive(Default, Serialize, Deserialize)]
struct Config {
    spotify_client_id: Option<String>,

    #[serde(default)]
    discord_client_id: Option<String>,
    #[serde(default)]
    settings: Settings,
    #[serde(default)]
    window: Option<WindowGeometry>,
}

static SAVE_LOCK: Mutex<()> = Mutex::new(());

enum ReadError {
    Corrupt,
    Io(String),
}

fn read_config() -> Result<Config, ReadError> {
    match std::fs::read_to_string(config_path()) {
        Ok(text) => serde_json::from_str(&text).map_err(|_| ReadError::Corrupt),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(ReadError::Io(e.to_string())),
    }
}

fn load() -> Config {
    read_config().unwrap_or_default()
}

fn update(change: impl FnOnce(&mut Config)) -> Result<(), String> {
    let _guard = SAVE_LOCK.lock().map_err(|e| e.to_string())?;
    let mut config = match read_config() {
        Ok(config) => config,
        Err(ReadError::Corrupt) => {
            let path = config_path();
            let _ = std::fs::rename(&path, path.with_extension("json.broken"));
            Config::default()
        }
        Err(ReadError::Io(e)) => return Err(format!("Couldn't read the config file: {e}")),
    };
    change(&mut config);
    write_config(&config)
}

fn write_config(config: &Config) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, &path).map_err(|e| e.to_string())
}

pub fn load_client_id() -> Option<String> {
    load().spotify_client_id.filter(|id| !id.is_empty())
}

pub fn save_client_id(client_id: &str) -> Result<(), String> {
    update(|config| config.spotify_client_id = Some(client_id.to_string()))
}

pub fn load_discord_client_id() -> Option<String> {
    load().discord_client_id.filter(|id| !id.is_empty())
}

pub fn save_discord_client_id(client_id: &str) -> Result<(), String> {
    update(|config| config.discord_client_id = Some(client_id.to_string()))
}

pub fn clear_discord_client_id() -> Result<(), String> {
    update(|config| config.discord_client_id = None)
}

pub fn load_settings() -> Settings {
    load().settings
}

pub fn load_window_geometry() -> Option<WindowGeometry> {
    load().window
}

pub fn save_window_geometry(geometry: WindowGeometry) -> Result<(), String> {
    update(|config| config.window = Some(geometry))
}

pub fn save_settings(settings: &Settings) -> Result<(), String> {
    update(|config| config.settings = settings.clone())
}
