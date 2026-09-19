use reqwest::Method;

use super::client::SpotifyClient;

const CONTAINS_CHUNK_SIZE: usize = 40;

pub async fn is_saved_batch(client: &mut SpotifyClient, uris: &[String]) -> Result<Vec<bool>, String> {
    let mut results = Vec::with_capacity(uris.len());
    for chunk in uris.chunks(CONTAINS_CHUNK_SIZE) {
        let joined = chunk.join(",");
        let chunk_results: Vec<bool> = client
            .get_json("/me/library/contains", &[("uris", joined.as_str())])
            .await?;
        results.extend(chunk_results);
    }
    Ok(results)
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
