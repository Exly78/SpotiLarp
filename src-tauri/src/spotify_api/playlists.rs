use std::time::{SystemTime, UNIX_EPOCH};

use librespot_core::session::Session;
use librespot_core::SpotifyId;
use librespot_protocol::playlist4_external::{op, Add, ChangeInfo, Delta, Item, ListChanges, Op, Rem, SelectedListContent};
use protobuf::Message;
use reqwest::Method;
use serde::Serialize;
use serde_json::{json, Value};

use super::client::SpotifyClient;
use super::library;
use super::models::{Lenient, Playlist, PlaylistTrackItem, SavedTrackItem, Track};
use super::pathfinder;

const PLAYLISTS_PAGE_SIZE: u32 = 50;
const PLAYLIST_ITEMS_PAGE_SIZE: u32 = 100;
const SESSION_PLAYLIST_PAGE_SIZE: u32 = 100;
const MAX_SESSION_PLAYLIST_ITEMS: u32 = 10_000;
const SAVED_TRACKS_PAGE_SIZE: u32 = 50;
const ADD_ITEMS_CHUNK_SIZE: usize = 100;
const ROOTLIST_PAGE_SIZE: usize = 500;
const MAX_ROOTLIST_PAGES: usize = 20;

/// One row of the library's playlist order, folders included. The Web API
/// has no folders; the Spotify apps read them from the "rootlist".
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RootlistEntry {
    Playlist { id: String },
    FolderStart { id: String, name: String },
    FolderEnd,
}

pub async fn rootlist(session: &Session) -> Result<Vec<RootlistEntry>, String> {
    let mut uris = Vec::new();
    for page in 0..MAX_ROOTLIST_PAGES {
        let from = page * ROOTLIST_PAGE_SIZE;
        let bytes = session
            .spclient()
            .get_rootlist(from, Some(ROOTLIST_PAGE_SIZE))
            .await
            .map_err(|e| format!("Couldn't read your playlist folders: {e}"))?;
        let list = SelectedListContent::parse_from_bytes(&bytes).map_err(|e| format!("Couldn't read your playlist folders: {e}"))?;
        let count = list.contents.items.len();
        uris.extend(list.contents.items.iter().map(|item| item.uri().to_string()));
        let total = if list.has_length() { list.length() as usize } else { 0 };
        if count < ROOTLIST_PAGE_SIZE || from + count >= total {
            break;
        }
    }
    Ok(parse_rootlist(&uris))
}

fn parse_rootlist(uris: &[String]) -> Vec<RootlistEntry> {
    let mut entries = Vec::new();
    let mut depth = 0usize;
    for uri in uris {
        if let Some(rest) = uri.strip_prefix("spotify:start-group:") {
            let (id, name) = rest.split_once(':').unwrap_or((rest, ""));
            entries.push(RootlistEntry::FolderStart { id: id.to_string(), name: decode_folder_name(name) });
            depth += 1;
        } else if uri.starts_with("spotify:end-group:") {
            if depth > 0 {
                entries.push(RootlistEntry::FolderEnd);
                depth -= 1;
            }
        } else if let Some(id) = uri.strip_prefix("spotify:playlist:") {
            entries.push(RootlistEntry::Playlist { id: id.to_string() });
        }
    }
    // A folder Spotify never closed still closes.
    entries.extend(std::iter::repeat_n(RootlistEntry::FolderEnd, depth));
    entries
}

