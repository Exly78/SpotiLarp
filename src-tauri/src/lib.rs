mod auth;
mod commands;
mod config;
mod discord;
mod lyrics;
mod playback;
mod spotify_api;
mod state;

use spotify_api::client::SpotifyClient;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    
    let client_id = config::load_client_id();

    let discord_sender = config::load_discord_client_id().map(discord::spawn);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState {
            spotify: tokio::sync::Mutex::new(SpotifyClient::new(client_id)),
            playback: tokio::sync::Mutex::new(None),
            discord: tokio::sync::Mutex::new(discord_sender),
        })
        .invoke_handler(tauri::generate_handler![
            commands::has_client_id,
            commands::set_client_id,
            commands::login,
            commands::login_status,
            commands::restore_session,
            commands::logout,
            commands::has_connect_session,
            commands::connect_playback,
            commands::play_track,
            commands::search,
            commands::get_playlists,
            commands::get_playlist_tracks,
            commands::get_liked_songs,
            commands::is_track_liked,
            commands::are_tracks_liked,
            commands::like_track,
            commands::unlike_track,
            commands::get_artist,
            commands::is_following_artist,
            commands::follow_artist,
            commands::unfollow_artist,
            commands::pause,
            commands::resume,
            commands::seek,
            commands::set_volume,
            commands::get_lyrics,
            commands::has_discord_client_id,
            commands::set_discord_client_id,
            commands::clear_discord_client_id
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
