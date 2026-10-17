use super::client::SpotifyClient;
use super::library;
use super::models::{Episode, EpisodePage, Page, SavedShowItem, Show};

const SAVED_SHOWS_PAGE_SIZE: u32 = 50;
/// Spotify's most episodes per request.
const MAX_EPISODES_PAGE: u32 = 50;

/// The podcasts in the user's library, most recently followed first.
pub async fn get_saved_shows(client: &mut SpotifyClient) -> Result<Vec<Show>, String> {
    let items: Vec<SavedShowItem> = client.get_all_pages("/me/shows", SAVED_SHOWS_PAGE_SIZE).await?;
    Ok(items
        .into_iter()
        .filter_map(|item| item.show)
        .filter(|show| !show.id.is_empty())
        .collect())
}

pub async fn save(client: &mut SpotifyClient, id: &str) -> Result<(), String> {
    library::save(client, &format!("spotify:show:{id}")).await
}

pub async fn remove(client: &mut SpotifyClient, id: &str) -> Result<(), String> {
    library::remove(client, &format!("spotify:show:{id}")).await
}

pub async fn get_show(client: &mut SpotifyClient, id: &str) -> Result<Show, String> {
    client.get_json(&format!("/shows/{id}"), &[]).await
}

/// A slice of a show's episodes, newest first. Spotify leaves out the ones
/// that can't be played here, so a page can hold fewer than `limit`.
pub async fn get_episodes(client: &mut SpotifyClient, id: &str, offset: u32, limit: u32) -> Result<EpisodePage, String> {
    let limit = limit.clamp(1, MAX_EPISODES_PAGE).to_string();
    let offset = offset.to_string();
    let page: Page<Episode> = client
        .get_json(&format!("/shows/{id}/episodes"), &[("limit", &limit), ("offset", &offset)])
        .await?;
    Ok(EpisodePage {
        items: page.items,
        total: page.total,
    })
}

/// The newest few episodes of each show, in the same order as `ids`; a show
/// Spotify won't answer for comes back empty.
pub async fn get_latest_episodes(client: &mut SpotifyClient, ids: &[String], limit: u32) -> Result<Vec<Vec<Episode>>, String> {
    let limit = limit.clamp(1, MAX_EPISODES_PAGE).to_string();
    let requests = ids
        .iter()
        .map(|id| {
            (
                format!("/shows/{id}/episodes"),
                vec![("limit", limit.clone()), ("offset", "0".to_string())],
            )
        })
        .collect();
    let pages: Vec<Option<Page<Episode>>> = client.get_json_concurrent_lenient(requests).await?;
    Ok(pages.into_iter().map(|page| page.map(|page| page.items).unwrap_or_default()).collect())
}

/// One episode, with its show.
pub async fn get_episode(client: &mut SpotifyClient, id: &str) -> Result<Episode, String> {
    client.get_json(&format!("/episodes/{id}"), &[]).await
}
