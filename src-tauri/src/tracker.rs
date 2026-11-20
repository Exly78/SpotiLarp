//! The listening tracker: every play heard in SpotiLarp, on Spotify's other
//! apps (from its recently played list) and from an imported Spotify data
//! download, kept on this computer in two append-only files. Stats are worked
//! out by the page from what `tracker_load` returns.
//!
//! `tracks.jsonl` holds what's known about each song, a later line filling in
//! or replacing an earlier one's fields. `plays.jsonl` holds the plays; a line
//! for the same song, start and source as an earlier one replaces it (a play
//! is saved once it counts, then again with how long it really lasted).

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::WebviewWindow;
use tauri_plugin_dialog::DialogExt;

/// Where a play was heard.
const FROM_SPOTILARP: u8 = 0;
const FROM_SPOTIFY: u8 = 1;
const FROM_IMPORT: u8 = 2;

/// Plays of one song this close together from different sources are the same play.
const SAME_PLAY_MS: i64 = 3 * 60_000;
/// Spotify's rule for a play counting, used for imported history too.
const COUNTS_AFTER_MS: u32 = 30_000;
/// How deep to look for the history files in the folder Spotify's download unzips to.
const IMPORT_DEPTH: usize = 4;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ArtistRef {
    #[serde(default)]
    pub id: String,
    pub name: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct AlbumRef {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub image: Option<String>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct TrackMeta {
    pub uri: String,
    pub name: String,
    #[serde(default)]
    pub artists: Vec<ArtistRef>,
    #[serde(default)]
    pub album: AlbumRef,
    #[serde(default)]
    pub duration_ms: u32,
}

#[derive(Serialize, Deserialize)]
struct PlayLine {
    t: i64,
    ms: u32,
    uri: String,
    src: u8,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    skip: bool,
}

/// A play to save: `t` is when it started (ms since 1970), `ms` how long was heard.
#[derive(Deserialize)]
pub struct NewPlay {
    pub t: i64,
    pub ms: u32,
    pub src: u8,
    #[serde(default)]
    pub skip: bool,
    pub track: TrackMeta,
}

/// `plays` are `[started, ms heard, index into tracks, source, skipped (0 or 1)]`, oldest first.
#[derive(Serialize)]
pub struct TrackerData {
    tracks: Vec<TrackMeta>,
    plays: Vec<(i64, u32, u32, u8, u8)>,
}

#[derive(Serialize, Default)]
pub struct ImportResult {
    files: usize,
    found: usize,
    added: usize,
    duplicates: usize,
    too_short: usize,
    unmatched: usize,
}

/// What `Store::apply` changed, to be saved.
struct Changes {
    track_lines: Vec<TrackMeta>,
    play_lines: Vec<PlayLine>,
    added: usize,
    duplicates: usize,
}

struct Play {
    t: i64,
    ms: u32,
    track: u32,
    src: u8,
    skip: bool,
}

#[derive(Default)]
struct Store {
    tracks: Vec<TrackMeta>,
    by_uri: HashMap<String, u32>,
    plays: Vec<Play>,
    /// Each song's plays, as indexes into `plays`.
    plays_of: HashMap<u32, Vec<usize>>,
}

static STORE: Mutex<Option<Store>> = Mutex::new(None);

fn folder() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("SpotiLarp")
        .join("listening")
}

fn tracks_path() -> PathBuf {
    folder().join("tracks.jsonl")
}

fn plays_path() -> PathBuf {
    folder().join("plays.jsonl")
}

/// Each line that reads as JSON (a line cut short by a crash is skipped).
fn read_lines<T: for<'de> Deserialize<'de>>(path: &Path) -> Vec<T> {
    let Ok(file) = File::open(path) else {
        return Vec::new();
    };
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str(&line).ok())
        .collect()
}

