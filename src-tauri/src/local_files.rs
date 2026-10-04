use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use sha2::{Digest, Sha256};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTagKey, StandardVisualKey, Tag, Visual};
use symphonia::core::probe::{Hint, ProbeResult};
use tauri::http::{header, Request, Response, StatusCode};

use crate::playback::local::LocalTrack;
use crate::spotify_api::models::{Album, Artist, Image, Track};

pub const COVER_SCHEME: &str = "localcover";
pub const URI_PREFIX: &str = "spotify:local:";

const EXTENSIONS: &[&str] = &["mp3", "flac", "m4a", "mp4", "aac", "ogg", "wav"];
const FOLDER_COVER_NAMES: &[&str] = &["cover", "folder", "front", "album"];
const FOLDER_COVER_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png"];
const COVER_TYPES: &[(&str, &str)] = &[
    ("jpg", "image/jpeg"),
    ("png", "image/png"),
    ("gif", "image/gif"),
    ("webp", "image/webp"),
    ("bmp", "image/bmp"),
];

#[derive(Clone, Default, Serialize)]
pub struct LocalLibrary {
    pub folders: Vec<String>,
    pub tracks: Vec<Track>,
    pub skipped: u32,
}

#[derive(Default)]
pub struct LocalIndex {
    pub library: LocalLibrary,
    paths: HashMap<String, (usize, PathBuf)>,
}

impl LocalIndex {
    pub fn is_scan_of(&self, folders: &[String]) -> bool {
        self.library.folders == folders
    }

    pub fn find(&self, uri: &str) -> Option<LocalTrack> {
        // Spotify may round a local file's length differently in playlists it saved.
        let near = || {
            let (rest, secs) = uri.rsplit_once(':')?;
            let secs: u64 = secs.parse().ok()?;
            [secs.checked_sub(1), Some(secs + 1)]
                .into_iter()
                .flatten()
                .find_map(|secs| self.paths.get(&format!("{rest}:{secs}")))
        };
        let (i, path) = self.paths.get(uri).or_else(near)?;
        let track = self.library.tracks.get(*i)?;
        Some(LocalTrack {
            path: path.clone(),
            uri: track.uri.clone(),
            name: track.name.clone(),
            artists: track.artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "),
            album: track.album.name.clone(),
            cover_url: track.album.images.first().map(|image| image.url.clone()),
            duration_ms: track.duration_ms,
        })
    }
}

fn covers_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|base| base.join("SpotiLarp").join("covers"))
}

fn cover_url(file_name: &str) -> String {
    if cfg!(windows) {
        format!("http://{COVER_SCHEME}.localhost/{file_name}")
    } else {
        format!("{COVER_SCHEME}://localhost/{file_name}")
    }
}

pub fn cover_content_type(file_name: &str) -> Option<&'static str> {
    let (hash, extension) = file_name.split_once('.')?;
    if hash.len() != 32 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    COVER_TYPES
        .iter()
        .find(|(ext, _)| *ext == extension)
        .map(|(_, content_type)| *content_type)
}

pub fn cover_file(url: &str) -> Option<PathBuf> {
    let file_name = url
        .strip_prefix(&format!("http://{COVER_SCHEME}.localhost/"))
        .or_else(|| url.strip_prefix(&format!("{COVER_SCHEME}://localhost/")))?;
    cover_content_type(file_name)?;
    Some(covers_dir()?.join(file_name))
}

pub fn serve_cover(request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let file_name = request.uri().path().trim_start_matches('/');
    let found = cover_content_type(file_name).and_then(|content_type| {
        let bytes = fs::read(covers_dir()?.join(file_name)).ok()?;
        Some((content_type, bytes))
    });
    let response = match found {
        Some((content_type, bytes)) => Response::builder()
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CACHE_CONTROL, "max-age=31536000, immutable")
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .body(bytes),
        None => Response::builder().status(StatusCode::NOT_FOUND).body(Vec::new()),
    };
    response.unwrap_or_default()
}

pub fn scan(folders: Vec<String>) -> LocalIndex {
    let mut scanner = Scanner::new();
    for folder in &folders {
        scanner.visit_dir(Path::new(folder));
    }

    let mut seen = HashSet::new();
    let mut files: Vec<ScannedFile> = scanner
        .found
        .into_iter()
        .filter(|file| seen.insert(file.track.uri.clone()))
        .collect();
    files.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));

    let paths = files
        .iter()
        .enumerate()
        .map(|(i, file)| (file.track.uri.clone(), (i, file.path.clone())))
        .collect();
    LocalIndex {
        library: LocalLibrary {
            folders,
            tracks: files.into_iter().map(|file| file.track).collect(),
            skipped: scanner.skipped,
        },
        paths,
    }
}

fn supported_extension(path: &Path) -> Option<&str> {
    let extension = path.extension()?.to_str()?;
    EXTENSIONS
        .contains(&extension.to_lowercase().as_str())
        .then_some(extension)
}

struct ScannedFile {
    path: PathBuf,
    track: Track,
    sort_key: (bool, String, u32, u32, String),
}

