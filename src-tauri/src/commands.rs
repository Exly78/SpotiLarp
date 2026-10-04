use std::sync::atomic::Ordering;

use librespot_core::session::Session;
use tauri::{AppHandle, Emitter, LogicalSize, State, WebviewWindow};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;
use tokio::sync::{MappedMutexGuard, MutexGuard};

use crate::auth::{loopback, oauth, AUTH_SCOPES, REDIRECT_PATH, REDIRECT_PORT};
use crate::config::{self, Settings};
use crate::discord;
use crate::discord::PresenceUpdate;
use crate::local_files::{self, LocalIndex, LocalLibrary};
use crate::lyrics::{self, LyricsResult};
use crate::media::MediaUpdate;
use crate::playback;
use crate::playback::local::LocalPlayer;
use crate::spotify_api::album;
use crate::spotify_api::artist;
use crate::spotify_api::home;
use crate::spotify_api::library;
use crate::spotify_api::models::{Album, AlbumDetails, ArtistDetails, ArtistPage, Image, Playlist, SearchResults, Track};
use crate::spotify_api::playlists;
use crate::spotify_api::search::{self, search_tracks};
use crate::state::{AppState, MiniRestore, PlaybackHandle};

#[tauri::command]
pub async fn has_client_id(state: State<'_, AppState>) -> Result<bool, String> {
    let client = state.spotify.lock().await;
    Ok(client.has_client_id())
}

#[tauri::command]
pub async fn set_client_id(state: State<'_, AppState>, client_id: String) -> Result<(), String> {
    let client_id = client_id.trim().to_string();
    if client_id.is_empty() {
        return Err("Client ID can't be empty".to_string());
    }
    if client_id.len() != 32 || !client_id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("That doesn't look like a Client ID, it should be 32 letters and numbers".to_string());
    }
    config::save_client_id(&client_id)?;
    let mut client = state.spotify.lock().await;
    client.set_client_id(client_id)
}

#[tauri::command]
pub async fn login(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let client_id = state.spotify.lock().await.require_client_id()?.to_string();
    let code = oauth::browser_login(&app, &client_id, AUTH_SCOPES, REDIRECT_PORT, REDIRECT_PATH, true).await?;

    let mut client = state.spotify.lock().await;
    client.exchange_code(&code).await?;
    let me = client.get_me().await?;
    Ok(me.display_name.unwrap_or(me.id))
}

#[tauri::command]
pub fn cancel_login() {
    loopback::cancel_all();
}

#[tauri::command]
pub async fn login_status(state: State<'_, AppState>) -> Result<bool, String> {
    let client = state.spotify.lock().await;
    Ok(client.has_refresh_token())
}

#[tauri::command]
pub async fn restore_session(state: State<'_, AppState>) -> Result<String, String> {
    let mut client = state.spotify.lock().await;
    let me = client.get_me().await?;
    Ok(me.display_name.unwrap_or(me.id))
}

#[tauri::command]
pub async fn logout(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    loopback::cancel_all();
    state.spotify.lock().await.logout()?;

    let _connecting = state.playback_connect.lock().await;
    state.local_active.store(false, Ordering::SeqCst);
    if let Some(local) = state.local_player.get() {
        local.stop();
    }
    if let Some(handle) = state.playback.lock().await.take() {
        handle.player.stop();
        handle.session.shutdown();
    }
    if let Some(discord) = state.discord.lock().await.as_ref() {
        let _ = discord.send(PresenceUpdate::Cleared);
    }
    if let Some(media) = state.media.get() {
        let _ = media.send(MediaUpdate::Stopped);
    }
    let _ = app.emit(PLAYBACK_CONNECTION_EVENT, false);
    playback::session::forget_credentials()
}

#[tauri::command]
pub fn has_connect_session() -> bool {
    playback::session::has_cached_credentials()
}

const PLAYBACK_CONNECTION_EVENT: &str = "playback-connection";

