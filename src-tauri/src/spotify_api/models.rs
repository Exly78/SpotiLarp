use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SearchResponse {
    pub tracks: Option<TrackPage>,
}

#[derive(Deserialize)]
pub struct TrackPage {
    pub items: Vec<Track>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub duration_ms: u32,
    pub artists: Vec<Artist>,
    pub album: Album,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Artist {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ArtistDetails {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
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
    pub name: String,
    #[serde(default)]
    pub images: Vec<Image>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Image {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Deserialize)]
pub struct PlaylistsResponse {
    pub items: Vec<Playlist>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub images: Vec<Image>,
    
    #[serde(rename(deserialize = "items"))]
    pub track_count: PlaylistTrackCount,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct PlaylistTrackCount {
    pub total: u32,
}

#[derive(Deserialize)]
pub struct PlaylistTracksResponse {
    pub items: Vec<PlaylistTrackItem>,
}

#[derive(Deserialize)]
pub struct PlaylistTrackItem {
    
    #[serde(alias = "track")]
    pub item: Option<Track>,
}

#[derive(Deserialize)]
pub struct SavedTracksResponse {
    pub items: Vec<SavedTrackItem>,
}

#[derive(Deserialize)]
pub struct SavedTrackItem {
    pub track: Option<Track>,
}
