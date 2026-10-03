mod auth;
mod commands;
mod config;
mod discord;
mod http;
mod lyrics;
mod media;
mod playback;
mod spotify_api;
mod state;
mod tray;
mod window_state;

use spotify_api::client::SpotifyClient;
use state::AppState;
use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

const MINIMIZED_ARG: &str = "--minimized";

#[cfg(target_os = "windows")]
fn main_window_handle(app: &tauri::App) -> Option<isize> {
    let window = app.get_webview_window("main")?;
    window.hwnd().ok().map(|hwnd| hwnd.0 as isize)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {

    let client_id = config::load_client_id();

    let discord_sender = config::load_discord_client_id().map(discord::spawn);
    let settings = config::load_settings();
    commands::apply_audio_settings(&settings);

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![MINIMIZED_ARG]),
        ))
        .manage(AppState {
            spotify: tokio::sync::Mutex::new(SpotifyClient::new(client_id)),
            playback: tokio::sync::Mutex::new(None),
            playback_connect: tokio::sync::Mutex::new(()),
            volume: std::sync::atomic::AtomicU16::new(u16::MAX / 2),
            media: std::sync::OnceLock::new(),
            settings: std::sync::Mutex::new(settings),
            mini_restore: std::sync::Mutex::new(None),
            discord: tokio::sync::Mutex::new(discord_sender),
        })
        .setup(|app| {
            #[cfg(target_os = "windows")]
            let hwnd = main_window_handle(app);
            #[cfg(target_os = "windows")]
            let media_supported = hwnd.is_some();
            #[cfg(not(target_os = "windows"))]
            let (hwnd, media_supported) = (None, true);

            if media_supported {
                let sender = media::spawn(app.handle().clone(), hwnd);
                let _ = app.state::<AppState>().media.set(sender);
            }

            let tray_created = tray::create(app).is_ok();

            if let Some(window) = app.get_webview_window("main") {
                window_state::restore(&window);
                let minimized_start = std::env::args().any(|arg| arg == MINIMIZED_ARG);
                let to_tray = tray_created && app.state::<AppState>().settings().close_to_tray;
                if !(minimized_start && to_tray) {
                    let _ = window.show();
                }
                if minimized_start && !to_tray {
                    let _ = window.minimize();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if let Some(webview_window) = window.app_handle().get_webview_window(window.label()) {
                    window_state::save(&webview_window);
                }
                let close_to_tray = window.state::<AppState>().settings().close_to_tray;
                if window.label() == "main" && close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::has_client_id,
            commands::set_client_id,
            commands::login,
            commands::cancel_login,
            commands::login_status,
            commands::restore_session,
            commands::logout,
            commands::has_connect_session,
            commands::connect_playback,
            commands::play_track,
            commands::preload_track,
            commands::get_autoplay_tracks,
            commands::search,
            commands::search_all,
            commands::search_tracks_page,
            commands::get_artist_albums,
            commands::get_artist_popular_tracks,
            commands::get_album,
            commands::get_recently_played,
            commands::get_top_tracks,
            commands::get_top_artists,
            commands::add_to_playlist,
            commands::remove_from_playlist,
            commands::create_playlist,
            commands::get_user_id,
            commands::missing_scopes,
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
            commands::clear_discord_client_id,
            commands::get_settings,
            commands::set_settings,
            commands::get_cache_size,
            commands::clear_cache,
            commands::get_autostart,
            commands::set_autostart,
            commands::set_mini_mode,
            commands::rename_playlist,
            commands::remove_playlist,
            commands::move_playlist_track,
            commands::list_output_devices,
            commands::preview_equalizer
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
