//! Spotify's own GraphQL API ("pathfinder"), the one the desktop and web players
//! build their pages from. The Web API has nothing like monthly listeners, play
//! counts or "Fans also like" (and only reads playlists you own), so pages that
//! should look like Spotify's ask pathfinder instead, signed in as the librespot
//! session.
//!
//! Queries are persisted: the request names a query by its hash rather than
//! sending its text. The hashes change when Spotify ships a new web player, so
//! when one stops working the current one is looked up in the web player's
//! JavaScript, the same place the hashes below came from.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use librespot_core::session::Session;
use librespot_core::version::SPOTIFY_SEMANTIC_VERSION;
use serde_json::{json, Value};

use super::models::{Album, Artist, Image, Track};

const ENDPOINT: &str = "https://api-partner.spotify.com/pathfinder/v2/query";
const WEB_PLAYER_URL: &str = "https://open.spotify.com/";
const WEB_PLAYER_BUILD_URL: &str = "https://open.spotifycdn.com/cdn/build/web-player/";
const WEB_PLAYER_BUNDLE: &str = "/cdn/build/web-player/web-player.";
const BROWSER_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";
const QUERY_HASH_LEN: usize = 64;
const CHUNK_HASH_LEN: usize = 8;

#[cfg(target_os = "windows")]
const APP_PLATFORM: &str = "Win32_x86_64";
#[cfg(target_os = "macos")]
const APP_PLATFORM: &str = "OSX";
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const APP_PLATFORM: &str = "Linux";

pub struct Operation {
    name: &'static str,
    hash: &'static str,
    /// The web player chunk that defines the query, when it isn't in the main bundle.
    chunk: Option<&'static str>,
}

const ARTIST_CHUNK: Option<&str> = Some("xpui-routes-artist");

pub const ARTIST_OVERVIEW: Operation = Operation {
    name: "queryArtistOverview",
    hash: "9f8134ef565e78621f1e1793555bd6633c5ac144ae0f89604ed3ae3f80b3c8e6",
    chunk: None,
};
pub const ARTIST_DISCOGRAPHY_ALL: Operation = Operation {
    name: "queryArtistDiscographyAll",
    hash: "5e07d323febb57b4a56a42abbf781490e58764aa45feb6e3dc0591564fc56599",
    chunk: ARTIST_CHUNK,
};
pub const ARTIST_DISCOGRAPHY_ALBUMS: Operation = Operation {
    name: "queryArtistDiscographyAlbums",
    hash: "5e07d323febb57b4a56a42abbf781490e58764aa45feb6e3dc0591564fc56599",
    chunk: ARTIST_CHUNK,
};
pub const ARTIST_DISCOGRAPHY_SINGLES: Operation = Operation {
    name: "queryArtistDiscographySingles",
    hash: "5e07d323febb57b4a56a42abbf781490e58764aa45feb6e3dc0591564fc56599",
    chunk: ARTIST_CHUNK,
};
pub const ARTIST_DISCOGRAPHY_COMPILATIONS: Operation = Operation {
    name: "queryArtistDiscographyCompilations",
    hash: "5e07d323febb57b4a56a42abbf781490e58764aa45feb6e3dc0591564fc56599",
    chunk: ARTIST_CHUNK,
};
pub const PLAYLIST_CONTENTS: Operation = Operation {
    name: "fetchPlaylistContents",
    hash: "8964e8eafb21aa992a7d951d256d83285c04be2105d209262901de70cb97584a",
    chunk: None,
};

enum QueryError {
    /// Spotify refused the request itself (an outdated hash looks like this).
    Rejected(String),
    Other(String),
}

impl From<QueryError> for String {
    fn from(error: QueryError) -> Self {
        match error {
            QueryError::Rejected(message) | QueryError::Other(message) => message,
        }
    }
}

struct HashCache {
    /// Hashes found in the web player, by operation name.
    found: HashMap<&'static str, String>,
    /// Operations already looked up this run, so a query that fails for some
    /// other reason doesn't download the web player every time.
    looked_up: Vec<&'static str>,
}

fn hash_cache() -> &'static Mutex<HashCache> {
    static CACHE: OnceLock<Mutex<HashCache>> = OnceLock::new();
    CACHE.get_or_init(|| {
        Mutex::new(HashCache {
            found: HashMap::new(),
            looked_up: Vec::new(),
        })
    })
}

fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(crate::http::client)
}

/// Runs a query and returns its `data`.
pub async fn query(session: &Session, operation: &Operation, variables: Value) -> Result<Value, String> {
    let hash = {
        let cache = hash_cache().lock().unwrap_or_else(|e| e.into_inner());
        cache.found.get(operation.name).cloned()
    }
    .unwrap_or_else(|| operation.hash.to_string());

    match send(session, operation, &hash, &variables).await {
        Err(QueryError::Rejected(message)) => {
            let Some(fresh) = look_up_hash(operation).await.filter(|fresh| *fresh != hash) else {
                return Err(message);
            };
            let data = send(session, operation, &fresh, &variables).await?;
            let mut cache = hash_cache().lock().unwrap_or_else(|e| e.into_inner());
            cache.found.insert(operation.name, fresh);
            Ok(data)
        }
        result => result.map_err(Into::into),
    }
}

async fn send(session: &Session, operation: &Operation, hash: &str, variables: &Value) -> Result<Value, QueryError> {
    let token = session
        .login5()
        .auth_token()
        .await
        .map_err(|e| QueryError::Other(format!("Couldn't get a Spotify token: {e}")))?;
    let body = json!({
        "variables": variables,
        "operationName": operation.name,
        "extensions": { "persistedQuery": { "version": 1, "sha256Hash": hash } },
    });
    let mut request = http()
        .post(ENDPOINT)
        .bearer_auth(&token.access_token)
        .header("app-platform", APP_PLATFORM)
        .header("spotify-app-version", SPOTIFY_SEMANTIC_VERSION)
        .json(&body);
    if let Ok(client_token) = session.spclient().client_token().await {
        request = request.header("client-token", client_token);
    }

    let response = request
        .send()
        .await
        .map_err(|e| QueryError::Other(format!("{} failed: {e}", operation.name)))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| QueryError::Other(format!("{} failed: {e}", operation.name)))?;
    let mut value: Value = serde_json::from_str(&text).unwrap_or(Value::Null);

    let error = value
        .pointer("/errors/0/message")
        .and_then(Value::as_str)
        .map(str::to_string);
    let data = value.get_mut("data").map(Value::take).filter(|data| !data.is_null());
    match (data, error) {
        (Some(data), _) if status.is_success() => Ok(data),
        (_, Some(error)) => Err(QueryError::Rejected(format!("{} failed: {error}", operation.name))),
        _ if status.is_client_error() && status.as_u16() != 401 && status.as_u16() != 429 => {
            Err(QueryError::Rejected(format!("{} failed ({status})", operation.name)))
        }
        _ => Err(QueryError::Other(format!("{} failed ({status})", operation.name))),
    }
}

/// Finds the hash the current web player uses for `operation`.
async fn look_up_hash(operation: &Operation) -> Option<String> {
    {
        let mut cache = hash_cache().lock().unwrap_or_else(|e| e.into_inner());
        if cache.looked_up.contains(&operation.name) {
            return None;
        }
        cache.looked_up.push(operation.name);
    }

    let page = fetch_text(WEB_PLAYER_URL).await?;
    let bundle = fetch_text(&bundle_url(&page)?).await?;
    if let Some(hash) = query_hash(&bundle, operation.name) {
        return Some(hash);
    }
    for url in chunk_urls(&bundle, operation.chunk?) {
        if let Some(hash) = fetch_text(&url).await.and_then(|chunk| query_hash(&chunk, operation.name)) {
            return Some(hash);
        }
    }
    None
}

async fn fetch_text(url: &str) -> Option<String> {
    let response = http()
        .get(url)
        .header(reqwest::header::USER_AGENT, BROWSER_USER_AGENT)
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    response.text().await.ok()
}

/// The page links `web-player.<hash>.js`, next to a stylesheet of the same name.
fn bundle_url(page: &str) -> Option<String> {
    page.match_indices(WEB_PLAYER_BUNDLE).find_map(|(at, _)| {
        let rest = &page[at + WEB_PLAYER_BUNDLE.len()..];
        let hash_len = rest.find(|c: char| !c.is_ascii_hexdigit())?;
        let hash = &rest[..hash_len];
        (hash_len > 0 && rest[hash_len..].starts_with(".js")).then(|| format!("{WEB_PLAYER_BUILD_URL}web-player.{hash}.js"))
    })
}