fn append_lines<T: Serialize>(path: &Path, lines: &[T]) -> Result<(), String> {
    if lines.is_empty() {
        return Ok(());
    }
    fs::create_dir_all(folder()).map_err(|e| format!("Couldn't save listening history: {e}"))?;
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("Couldn't save listening history: {e}"))?;
    let mut writer = BufWriter::new(file);
    for line in lines {
        let json = serde_json::to_string(line).map_err(|e| e.to_string())?;
        writeln!(writer, "{json}").map_err(|e| format!("Couldn't save listening history: {e}"))?;
    }
    writer.flush().map_err(|e| format!("Couldn't save listening history: {e}"))
}

/// Takes what `new` knows that `old` doesn't. Imported history only names the
/// album's artist and has no pictures, so it never replaces what a play in an
/// app said. Returns whether anything changed.
fn merge(old: &mut TrackMeta, new: &TrackMeta) -> bool {
    let mut changed = false;
    if old.name.is_empty() && !new.name.is_empty() {
        old.name = new.name.clone();
        changed = true;
    }
    let has_ids = |artists: &[ArtistRef]| artists.iter().any(|a| !a.id.is_empty());
    if (old.artists.is_empty() && !new.artists.is_empty()) || (!has_ids(&old.artists) && has_ids(&new.artists)) {
        old.artists = new.artists.clone();
        changed = true;
    }
    if old.album.id.is_empty() && !new.album.id.is_empty() {
        old.album.id = new.album.id.clone();
        changed = true;
    }
    if old.album.name.is_empty() && !new.album.name.is_empty() {
        old.album.name = new.album.name.clone();
        changed = true;
    }
    if old.album.image.is_none() && new.album.image.is_some() {
        old.album.image = new.album.image.clone();
        changed = true;
    }
    if old.duration_ms == 0 && new.duration_ms > 0 {
        old.duration_ms = new.duration_ms;
        changed = true;
    }
    changed
}

impl Store {
    fn load() -> Store {
        let mut store = Store::default();
        for track in read_lines::<TrackMeta>(&tracks_path()) {
            store.track_index(&track);
        }
        // A later line for the same play replaces the earlier one.
        let mut seen: HashMap<(u32, i64, u8), usize> = HashMap::new();
        for line in read_lines::<PlayLine>(&plays_path()) {
            let Some(&track) = store.by_uri.get(&line.uri) else {
                continue;
            };
            let play = Play { t: line.t, ms: line.ms, track, src: line.src, skip: line.skip };
            match seen.get(&(track, line.t, line.src)) {
                Some(&i) => store.plays[i] = play,
                None => {
                    seen.insert((track, line.t, line.src), store.plays.len());
                    store.plays.push(play);
                }
            }
        }
        store.plays.sort_by_key(|play| play.t);
        store.reindex();
        store
    }

    fn reindex(&mut self) {
        self.plays_of.clear();
        for (i, play) in self.plays.iter().enumerate() {
            self.plays_of.entry(play.track).or_default().push(i);
        }
    }

    /// The song's index, adding it or filling it in. The bool says whether it needs saving.
    fn track_index(&mut self, track: &TrackMeta) -> (u32, bool) {
        if let Some(&i) = self.by_uri.get(&track.uri) {
            let changed = merge(&mut self.tracks[i as usize], track);
            return (i, changed);
        }
        let i = self.tracks.len() as u32;
        self.by_uri.insert(track.uri.clone(), i);
        self.tracks.push(track.clone());
        (i, true)
    }

    /// Adds plays and saves them. Returns how many were new and how many were
    /// already here from another source.
    fn record(&mut self, plays: Vec<NewPlay>) -> Result<(usize, usize), String> {
        let changes = self.apply(plays);
        // Tracks first, so a play is never saved for a song the file doesn't know.
        append_lines(&tracks_path(), &changes.track_lines)?;
        append_lines(&plays_path(), &changes.play_lines)?;
        Ok((changes.added, changes.duplicates))
    }

