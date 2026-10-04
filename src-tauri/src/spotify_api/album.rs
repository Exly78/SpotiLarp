use super::client::SpotifyClient;
use super::library;
use super::models::{Album, AlbumDetails, AlbumTrack, SavedAlbumItem};

const ALBUM_TRACKS_PAGE_SIZE: u32 = 50;
const SAVED_ALBUMS_PAGE_SIZE: u32 = 50;

/// The albums in the user's library, most recently saved first.
pub async fn get_saved_albums(client: &mut SpotifyClient) -> Result<Vec<Album>, String> {
    let items: Vec<SavedAlbumItem> = client.get_all_pages("/me/albums", SAVED_ALBUMS_PAGE_SIZE).await?;
    Ok(items
        .into_iter()
        .filter_map(|item| item.album)
        .filter(|album| !album.id.is_empty())
        .collect())
}

pub async fn save(client: &mut SpotifyClient, id: &str) -> Result<(), String> {
    library::save(client, &format!("spotify:album:{id}")).await
}

pub async fn remove(client: &mut SpotifyClient, id: &str) -> Result<(), String> {
    library::remove(client, &format!("spotify:album:{id}")).await
}

pub async fn get_album(client: &mut SpotifyClient, id: &str) -> Result<AlbumDetails, String> {
    let album: Album = client.get_json(&format!("/albums/{id}"), &[]).await?;
    let tracks: Vec<AlbumTrack> = client
        .get_all_pages(&format!("/albums/{id}/tracks"), ALBUM_TRACKS_PAGE_SIZE)
        .await?;
    Ok(AlbumDetails {
        tracks: tracks.into_iter().map(|track| track.into_track(&album)).collect(),
        album,
    })
}
