use super::client::SpotifyClient;
use super::library;
use super::models::ArtistDetails;

pub async fn get_artist(client: &mut SpotifyClient, id: &str) -> Result<ArtistDetails, String> {
    let path = format!("/artists/{id}");
    client.get_json(&path, &[]).await
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
