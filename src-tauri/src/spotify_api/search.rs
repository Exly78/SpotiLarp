use super::client::SpotifyClient;
use super::models::{SearchResponse, Track};

pub async fn search_tracks(client: &mut SpotifyClient, query: &str) -> Result<Vec<Track>, String> {
    
    let response: SearchResponse = client
        .get_json("/search", &[("q", query), ("type", "track"), ("limit", "10")])
        .await?;
    Ok(response.tracks.map(|page| page.items).unwrap_or_default())
}