struct Scanner {
    found: Vec<ScannedFile>,
    skipped: u32,
    covers_dir: Option<PathBuf>,
    folder_covers: HashMap<PathBuf, Option<String>>,
}

impl Scanner {
    fn new() -> Self {
        let covers_dir = covers_dir().filter(|dir| fs::create_dir_all(dir).is_ok());
        Self {
            found: Vec::new(),
            skipped: 0,
            covers_dir,
            folder_covers: HashMap::new(),
        }
    }

    fn visit_dir(&mut self, dir: &Path) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            // file_type() doesn't follow links, so a link back up the tree can't loop forever.
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                self.visit_dir(&path);
            } else if let Some(extension) = supported_extension(&path) {
                match self.read_file(&path, extension) {
                    Some(file) => self.found.push(file),
                    None => self.skipped += 1,
                }
            }
        }
    }

    fn read_file(&mut self, path: &Path, extension: &str) -> Option<ScannedFile> {
        let file = File::open(path).ok()?;
        let added_at = file
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .and_then(rfc3339);
        let mut hint = Hint::new();
        hint.with_extension(extension);
        let mut probed = symphonia::default::get_probe()
            .format(
                &hint,
                MediaSourceStream::new(Box::new(file), Default::default()),
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .ok()?;

        let (tags, visual) = latest_metadata(&mut probed).unwrap_or_default();
        let codec = &probed.format.default_track()?.codec_params;
        // Leaves out files rodio couldn't play anyway, like DRM-protected m4p.
        symphonia::default::get_codecs()
            .make(codec, &DecoderOptions::default())
            .ok()?;
        let time = codec.time_base?.calc_time(codec.n_frames?);
        let duration_ms = time.seconds * 1000 + (time.frac * 1000.0) as u64;
        if duration_ms == 0 {
            return None;
        }

        let mut title = None;
        let mut artist = None;
        let mut album = None;
        let mut album_artist = None;
        let mut date = None;
        let mut number = None;
        let mut disc = None;
        for tag in &tags {
            let Some(key) = tag_key(tag) else {
                continue;
            };
            let value = tag.value.to_string();
            match key {
                StandardTagKey::TrackTitle => title = Some(value),
                StandardTagKey::Artist => artist = Some(value),
                StandardTagKey::Album => album = Some(value),
                StandardTagKey::AlbumArtist => album_artist = Some(value),
                StandardTagKey::Date | StandardTagKey::ReleaseDate => date = Some(value),
                StandardTagKey::TrackNumber => number = leading_number(&value),
                StandardTagKey::DiscNumber => disc = leading_number(&value),
                _ => {}
            }
        }

        let file_name = path.file_stem().map(|stem| stem.to_string_lossy().into_owned());
        // Spotify's own format for local files, so local tracks saved in Spotify
        // playlists find the matching file here. Untagged files go by file name,
        // or every untagged song of the same length would share one URI.
        let uri = format!(
            "{URI_PREFIX}{}:{}:{}:{}",
            uri_part(&artist),
            uri_part(&album),
            uri_part(&title.clone().or_else(|| file_name.clone())),
            time.seconds
        );

        let name = non_empty(&title).or(file_name).unwrap_or_default();
        let artist_name = non_empty(&artist).or_else(|| non_empty(&album_artist));
        let album_artist_name = non_empty(&album_artist).or_else(|| artist_name.clone());
        let album_name = non_empty(&album).unwrap_or_default();
        let cover = self.cover(path, visual);

        let sort_key = (
            album_name.is_empty(),
            album_name.to_lowercase(),
            disc.unwrap_or(0),
            number.unwrap_or(0),
            name.to_lowercase(),
        );
        let local_artist = |name: String| Artist { id: String::new(), name };
        let track = Track {
            id: String::new(),
            name,
            uri,
            duration_ms: duration_ms.min(u32::MAX as u64) as u32,
            artists: artist_name.into_iter().map(local_artist).collect(),
            album: Album {
                id: String::new(),
                name: album_name,
                images: cover
                    .map(|file_name| Image {
                        url: cover_url(&file_name),
                        width: None,
                        height: None,
                    })
                    .into_iter()
                    .collect(),
                artists: album_artist_name.into_iter().map(local_artist).collect(),
                release_date: non_empty(&date),
                album_type: None,
                total_tracks: None,
            },
            added_at,
            position: None,
        };
        Some(ScannedFile {
            path: path.to_path_buf(),
            track,
            sort_key,
        })
    }

    fn cover(&mut self, path: &Path, visual: Option<Visual>) -> Option<String> {
        if let Some(file_name) = visual.and_then(|visual| self.store_cover(&visual.data)) {
            return Some(file_name);
        }
        let dir = path.parent()?;
        if let Some(cached) = self.folder_covers.get(dir) {
            return cached.clone();
        }
        let found = folder_cover(dir)
            .and_then(|image| fs::read(image).ok())
            .and_then(|data| self.store_cover(&data));
        self.folder_covers.insert(dir.to_path_buf(), found.clone());
        found
    }

    fn store_cover(&self, data: &[u8]) -> Option<String> {
        let extension = image_extension(data)?;
        let digest = Sha256::digest(data);
        let hash: String = digest[..16].iter().map(|b| format!("{b:02x}")).collect();
        let file_name = format!("{hash}.{extension}");
        let path = self.covers_dir.as_ref()?.join(&file_name);
        if !path.exists() {
            let temp = path.with_extension("tmp");
            fs::write(&temp, data).ok()?;
            fs::rename(&temp, &path).ok()?;
        }
        Some(file_name)
    }
}

