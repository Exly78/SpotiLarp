use super::client::SpotifyClient;
use super::models::{SearchResponse, SearchResults, Track};

const SEARCH_PAGE_SIZE: &str = "10";

pub async fn search_tracks(client: &mut SpotifyClient, query: &str) -> Result<Vec<Track>, String> {
    search_tracks_page(client, query, 0).await
}

pub async fn search_tracks_page(
    client: &mut SpotifyClient,
    query: &str,
    offset: u32,
) -> Result<Vec<Track>, String> {
    let offset = offset.to_string();
    let response: SearchResponse = client
        .get_json(
            "/search",
            &[("q", query), ("type", "track"), ("limit", SEARCH_PAGE_SIZE), ("offset", &offset)],
        )
        .await?;
    Ok(response.tracks.map(|page| page.items).unwrap_or_default())
}

pub async fn search_all(client: &mut SpotifyClient, query: &str) -> Result<SearchResults, String> {
    let response: SearchResponse = client
        .get_json(
            "/search",
            &[("q", query), ("type", "track,artist,album"), ("limit", SEARCH_PAGE_SIZE)],
        )
        .await?;
    Ok(SearchResults {
        tracks: response.tracks.map(|page| page.items).unwrap_or_default(),
        artists: response.artists.map(|page| page.items).unwrap_or_default(),
        albums: response.albums.map(|page| page.items).unwrap_or_default(),
    })
}