async fn ensure_playback(app: &AppHandle, state: &AppState, interactive: bool) -> Result<(), String> {
    let _connecting = state.playback_connect.lock().await;
    {
        let mut playback_state = state.playback.lock().await;
        match playback_state.as_ref() {
            Some(handle) if !handle.session.is_invalid() => return Ok(()),
            Some(_) => *playback_state = None,
            None => {}
        }
    }

    match playback::session::connect(app, state.settings().cache_limit_mb, interactive).await {
        Ok(session) => {
            *state.playback.lock().await = Some(new_player(app, state, session));
            let _ = app.emit(PLAYBACK_CONNECTION_EVENT, true);
            Ok(())
        }
        Err(e) => {
            let _ = app.emit(PLAYBACK_CONNECTION_EVENT, false);
            Err(e)
        }
    }
}

fn new_player(app: &AppHandle, state: &AppState, session: Session) -> PlaybackHandle {
    let handle = playback::player::start(session.clone(), &state.settings());
    handle.mixer.set_volume(state.volume.load(Ordering::Relaxed));
    playback::events::spawn_forwarder(app.clone(), &handle.player);
    PlaybackHandle {
        session,
        player: handle.player,
        mixer: handle.mixer,
    }
}

pub fn apply_audio_settings(settings: &Settings) {
    playback::equalizer::configure(settings.eq_enabled, &settings.eq_gains);
    playback::sink::set_output_device(settings.output_device.clone());
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

#[tauri::command]
pub async fn set_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> Result<bool, String> {
    config::save_settings(&settings)?;
    let previous = state
        .settings
        .lock()
        .map(|mut current| std::mem::replace(&mut *current, settings.clone()))
        .map_err(|e| e.to_string())?;
    apply_audio_settings(&settings);
    let player_changed = previous.audio_quality != settings.audio_quality
        || previous.normalize_volume != settings.normalize_volume;
    if !player_changed {
        return Ok(false);
    }

    let Ok(_connecting) = state.playback_connect.try_lock() else {
        return Ok(false);
    };
    let mut playback_state = state.playback.lock().await;
    let Some(old) = playback_state.take() else {
        return Ok(false);
    };
    if old.session.is_invalid() {
        return Ok(false);
    }
    let session = old.session.clone();
    drop(old);
    *playback_state = Some(new_player(&app, &state, session));
    // A local file playing right now isn't affected, so it shouldn't restart.
    Ok(!state.local_active.load(Ordering::SeqCst))
}

#[tauri::command]
pub async fn pick_folder(window: WebviewWindow) -> Option<String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("Choose a music folder")
        .pick_folder(move |folder| {
            let _ = tx.send(folder);
        });
    let folder = rx.await.ok().flatten()?;
    folder.into_path().ok().map(|path| path.to_string_lossy().into_owned())
}

async fn local_index(state: &AppState, rescan: bool) -> Result<MappedMutexGuard<'_, LocalIndex>, String> {
    let mut index = state.local_files.lock().await;
    let folders = state.settings().local_folders;
    if rescan || !index.as_ref().is_some_and(|index| index.is_scan_of(&folders)) {
        let scanned = tokio::task::spawn_blocking(move || local_files::scan(folders))
            .await
            .map_err(|e| format!("Couldn't scan your local files: {e}"))?;
        *index = Some(scanned);
    }
    Ok(MutexGuard::map(index, |index| index.get_or_insert_with(LocalIndex::default)))
}

#[tauri::command]
pub async fn get_local_files(state: State<'_, AppState>) -> Result<LocalLibrary, String> {
    Ok(local_index(&state, false).await?.library.clone())
}

#[tauri::command]
pub async fn rescan_local_files(state: State<'_, AppState>) -> Result<LocalLibrary, String> {
    Ok(local_index(&state, true).await?.library.clone())
}

