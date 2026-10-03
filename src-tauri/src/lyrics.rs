use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

const LRCLIB_BASE: &str = "https://lrclib.net/api";
const DURATION_TOLERANCE_SECS: f64 = 3.0;

#[derive(Deserialize)]
struct LrcLibTrack {
    #[serde(rename = "plainLyrics")]
    plain_lyrics: Option<String>,
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    #[serde(default)]
    instrumental: bool,
    #[serde(default)]
    duration: Option<f64>,
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

pub async fn fetch(
    track_name: &str,
    artist_name: &str,
    album_name: &str,
    duration_ms: u32,
) -> Result<Option<LyricsResult>, String> {
    let http = http();
    let duration_secs = (duration_ms as f64 / 1000.0).round();

    let exact = http
        .get(format!("{LRCLIB_BASE}/get"))
        .query(&[
            ("track_name", track_name),
            ("artist_name", artist_name),
            ("album_name", album_name),
            ("duration", &duration_secs.to_string()),
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
        let results = search
            .json::<Vec<LrcLibTrack>>()
            .await
            .map_err(|e| e.to_string())?;
        best_match(results, duration_secs)
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

fn best_match(results: Vec<LrcLibTrack>, duration_secs: f64) -> Option<LrcLibTrack> {
    results
        .into_iter()
        .filter(|t| t.instrumental || t.synced_lyrics.is_some() || t.plain_lyrics.is_some())
        .min_by_key(|t| {
            let offset = t
                .duration
                .map(|d| (d - duration_secs).abs())
                .unwrap_or(f64::MAX);
            (
                offset > DURATION_TOLERANCE_SECS,
                t.synced_lyrics.is_none(),
                (offset.min(1e9) * 1000.0) as u64,
            )
        })
}

fn parse_timestamp(tag: &str) -> Option<u32> {
    let (min_str, sec_str) = tag.split_once(':')?;
    let minutes = min_str.trim().parse::<u32>().ok()?;
    let seconds = sec_str.trim().parse::<f64>().ok()?;
    Some(minutes * 60_000 + (seconds * 1000.0).round() as u32)
}

fn parse_lrc(lrc: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();
    for line in lrc.lines() {
        let mut rest = line.trim();
        let mut times = Vec::new();
        while let Some(after_open) = rest.strip_prefix('[') {
            let Some(close) = after_open.find(']') else {
                break;
            };
            let Some(time_ms) = parse_timestamp(&after_open[..close]) else {
                break;
            };
            times.push(time_ms);
            rest = &after_open[close + 1..];
        }
        let text = rest.trim();
        for time_ms in times {
            lines.push(LyricLine {
                time_ms,
                text: text.to_string(),
            });
        }
    }
    lines.sort_by_key(|l| l.time_ms);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_repeated_and_metadata_lines() {
        let lines = parse_lrc("[ar:Someone]\n[00:12.50][01:00.00]Chorus\n[00:05.00]Intro\n[00:20.00]");
        let parsed: Vec<_> = lines.iter().map(|l| (l.time_ms, l.text.as_str())).collect();
        assert_eq!(
            parsed,
            vec![(5_000, "Intro"), (12_500, "Chorus"), (20_000, ""), (60_000, "Chorus")]
        );
    }

    #[test]
    fn prefers_matching_duration_then_synced() {
        let track = |duration: f64, synced: bool| LrcLibTrack {
            plain_lyrics: Some("plain".into()),
            synced_lyrics: synced.then(|| "[00:01.00]x".into()),
            instrumental: false,
            duration: Some(duration),
        };
        let best = best_match(vec![track(300.0, true), track(201.0, false), track(199.0, true)], 200.0).unwrap();
        assert_eq!(best.duration, Some(199.0));
    }
}
