use std::sync::Arc;

use librespot_core::session::Session;
use librespot_core::{SpotifyId, SpotifyUri};
use librespot_metadata::image::ImageSize;
use librespot_metadata::{Metadata, Track as TrackMetadata};
use librespot_protocol::autoplay_context_request::AutoplayContextRequest;
use librespot_protocol::context::Context;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::spotify_api::models::{Album, Artist, Image, Track};

const BATCH_SIZE: usize = 25;
const MAX_CONCURRENT_LOOKUPS: usize = 8;
const DEFAULT_IMAGE_URL: &str = "https://i.scdn.co/image/{file_id}";

const MAX_STATION_TRACKS: usize = 100;

pub async fn tracks(session: &Session, seed_uri: &str, recent_uris: Vec<String>) -> Result<Vec<Track>, String> {
    let request = AutoplayContextRequest {
        context_uri: Some(seed_uri.to_string()),
        recent_track_uri: recent_uris.clone(),
        ..Default::default()
    };
    let context = session
        .spclient()
        .get_autoplay_context(&request)
        .await
        .map_err(|e| format!("Couldn't get songs to autoplay: {e}"))?;
    let uris = context_track_uris(&context, &recent_uris, BATCH_SIZE);
    Ok(look_up(session, uris).await)
}

/// The radio station Spotify mixes for a song, playlist, album or artist
/// (a fresh mix each time it's asked).
pub async fn station(session: &Session, seed_uri: &str) -> Result<Vec<Track>, String> {
    let station = station_uri(seed_uri).ok_or_else(|| "There's no radio for this".to_string())?;
    let context = session
        .spclient()
        .get_context(&station)
        .await
        .map_err(|e| format!("Couldn't load this radio: {e}"))?;
    let uris = context_track_uris(&context, &[], MAX_STATION_TRACKS);
    if uris.is_empty() {
        return Err("Spotify has no songs for this radio".to_string());
    }
    Ok(look_up(session, uris).await)
}

/// Songs by URI (others are skipped), in the order asked.
pub async fn tracks_by_uri(session: &Session, uris: &[String]) -> Vec<Track> {
    let uris = uris
        .iter()
        .filter_map(|uri| SpotifyUri::from_uri(uri).ok())
        .filter(|uri| matches!(uri, SpotifyUri::Track { .. }))
        .collect();
    look_up(session, uris).await
}

fn station_uri(seed_uri: &str) -> Option<String> {
    let mut parts = seed_uri.split(':');
    let (Some("spotify"), Some(kind), Some(id), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return None;
    };
    let valid_id = id.len() == 22 && id.chars().all(|c| c.is_ascii_alphanumeric());
    (valid_id && matches!(kind, "track" | "playlist" | "album" | "artist")).then(|| format!("spotify:station:{kind}:{id}"))
}

/// Each song in a resolved context once, named by URI or (on some stations) by raw id.
fn context_track_uris(context: &Context, skip: &[String], limit: usize) -> Vec<SpotifyUri> {
    let mut uris: Vec<SpotifyUri> = Vec::new();
    for track in context.pages.iter().flat_map(|page| &page.tracks) {
        let uri = match track.uri.as_deref().filter(|uri| !uri.is_empty()) {
            Some(uri) => SpotifyUri::from_uri(uri).ok(),
            None => track
                .gid
                .as_deref()
                .and_then(|gid| SpotifyId::from_raw(gid).ok())
                .map(|id| SpotifyUri::Track { id }),
        };
        let Some(uri) = uri.filter(|uri| matches!(uri, SpotifyUri::Track { .. })) else {
            continue;
        };
        let Ok(text) = uri.to_uri() else {
            continue;
        };
        if skip.iter().any(|recent| *recent == text) || uris.contains(&uri) {
            continue;
        }
        uris.push(uri);
        if uris.len() == limit {
            break;
        }
    }
    uris
}

async fn look_up(session: &Session, uris: Vec<SpotifyUri>) -> Vec<Track> {
    let image_url = session
        .get_user_attribute("image-url")
        .unwrap_or_else(|| DEFAULT_IMAGE_URL.to_string());
    let limiter = Arc::new(Semaphore::new(MAX_CONCURRENT_LOOKUPS));
    let mut lookups = JoinSet::new();
    for (index, uri) in uris.into_iter().enumerate() {
        let session = session.clone();
        let limiter = limiter.clone();
        lookups.spawn(async move {
            let _permit = limiter.acquire_owned().await.ok()?;
            let metadata = TrackMetadata::get(&session, &uri).await.ok()?;
            Some((index, metadata))
        });
    }
    let mut found = Vec::new();
    while let Some(joined) = lookups.join_next().await {
        if let Ok(Some((index, metadata))) = joined {
            if let Some(track) = to_track(metadata, &image_url) {
                found.push((index, track));
            }
        }
    }
    found.sort_by_key(|(index, _)| *index);
    found.into_iter().map(|(_, track)| track).collect()
}

fn to_track(metadata: TrackMetadata, image_url: &str) -> Option<Track> {
    if metadata.duration <= 0 {
        return None;
    }
    let artists = |list: &librespot_metadata::artist::Artists| -> Vec<Artist> {
        list.iter()
            .map(|artist| Artist {
                id: artist.id.to_id().unwrap_or_default(),
                name: artist.name.clone(),
            })
            .collect()
    };
    let images = metadata
        .album
        .covers
        .iter()
        .filter_map(|cover| {
            let (width, height) = match (cover.width, cover.height) {
                (w, h) if w > 0 && h > 0 => (w as u32, h as u32),
                _ => {
                    let side = nominal_size(cover.size);
                    (side, side)
                }
            };
            Some(Image {
                url: image_url.replace("{file_id}", &cover.id.to_base16().ok()?),
                width: Some(width),
                height: Some(height),
            })
        })
        .collect();
    Some(Track {
        id: metadata.id.to_id().ok()?,
        uri: metadata.id.to_uri().ok()?,
        name: metadata.name,
        duration_ms: metadata.duration as u32,
        artists: artists(&metadata.artists),
        album: Album {
            id: metadata.album.id.to_id().unwrap_or_default(),
            name: metadata.album.name,
            images,
            artists: artists(&metadata.album.artists),
            release_date: None,
            album_type: None,
            total_tracks: None,
        },
        added_at: None,
        position: None,
    })
}

fn nominal_size(size: ImageSize) -> u32 {
    match size {
        ImageSize::SMALL => 64,
        ImageSize::DEFAULT => 300,
        ImageSize::LARGE => 640,
        ImageSize::XLARGE => 1280,
    }
}

#[cfg(test)]
mod tests {
    use super::station_uri;

    #[test]
    fn stations_exist_for_songs_playlists_albums_and_artists() {
        let id = "4uLU6hMCjMI75M1A2tKUQC";
        assert_eq!(station_uri(&format!("spotify:track:{id}")), Some(format!("spotify:station:track:{id}")));
        assert_eq!(station_uri(&format!("spotify:artist:{id}")), Some(format!("spotify:station:artist:{id}")));
        assert_eq!(station_uri(&format!("spotify:show:{id}")), None);
        assert_eq!(station_uri("spotify:track:short"), None);
        assert_eq!(station_uri(&format!("spotify:track:{id}:extra")), None);
    }
}
