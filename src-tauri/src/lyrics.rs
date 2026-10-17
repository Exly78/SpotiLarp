//! Lyrics for the playing song: Spotify's own (through the playback session)
//! first, since they follow the recording exactly, then LRCLIB.
//!
//! Title/artist cleanup and result scoring are adapted from Spotifast
//! (https://github.com/crmne/spotifast, MIT).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use librespot_core::session::Session;
use librespot_core::{Error as LibrespotError, SpotifyId};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const LRCLIB_BASE: &str = "https://lrclib.net/api";
/// Lyrics don't change, so a cached answer (a miss included) is good for this long.
const CACHE_LIFETIME: Duration = Duration::from_secs(30 * 24 * 60 * 60);
/// A result this far from the song's length is another recording, however well the names match.
const MAX_DRIFT_SECS: f64 = 30.0;
const RATE_LIMIT_RETRIES: u32 = 3;
const MAX_RETRY_AFTER: Duration = Duration::from_secs(5);

#[derive(Clone, Serialize)]
pub struct LyricLine {
    pub time_ms: u32,
    pub text: String,
}

#[derive(Serialize)]
pub struct LyricsResult {
    pub instrumental: bool,
    pub synced: Option<Vec<LyricLine>>,
    pub plain: Option<String>,
    /// "spotify" or "lrclib".
    pub source: &'static str,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct Line {
    at_ms: Option<u32>,
    text: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct Lyrics {
    lines: Vec<Line>,
    synced: bool,
    instrumental: bool,
}

impl Lyrics {
    fn into_result(self, source: &'static str) -> LyricsResult {
        if self.synced {
            let synced = self
                .lines
                .into_iter()
                .filter_map(|line| Some(LyricLine { time_ms: line.at_ms?, text: line.text }))
                .collect();
            return LyricsResult { instrumental: self.instrumental, synced: Some(synced), plain: None, source };
        }
        let plain = (!self.lines.is_empty())
            .then(|| self.lines.iter().map(|line| line.text.as_str()).collect::<Vec<_>>().join("\n"));
        LyricsResult { instrumental: self.instrumental, synced: None, plain, source }
    }
}

pub struct Query<'a> {
    pub title: &'a str,
    pub artist: &'a str,
    pub album: &'a str,
    pub duration_ms: u32,
}

fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        crate::http::client_builder()
            .user_agent(concat!(
                "SpotiLarp/",
                env!("CARGO_PKG_VERSION"),
                " (https://github.com/Exly78/SpotiLarp)"
            ))
            .build()
            .unwrap_or_default()
    })
}

fn cache_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|base| base.join("SpotiLarp").join("lyrics"))
}

pub async fn fetch(session: Option<Session>, track_id: Option<&str>, query: &Query<'_>) -> Result<Option<LyricsResult>, String> {
    let cache = cache_dir();
    if let (Some(session), Some(id)) = (session, track_id) {
        if let Some(found) = spotify_lyrics(&session, id, cache.as_deref()).await {
            return Ok(Some(found.into_result("spotify")));
        }
    }

    let cache_path = cache.map(|dir| dir.join(format!("lrclib-{}.json", cache_key(query))));
    if let Some(cached) = cache_path.as_deref().and_then(read_cache) {
        return Ok(cached.map(|lyrics| lyrics.into_result("lrclib")));
    }
    let found = lrclib_lookup(query).await?;
    if let Some(path) = &cache_path {
        write_cache(path, &found);
    }
    Ok(found.map(|lyrics| lyrics.into_result("lrclib")))
}

/// Spotify's transcription, or `None` when it has none or asking failed
/// (failures aren't cached, so they're asked again next time).
async fn spotify_lyrics(session: &Session, track_id: &str, cache: Option<&Path>) -> Option<Lyrics> {
    let id = SpotifyId::from_base62(track_id).ok()?;
    let path = cache.map(|dir| dir.join(format!("spotify-{track_id}.json")));
    if let Some(cached) = path.as_deref().and_then(read_cache) {
        return cached;
    }
    let found = match session.spclient().get_lyrics(&id).await {
        Ok(bytes) => serde_json::from_slice::<serde_json::Value>(&bytes)
            .ok()
            .as_ref()
            .and_then(from_spotify),
        Err(e) if is_not_found(&e) => None,
        Err(_) => return None,
    };
    if let Some(path) = &path {
        write_cache(path, &found);
    }
    found
}

fn is_not_found(error: &LibrespotError) -> bool {
    error.kind == librespot_core::error::ErrorKind::NotFound
}

