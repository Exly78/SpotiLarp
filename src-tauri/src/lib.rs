mod auth;
mod background;
mod commands;
mod config;
mod discord;
mod http;
mod local_files;
mod lyrics;
mod media;
mod playback;
mod remote;
mod spotify_api;
mod state;
mod themes;
#[cfg(target_os = "windows")]
mod thumbbar;
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
    let launch_command = remote::parse(&std::env::args().collect::<Vec<_>>());
    commands::apply_audio_settings(&settings);

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| match remote::parse(&args) {
            Some(command) => remote::dispatch(app, command),
            None => tray::show_main_window(app),
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
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
            local_files: tokio::sync::Mutex::new(None),
            local_player: std::sync::OnceLock::new(),
            local_active: std::sync::atomic::AtomicBool::new(false),
            playback_events: std::sync::OnceLock::new(),
            pending_commands: std::sync::Mutex::new(launch_command.into_iter().collect()),
            background: std::sync::atomic::AtomicBool::new(false),
            clock_generation: std::sync::atomic::AtomicU64::new(0),
        })
        .register_uri_scheme_protocol(local_files::COVER_SCHEME, |_ctx, request| {
            local_files::serve_cover(&request)
        })
        .register_uri_scheme_protocol(themes::THEME_SCHEME, |_ctx, request| themes::serve(&request))
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

            let state = app.state::<AppState>();
            let _ = state.playback_events.set(playback::events::spawn_hub(app.handle().clone()));
            let _ = state.local_player.set(playback::local::LocalPlayer::spawn(app.handle().clone()));

            let tray_created = tray::create(app).is_ok();

            #[cfg(target_os = "windows")]
            if let Some(hwnd) = hwnd {
                thumbbar::attach(app.handle(), hwnd);
            }

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
                background::update(&window);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            let webview_window = || window.app_handle().get_webview_window(window.label());
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    let webview_window = webview_window();
                    if let Some(webview_window) = &webview_window {
                        window_state::save(webview_window);
                    }
                    let close_to_tray = window.state::<AppState>().settings().close_to_tray;
                    if window.label() == "main" && close_to_tray {
                        api.prevent_close();
                        let _ = window.hide();
                        if let Some(webview_window) = &webview_window {
                            background::update(webview_window);
                        }
                    }
                }
                // Minimizing and restoring come as these.
                WindowEvent::Resized(_) | WindowEvent::Focused(_) => {
                    if let Some(webview_window) = webview_window() {
                        background::update(&webview_window);
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::has_client_id,
            commands::get_client_id,
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
            commands::get_radio_tracks,
            commands::take_launch_commands,
            commands::get_tracks_by_uri,
            commands::get_playlist,
            commands::get_library_folders,
            commands::quit_app,
            commands::update_playlist_details,
            commands::search,
            commands::search_all,
            commands::search_tracks_page,
            commands::get_artist_page,
            commands::get_artist_discography,
            commands::get_album,
            commands::get_saved_albums,
            commands::save_album,
            commands::remove_album,
            commands::get_saved_shows,
            commands::get_show,
            commands::get_show_episodes,
            commands::get_latest_episodes,
            commands::get_episode,
            commands::save_show,
            commands::remove_show,
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
            commands::preview_equalizer,
            commands::pick_folder,
            commands::get_local_files,
            commands::rescan_local_files,
            commands::window_hidden,
            commands::set_clock,
            commands::list_themes,
            commands::get_theme,
            commands::import_themes,
            commands::open_themes_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
