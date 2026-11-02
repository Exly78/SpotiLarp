//! A Spotify user's profile page, as the desktop app builds it: the
//! user-profile-view endpoints of the playback session's spclient. The Web API
//! has nothing like the follower lists or "Recently played artists".

use librespot_core::session::Session;
use serde_json::Value;

use super::models::{ProfileEntry, ProfilePlaylist, UserProfile};
use super::pathfinder::{count, id_from_uri, list, text};

/// How many public playlists and recently played artists to ask for.
const PLAYLIST_LIMIT: u32 = 50;
const ARTIST_LIMIT: u32 = 20;
const IMAGE_BASE: &str = "https://i.scdn.co/image/";

pub async fn profile(session: &Session, user_id: &str) -> Result<UserProfile, String> {
    let spclient = session.spclient();
    let (profile, followers, following) = tokio::join!(
        spclient.get_user_profile(user_id, Some(PLAYLIST_LIMIT), Some(ARTIST_LIMIT)),
        spclient.get_user_followers(user_id),
        spclient.get_user_following(user_id),
    );
    let profile: Value = profile
        .map_err(|e| format!("Couldn't load the profile: {e}"))
        .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|e| format!("Unexpected profile from Spotify: {e}")))?;
    Ok(UserProfile {
        id: user_id.to_string(),
        name: text(&profile, "/name"),
        image: image(&profile),
        color: profile
            .get("color")
            .and_then(Value::as_u64)
            .map(|rgb| format!("#{:06x}", rgb & 0xff_ffff)),
        followers: count(&profile, "/followers_count").unwrap_or(0),
        following: count(&profile, "/following_count").unwrap_or(0),
        playlist_count: count(&profile, "/total_public_playlists_count").unwrap_or(0),
        is_current_user: profile.get("is_current_user").and_then(Value::as_bool).unwrap_or(false),
        playlists: list(&profile, "/public_playlists").iter().filter_map(playlist).collect(),
        recently_played_artists: list(&profile, "/recently_played_artists").iter().filter_map(entry).collect(),
        followers_list: entries(followers),
        following_list: entries(following),
    })
}

/// A follower or following list. They're extras: a profile that hides them
/// still shows, without them.
fn entries<B: AsRef<[u8]>, E>(result: Result<B, E>) -> Vec<ProfileEntry> {
    result
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(bytes.as_ref()).ok())
        .map(|value| list(&value, "/profiles").iter().filter_map(entry).collect())
        .unwrap_or_default()
}

fn image(value: &Value) -> Option<String> {
    Some(text(value, "/image_url")).filter(|url| url.starts_with("https://"))
}

/// An artist or user in one of the profile's lists.
fn entry(value: &Value) -> Option<ProfileEntry> {
    let uri = value.get("uri")?.as_str()?;
    let kind = match uri.split(':').nth(1)? {
        "artist" => "artist",
        "user" => "user",
        _ => return None,
    };
    Some(ProfileEntry {
        kind,
        id: id_from_uri(uri),
        name: text(value, "/name"),
        image: image(value),
        followers: count(value, "/followers_count"),
    })
}

/// A playlist's cover is either an image URL or, for one Spotify draws from
/// its first tracks, `spotify:mosaic:<id>:<id>:<id>:<id>`.
fn playlist(value: &Value) -> Option<ProfilePlaylist> {
    let uri = value.get("uri")?.as_str()?;
    if !uri.starts_with("spotify:playlist:") {
        return None;
    }
    let cover = text(value, "/image_url");
    let images = match cover.strip_prefix("spotify:mosaic:") {
        Some(ids) => ids.split(':').filter(|id| !id.is_empty()).map(|id| format!("{IMAGE_BASE}{id}")).collect(),
        None if cover.starts_with("https://") => vec![cover],
        None => Vec::new(),
    };
    Some(ProfilePlaylist {
        id: id_from_uri(uri),
        name: text(value, "/name"),
        images,
        owner: Some(text(value, "/owner_name")).filter(|owner| !owner.is_empty()),
    })
}