/// Parses Spotify's `color-lyrics` response.
fn from_spotify(json: &serde_json::Value) -> Option<Lyrics> {
    let lyrics = json.get("lyrics")?;
    let synced = lyrics.get("syncType").and_then(|value| value.as_str()) == Some("LINE_SYNCED");
    let lines: Vec<Line> = lyrics
        .get("lines")?
        .as_array()?
        .iter()
        .filter_map(|line| {
            let text = line.get("words")?.as_str()?.trim();
            if text.is_empty() || text == "\u{266a}" {
                return None;
            }
            let at_ms = line
                .get("startTimeMs")
                .and_then(|value| {
                    value
                        .as_str()
                        .and_then(|text| text.parse().ok())
                        .or_else(|| value.as_u64().and_then(|n| u32::try_from(n).ok()))
                })
                .filter(|_| synced);
            Some(Line { at_ms, text: text.to_string() })
        })
        .collect();
    if lines.is_empty() {
        return None;
    }
    let synced = synced && lines.iter().all(|line| line.at_ms.is_some());
    Some(Lyrics { lines, synced, instrumental: false })
}

async fn lrclib_lookup(query: &Query<'_>) -> Result<Option<Lyrics>, String> {
    let title = clean_title(query.title);
    let artist = clean_artist(query.artist);
    if title.is_empty() || artist.is_empty() {
        return Ok(None);
    }
    let duration = (query.duration_ms / 1000).to_string();
    let mut exact = vec![("artist_name", artist.as_str()), ("track_name", title.as_str())];
    if !query.album.trim().is_empty() {
        exact.push(("album_name", query.album.trim()));
    }
    if query.duration_ms > 0 {
        exact.push(("duration", duration.as_str()));
    }
    if let Some(lyrics) = lrclib_get::<Record>("/get", &exact).await?.and_then(|record| record.lyrics()) {
        return Ok(Some(lyrics));
    }
    let candidates = lrclib_get::<Vec<Record>>("/search", &[("artist_name", artist.as_str()), ("track_name", title.as_str())])
        .await?
        .unwrap_or_default();
    Ok(pick(candidates, query).and_then(|record| record.lyrics()))
}

/// One LRCLIB request. A 404 or 400 just means there are no lyrics; a 429
/// waits as long as LRCLIB asks (briefly) and tries again.
async fn lrclib_get<T: DeserializeOwned>(path: &str, params: &[(&str, &str)]) -> Result<Option<T>, String> {
    let mut attempt = 0;
    let response = loop {
        let response = http()
            .get(format!("{LRCLIB_BASE}{path}"))
            .query(params)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("Couldn't reach LRCLIB: {e}"))?;
        if response.status() != reqwest::StatusCode::TOO_MANY_REQUESTS || attempt == RATE_LIMIT_RETRIES {
            break response;
        }
        let wait = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .map_or(Duration::from_secs(1), Duration::from_secs)
            .min(MAX_RETRY_AFTER);
        attempt += 1;
        tokio::time::sleep(wait).await;
    };
    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND || status == reqwest::StatusCode::BAD_REQUEST {
        return Ok(None);
    }
    if !status.is_success() {
        return Err(format!("LRCLIB answered {status}"));
    }
    response
        .json()
        .await
        .map(Some)
        .map_err(|e| format!("Unexpected answer from LRCLIB: {e}"))
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Record {
    track_name: String,
    artist_name: String,
    duration: Option<f64>,
    instrumental: bool,
    synced_lyrics: Option<String>,
    plain_lyrics: Option<String>,
}

impl Record {
    fn synced(&self) -> Option<&str> {
        self.synced_lyrics.as_deref().filter(|text| !text.trim().is_empty())
    }

    fn plain(&self) -> Option<&str> {
        self.plain_lyrics.as_deref().filter(|text| !text.trim().is_empty())
    }

    fn lyrics(&self) -> Option<Lyrics> {
        if self.instrumental {
            return Some(Lyrics { instrumental: true, ..Lyrics::default() });
        }
        if let Some(synced) = self.synced() {
            let lines = parse_lrc(synced);
            if !lines.is_empty() {
                return Some(Lyrics { lines, synced: true, instrumental: false });
            }
        }
        let plain = self.plain()?;
        Some(Lyrics {
            lines: plain
                .lines()
                .map(|line| Line { at_ms: None, text: line.trim_end().to_string() })
                .collect(),
            synced: false,
            instrumental: false,
        })
    }
}

