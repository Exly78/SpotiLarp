use serde::{Deserialize, Serialize};

const LRCLIB_BASE: &str = "https://lrclib.net/api";

#[derive(Deserialize)]
struct LrcLibTrack {
    #[serde(rename = "plainLyrics")]
    plain_lyrics: Option<String>,
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    instrumental: bool,
}

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
}

pub async fn fetch(
    track_name: &str,
    artist_name: &str,
    album_name: &str,
    duration_ms: u32,
) -> Result<Option<LyricsResult>, String> {
    let http = reqwest::Client::new();
    let duration_secs = (duration_ms / 1000).to_string();

    let exact = http
        .get(format!("{LRCLIB_BASE}/get"))
        .query(&[
            ("track_name", track_name),
            ("artist_name", artist_name),
            ("album_name", album_name),
            ("duration", &duration_secs),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let found = if exact.status().is_success() {
        Some(exact.json::<LrcLibTrack>().await.map_err(|e| e.to_string())?)
    } else {
        let search = http
            .get(format!("{LRCLIB_BASE}/search"))
            .query(&[("track_name", track_name), ("artist_name", artist_name)])
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !search.status().is_success() {
            return Ok(None);
        }
        let mut results = search
            .json::<Vec<LrcLibTrack>>()
            .await
            .map_err(|e| e.to_string())?;
        if results.is_empty() {
            return Ok(None);
        }
        Some(results.remove(0))
    };

    let Some(track) = found else {
        return Ok(None);
    };

    Ok(Some(LyricsResult {
        instrumental: track.instrumental,
        synced: track.synced_lyrics.as_deref().map(parse_lrc),
        plain: track.plain_lyrics,
    }))
}

fn parse_lrc(lrc: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();
    for line in lrc.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix('[') else {
            continue;
        };
        let Some(close) = rest.find(']') else {
            continue;
        };
        let (tag, text) = rest.split_at(close);
        let text = text[1..].trim().to_string();

        let mut parts = tag.splitn(2, ':');
        let (Some(min_str), Some(sec_str)) = (parts.next(), parts.next()) else {
            continue;
        };
        let (Ok(minutes), Ok(seconds)) = (min_str.parse::<u32>(), sec_str.parse::<f64>()) else {
            continue;
        };
        let time_ms = minutes * 60_000 + (seconds * 1000.0).round() as u32;
        lines.push(LyricLine { time_ms, text });
    }
    lines.sort_by_key(|l| l.time_ms);
    lines
}