/// The web player declares each query as `"<name>","query","<sha256>"`.
fn query_hash(js: &str, name: &str) -> Option<String> {
    let needle = format!("\"{name}\",\"query\",\"");
    let start = js.find(&needle)? + needle.len();
    let hash = js.get(start..start + QUERY_HASH_LEN)?;
    hash.bytes().all(|b| b.is_ascii_hexdigit()).then(|| hash.to_string())
}

/// The bundle maps chunk ids to names (`7161:"xpui-routes-artist"`) and, once
/// for the JavaScript and once for the CSS, ids to file hashes
/// (`7161:"b0013a94"`). Both candidates are returned, since only one is a script.
fn chunk_urls(bundle: &str, chunk: &str) -> Vec<String> {
    let Some(name_at) = bundle.find(&format!(":\"{chunk}\"")) else {
        return Vec::new();
    };
    let id_start = bundle[..name_at]
        .rfind(|c: char| !c.is_ascii_digit())
        .map_or(0, |i| i + 1);
    let id = &bundle[id_start..name_at];
    if id.is_empty() {
        return Vec::new();
    }
    let key = format!("{id}:\"");
    bundle
        .match_indices(&key)
        .filter_map(|(at, _)| {
            if at > 0 && bundle.as_bytes()[at - 1].is_ascii_digit() {
                return None;
            }
            let start = at + key.len();
            let hash = bundle.get(start..start + CHUNK_HASH_LEN)?;
            let closed = bundle.as_bytes().get(start + CHUNK_HASH_LEN) == Some(&b'"');
            (closed && hash.bytes().all(|b| b.is_ascii_hexdigit()))
                .then(|| format!("{WEB_PLAYER_BUILD_URL}{chunk}.{hash}.js"))
        })
        .collect()
}

// Reading responses. Pathfinder's schema shifts between web player releases, so
// everything is looked up leniently: a missing field leaves a gap on the page
// instead of failing it.

