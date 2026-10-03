use reqwest::Method;
use serde_json::{json, Value};

use super::client::SpotifyClient;
use super::library;
use super::models::{Lenient, Playlist, PlaylistTrackItem, SavedTrackItem, Track};

const PLAYLISTS_PAGE_SIZE: u32 = 50;
const PLAYLIST_ITEMS_PAGE_SIZE: u32 = 100;
const SAVED_TRACKS_PAGE_SIZE: u32 = 50;
const ADD_ITEMS_CHUNK_SIZE: usize = 100;

pub async fn get_playlists(client: &mut SpotifyClient) -> Result<Vec<Playlist>, String> {
    client.get_all_pages("/me/playlists", PLAYLISTS_PAGE_SIZE).await
}

pub async fn get_playlist_tracks(
    client: &mut SpotifyClient,
    playlist_id: &str,
) -> Result<Vec<Track>, String> {
    let path = format!("/playlists/{playlist_id}/items");
    let items: Vec<Lenient<PlaylistTrackItem>> = client.get_all_pages(&path, PLAYLIST_ITEMS_PAGE_SIZE).await?;
    if !items.is_empty() && items.iter().all(|item| item.0.is_none()) {
        return Err("Couldn't read this playlist's tracks".to_string());
    }
    Ok(items
        .into_iter()
        .enumerate()
        .filter_map(|(position, item)| {
            let mut track = item.0?.into_track()?;
            track.position = Some(position as u32);
            Some(track)
        })
        .collect())
}

pub async fn get_liked_songs(client: &mut SpotifyClient) -> Result<Vec<Track>, String> {
    let items: Vec<SavedTrackItem> = client.get_all_pages("/me/tracks", SAVED_TRACKS_PAGE_SIZE).await?;
    Ok(items.into_iter().filter_map(SavedTrackItem::into_track).collect())
}

pub async fn add_tracks(client: &mut SpotifyClient, playlist_id: &str, uris: &[String]) -> Result<(), String> {
    let path = format!("/playlists/{playlist_id}/items");
    for chunk in uris.chunks(ADD_ITEMS_CHUNK_SIZE) {
        let _: Value = client.send_json(Method::POST, &path, &json!({ "uris": chunk })).await?;
    }
    Ok(())
}

pub async fn remove_track(client: &mut SpotifyClient, playlist_id: &str, uri: &str) -> Result<(), String> {
    let path = format!("/playlists/{playlist_id}/items");
    let _: Value = client
        .send_json(Method::DELETE, &path, &json!({ "items": [{ "uri": uri }] }))
        .await?;
    Ok(())
}

pub async fn create_playlist(client: &mut SpotifyClient, name: &str) -> Result<Playlist, String> {
    client
        .send_json(Method::POST, "/me/playlists", &json!({ "name": name, "public": false }))
        .await
}

pub async fn rename_playlist(client: &mut SpotifyClient, playlist_id: &str, name: &str) -> Result<(), String> {
    let path = format!("/playlists/{playlist_id}");
    client.send_json(Method::PUT, &path, &json!({ "name": name })).await
}

pub async fn remove_playlist(client: &mut SpotifyClient, playlist_id: &str) -> Result<(), String> {
    library::remove(client, &format!("spotify:playlist:{playlist_id}")).await
}

pub async fn move_track(
    client: &mut SpotifyClient,
    playlist_id: &str,
    from: u32,
    insert_before: u32,
) -> Result<(), String> {
    let path = format!("/playlists/{playlist_id}/items");
    let body = json!({ "range_start": from, "insert_before": insert_before, "range_length": 1 });
    let _: Value = client.send_json(Method::PUT, &path, &body).await?;
    Ok(())
}
