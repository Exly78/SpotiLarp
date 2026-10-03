use reqwest::Method;

use super::client::SpotifyClient;

const CONTAINS_CHUNK_SIZE: usize = 40;

pub async fn is_saved_batch(client: &mut SpotifyClient, uris: &[String]) -> Result<Vec<bool>, String> {
    let requests = uris
        .chunks(CONTAINS_CHUNK_SIZE)
        .map(|chunk| ("/me/library/contains".to_string(), vec![("uris", chunk.join(","))]))
        .collect();
    let chunks: Vec<Vec<bool>> = client.get_json_concurrent(requests).await?;
    Ok(chunks.into_iter().flatten().collect())
}

pub async fn is_saved(client: &mut SpotifyClient, uri: &str) -> Result<bool, String> {
    let result = is_saved_batch(client, std::slice::from_ref(&uri.to_string())).await?;
    Ok(result.first().copied().unwrap_or(false))
}

pub async fn save(client: &mut SpotifyClient, uri: &str) -> Result<(), String> {
    client.send_empty(Method::PUT, "/me/library", &[("uris", uri)]).await
}

pub async fn remove(client: &mut SpotifyClient, uri: &str) -> Result<(), String> {
    client.send_empty(Method::DELETE, "/me/library", &[("uris", uri)]).await
}
