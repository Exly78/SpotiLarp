use std::collections::HashSet;

use super::client::SpotifyClient;
use super::models::{ArtistDetails, Page, SavedTrackItem, Track};

pub async fn recently_played(client: &mut SpotifyClient) -> Result<Vec<Track>, String> {
    let page: Page<SavedTrackItem> = client
        .get_json("/me/player/recently-played", &[("limit", "50")])
        .await?;
    let mut seen = HashSet::new();
    Ok(page
        .items
        .into_iter()
        .filter_map(SavedTrackItem::into_track)
        .filter(|track| seen.insert(track.uri.clone()))
        .collect())
}

fn time_range(range: &str) -> Result<&'static str, String> {
    match range {
        "short_term" => Ok("short_term"),
        "medium_term" => Ok("medium_term"),
        "long_term" => Ok("long_term"),
        other => Err(format!("unknown time range: {other}")),
    }
}

pub async fn top_tracks(client: &mut SpotifyClient, range: &str, limit: u32) -> Result<Vec<Track>, String> {
    let limit = limit.clamp(1, 50).to_string();
    let page: Page<Track> = client
        .get_json("/me/top/tracks", &[("limit", &limit), ("time_range", time_range(range)?)])
        .await?;
    Ok(page.items)
}

pub async fn top_artists(client: &mut SpotifyClient, range: &str, limit: u32) -> Result<Vec<ArtistDetails>, String> {
    let limit = limit.clamp(1, 50).to_string();
    let page: Page<ArtistDetails> = client
        .get_json("/me/top/artists", &[("limit", &limit), ("time_range", time_range(range)?)])
        .await?;
    Ok(page.items)
}