/// Folder names arrive percent-encoded, with `+` for a space.
fn decode_folder_name(encoded: &str) -> String {
    let bytes = encoded.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |at: usize| bytes.get(at).and_then(|byte| (*byte as char).to_digit(16));
        match (bytes[i], hex(i + 1), hex(i + 2)) {
            (b'+', _, _) => {
                out.push(b' ');
                i += 1;
            }
            (b'%', Some(high), Some(low)) => {
                out.push((high << 4 | low) as u8);
                i += 3;
            }
            (byte, _, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub async fn get_playlists(client: &mut SpotifyClient) -> Result<Vec<Playlist>, String> {
    client.get_all_pages("/me/playlists", PLAYLISTS_PAGE_SIZE).await
}

pub async fn get_playlist(client: &mut SpotifyClient, playlist_id: &str) -> Result<Playlist, String> {
    client.get_json(&format!("/playlists/{playlist_id}"), &[]).await
}

pub async fn get_playlist_tracks(
    client: &mut SpotifyClient,
    playlist_id: &str,
) -> Result<Vec<Track>, String> {
    let path = format!("/playlists/{playlist_id}/items");
    let items: Vec<Lenient<PlaylistTrackItem>> = client.get_all_pages(&path, PLAYLIST_ITEMS_PAGE_SIZE).await?;
    if !items.is_empty() && items.iter().all(|item| item.0.is_none()) {
        return Err("Couldn't read this playlist's tracks".to_string());
    }
    Ok(items
        .into_iter()
        .enumerate()
        .filter_map(|(position, item)| {
            let mut track = item.0?.into_track()?;
            track.position = Some(position as u32);
            Some(track)
        })
        .collect())
}

/// Reads any playlist, including the ones the Web API won't show (editorial
/// playlists and other people's), the way the Spotify apps do.
pub async fn get_playlist_tracks_via_session(session: &Session, playlist_id: &str) -> Result<Vec<Track>, String> {
    let uri = format!("spotify:playlist:{playlist_id}");
    let mut tracks = Vec::new();
    let mut offset = 0;
    while offset < MAX_SESSION_PLAYLIST_ITEMS {
        let variables = json!({ "uri": uri, "offset": offset, "limit": SESSION_PLAYLIST_PAGE_SIZE });
        let data = pathfinder::query(session, &pathfinder::PLAYLIST_CONTENTS, variables).await?;
        let content = data
            .pointer("/playlistV2/content")
            .ok_or_else(|| "Couldn't read this playlist's tracks".to_string())?;
        let items = pathfinder::list(content, "/items");
        for (index, item) in items.iter().enumerate() {
            let Some(mut track) = item.pointer("/itemV2/data").and_then(pathfinder::track) else {
                continue;
            };
            track.position = Some(offset + index as u32);
            track.added_at = item
                .pointer("/addedAt/isoString")
                .and_then(Value::as_str)
                .map(str::to_string);
            tracks.push(track);
        }
        offset += items.len() as u32;
        let total = pathfinder::count(content, "/totalCount").unwrap_or(0);
        if items.is_empty() || u64::from(offset) >= total {
            break;
        }
    }
    Ok(tracks)
}

// The Web API won't put local files in playlists (or take them out), but the
// Spotify apps do it by sending changes to the playlist service, which takes
// any URI. These do the same as the librespot session.

pub async fn add_local_tracks(session: &Session, playlist_id: &str, uris: &[String]) -> Result<(), String> {
    let playlist = playlist_contents(session, playlist_id).await?;
    let add = Add {
        items: uris.iter().map(|uri| playlist_item(uri)).collect(),
        add_last: Some(true),
        ..Default::default()
    };
    let op = Op {
        kind: Some(op::Kind::ADD.into()),
        add: Some(add).into(),
        ..Default::default()
    };
    send_changes(session, playlist_id, playlist.revision(), vec![op]).await
}

/// Removes every copy of a local file, as the Web API does for songs.
pub async fn remove_local_track(session: &Session, playlist_id: &str, uri: &str) -> Result<(), String> {
    let playlist = playlist_contents(session, playlist_id).await?;
    let items = &playlist.contents.items;
    // Last to first, so each removal leaves the earlier positions as they were.
    let ops: Vec<Op> = (0..items.len())
        .rev()
        .filter(|&index| items[index].uri() == uri)
        .map(|index| Op {
            kind: Some(op::Kind::REM.into()),
            rem: Some(Rem {
                from_index: Some(index as i32),
                length: Some(1),
                items: vec![playlist_item(uri)],
                ..Default::default()
            })
            .into(),
            ..Default::default()
        })
        .collect();
    if ops.is_empty() {
        let message = if playlist.contents.truncated() {
            "This playlist is too long to remove local files from here"
        } else {
            "That file isn't in this playlist anymore"
        };
        return Err(message.to_string());
    }
    send_changes(session, playlist_id, playlist.revision(), ops).await
}

async fn playlist_contents(session: &Session, playlist_id: &str) -> Result<SelectedListContent, String> {
    let id = SpotifyId::from_base62(playlist_id).map_err(|_| format!("\"{playlist_id}\" isn't a playlist id"))?;
    let bytes = session
        .spclient()
        .get_playlist(&id)
        .await
        .map_err(|e| format!("Couldn't read the playlist: {e}"))?;
    SelectedListContent::parse_from_bytes(&bytes).map_err(|e| format!("Couldn't read the playlist: {e}"))
}

fn playlist_item(uri: &str) -> Item {
    Item {
        uri: Some(uri.to_string()),
        ..Default::default()
    }
}

async fn send_changes(session: &Session, playlist_id: &str, revision: &[u8], ops: Vec<Op>) -> Result<(), String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as i64);
    let changes = ListChanges {
        base_revision: Some(revision.to_vec()),
        deltas: vec![Delta {
            ops,
            info: Some(ChangeInfo {
                timestamp: Some(timestamp),
                ..Default::default()
            })
            .into(),
            ..Default::default()
        }],
        ..Default::default()
    };
    session
        .spclient()
        .request_with_protobuf(&Method::POST, &format!("/playlist/v2/playlist/{playlist_id}/changes"), None, &changes)
        .await
        .map(|_| ())
        .map_err(|e| format!("Spotify didn't accept the change: {e}"))
}

pub async fn get_liked_songs(client: &mut SpotifyClient) -> Result<Vec<Track>, String> {
    let items: Vec<SavedTrackItem> = client.get_all_pages("/me/tracks", SAVED_TRACKS_PAGE_SIZE).await?;
    Ok(items.into_iter().filter_map(SavedTrackItem::into_track).collect())
}

pub async fn add_tracks(client: &mut SpotifyClient, playlist_id: &str, uris: &[String]) -> Result<(), String> {
    let path = format!("/playlists/{playlist_id}/items");
    for chunk in uris.chunks(ADD_ITEMS_CHUNK_SIZE) {
        let _: Value = client.send_json(Method::POST, &path, &json!({ "uris": chunk })).await?;
    }
    Ok(())
}

/// Removes every copy of each song.
pub async fn remove_tracks(client: &mut SpotifyClient, playlist_id: &str, uris: &[String]) -> Result<(), String> {
    let path = format!("/playlists/{playlist_id}/items");
    for chunk in uris.chunks(ADD_ITEMS_CHUNK_SIZE) {
        let items: Vec<Value> = chunk.iter().map(|uri| json!({ "uri": uri })).collect();
        let _: Value = client.send_json(Method::DELETE, &path, &json!({ "items": items })).await?;
    }
    Ok(())
}

pub async fn create_playlist(client: &mut SpotifyClient, name: &str) -> Result<Playlist, String> {
    client
        .send_json(Method::POST, "/me/playlists", &json!({ "name": name, "public": false }))
        .await
}

pub async fn rename_playlist(client: &mut SpotifyClient, playlist_id: &str, name: &str) -> Result<(), String> {
    let path = format!("/playlists/{playlist_id}");
    client.send_json(Method::PUT, &path, &json!({ "name": name })).await
}

/// Spotify won't let apps clear a description, so an empty one leaves it as it was.
pub async fn update_details(
    client: &mut SpotifyClient,
    playlist_id: &str,
    name: &str,
    description: &str,
    public: bool,
) -> Result<(), String> {
    let path = format!("/playlists/{playlist_id}");
    let mut body = json!({ "name": name, "public": public });
    if !description.is_empty() {
        body["description"] = json!(description);
    }
    client.send_json(Method::PUT, &path, &body).await
}

pub async fn remove_playlist(client: &mut SpotifyClient, playlist_id: &str) -> Result<(), String> {
    library::remove(client, &format!("spotify:playlist:{playlist_id}")).await
}

pub async fn move_track(
    client: &mut SpotifyClient,
    playlist_id: &str,
    from: u32,
    insert_before: u32,
) -> Result<(), String> {
    let path = format!("/playlists/{playlist_id}/items");
    let body = json!({ "range_start": from, "insert_before": insert_before, "range_length": 1 });
    let _: Value = client.send_json(Method::PUT, &path, &body).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rootlists_turn_into_folders() {
        let uris: Vec<String> = [
            "spotify:playlist:a",
            "spotify:start-group:f1:Road+Trip%20%F0%9F%9A%97",
            "spotify:playlist:b",
            "spotify:start-group:f2:Inner",
            "spotify:playlist:c",
            "spotify:end-group:f2",
            "spotify:end-group:f1",
            "spotify:end-group:stray",
            "spotify:start-group:f3:Unclosed",
            "spotify:playlist:d",
        ]
        .map(String::from)
        .to_vec();
        use RootlistEntry::*;
        assert_eq!(
            parse_rootlist(&uris),
            vec![
                Playlist { id: "a".into() },
                FolderStart { id: "f1".into(), name: "Road Trip 🚗".into() },
                Playlist { id: "b".into() },
                FolderStart { id: "f2".into(), name: "Inner".into() },
                Playlist { id: "c".into() },
                FolderEnd,
                FolderEnd,
                FolderStart { id: "f3".into(), name: "Unclosed".into() },
                Playlist { id: "d".into() },
                FolderEnd,
            ]
        );
    }

    #[test]
    fn broken_escapes_stay_as_they_are() {
        assert_eq!(decode_folder_name("100%"), "100%");
        assert_eq!(decode_folder_name("%zz%é"), "%zz%é");
    }
}