fn local_player(state: &AppState) -> Result<&LocalPlayer, String> {
    state
        .local_player
        .get()
        .ok_or_else(|| "Local playback isn't ready yet".to_string())
}

fn is_local_uri(uri: &str) -> bool {
    uri.starts_with(local_files::URI_PREFIX)
}

/// Hands playback to the local player or back to librespot, stopping the other.
async fn set_local_active(state: &AppState, local: bool) {
    if state.local_active.swap(local, Ordering::SeqCst) == local {
        return;
    }
    if local {
        if let Some(handle) = state.playback.lock().await.as_ref() {
            handle.player.stop();
        }
    } else if let Some(player) = state.local_player.get() {
        player.stop();
    }
}

async fn play_local(state: &AppState, uri: &str, position_ms: u32) -> Result<(), String> {
    let track = local_index(state, false).await?.find(uri);
    set_local_active(state, true).await;
    let player = local_player(state)?;
    match track {
        Some(track) => player.load(track, position_ms),
        None => player.missing(),
    }
    Ok(())
}

#[tauri::command]
pub async fn get_cache_size() -> u64 {
    let Some(dir) = playback::session::audio_cache_dir() else {
        return 0;
    };
    tokio::task::spawn_blocking(move || dir_size(&dir)).await.unwrap_or(0)
}

fn dir_size(dir: &std::path::Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.metadata() {
            Ok(meta) if meta.is_dir() => dir_size(&entry.path()),
            Ok(meta) => meta.len(),
            Err(_) => 0,
        })
        .sum()
}