pub fn text(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub fn list<'a>(value: &'a Value, pointer: &str) -> &'a [Value] {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// Large numbers (play counts) arrive as strings, the rest as numbers.
pub fn count(value: &Value, pointer: &str) -> Option<u64> {
    match value.pointer(pointer)? {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

pub fn id_from_uri(uri: &str) -> String {
    uri.rsplit(':').next().unwrap_or_default().to_string()
}

/// `pointer` names an image object, `{ sources: [{ url, width, height }] }`.
/// Header images call the size `maxWidth`/`maxHeight`, and some covers leave
/// it out, in which case it's read from the image id.
pub fn images(value: &Value, pointer: &str) -> Vec<Image> {
    list(value, &format!("{pointer}/sources"))
        .iter()
        .filter_map(|source| {
            let url = source.get("url")?.as_str()?.to_string();
            let size = |key: &str, alt: &str| {
                source
                    .get(key)
                    .or_else(|| source.get(alt))
                    .and_then(Value::as_u64)
                    .map(|n| n as u32)
                    .or_else(|| size_from_image_id(&url))
            };
            Some(Image {
                width: size("width", "maxWidth"),
                height: size("height", "maxHeight"),
                url,
            })
        })
        .collect()
}

/// Spotify's square image ids start with a code for their size, e.g.
/// `ab67616d00004851...` is a 64px album cover.
fn size_from_image_id(url: &str) -> Option<u32> {
    let id = url.rsplit('/').next()?;
    match id.get(8..16)? {
        "00004851" => Some(64),
        "0000f178" => Some(160),
        "00001e02" => Some(300),
        "00005174" => Some(320),
        "0000b273" | "0000e5eb" => Some(640),
        _ => None,
    }
}

/// `{ artists: { items: [{ uri, profile: { name } }] } }`
pub fn artists(value: &Value) -> Vec<Artist> {
    list(value, "/artists/items")
        .iter()
        .map(|artist| {
            let artist = artist.get("data").unwrap_or(artist);
            Artist {
                id: id_from_uri(&text(artist, "/uri")),
                name: text(artist, "/profile/name"),
            }
        })
        .collect()
}

pub fn album(value: &Value) -> Option<Album> {
    let uri = value.get("uri")?.as_str()?;
    let release_date = match value.pointer("/date/isoString").and_then(Value::as_str) {
        Some(iso) => Some(iso.chars().take(10).collect()),
        None => count(value, "/date/year").map(|year| year.to_string()),
    };
    Some(Album {
        id: id_from_uri(uri),
        name: text(value, "/name"),
        images: images(value, "/coverArt"),
        artists: artists(value),
        release_date,
        album_type: value.get("type").and_then(Value::as_str).map(str::to_lowercase),
        total_tracks: count(value, "/tracks/totalCount").map(|n| n as u32),
    })
}

/// Discographies list each release as a group of its editions, newest first.
pub fn grouped_releases(value: &Value, pointer: &str) -> Vec<Album> {
    list(value, &format!("{pointer}/items"))
        .iter()
        .filter_map(|group| list(group, "/releases/items").first().and_then(album))
        .collect()
}

pub fn track(value: &Value) -> Option<Track> {
    let uri = value.get("uri")?.as_str()?;
    if !uri.starts_with("spotify:track:") {
        return None;
    }
    let duration = count(value, "/duration/totalMilliseconds").or_else(|| count(value, "/trackDuration/totalMilliseconds"));
    Some(Track {
        id: id_from_uri(uri),
        name: text(value, "/name"),
        uri: uri.to_string(),
        duration_ms: duration.unwrap_or(0) as u32,
        artists: artists(value),
        album: value.get("albumOfTrack").and_then(album).unwrap_or_default(),
        added_at: None,
        position: None,
    })
}

pub fn is_explicit(track: &Value) -> bool {
    track.pointer("/contentRating/label").and_then(Value::as_str) == Some("EXPLICIT")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_query_hashes() {
        let js = r#"let u=new l.l("queryArtistOverview","query","9f8134ef565e78621f1e1793555bd6633c5ac144ae0f89604ed3ae3f80b3c8e6",null);"#;
        assert_eq!(
            query_hash(js, "queryArtistOverview").as_deref(),
            Some("9f8134ef565e78621f1e1793555bd6633c5ac144ae0f89604ed3ae3f80b3c8e6")
        );
        assert_eq!(query_hash(js, "queryArtistRelated"), None);
    }

    #[test]
    fn finds_the_main_bundle() {
        let page = r#"<link href="https://open.spotifycdn.com/cdn/build/web-player/web-player.46a308ba.css"/><script src="https://open.spotifycdn.com/cdn/build/web-player/vendor~web-player.67984718.js"></script><script src="https://open.spotifycdn.com/cdn/build/web-player/web-player.06a1e8e8.js"></script>"#;
        assert_eq!(
            bundle_url(page).as_deref(),
            Some("https://open.spotifycdn.com/cdn/build/web-player/web-player.06a1e8e8.js")
        );
    }

    #[test]
    fn reads_sizes_from_image_ids() {
        let cover = json!({ "sources": [
            { "url": "https://i.scdn.co/image/ab67616d00004851de79f330bc297af3fae736da" },
            { "url": "https://i.scdn.co/image/ab67616d0000b273de79f330bc297af3fae736da", "width": 600 },
            { "url": "https://example.com/unknown" },
        ] });
        let widths: Vec<_> = images(&cover, "").iter().map(|image| image.width).collect();
        assert_eq!(widths, vec![Some(64), Some(600), None]);
    }

    /// Downloads the web player: `cargo test -- --ignored` tells whether the
    /// hashes above are still the current ones.
    #[test]
    #[ignore]
    fn hashes_match_the_web_player() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        for operation in [&ARTIST_OVERVIEW, &ARTIST_DISCOGRAPHY_ALL, &PLAYLIST_CONTENTS] {
            let live = runtime.block_on(look_up_hash(operation));
            assert_eq!(live.as_deref(), Some(operation.hash), "{}", operation.name);
        }
    }

    #[test]
    fn finds_chunk_files() {
        let bundle = r#"{6883:"xpui-routes-cultural-moment-hub",7161:"xpui-routes-artist"}[e]+"."+{17161:"deadbeef",7161:"b0013a94"}[e]+".js",{7161:"b87b0052",12:"x"}[e]+".css""#;
        assert_eq!(
            chunk_urls(bundle, "xpui-routes-artist"),
            vec![
                format!("{WEB_PLAYER_BUILD_URL}xpui-routes-artist.b0013a94.js"),
                format!("{WEB_PLAYER_BUILD_URL}xpui-routes-artist.b87b0052.js"),
            ]
        );
    }
}
