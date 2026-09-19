use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::auth::{loopback, pkce, REDIRECT_PORT};
use crate::config;
use crate::discord;
use crate::discord::PresenceUpdate;
use crate::lyrics::{self, LyricsResult};
use crate::playback;
use crate::spotify_api::artist;
use crate::spotify_api::library;
use crate::spotify_api::models::{ArtistDetails, Playlist, Track};
use crate::spotify_api::playlists;
use crate::spotify_api::search::search_tracks;
use crate::state::{AppState, PlaybackHandle};

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
    config::save_client_id(&client_id)?;
    let mut client = state.spotify.lock().await;
    client.set_client_id(client_id);
    Ok(())
}

#[tauri::command]
pub async fn login(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let pkce_pair = pkce::generate();
    let csrf_state = pkce::generate_state();

    let authorize_url = {
        let client = state.spotify.lock().await;
        client.build_authorize_url(&pkce_pair.challenge, &csrf_state)?
    };

    app.opener()
        .open_url(authorize_url, None::<&str>)
        .map_err(|e| format!("failed to open browser: {e}"))?;

    let expected_state = csrf_state;
    let callback = tokio::task::spawn_blocking(move || loopback::await_callback(REDIRECT_PORT))
        .await
        .map_err(|e| format!("loopback listener task panicked: {e}"))??;

    if callback.state != expected_state {
        return Err("OAuth state mismatch, possible CSRF, aborting login".to_string());
    }

    let mut client = state.spotify.lock().await;
    client.exchange_code(&callback.code, &pkce_pair.verifier).await?;
    let me = client.get_me().await?;
    Ok(me.display_name.unwrap_or(me.id))
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
pub async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    let mut client = state.spotify.lock().await;
    client.logout()
}

#[tauri::command]
pub fn has_connect_session() -> bool {
    playback::session::has_cached_credentials()
}

#[tauri::command]
pub async fn connect_playback(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut playback_state = state.playback.lock().await;
    if playback_state.is_some() {
        return Ok(());
    }

    let session = playback::session::connect().await?;
    let handle = playback::player::start(session.clone());
    playback::events::spawn_forwarder(app, &handle.player);
    *playback_state = Some(PlaybackHandle {
        session,
        player: handle.player,
        mixer: handle.mixer,
    });
    Ok(())
}

#[tauri::command]
pub async fn play_track(state: State<'_, AppState>, uri: String) -> Result<(), String> {
    let playback_state = state.playback.lock().await;
    let handle = require_playback(&playback_state)?;
    playback::player::play_uri(&handle.player, &uri)
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
    state: State<'_, AppState>,
    playlist_id: String,
) -> Result<Vec<Track>, String> {
    let mut client = state.spotify.lock().await;
    playlists::get_playlist_tracks(&mut client, &playlist_id).await
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
    let mut client = state.spotify.lock().await;
    let uris: Vec<String> = track_ids.iter().map(|id| format!("spotify:track:{id}")).collect();
    library::is_saved_batch(&mut client, &uris).await
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

fn require_playback(
    playback_state: &Option<PlaybackHandle>,
) -> Result<&PlaybackHandle, String> {
    playback_state
        .as_ref()
        .ok_or_else(|| "not connected, call connect_playback first".to_string())
}

#[tauri::command]
pub async fn pause(state: State<'_, AppState>) -> Result<(), String> {
    let playback_state = state.playback.lock().await;
    require_playback(&playback_state)?.player.pause();
    Ok(())
}

#[tauri::command]
pub async fn resume(state: State<'_, AppState>) -> Result<(), String> {
    let playback_state = state.playback.lock().await;
    require_playback(&playback_state)?.player.play();
    Ok(())
}

#[tauri::command]
pub async fn seek(state: State<'_, AppState>, position_ms: u32) -> Result<(), String> {
    let playback_state = state.playback.lock().await;
    require_playback(&playback_state)?.player.seek(position_ms);
    Ok(())
}

#[tauri::command]
pub async fn set_volume(state: State<'_, AppState>, volume: u16) -> Result<(), String> {
    let playback_state = state.playback.lock().await;
    let handle = require_playback(&playback_state)?;
    handle.mixer.set_volume(volume);
    handle.player.emit_volume_changed_event(volume);
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