#[tauri::command]
pub async fn clear_cache() -> Result<(), String> {
    let Some(dir) = playback::session::audio_cache_dir() else {
        return Ok(());
    };
    tokio::task::spawn_blocking(move || match std::fs::remove_dir_all(&dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Couldn't clear the cache: {e}")),
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    }
    .map_err(|e| e.to_string())
}

const NORMAL_MIN_SIZE: LogicalSize<f64> = LogicalSize::new(960.0, 600.0);
const MINI_SIZE: LogicalSize<f64> = LogicalSize::new(400.0, 136.0);

#[tauri::command]
pub async fn set_mini_mode(window: WebviewWindow, state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    let result: tauri::Result<()> = (|| {
        if enabled {
            let restore = MiniRestore {
                size: window.inner_size()?,
                maximized: window.is_maximized()?,
            };
            if restore.maximized {
                window.unmaximize()?;
            }
            if let Ok(mut saved) = state.mini_restore.lock() {
                *saved = Some(restore);
            }
            window.set_min_size(Some(MINI_SIZE))?;
            window.set_size(MINI_SIZE)?;
            window.set_resizable(false)?;
            window.set_maximizable(false)?;
            window.set_always_on_top(true)?;
        } else {
            window.set_always_on_top(false)?;
            window.set_resizable(true)?;
            window.set_maximizable(true)?;
            window.set_min_size(Some(NORMAL_MIN_SIZE))?;
            let restore = state.mini_restore.lock().ok().and_then(|mut saved| saved.take());
            match restore {
                Some(restore) => {
                    window.set_size(restore.size)?;
                    if restore.maximized {
                        window.maximize()?;
                    }
                }
                None => window.set_size(LogicalSize::new(1280.0, 800.0))?,
            }
        }
        Ok(())
    })();
    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn connect_playback(app: AppHandle, state: State<'_, AppState>, interactive: bool) -> Result<(), String> {
    ensure_playback(&app, &state, interactive).await
}

#[tauri::command]
pub async fn play_track(
    app: AppHandle,
    state: State<'_, AppState>,
    uri: String,
    position_ms: Option<u32>,
) -> Result<(), String> {
    if is_local_uri(&uri) {
        return play_local(&state, &uri, position_ms.unwrap_or(0)).await;
    }
    ensure_playback(&app, &state, false).await?;
    set_local_active(&state, false).await;
    let playback_state = state.playback.lock().await;
    let handle = require_playback(&playback_state)?;
    playback::player::play_uri(&handle.player, &uri, position_ms.unwrap_or(0))
}

#[tauri::command]
pub async fn get_autoplay_tracks(
    app: AppHandle,
    state: State<'_, AppState>,
    seed_uri: String,
    recent_uris: Vec<String>,
) -> Result<Vec<Track>, String> {
    ensure_playback(&app, &state, false).await?;
    let session = {
        let playback_state = state.playback.lock().await;
        require_playback(&playback_state)?.session.clone()
    };
    playback::autoplay::tracks(&session, &seed_uri, recent_uris).await
}

#[tauri::command]
pub async fn preload_track(state: State<'_, AppState>, uri: String) -> Result<(), String> {
    // librespot reports a failed preload as the current song being unavailable.
    if is_local_uri(&uri) {
        return Ok(());
    }
    let playback_state = state.playback.lock().await;
    let handle = require_playback(&playback_state)?;
    playback::player::preload_uri(&handle.player, &uri)
}

#[tauri::command]
pub async fn search(state: State<'_, AppState>, query: String) -> Result<Vec<Track>, String> {
    let mut client = state.spotify.lock().await;
    search_tracks(&mut client, &query).await
}

#[tauri::command]
pub async fn get_playlists(state: State<'_, AppState>) -> Result<Vec<Playlist>, String> {
    let mut client = state.spotify.lock().await;
    playlists::get_playlists(&mut client).await
}

#[tauri::command]
pub async fn get_playlist_tracks(
    app: AppHandle,
    state: State<'_, AppState>,
    playlist_id: String,
) -> Result<Vec<Track>, String> {
    let mut tracks = load_playlist_tracks(&app, &state, &playlist_id).await?;
    add_local_covers(&state, &mut tracks).await;
    Ok(tracks)
}

async fn load_playlist_tracks(app: &AppHandle, state: &AppState, playlist_id: &str) -> Result<Vec<Track>, String> {
    let result = {
        let mut client = state.spotify.lock().await;
        playlists::get_playlist_tracks(&mut client, playlist_id).await
    };
    if result.as_ref().is_ok_and(|tracks| !tracks.is_empty()) {
        return result;
    }
    // The Web API only shows playlists you own or collaborate on.
    if let Some(session) = spotify_session(app, state).await {
        match playlists::get_playlist_tracks_via_session(&session, playlist_id).await {
            Ok(tracks) if !tracks.is_empty() || result.is_err() => return Ok(tracks),
            Ok(_) => {}
            Err(e) => eprintln!("Couldn't read playlist {playlist_id} through the session: {e}"),
        }
    }
    result
}

/// Playlists only keep a local file's tags, so its cover comes from the file
/// itself when it's in Local Files.
async fn add_local_covers(state: &AppState, tracks: &mut [Track]) {
    if !tracks.iter().any(|track| is_local_uri(&track.uri) && track.album.images.is_empty()) {
        return;
    }
    let Ok(index) = local_index(state, false).await else {
        return;
    };
    for track in tracks
        .iter_mut()
        .filter(|track| is_local_uri(&track.uri) && track.album.images.is_empty())
    {
        if let Some(url) = index.find(&track.uri).and_then(|local| local.cover_url) {
            track.album.images = vec![Image { url, width: None, height: None }];
        }
    }
}

#[tauri::command]
pub async fn get_liked_songs(state: State<'_, AppState>) -> Result<Vec<Track>, String> {
    let mut client = state.spotify.lock().await;
    playlists::get_liked_songs(&mut client).await
}

#[tauri::command]
pub async fn is_track_liked(state: State<'_, AppState>, track_id: String) -> Result<bool, String> {
    let mut client = state.spotify.lock().await;
    library::is_saved(&mut client, &format!("spotify:track:{track_id}")).await
}

#[tauri::command]
pub async fn are_tracks_liked(state: State<'_, AppState>, track_ids: Vec<String>) -> Result<Vec<bool>, String> {
    let uris: Vec<String> = track_ids
        .iter()
        .filter(|id| !id.is_empty())
        .map(|id| format!("spotify:track:{id}"))
        .collect();
    let mut client = state.spotify.lock().await;
    let mut saved = library::is_saved_batch(&mut client, &uris).await?.into_iter();
    Ok(track_ids
        .iter()
        .map(|id| !id.is_empty() && saved.next().unwrap_or(false))
        .collect())
}

#[tauri::command]
pub async fn like_track(state: State<'_, AppState>, track_id: String) -> Result<(), String> {
    let mut client = state.spotify.lock().await;
    library::save(&mut client, &format!("spotify:track:{track_id}")).await
}

#[tauri::command]
pub async fn unlike_track(state: State<'_, AppState>, track_id: String) -> Result<(), String> {
    let mut client = state.spotify.lock().await;
    library::remove(&mut client, &format!("spotify:track:{track_id}")).await
}

#[tauri::command]
pub async fn get_artist(state: State<'_, AppState>, artist_id: String) -> Result<ArtistDetails, String> {
    let mut client = state.spotify.lock().await;
    artist::get_artist(&mut client, &artist_id).await
}

#[tauri::command]
pub async fn is_following_artist(state: State<'_, AppState>, artist_id: String) -> Result<bool, String> {
    let mut client = state.spotify.lock().await;
    artist::is_following(&mut client, &artist_id).await
}

#[tauri::command]
pub async fn follow_artist(state: State<'_, AppState>, artist_id: String) -> Result<(), String> {
    let mut client = state.spotify.lock().await;
    artist::follow(&mut client, &artist_id).await
}

#[tauri::command]
pub async fn unfollow_artist(state: State<'_, AppState>, artist_id: String) -> Result<(), String> {
    let mut client = state.spotify.lock().await;
    artist::unfollow(&mut client, &artist_id).await
}

#[tauri::command]
pub async fn search_all(state: State<'_, AppState>, query: String) -> Result<SearchResults, String> {
    let mut client = state.spotify.lock().await;
    search::search_all(&mut client, &query).await
}

#[tauri::command]
pub async fn search_tracks_page(
    state: State<'_, AppState>,
    query: String,
    offset: u32,
) -> Result<Vec<Track>, String> {
    let mut client = state.spotify.lock().await;
    search::search_tracks_page(&mut client, &query, offset).await
}

#[tauri::command]
pub async fn get_artist_page(
    app: AppHandle,
    state: State<'_, AppState>,
    artist_id: String,
) -> Result<ArtistPage, String> {
    if let Some(session) = spotify_session(&app, &state).await {
        match artist::page(&session, &artist_id).await {
            Ok(page) => return Ok(page),
            Err(e) => eprintln!("Falling back to the Web API for artist {artist_id}: {e}"),
        }
    }
    let mut client = state.spotify.lock().await;
    artist::basic_page(&mut client, &artist_id).await
}

#[tauri::command]
pub async fn get_artist_discography(
    app: AppHandle,
    state: State<'_, AppState>,
    artist_id: String,
    group: String,
) -> Result<Vec<Album>, String> {
    if let Some(session) = spotify_session(&app, &state).await {
        match artist::discography(&session, &artist_id, &group).await {
            Ok(releases) => return Ok(releases),
            Err(e) => eprintln!("Falling back to the Web API for artist {artist_id}'s releases: {e}"),
        }
    }
    let mut client = state.spotify.lock().await;
    artist::basic_discography(&mut client, &artist_id, &group).await
}

#[tauri::command]
pub async fn get_album(state: State<'_, AppState>, album_id: String) -> Result<AlbumDetails, String> {
    let mut client = state.spotify.lock().await;
    album::get_album(&mut client, &album_id).await
}

#[tauri::command]
pub async fn get_saved_albums(state: State<'_, AppState>) -> Result<Vec<Album>, String> {
    let mut client = state.spotify.lock().await;
    album::get_saved_albums(&mut client).await
}

#[tauri::command]
pub async fn save_album(state: State<'_, AppState>, album_id: String) -> Result<(), String> {
    let mut client = state.spotify.lock().await;
    album::save(&mut client, &album_id).await
}

#[tauri::command]
pub async fn remove_album(state: State<'_, AppState>, album_id: String) -> Result<(), String> {
    let mut client = state.spotify.lock().await;
    album::remove(&mut client, &album_id).await
}

#[tauri::command]
pub async fn get_recently_played(state: State<'_, AppState>) -> Result<Vec<Track>, String> {
    let mut client = state.spotify.lock().await;
    home::recently_played(&mut client).await
}

#[tauri::command]
pub async fn get_top_tracks(
    state: State<'_, AppState>,
    time_range: String,
    limit: u32,
) -> Result<Vec<Track>, String> {
    let mut client = state.spotify.lock().await;
    home::top_tracks(&mut client, &time_range, limit).await
}

#[tauri::command]
pub async fn get_top_artists(
    state: State<'_, AppState>,
    time_range: String,
    limit: u32,
) -> Result<Vec<ArtistDetails>, String> {
    let mut client = state.spotify.lock().await;
    home::top_artists(&mut client, &time_range, limit).await
}

#[tauri::command]
pub async fn rename_playlist(state: State<'_, AppState>, playlist_id: String, name: String) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Playlist name can't be empty".to_string());
    }
    let mut client = state.spotify.lock().await;
    playlists::rename_playlist(&mut client, &playlist_id, name).await
}

#[tauri::command]
pub async fn remove_playlist(state: State<'_, AppState>, playlist_id: String) -> Result<(), String> {
    let mut client = state.spotify.lock().await;
    playlists::remove_playlist(&mut client, &playlist_id).await
}

#[tauri::command]
pub async fn move_playlist_track(
    state: State<'_, AppState>,
    playlist_id: String,
    from: u32,
    insert_before: u32,
) -> Result<(), String> {
    let mut client = state.spotify.lock().await;
    playlists::move_track(&mut client, &playlist_id, from, insert_before).await
}

#[tauri::command]
pub fn list_output_devices() -> Vec<String> {
    playback::sink::output_device_names()
}

#[tauri::command]
pub fn preview_equalizer(enabled: bool, gains: Vec<f32>) {
    playback::equalizer::configure(enabled, &gains);
}

const LOCAL_FILES_NEED_PLAYBACK: &str = "Connect playback to put local files in playlists";

#[tauri::command]
pub async fn add_to_playlist(
    app: AppHandle,
    state: State<'_, AppState>,
    playlist_id: String,
    uris: Vec<String>,
) -> Result<(), String> {
    let (local, songs): (Vec<String>, Vec<String>) = uris.into_iter().partition(|uri| is_local_uri(uri));
    if !songs.is_empty() {
        let mut client = state.spotify.lock().await;
        playlists::add_tracks(&mut client, &playlist_id, &songs).await?;
    }
    if !local.is_empty() {
        let session = spotify_session(&app, &state)
            .await
            .ok_or_else(|| LOCAL_FILES_NEED_PLAYBACK.to_string())?;
        playlists::add_local_tracks(&session, &playlist_id, &local).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn remove_from_playlist(
    app: AppHandle,
    state: State<'_, AppState>,
    playlist_id: String,
    uri: String,
) -> Result<(), String> {
    if is_local_uri(&uri) {
        let session = spotify_session(&app, &state)
            .await
            .ok_or_else(|| LOCAL_FILES_NEED_PLAYBACK.to_string())?;
        return playlists::remove_local_track(&session, &playlist_id, &uri).await;
    }
    let mut client = state.spotify.lock().await;
    playlists::remove_track(&mut client, &playlist_id, &uri).await
}

#[tauri::command]
pub async fn create_playlist(state: State<'_, AppState>, name: String) -> Result<Playlist, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Playlist name can't be empty".to_string());
    }
    let mut client = state.spotify.lock().await;
    playlists::create_playlist(&mut client, name).await
}

#[tauri::command]
pub async fn get_user_id(state: State<'_, AppState>) -> Result<String, String> {
    let mut client = state.spotify.lock().await;
    client.user_id().await
}

#[tauri::command]
pub async fn missing_scopes(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut client = state.spotify.lock().await;
    client.missing_scopes().await
}

fn require_playback(
    playback_state: &Option<PlaybackHandle>,
) -> Result<&PlaybackHandle, String> {
    playback_state
        .as_ref()
        .ok_or_else(|| "Playback isn't connected yet, click \"Connect playback\" first".to_string())
}

/// The librespot session, for Spotify's internal APIs, if playback is or can
/// silently be connected.
async fn spotify_session(app: &AppHandle, state: &AppState) -> Option<Session> {
    ensure_playback(app, state, false).await.ok()?;
    let playback_state = state.playback.lock().await;
    playback_state.as_ref().map(|handle| handle.session.clone())
}

#[tauri::command]
pub async fn pause(state: State<'_, AppState>) -> Result<(), String> {
    if state.local_active.load(Ordering::SeqCst) {
        local_player(&state)?.pause();
        return Ok(());
    }
    let playback_state = state.playback.lock().await;
    require_playback(&playback_state)?.player.pause();
    Ok(())
}

#[tauri::command]
pub async fn resume(state: State<'_, AppState>) -> Result<(), String> {
    if state.local_active.load(Ordering::SeqCst) {
        local_player(&state)?.play();
        return Ok(());
    }
    let playback_state = state.playback.lock().await;
    require_playback(&playback_state)?.player.play();
    Ok(())
}

#[tauri::command]
pub async fn seek(state: State<'_, AppState>, position_ms: u32) -> Result<(), String> {
    if state.local_active.load(Ordering::SeqCst) {
        local_player(&state)?.seek(position_ms);
        return Ok(());
    }
    let playback_state = state.playback.lock().await;
    require_playback(&playback_state)?.player.seek(position_ms);
    Ok(())
}

#[tauri::command]
pub async fn set_volume(state: State<'_, AppState>, volume: u16) -> Result<(), String> {
    state.volume.store(volume, Ordering::Relaxed);
    let playback_state = state.playback.lock().await;
    if let Some(handle) = playback_state.as_ref() {
        handle.mixer.set_volume(volume);
        handle.player.emit_volume_changed_event(volume);
    }
    Ok(())
}

#[tauri::command]
pub async fn get_lyrics(
    track_name: String,
    artist_name: String,
    album_name: String,
    duration_ms: u32,
) -> Result<Option<LyricsResult>, String> {
    lyrics::fetch(&track_name, &artist_name, &album_name, duration_ms).await
}

#[tauri::command]
pub async fn has_discord_client_id(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.discord.lock().await.is_some())
}

#[tauri::command]
pub async fn set_discord_client_id(
    state: State<'_, AppState>,
    client_id: String,
) -> Result<(), String> {
    let client_id = client_id.trim().to_string();
    if client_id.is_empty() {
        return Err("Application ID can't be empty".to_string());
    }
    config::save_discord_client_id(&client_id)?;
    let mut discord = state.discord.lock().await;
    if let Some(old) = discord.take() {
        let _ = old.send(PresenceUpdate::Shutdown);
    }
    *discord = Some(discord::spawn(client_id));
    Ok(())
}

#[tauri::command]
pub async fn clear_discord_client_id(state: State<'_, AppState>) -> Result<(), String> {
    config::clear_discord_client_id()?;
    let mut discord = state.discord.lock().await;
    if let Some(sender) = discord.take() {
        let _ = sender.send(PresenceUpdate::Shutdown);
    }
    Ok(())
}
