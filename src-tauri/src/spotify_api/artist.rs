use super::client::SpotifyClient;
use super::library;
use super::models::{Album, ArtistDetails, Page, Track};
use super::search;

pub async fn get_artist(client: &mut SpotifyClient, id: &str) -> Result<ArtistDetails, String> {
    let path = format!("/artists/{id}");
    client.get_json(&path, &[]).await
}

pub async fn get_albums(client: &mut SpotifyClient, id: &str) -> Result<Vec<Album>, String> {
    let path = format!("/artists/{id}/albums");
    let page: Page<Album> = client
        .get_json(&path, &[("include_groups", "album,single"), ("limit", "50")])
        .await?;
    Ok(page.items)
}

pub async fn popular_tracks(client: &mut SpotifyClient, id: &str, name: &str) -> Result<Vec<Track>, String> {
    let query = format!("artist:\"{}\"", name.replace('"', ""));
    let tracks = search::search_tracks(client, &query).await?;
    Ok(tracks
        .into_iter()
        .filter(|track| track.artists.iter().any(|artist| artist.id == id))
        .collect())
}

pub async fn is_following(client: &mut SpotifyClient, id: &str) -> Result<bool, String> {
    library::is_saved(client, &format!("spotify:artist:{id}")).await
}

pub async fn follow(client: &mut SpotifyClient, id: &str) -> Result<(), String> {
    library::save(client, &format!("spotify:artist:{id}")).await
}

pub async fn unfollow(client: &mut SpotifyClient, id: &str) -> Result<(), String> {
    library::remove(client, &format!("spotify:artist:{id}")).await
}
