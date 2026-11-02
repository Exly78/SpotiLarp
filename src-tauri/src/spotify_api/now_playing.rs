//! The Now Playing panel's extras (the artist, the song's credits, the artist's
//! concerts and the song's Canvas) from the one pathfinder query Spotify's own
//! Now Playing view makes per song.

use librespot_core::session::Session;
use serde_json::{json, Value};

use super::artist;
use super::models::{Canvas, Concert, Credit, NowPlayingInfo};
use super::pathfinder::{self, count, id_from_uri, list, text};

/// Spotify's own view asks for 10, then offers the rest behind "Show all".
const CONTRIBUTORS_LIMIT: u32 = 30;

pub async fn info(session: &Session, track_id: &str, artist_id: &str) -> Result<NowPlayingInfo, String> {
    let data = pathfinder::query(
        session,
        &pathfinder::NPV_ARTIST,
        json!({
            "artistUri": format!("spotify:artist:{artist_id}"),
            "trackUri": format!("spotify:track:{track_id}"),
            "contributorsLimit": CONTRIBUTORS_LIMIT,
            "contributorsOffset": 0,
            "enableRelatedVideos": false,
            "enableRelatedAudioTracks": false,
        }),
    )
    .await?;
    let artist_data = data
        .get("artistUnion")
        .filter(|artist| artist["__typename"] == "Artist")
        .ok_or_else(|| "Spotify doesn't have a page for this artist".to_string())?;
    let track = data
        .get("trackUnion")
        .filter(|track| track["__typename"] == "Track")
        .unwrap_or(&Value::Null);

    Ok(NowPlayingInfo {
        artist: artist::about(artist_data, artist_id),
        credits: credits(track),
        credit_sources: list(track, "/creditsTrait/sources/items")
            .iter()
            .map(|source| text(source, "/name"))
            .filter(|name| !name.is_empty())
            .collect(),
        concerts: list(artist_data, "/goods/concerts/items")
            .iter()
            .filter_map(|item| concert(item.get("data")?))
            .collect(),
        concert_count: count(artist_data, "/goods/concerts/totalCount").unwrap_or(0),
        canvas: canvas(track),
    })
}

/// Spotify lists a person once per role ("Composer", then "Lyricist"); they're
/// shown once, with their roles together, in the order they first appear.
fn credits(track: &Value) -> Vec<Credit> {
    let mut credits: Vec<Credit> = Vec::new();
    for item in list(track, "/creditsTrait/contributors/items") {
        let name = text(item, "/name");
        if name.is_empty() {
            continue;
        }
        let role = text(item, "/role");
        let artist_id = item.get("uri").and_then(Value::as_str).map(id_from_uri);
        match credits.iter_mut().find(|credit| credit.name == name && credit.artist_id == artist_id) {
            Some(credit) => {
                if !role.is_empty() && !credit.roles.contains(&role) {
                    credit.roles.push(role);
                }
            }
            None => credits.push(Credit {
                name,
                roles: if role.is_empty() { Vec::new() } else { vec![role] },
                artist_id,
            }),
        }
    }
    credits
}

fn concert(data: &Value) -> Option<Concert> {
    if data["__typename"] != "ConcertV2" {
        return None;
    }
    Some(Concert {
        id: id_from_uri(data.get("uri")?.as_str()?),
        title: text(data, "/title"),
        date: text(data, "/startDateIsoString"),
        venue: text(data, "/location/name"),
        city: text(data, "/location/city"),
        festival: data.get("festival").and_then(Value::as_bool).unwrap_or(false),
    })
}

fn canvas(track: &Value) -> Option<Canvas> {
    let url = text(track, "/canvas/url");
    if url.is_empty() {
        return None;
    }
    // VIDEO, VIDEO_LOOPING, VIDEO_LOOPING_RANDOM; the rest are IMAGE and GIF.
    let is_video = text(track, "/canvas/type").starts_with("VIDEO");
    Some(Canvas { url, is_video })
}
