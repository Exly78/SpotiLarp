use super::client::SpotifyClient;
use super::models::{Album, AlbumDetails, AlbumTrack};

const ALBUM_TRACKS_PAGE_SIZE: u32 = 50;

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