    /// Adds plays that aren't already here; a SpotiLarp play saved again
    /// (with its final length) replaces itself. Returns the lines to save.
    fn apply(&mut self, plays: Vec<NewPlay>) -> Changes {
        let mut track_lines = Vec::new();
        let mut play_lines = Vec::new();
        let mut added = 0;
        let mut duplicates = 0;
        for new in plays {
            if new.track.uri.is_empty() {
                continue;
            }
            let (track, track_changed) = self.track_index(&new.track);
            if track_changed {
                track_lines.push(self.tracks[track as usize].clone());
            }
            let existing = self.plays_of.get(&track).and_then(|indexes| {
                indexes.iter().copied().find(|&i| {
                    let play = &self.plays[i];
                    if play.src == FROM_SPOTILARP && new.src == FROM_SPOTILARP {
                        play.t == new.t
                    } else {
                        (play.t - new.t).abs() < SAME_PLAY_MS
                    }
                })
            });
            match existing {
                Some(i) => {
                    let play = &mut self.plays[i];
                    // The same play again: SpotiLarp's with its final length, or
                    // an imported one saying how long a synced one really was.
                    let replaces = play.src == new.src || (play.src == FROM_SPOTIFY && new.src == FROM_IMPORT);
                    if replaces && (play.ms != new.ms || play.skip != new.skip) {
                        play.ms = new.ms;
                        play.skip = new.skip;
                        play_lines.push(PlayLine { t: play.t, ms: new.ms, uri: new.track.uri.clone(), src: play.src, skip: new.skip });
                    }
                    if play.src != new.src || play.t != new.t {
                        duplicates += 1;
                    }
                }
                None => {
                    self.plays_of.entry(track).or_default().push(self.plays.len());
                    self.plays.push(Play { t: new.t, ms: new.ms, track, src: new.src, skip: new.skip });
                    play_lines.push(PlayLine { t: new.t, ms: new.ms, uri: new.track.uri.clone(), src: new.src, skip: new.skip });
                    added += 1;
                }
            }
        }
        if added > 0 && self.plays.windows(2).any(|pair| pair[0].t > pair[1].t) {
            self.plays.sort_by_key(|play| play.t);
            self.reindex();
        }
        Changes { track_lines, play_lines, added, duplicates }
    }

    fn data(&self) -> TrackerData {
        TrackerData {
            tracks: self.tracks.clone(),
            plays: self.plays.iter().map(|p| (p.t, p.ms, p.track, p.src, p.skip as u8)).collect(),
        }
    }
}

fn with_store<T>(f: impl FnOnce(&mut Store) -> T) -> T {
    let mut guard = STORE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let store = guard.get_or_insert_with(Store::load);
    f(store)
}

#[tauri::command]
pub async fn tracker_load() -> TrackerData {
    tauri::async_runtime::spawn_blocking(|| with_store(|store| store.data()))
        .await
        .unwrap_or(TrackerData { tracks: Vec::new(), plays: Vec::new() })
}

/// Saves plays; returns how many weren't already saved.
#[tauri::command]
pub async fn tracker_record(plays: Vec<NewPlay>) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || with_store(|store| store.record(plays).map(|(added, _)| added)))
        .await
        .map_err(|e| e.to_string())?
}