// symphonia only knows some spellings, e.g. not the "ALBUM_ARTIST" some taggers write.
fn tag_key(tag: &Tag) -> Option<StandardTagKey> {
    tag.std_key.or_else(|| {
        let key: String = tag
            .key
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .map(|c| c.to_ascii_lowercase())
            .collect();
        match key.as_str() {
            "albumartist" => Some(StandardTagKey::AlbumArtist),
            "track" => Some(StandardTagKey::TrackNumber),
            _ => None,
        }
    })
}

// Container tags first, falling back to tags found while probing (ID3 on MP3s).
fn latest_metadata(probed: &mut ProbeResult) -> Option<(Vec<Tag>, Option<Visual>)> {
    let mut metadata = probed.format.metadata();
    if metadata.current().is_none() {
        if let Some(inner) = probed.metadata.get() {
            metadata = inner;
        }
    }
    metadata.skip_to_latest();
    let revision = metadata.current()?;
    let visuals = revision.visuals();
    let visual = visuals
        .iter()
        .find(|visual| visual.usage == Some(StandardVisualKey::FrontCover))
        .or_else(|| visuals.first())
        .cloned();
    Some((revision.tags().to_vec(), visual))
}

fn folder_cover(dir: &Path) -> Option<PathBuf> {
    fs::read_dir(dir).ok()?.flatten().map(|entry| entry.path()).find(|path| {
        let matches = |part: Option<&std::ffi::OsStr>, allowed: &[&str]| {
            part.and_then(|p| p.to_str())
                .is_some_and(|p| allowed.contains(&p.to_lowercase().as_str()))
        };
        matches(path.file_stem(), FOLDER_COVER_NAMES) && matches(path.extension(), FOLDER_COVER_EXTENSIONS)
    })
}

fn image_extension(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if data.starts_with(b"GIF8") {
        Some("gif")
    } else if data.len() > 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        Some("webp")
    } else if data.starts_with(b"BM") {
        Some("bmp")
    } else {
        None
    }
}

fn uri_part(value: &Option<String>) -> String {
    value
        .as_deref()
        .map(|value| url::form_urlencoded::byte_serialize(value.as_bytes()).collect())
        .unwrap_or_default()
}

fn non_empty(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn leading_number(value: &str) -> Option<u32> {
    let digits: String = value.trim().chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

fn rfc3339(time: SystemTime) -> Option<String> {
    let secs = time.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn uri_parts_use_spotify_local_file_encoding() {
        let album = Some("Donkey Kong Country: Tropical Freeze".to_string());
        assert_eq!(uri_part(&album), "Donkey+Kong+Country%3A+Tropical+Freeze");
        assert_eq!(uri_part(&None), "");
    }

    #[test]
    fn formats_file_times_as_rfc3339() {
        assert_eq!(rfc3339(UNIX_EPOCH).as_deref(), Some("1970-01-01T00:00:00Z"));
        let time = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        assert_eq!(rfc3339(time).as_deref(), Some("2023-11-14T22:13:20Z"));
        let leap = UNIX_EPOCH + Duration::from_secs(951_782_400);
        assert_eq!(rfc3339(leap).as_deref(), Some("2000-02-29T00:00:00Z"));
    }

    #[test]
    fn recognizes_tag_spellings_symphonia_misses() {
        use symphonia::core::meta::Value;
        let tag = |key: &str| Tag::new(None, key, Value::from("x"));
        assert_eq!(tag_key(&tag("album_artist")), Some(StandardTagKey::AlbumArtist));
        assert_eq!(tag_key(&tag("Album-Artist")), Some(StandardTagKey::AlbumArtist));
        assert_eq!(tag_key(&tag("TRACK")), Some(StandardTagKey::TrackNumber));
        assert_eq!(tag_key(&tag("ENCODER")), None);
        let known = Tag::new(Some(StandardTagKey::Artist), "ARTIST", Value::from("x"));
        assert_eq!(tag_key(&known), Some(StandardTagKey::Artist));
    }

    #[test]
    fn reads_leading_track_numbers() {
        assert_eq!(leading_number("3/12"), Some(3));
        assert_eq!(leading_number(" 07 "), Some(7));
        assert_eq!(leading_number("A1"), None);
    }

    #[test]
    fn cover_urls_round_trip_to_cache_files() {
        let name = "0123456789abcdef0123456789abcdef.jpg";
        let file = cover_file(&cover_url(name)).expect("cover file");
        assert!(file.ends_with(name));
        assert!(cover_file(&cover_url("../config.json")).is_none());
        assert!(cover_file("https://i.scdn.co/image/abc").is_none());
    }
}
