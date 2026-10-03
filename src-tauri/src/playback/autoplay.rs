use std::sync::Arc;

use librespot_core::session::Session;
use librespot_core::SpotifyUri;
use librespot_metadata::image::ImageSize;
use librespot_metadata::{Metadata, Track as TrackMetadata};
use librespot_protocol::autoplay_context_request::AutoplayContextRequest;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::spotify_api::models::{Album, Artist, Image, Track};

const BATCH_SIZE: usize = 25;
const MAX_CONCURRENT_LOOKUPS: usize = 8;
const DEFAULT_IMAGE_URL: &str = "https://i.scdn.co/image/{file_id}";

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

    let mut uris: Vec<SpotifyUri> = Vec::new();
    for uri in context
        .pages
        .iter()
        .flat_map(|page| &page.tracks)
        .filter_map(|track| track.uri.as_deref())
    {
        if !uri.starts_with("spotify:track:") || recent_uris.iter().any(|recent| recent == uri) {
            continue;
        }
        if let Ok(uri) = SpotifyUri::from_uri(uri) {
            if !uris.contains(&uri) {
                uris.push(uri);
            }
        }
        if uris.len() == BATCH_SIZE {
            break;
        }
    }

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
    Ok(found.into_iter().map(|(_, track)| track).collect())
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