/// How well a search result fits what's playing; `None` rules it out.
fn score(record: &Record, query: &Query) -> Option<i64> {
    if !loose_match(&record.track_name, &clean_title(query.title)) {
        return None;
    }
    let mut score = 0;
    if loose_match(&record.artist_name, &clean_artist(query.artist)) {
        score += 1000;
    }
    if let Some(duration) = record.duration.filter(|d| *d > 0.0 && query.duration_ms > 0) {
        let drift = (duration - f64::from(query.duration_ms) / 1000.0).abs();
        if drift > MAX_DRIFT_SECS {
            return None;
        }
        score += ((MAX_DRIFT_SECS - drift) * 10.0) as i64;
    }
    // Timing is the point, so a synced upload wins a tie.
    if record.synced().is_some() {
        score += 200;
    } else if record.plain().is_some() {
        score += 50;
    }
    Some(score)
}

fn pick(records: Vec<Record>, query: &Query) -> Option<Record> {
    let mut best: Option<(i64, Record)> = None;
    for record in records {
        let Some(score) = score(&record, query) else {
            continue;
        };
        if best.as_ref().is_none_or(|(held, _)| score > *held) {
            best = Some((score, record));
        }
    }
    best.map(|(_, record)| record)
}

/// Words a title carries in brackets that a lyrics database doesn't.
const BRACKET_NOISE: &[&str] = &[
    "remaster", "remastered", "remix", "live", "acoustic", "version", "edit", "mix", "mono", "stereo", "deluxe",
    "bonus", "expanded", "explicit", "anniversary", "feat", "featuring", "with",
];

/// What a " - " suffix says when it isn't part of the title.
const SUFFIX_NOISE: &[&str] = &[
    "remaster", "remastered", "radio edit", "single version", "album version", "live", "mono", "stereo",
    "rerecorded", "re-recorded",
];

/// Zero-width space, word joiner, invisible operators and byte order mark
/// (but not the zero-width joiners, which hold emoji and some scripts together).
fn is_disposable(c: char) -> bool {
    matches!(c, '\u{200b}' | '\u{2060}'..='\u{2064}' | '\u{feff}')
}

/// Whether `text` contains `phrase` as whole words, ignoring case.
fn has_phrase(text: &str, phrase: &str) -> bool {
    let words: Vec<String> = text
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|word| !word.is_empty())
        .map(|word| word.trim_matches('-').to_string())
        .collect();
    let wanted: Vec<&str> = phrase.split(' ').collect();
    words
        .windows(wanted.len())
        .any(|window| window.iter().zip(&wanted).all(|(a, b)| a == b))
}

/// Strips what a player adds to a title and a database leaves out:
/// "(Remastered 2011)", " - Live at Wembley", "feat. Someone".
fn clean_title(title: &str) -> String {
    let original: String = title.chars().filter(|c| !is_disposable(*c)).collect();
    let mut cleaned = String::with_capacity(original.len());
    let mut rest = original.as_str();
    while let Some(open) = rest.find(['(', '[']) {
        let closer = if rest[open..].starts_with('(') { ')' } else { ']' };
        let Some(close) = rest[open..].find(closer) else {
            break;
        };
        let inner = &rest[open + 1..open + close];
        if BRACKET_NOISE.iter().any(|word| has_phrase(inner, word)) {
            cleaned.push_str(rest[..open].trim_end());
        } else {
            cleaned.push_str(&rest[..open + close + 1]);
        }
        rest = &rest[open + close + 1..];
    }
    cleaned.push_str(rest);
    let mut cleaned = cleaned.trim().to_string();
    // Cut at the first " - " whose tail is noise; a dash inside a real title stays.
    let cut = cleaned.match_indices(" - ").map(|(index, _)| index).find(|&index| {
        let tail = &cleaned[index + 3..];
        let year_version = tail
            .split_whitespace()
            .next()
            .is_some_and(|word| word.len() == 4 && word.chars().all(|c| c.is_ascii_digit()))
            && has_phrase(tail, "version");
        year_version || SUFFIX_NOISE.iter().any(|phrase| has_phrase(tail, phrase))
    });
    if let Some(index) = cut {
        cleaned.truncate(index);
    }
    let cleaned = strip_featuring(cleaned.trim());
    if cleaned.is_empty() {
        original.trim().to_string()
    } else {
        cleaned
    }
}

