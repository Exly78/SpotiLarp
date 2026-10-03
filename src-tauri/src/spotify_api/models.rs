use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(Option::deserialize(deserializer)?.unwrap_or_default())
}

fn skip_invalid<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    T: DeserializeOwned,
    D: Deserializer<'de>,
{
    let values = Option::<Vec<Value>>::deserialize(deserializer)?.unwrap_or_default();
    let count = values.len();
    let mut first_error = None;
    let items: Vec<T> = values
        .into_iter()
        .filter_map(|value| match serde_json::from_value(value) {
            Ok(item) => Some(item),
            Err(e) => {
                first_error.get_or_insert(e);
                None
            }
        })
        .collect();
    match first_error {
        Some(e) if items.is_empty() && count > 0 => Err(serde::de::Error::custom(e)),
        _ => Ok(items),
    }
}

pub struct Lenient<T>(pub Option<T>);

impl<'de, T: DeserializeOwned> Deserialize<'de> for Lenient<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        Ok(Self(serde_json::from_value(value).ok()))
    }
}

fn track_or_none<'de, D>(deserializer: D) -> Result<Option<Track>, D::Error>
where
    D: Deserializer<'de>,
{
    let Some(value) = Option::<Value>::deserialize(deserializer)? else {
        return Ok(None);
    };
    let kind = value.get("type").and_then(Value::as_str);
    if kind.is_some_and(|kind| kind != "track") {
        return Ok(None);
    }
    serde_json::from_value(value)
        .map(Some)
        .map_err(serde::de::Error::custom)
}

#[derive(Deserialize)]
#[serde(bound(deserialize = "T: DeserializeOwned"))]
pub struct Page<T> {
    #[serde(default, deserialize_with = "skip_invalid")]
    pub items: Vec<T>,
    #[serde(default)]
    pub total: u32,
}

#[derive(Deserialize)]
pub struct SearchResponse {
    pub tracks: Option<Page<Track>>,
    pub artists: Option<Page<ArtistDetails>>,
    pub albums: Option<Page<Album>>,
}

#[derive(Serialize)]
pub struct SearchResults {
    pub tracks: Vec<Track>,
    pub artists: Vec<ArtistDetails>,
    pub albums: Vec<Album>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Track {
    #[serde(default, deserialize_with = "null_as_default")]
    pub id: String,
    pub name: String,
    pub uri: String,
    pub duration_ms: u32,
    pub artists: Vec<Artist>,
    pub album: Album,
    #[serde(default, skip_deserializing)]
    pub added_at: Option<String>,
    #[serde(default, skip_deserializing)]
    pub position: Option<u32>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Artist {
    #[serde(default, deserialize_with = "null_as_default")]
    pub id: String,
    pub name: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ArtistDetails {
    pub id: String,
    pub name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub genres: Vec<String>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub images: Vec<Image>,

    #[serde(default)]
    pub followers: Option<Followers>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Followers {
    pub total: u32,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Album {
    #[serde(default, deserialize_with = "null_as_default")]
    pub id: String,
    pub name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub images: Vec<Image>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub artists: Vec<Artist>,
    #[serde(default)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub album_type: Option<String>,
    #[serde(default)]
    pub total_tracks: Option<u32>,
}

#[derive(Deserialize)]
pub struct AlbumTrack {
    #[serde(default, deserialize_with = "null_as_default")]
    pub id: String,
    pub name: String,
    pub uri: String,
    pub duration_ms: u32,
    pub artists: Vec<Artist>,
}

impl AlbumTrack {
    pub fn into_track(self, album: &Album) -> Track {
        Track {
            id: self.id,
            name: self.name,
            uri: self.uri,
            duration_ms: self.duration_ms,
            artists: self.artists,
            album: album.clone(),
            added_at: None,
            position: None,
        }
    }
}

#[derive(Serialize)]
pub struct AlbumDetails {
    #[serde(flatten)]
    pub album: Album,
    pub tracks: Vec<Track>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Image {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub images: Vec<Image>,

    #[serde(rename(deserialize = "items"), default)]
    pub track_count: PlaylistTrackCount,

    #[serde(default)]
    pub owner: Option<PlaylistOwner>,
    #[serde(default)]
    pub collaborative: bool,
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct PlaylistTrackCount {
    pub total: u32,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct PlaylistOwner {
    pub id: String,
    #[serde(default)]
    pub display_name: Option<String>,
}

#[derive(Deserialize)]
pub struct PlaylistTrackItem {

    #[serde(alias = "track", default, deserialize_with = "track_or_none")]
    pub item: Option<Track>,
    #[serde(default)]
    pub added_at: Option<String>,
}

impl PlaylistTrackItem {
    pub fn into_track(self) -> Option<Track> {
        let mut track = self.item?;
        track.added_at = self.added_at;
        Some(track)
    }
}

#[derive(Deserialize)]
pub struct SavedTrackItem {
    pub track: Option<Track>,
    #[serde(default, alias = "played_at")]
    pub added_at: Option<String>,
}

impl SavedTrackItem {
    pub fn into_track(self) -> Option<Track> {
        let mut track = self.track?;
        track.added_at = self.added_at;
        Some(track)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRACK: &str = r#"{"type":"track","id":"t1","name":"Song","uri":"spotify:track:t1","duration_ms":1000,"artists":[{"id":"a","name":"A"}],"album":{"name":"Al","images":null}}"#;

    #[test]
    fn playlist_page_skips_episodes_and_bad_items() {
        let json = format!(
            r#"{{"total":4,"items":[{{"item":{TRACK}}},{{"item":{{"type":"episode","id":"e","name":"Ep"}}}},{{"item":null}},null]}}"#
        );
        let page: Page<PlaylistTrackItem> = serde_json::from_str(&json).unwrap();
        let tracks: Vec<_> = page.items.into_iter().filter_map(|i| i.item).collect();
        assert_eq!(tracks.len(), 1);
        assert_eq!(page.total, 4);
    }

    #[test]
    fn lenient_items_keep_their_slots() {
        let json = format!(r#"{{"total":3,"items":[null,{{"item":{{"type":"track"}}}},{{"item":{TRACK}}}]}}"#);
        let page: Page<Lenient<PlaylistTrackItem>> = serde_json::from_str(&json).unwrap();
        assert_eq!(page.items.len(), 3);
        assert!(page.items[0].0.is_none() && page.items[1].0.is_none());
        assert!(page.items[2].0.as_ref().is_some_and(|item| item.item.is_some()));
    }

    #[test]
    fn page_errors_when_every_item_is_invalid() {
        let result = serde_json::from_str::<Page<Track>>(r#"{"items":[{"name":"no uri"}]}"#);
        assert!(result.is_err());
    }
}
