use super::client::SpotifyClient;
use super::models::{Playlist, PlaylistTracksResponse, PlaylistsResponse, SavedTracksResponse, Track};

pub async fn get_playlists(client: &mut SpotifyClient) -> Result<Vec<Playlist>, String> {
    let response: PlaylistsResponse = client.get_json("/me/playlists", &[("limit", "50")]).await?;
    Ok(response.items)
}

pub async fn get_playlist_tracks(
    client: &mut SpotifyClient,
    playlist_id: &str,
) -> Result<Vec<Track>, String> {
    
    let path = format!("/playlists/{playlist_id}/items");
    let response: PlaylistTracksResponse = client.get_json(&path, &[("limit", "50")]).await?;
    Ok(response
        .items
        .into_iter()
        .filter_map(|item| item.item)
        .collect())
}

pub async fn get_liked_songs(client: &mut SpotifyClient) -> Result<Vec<Track>, String> {
    let mut tracks = Vec::new();
    let mut offset = 0u32;
    const LIMIT: u32 = 50;
    loop {
        let limit_str = LIMIT.to_string();
        let offset_str = offset.to_string();
        let response: SavedTracksResponse = client
            .get_json("/me/tracks", &[("limit", &limit_str), ("offset", &offset_str)])
            .await?;
        let page_len = response.items.len();
        tracks.extend(response.items.into_iter().filter_map(|item| item.track));
        if page_len < LIMIT as usize {
            break;
        }
        offset += LIMIT;
    }
    Ok(tracks)
}