/// Everything from a standalone "feat", "ft" or "featuring" on.
fn strip_featuring(text: &str) -> String {
    // Lower-casing can change a string's length, so offsets found in the
    // lower-cased copy are mapped back to `text` through `starts`.
    let mut lower = String::with_capacity(text.len());
    let mut starts: Vec<usize> = Vec::with_capacity(text.len() + 1);
    for (at, character) in text.char_indices() {
        for lowered in character.to_lowercase() {
            for _ in 0..lowered.len_utf8() {
                starts.push(at);
            }
            lower.push(lowered);
        }
    }
    starts.push(text.len());
    let mut cut: Option<usize> = None;
    for marker in ["featuring", "feat", "ft"] {
        let mut from = 0;
        while let Some(found) = lower[from..].find(marker) {
            let start = from + found;
            let end = start + marker.len();
            let before = lower[..start].trim_end_matches(['-', '(']).trim_end();
            let preceded = before.len() < start && !before.is_empty();
            let followed = lower[end..].strip_prefix('.').unwrap_or(&lower[end..]).starts_with(' ');
            if preceded && followed {
                let at = before.len();
                cut = Some(cut.map_or(at, |held| held.min(at)));
                break;
            }
            from = end;
        }
    }
    match cut {
        Some(at) => text[..starts[at]].trim().to_string(),
        None => text.trim().to_string(),
    }
}

/// The first credited artist, without "feat." guests (LRCLIB sometimes
/// stores "TOOL;Tool" for one artist, too).
fn clean_artist(artist: &str) -> String {
    let original: String = artist.chars().filter(|c| !is_disposable(*c)).collect();
    let cleaned = strip_featuring(&original);
    let cleaned = cleaned.split(';').next().unwrap_or_default().trim();
    if cleaned.is_empty() {
        original.trim().to_string()
    } else {
        cleaned.to_string()
    }
}

/// Lowercase, common accents folded, punctuation gone, one space between words.
fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for c in text.chars() {
        let folded = fold(c);
        if matches!(folded, '\'' | '’' | '`') {
            continue;
        }
        if folded == '&' {
            for c in [' ', 'a', 'n', 'd', ' '] {
                push_normalized(&mut out, &mut space, c);
            }
            continue;
        }
        push_normalized(&mut out, &mut space, folded);
    }
    out.trim().to_string()
}

fn push_normalized(out: &mut String, space: &mut bool, c: char) {
    if c.is_alphanumeric() {
        out.extend(c.to_lowercase());
        *space = false;
    } else if !*space {
        out.push(' ');
        *space = true;
    }
}

fn fold(c: char) -> char {
    match c {
        'À'..='Å' | 'à'..='å' => 'a',
        'Ç' | 'ç' => 'c',
        'È'..='Ë' | 'è'..='ë' => 'e',
        'Ì'..='Ï' | 'ì'..='ï' => 'i',
        'Ñ' | 'ñ' => 'n',
        'Ò'..='Ö' | 'Ø' | 'ò'..='ö' | 'ø' => 'o',
        'Ù'..='Ü' | 'ù'..='ü' => 'u',
        'Ý' | 'ý' | 'ÿ' => 'y',
        'ß' => 's',
        _ => c,
    }
}

fn loose_match(left: &str, right: &str) -> bool {
    let a = normalize(left);
    let b = normalize(right);
    !a.is_empty() && !b.is_empty() && (a == b || a.contains(&b) || b.contains(&a))
}

/// Parses `[mm:ss.xx]` lines; a line may start with several stamps when the
/// same words repeat. Tags like `[ar:...]` are skipped. Sorted by time.
fn parse_lrc(text: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    for raw in text.lines() {
        let mut rest = raw.trim_start();
        let mut times = Vec::new();
        while let Some((at_ms, length)) = leading_stamp(rest) {
            times.push(at_ms);
            rest = &rest[length..];
        }
        let body = rest.trim();
        for at_ms in times {
            lines.push(Line { at_ms: Some(at_ms), text: body.to_string() });
        }
    }
    lines.sort_by_key(|line| line.at_ms);
    lines
}

/// A timestamp at the start of `text`: its milliseconds and its length.
fn leading_stamp(text: &str) -> Option<(u32, usize)> {
    let inner = text.strip_prefix('[')?;
    let close = inner.find(']')?;
    let (minutes, seconds) = inner[..close].split_once(':')?;
    let (seconds, fraction) = match seconds.find(['.', ':']) {
        Some(dot) => (&seconds[..dot], Some(&seconds[dot + 1..])),
        None => (seconds, None),
    };
    let minutes: u32 = minutes.trim().parse().ok()?;
    let seconds: u32 = seconds.trim().parse().ok()?;
    let fraction_ms = match fraction {
        Some(digits) if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) => {
            let value: u32 = digits.chars().take(3).collect::<String>().parse().ok()?;
            value * 10u32.pow(3u32.saturating_sub(digits.len().min(3) as u32))
        }
        Some(_) => return None,
        None => 0,
    };
    Some((minutes * 60_000 + seconds * 1_000 + fraction_ms, close + 2))
}

