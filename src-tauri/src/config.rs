use std::path::PathBuf;

use serde::{Deserialize, Serialize};

fn config_path() -> PathBuf {
    let base = dirs::config_dir().expect("could not determine OS config directory");
    base.join("SpotiLarp")
        .join("config.json")
}

#[derive(Default, Serialize, Deserialize)]
struct Config {
    spotify_client_id: Option<String>,
    
    #[serde(default)]
    discord_client_id: Option<String>,
}

fn read_config() -> Config {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_config(config: &Config) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| e.to_string())
}

pub fn load_client_id() -> Option<String> {
    read_config().spotify_client_id.filter(|id| !id.is_empty())
}

pub fn save_client_id(client_id: &str) -> Result<(), String> {
    let mut config = read_config();
    config.spotify_client_id = Some(client_id.to_string());
    write_config(&config)
}

pub fn load_discord_client_id() -> Option<String> {
    read_config().discord_client_id.filter(|id| !id.is_empty())
}

pub fn save_discord_client_id(client_id: &str) -> Result<(), String> {
    let mut config = read_config();
    config.discord_client_id = Some(client_id.to_string());
    write_config(&config)
}

pub fn clear_discord_client_id() -> Result<(), String> {
    let mut config = read_config();
    config.discord_client_id = None;
    write_config(&config)
}