/// Fills in songs' details (pictures for imported history, found later).
#[tauri::command]
pub async fn tracker_update_tracks(tracks: Vec<TrackMeta>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        with_store(|store| {
            let changed: Vec<TrackMeta> = tracks
                .iter()
                .filter_map(|track| {
                    let (i, changed) = store.track_index(track);
                    changed.then(|| store.tracks[i as usize].clone())
                })
                .collect();
            append_lines(&tracks_path(), &changed)
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn tracker_clear() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        with_store(|store| {
            *store = Store::default();
            for path in [plays_path(), tracks_path()] {
                if path.exists() {
                    fs::remove_file(&path).map_err(|e| format!("Couldn't clear listening history: {e}"))?;
                }
            }
            Ok(())
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Asks for the folder Spotify's data download unzipped to and adds the
/// plays in it. `None` when no folder was picked.
#[tauri::command]
pub async fn tracker_import(window: WebviewWindow) -> Result<Option<ImportResult>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("Choose the folder your Spotify data unzipped to")
        .pick_folder(move |folder| {
            let _ = tx.send(folder);
        });
    let Some(folder) = rx.await.ok().flatten().and_then(|folder| folder.into_path().ok()) else {
        return Ok(None);
    };
    tauri::async_runtime::spawn_blocking(move || import_folder(&folder))
        .await
        .map_err(|e| e.to_string())?
        .map(Some)
}

/// The history files: the extended streaming history (`Streaming_History_Audio_*.json`,
/// or `endsong_*.json` in older downloads) and the account data's year of it
/// (`StreamingHistory0.json`, `StreamingHistory_music_0.json`). Podcasts and videos are left out.
fn history_files(folder: &Path, depth: usize, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if depth > 0 {
                history_files(&path, depth - 1, found);
            }
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        let wanted = name.ends_with(".json")
            && (name.starts_with("streaming_history_audio")
                || name.starts_with("endsong")
                || (name.starts_with("streaminghistory") && !name.contains("podcast")));
        if wanted {
            found.push(path);
        }
    }
}

/// A key for matching a song by its artist and name, for history that has no links.
fn name_key(artist: &str, track: &str) -> String {
    format!("{}\u{1f}{}", artist.trim().to_lowercase(), track.trim().to_lowercase())
}

fn import_folder(folder: &Path) -> Result<ImportResult, String> {
    let mut files = Vec::new();
    history_files(folder, IMPORT_DEPTH, &mut files);
    if files.is_empty() {
        return Err("No Spotify listening history in that folder. Pick the folder the download unzipped to: it has files named like Streaming_History_Audio_2024.json".into());
    }
    let mut result = ImportResult { files: files.len(), ..Default::default() };
    let mut extended = Vec::new();
    let mut basic = Vec::new();
    for path in &files {
        let text = fs::read_to_string(path).map_err(|e| format!("Couldn't read {}: {e}", path.display()))?;
        let Ok(Value::Array(entries)) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        for entry in entries {
            if entry.get("endTime").is_some() {
                basic.push(entry);
            } else {
                extended.push(entry);
            }
        }
    }

    // Songs by artist and name, for the account data's history, which only names them.
    let by_name: HashMap<String, String> = with_store(|store| {
        store
            .tracks
            .iter()
            .filter_map(|track| Some((name_key(&track.artists.first()?.name, &track.name), track.uri.clone())))
            .collect()
    });
    let plays = plays_from_history(&extended, &basic, by_name, &mut result);
    let (added, duplicates) = with_store(|store| store.record(plays))?;
    result.added = added;
    result.duplicates = duplicates;
    Ok(result)
}

/// The plays in Spotify's history files, counting what's left out in `result`.
/// `by_name` finds songs for the account data's entries, which have no links.
fn plays_from_history(
    extended: &[Value],
    basic: &[Value],
    mut by_name: HashMap<String, String>,
    result: &mut ImportResult,
) -> Vec<NewPlay> {
    let str_of = |entry: &Value, key: &str| entry.get(key).and_then(Value::as_str).unwrap_or("").to_string();
    let mut plays = Vec::new();

    for entry in extended {
        let uri = str_of(entry, "spotify_track_uri");
        if !uri.starts_with("spotify:track:") {
            continue;
        }
        result.found += 1;
        let ms = entry.get("ms_played").and_then(Value::as_u64).unwrap_or(0) as u32;
        if ms < COUNTS_AFTER_MS {
            result.too_short += 1;
            continue;
        }
        // `ts` is when the play ended.
        let Some(ended) = parse_utc(&str_of(entry, "ts")) else {
            continue;
        };
        let name = str_of(entry, "master_metadata_track_name");
        let artist = str_of(entry, "master_metadata_album_artist_name");
        by_name.entry(name_key(&artist, &name)).or_insert_with(|| uri.clone());
        let skipped = entry.get("skipped").and_then(Value::as_bool).unwrap_or(false) || str_of(entry, "reason_end") == "fwdbtn";
        plays.push(NewPlay {
            t: ended - ms as i64,
            ms,
            src: FROM_IMPORT,
            skip: skipped,
            track: TrackMeta {
                uri,
                name,
                artists: if artist.is_empty() { Vec::new() } else { vec![ArtistRef { id: String::new(), name: artist }] },
                album: AlbumRef { name: str_of(entry, "master_metadata_album_album_name"), ..Default::default() },
                duration_ms: 0,
            },
        });
    }

    for entry in basic {
        result.found += 1;
        let ms = entry.get("msPlayed").and_then(Value::as_u64).unwrap_or(0) as u32;
        if ms < COUNTS_AFTER_MS {
            result.too_short += 1;
            continue;
        }
        let Some(ended) = parse_utc(&str_of(entry, "endTime")) else {
            continue;
        };
        let name = str_of(entry, "trackName");
        let artist = str_of(entry, "artistName");
        // Without a link it can't be played, but it still counts in the stats.
        let uri = match by_name.get(&name_key(&artist, &name)) {
            Some(uri) => uri.clone(),
            None => {
                result.unmatched += 1;
                format!("spotilarp:unmatched:{}", name_key(&artist, &name))
            }
        };
        plays.push(NewPlay {
            t: ended - ms as i64,
            ms,
            src: FROM_IMPORT,
            skip: false,
            track: TrackMeta {
                uri,
                name,
                artists: vec![ArtistRef { id: String::new(), name: artist }],
                ..Default::default()
            },
        });
    }
    plays
}

/// Milliseconds since 1970 for "2023-01-02T03:04:05Z" (the extended history)
/// or "2023-01-02 03:04" (the account data's); both are UTC.
fn parse_utc(text: &str) -> Option<i64> {
    let number = |from: usize, to: usize| text.get(from..to)?.parse::<i64>().ok();
    let (year, month, day) = (number(0, 4)?, number(5, 7)?, number(8, 10)?);
    let (hour, minute) = (number(11, 13)?, number(14, 16)?);
    let second = if text.as_bytes().get(16) == Some(&b':') { number(17, 19)? } else { 0 };
    Some(((days_from_civil(year, month, day) * 24 + hour) * 60 + minute) * 60_000 + second * 1000)
}

/// Days since 1970-01-01 (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = (if year >= 0 { year } else { year - 399 }) / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_history_timestamps() {
        assert_eq!(parse_utc("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_utc("2023-01-02T03:04:05Z"), Some(1_672_628_645_000));
        assert_eq!(parse_utc("2023-01-02 03:04"), Some(1_672_628_640_000));
        assert_eq!(parse_utc("2024-02-29T12:00:00Z"), Some(1_709_208_000_000));
        assert_eq!(parse_utc("nonsense"), None);
    }

    fn play(t: i64, src: u8, ms: u32) -> NewPlay {
        NewPlay {
            t,
            ms,
            src,
            skip: false,
            track: TrackMeta { uri: "spotify:track:a".into(), name: "A".into(), ..Default::default() },
        }
    }

    #[test]
    fn the_same_play_from_two_sources_counts_once() {
        let mut store = Store::default();
        assert_eq!(store.apply(vec![play(1_000_000, FROM_SPOTIFY, 200_000)]).added, 1);
        // The import says how long the synced play really was: updated, not added.
        let changes = store.apply(vec![play(1_060_000, FROM_IMPORT, 90_000)]);
        assert_eq!((changes.added, changes.duplicates), (0, 1));
        assert_eq!(store.plays.len(), 1);
        assert_eq!(store.plays[0].ms, 90_000);
        // A SpotiLarp play on repeat two minutes later is its own play...
        assert_eq!(store.apply(vec![play(2_000_000, FROM_SPOTILARP, 60_000), play(2_120_000, FROM_SPOTILARP, 60_000)]).added, 2);
        // ...and saving one again with its final length replaces it.
        let changes = store.apply(vec![play(2_000_000, FROM_SPOTILARP, 118_000)]);
        assert_eq!((changes.added, changes.duplicates, changes.play_lines.len()), (0, 0, 1));
        assert_eq!(store.plays.len(), 3);
        assert_eq!(store.plays[1].ms, 118_000);
    }

    #[test]
    fn imported_details_never_replace_an_apps() {
        let mut old = TrackMeta {
            uri: "spotify:track:a".into(),
            name: "A".into(),
            artists: vec![ArtistRef { id: "x".into(), name: "Artist, feat. B".into() }],
            album: AlbumRef { id: "al".into(), name: "Album".into(), image: Some("cover".into()) },
            duration_ms: 1000,
        };
        let imported = TrackMeta {
            uri: "spotify:track:a".into(),
            name: "A".into(),
            artists: vec![ArtistRef { id: String::new(), name: "Artist".into() }],
            album: AlbumRef { name: "Album".into(), ..Default::default() },
            duration_ms: 0,
        };
        assert!(!merge(&mut old, &imported));
        assert_eq!(old.artists[0].id, "x");
        // The other way round, the app's details fill in the import's.
        let mut sparse = imported.clone();
        assert!(merge(&mut sparse, &old));
        assert_eq!(sparse.album.image.as_deref(), Some("cover"));
    }

    #[test]
    fn reads_both_kinds_of_spotify_history() {
        // Shaped like Streaming_History_Audio_2023.json, and the account data's StreamingHistory0.json.
        let extended: Vec<Value> = serde_json::from_str(
            r#"[
                {"ts": "2023-01-02T03:04:05Z", "ms_played": 200000, "master_metadata_track_name": "Song",
                 "master_metadata_album_artist_name": "Band", "master_metadata_album_album_name": "Record",
                 "spotify_track_uri": "spotify:track:abc", "reason_end": "trackdone", "skipped": null},
                {"ts": "2023-01-02T03:10:00Z", "ms_played": 45000, "master_metadata_track_name": "Other",
                 "master_metadata_album_artist_name": "Band", "spotify_track_uri": "spotify:track:def",
                 "reason_end": "fwdbtn", "skipped": true},
                {"ts": "2023-01-02T03:11:00Z", "ms_played": 5000, "spotify_track_uri": "spotify:track:ghi"},
                {"ts": "2023-01-02T04:00:00Z", "ms_played": 900000, "spotify_track_uri": null,
                 "spotify_episode_uri": "spotify:episode:xyz", "episode_name": "A podcast"}
            ]"#,
        )
        .unwrap();
        let basic: Vec<Value> = serde_json::from_str(
            r#"[
                {"endTime": "2022-06-01 10:00", "artistName": "Band", "trackName": "Song", "msPlayed": 180000},
                {"endTime": "2022-06-01 10:05", "artistName": "Nobody", "trackName": "Unknown", "msPlayed": 60000}
            ]"#,
        )
        .unwrap();
        let mut result = ImportResult::default();
        let plays = plays_from_history(&extended, &basic, HashMap::new(), &mut result);

        // The episode isn't counted, the 5 second play is too short.
        assert_eq!((result.found, result.too_short, result.unmatched), (5, 1, 1));
        assert_eq!(plays.len(), 4);
        // `ts` is when it ended, so the play started that long before.
        assert_eq!(plays[0].t, 1_672_628_645_000 - 200_000);
        assert_eq!(plays[0].track.album.name, "Record");
        assert!(plays[1].skip);
        // The account data's play of the same song finds its link by name.
        assert_eq!(plays[2].track.uri, "spotify:track:abc");
        assert!(plays[3].track.uri.starts_with("spotilarp:unmatched:"));
    }
}
