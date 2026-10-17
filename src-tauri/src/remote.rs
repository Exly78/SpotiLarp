//! Controlling SpotiLarp from the command line: `spotilarp next`,
//! `spotilarp volume 40`, `spotilarp spotify:album:...`. Launching it again
//! hands the arguments to the copy already running (the single-instance
//! plugin), which acts on them without opening a second window.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;

pub const REMOTE_COMMAND_EVENT: &str = "remote-command";
const DEFAULT_VOLUME_STEP: i32 = 10;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum RemoteCommand {
    Toggle,
    Play,
    Pause,
    Next,
    Previous,
    Volume { percent: u8 },
    VolumeBy { delta: i32 },
    Mute,
    /// "off", "on" or "smart"; `None` cycles through them.
    Shuffle { mode: Option<String> },
    /// "off", "all" or "one"; `None` cycles.
    Repeat { mode: Option<String> },
    Like,
    SeekBy { seconds: i64 },
    SeekTo { seconds: u64 },
    /// A `spotify:` URI or `open.spotify.com` link, played or opened by the UI.
    Open { link: String },
    Search { query: String },
    Show,
    /// Back to the built-in look, for a theme that made the window unusable.
    ResetTheme,
}

impl RemoteCommand {
    /// Whether the window should come forward for this command.
    fn shows_window(&self) -> bool {
        matches!(self, Self::Show | Self::Open { .. } | Self::Search { .. } | Self::ResetTheme)
    }
}

/// The command in a launch's arguments (the first being the program), if any.
pub fn parse(args: &[String]) -> Option<RemoteCommand> {
    let words: Vec<&str> = args
        .iter()
        .skip(1)
        .map(String::as_str)
        // Flags like --minimized (autostart), and the "--" before a negative seek.
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    let (&verb, rest) = words.split_first()?;
    if is_link(verb) {
        return Some(RemoteCommand::Open { link: verb.to_string() });
    }
    let first = rest.first().copied();
    let command = match verb.to_ascii_lowercase().as_str() {
        "play-pause" | "toggle" => RemoteCommand::Toggle,
        "play" => RemoteCommand::Play,
        "pause" => RemoteCommand::Pause,
        "next" => RemoteCommand::Next,
        "previous" | "prev" => RemoteCommand::Previous,
        "volume" => RemoteCommand::Volume { percent: first?.parse::<u8>().ok()?.min(100) },
        "volume-up" => RemoteCommand::VolumeBy { delta: step(first)? },
        "volume-down" => RemoteCommand::VolumeBy { delta: -step(first)? },
        "mute" => RemoteCommand::Mute,
        "shuffle" => RemoteCommand::Shuffle { mode: optional(first, shuffle_mode)? },
        "repeat" => RemoteCommand::Repeat { mode: optional(first, repeat_mode)? },
        "like" => RemoteCommand::Like,
        "seek" => RemoteCommand::SeekBy { seconds: first?.parse().ok()? },
        "seek-to" => RemoteCommand::SeekTo { seconds: first?.parse().ok()? },
        "play-uri" | "open" => RemoteCommand::Open { link: first.filter(|link| is_link(link))?.to_string() },
        "search" => RemoteCommand::Search { query: rest.join(" ") },
        "show" => RemoteCommand::Show,
        "reset-theme" => RemoteCommand::ResetTheme,
        _ => return None,
    };
    Some(command)
}

fn is_link(arg: &str) -> bool {
    arg.starts_with("spotify:") || arg.contains("open.spotify.com/")
}

/// `Some(None)` without an argument, `None` when the argument is invalid.
fn optional<T>(arg: Option<&str>, parse: impl FnOnce(&str) -> Option<T>) -> Option<Option<T>> {
    match arg {
        Some(arg) => parse(arg).map(Some),
        None => Some(None),
    }
}

fn step(arg: Option<&str>) -> Option<i32> {
    match arg {
        Some(value) => value.parse::<i32>().ok().filter(|step| (1..=100).contains(step)),
        None => Some(DEFAULT_VOLUME_STEP),
    }
}

fn shuffle_mode(arg: &str) -> Option<String> {
    let mode = match arg.to_ascii_lowercase().as_str() {
        "on" | "true" | "1" => "on",
        "off" | "false" | "0" => "off",
        "smart" => "smart",
        _ => return None,
    };
    Some(mode.to_string())
}

fn repeat_mode(arg: &str) -> Option<String> {
    let mode = match arg.to_ascii_lowercase().as_str() {
        "off" => "off",
        "all" | "context" | "playlist" => "all",
        "one" | "track" | "song" => "one",
        _ => return None,
    };
    Some(mode.to_string())
}

/// Hands a command from another launch to the UI.
pub fn dispatch(app: &AppHandle, command: RemoteCommand) {
    if command.shows_window() {
        crate::tray::show_main_window(app);
    }
    if command != RemoteCommand::Show {
        let _ = app.emit(REMOTE_COMMAND_EVENT, command);
    }
}

/// Commands this launch started with, for the UI to pick up once it's ready.
pub fn take_pending(app: &AppHandle) -> Vec<RemoteCommand> {
    app.state::<AppState>()
        .pending_commands
        .lock()
        .map(|mut pending| std::mem::take(&mut *pending))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_words(words: &[&str]) -> Option<RemoteCommand> {
        let args: Vec<String> = std::iter::once("spotilarp").chain(words.iter().copied()).map(String::from).collect();
        parse(&args)
    }

    #[test]
    fn plain_launches_have_no_command() {
        assert_eq!(parse_words(&[]), None);
        assert_eq!(parse_words(&["--minimized"]), None);
        assert_eq!(parse_words(&["bogus"]), None);
    }

    #[test]
    fn verbs_parse() {
        assert_eq!(parse_words(&["next"]), Some(RemoteCommand::Next));
        assert_eq!(parse_words(&["volume", "140"]), Some(RemoteCommand::Volume { percent: 100 }));
        assert_eq!(parse_words(&["volume-down"]), Some(RemoteCommand::VolumeBy { delta: -10 }));
        assert_eq!(parse_words(&["volume-up", "5"]), Some(RemoteCommand::VolumeBy { delta: 5 }));
        assert_eq!(parse_words(&["shuffle"]), Some(RemoteCommand::Shuffle { mode: None }));
        assert_eq!(parse_words(&["shuffle", "off"]), Some(RemoteCommand::Shuffle { mode: Some("off".into()) }));
        assert_eq!(parse_words(&["shuffle", "Smart"]), Some(RemoteCommand::Shuffle { mode: Some("smart".into()) }));
        assert_eq!(parse_words(&["shuffle", "maybe"]), None);
        assert_eq!(parse_words(&["repeat", "track"]), Some(RemoteCommand::Repeat { mode: Some("one".into()) }));
        assert_eq!(parse_words(&["seek", "--", "-15"]), Some(RemoteCommand::SeekBy { seconds: -15 }));
        assert_eq!(parse_words(&["search", "here", "comes"]), Some(RemoteCommand::Search { query: "here comes".into() }));
    }

    #[test]
    fn links_open() {
        let link = "https://open.spotify.com/album/4aawyAB9vmqN3uQ7FjRGTy?si=x";
        assert_eq!(parse_words(&[link]), Some(RemoteCommand::Open { link: link.into() }));
        assert_eq!(
            parse_words(&["play-uri", "spotify:playlist:37i9dQZF1DXcBWIGoYBM5M"]),
            Some(RemoteCommand::Open { link: "spotify:playlist:37i9dQZF1DXcBWIGoYBM5M".into() })
        );
        assert_eq!(parse_words(&["play-uri", "nope"]), None);
    }
}