#[derive(Serialize, Deserialize)]
struct Cached {
    found: Option<Lyrics>,
}

fn cache_key(query: &Query) -> String {
    let digest = Sha256::digest(format!("{}|{}|{}|{}", query.artist, query.title, query.album, query.duration_ms).as_bytes());
    digest.iter().take(16).map(|byte| format!("{byte:02x}")).collect()
}

fn read_cache(path: &Path) -> Option<Option<Lyrics>> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    if modified.elapsed().unwrap_or(CACHE_LIFETIME) >= CACHE_LIFETIME {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<Cached>(&text).ok().map(|cached| cached.found)
}

fn write_cache(path: &Path, found: &Option<Lyrics>) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string(&Cached { found: found.clone() }) {
        let _ = std::fs::write(path, text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query<'a>(title: &'a str, artist: &'a str, duration_ms: u32) -> Query<'a> {
        Query { title, artist, album: "", duration_ms }
    }

    fn record(track: &str, artist: &str, duration: f64, synced: bool) -> Record {
        Record {
            track_name: track.into(),
            artist_name: artist.into(),
            duration: Some(duration),
            synced_lyrics: synced.then(|| "[00:01.00]x".into()),
            plain_lyrics: Some("x".into()),
            ..Record::default()
        }
    }

    #[test]
    fn parses_repeated_and_metadata_lines() {
        let lines = parse_lrc("[ar:Someone]\n[00:12.50][01:00.00]Chorus\n[00:05.00]Intro\n[00:20.00]");
        let parsed: Vec<_> = lines.iter().map(|l| (l.at_ms.unwrap(), l.text.as_str())).collect();
        assert_eq!(parsed, vec![(5_000, "Intro"), (12_500, "Chorus"), (20_000, ""), (60_000, "Chorus")]);
    }

    #[test]
    fn titles_lose_what_a_database_leaves_out() {
        assert_eq!(clean_title("Come Together - Remastered 2009"), "Come Together");
        assert_eq!(clean_title("Song (feat. Someone)"), "Song");
        assert_eq!(clean_title("Song [Live at Wembley]"), "Song");
        assert_eq!(clean_title("Song feat. Someone"), "Song");
        assert_eq!(clean_title("Rock - n - Roll"), "Rock - n - Roll");
        assert_eq!(clean_title("(Don't Fear) The Reaper"), "(Don't Fear) The Reaper");
    }

    #[test]
    fn artists_keep_their_first_name() {
        assert_eq!(clean_artist("TOOL;Tool"), "TOOL");
        assert_eq!(clean_artist("Artist feat. Guest"), "Artist");
    }

    #[test]
    fn featuring_is_cut_safely_in_any_case() {
        assert_eq!(strip_featuring("İstanbul ft. Someone"), "İstanbul");
        assert_eq!(strip_featuring("Left"), "Left");
    }

    #[test]
    fn matching_is_loose_about_case_accents_and_punctuation() {
        assert!(loose_match("Beyoncé", "beyonce"));
        assert!(loose_match("Simon & Garfunkel", "Simon and Garfunkel"));
        assert!(!loose_match("", "x"));
    }

    #[test]
    fn the_closest_length_wins_and_synced_breaks_ties() {
        let picked = pick(
            vec![record("Song", "Artist", 300.0, true), record("Song", "Artist", 201.0, false), record("Song", "Artist", 199.0, true)],
            &query("Song", "Artist", 200_000),
        )
        .unwrap();
        assert_eq!(picked.duration, Some(199.0));
        assert!(pick(vec![record("Song", "Artist", 400.0, true)], &query("Song", "Artist", 200_000)).is_none());
    }

    #[test]
    fn spotify_lyrics_parse() {
        let json = serde_json::json!({
            "lyrics": { "syncType": "LINE_SYNCED", "lines": [
                { "startTimeMs": "1000", "words": "Hello" },
                { "startTimeMs": "2000", "words": "\u{266a}" },
                { "startTimeMs": "3000", "words": "World" }
            ]}
        });
        let lyrics = from_spotify(&json).unwrap();
        assert!(lyrics.synced);
        let result = lyrics.into_result("spotify");
        let lines: Vec<_> = result.synced.unwrap().into_iter().map(|l| (l.time_ms, l.text)).collect();
        assert_eq!(lines, vec![(1000, "Hello".to_string()), (3000, "World".to_string())]);
    }
}
