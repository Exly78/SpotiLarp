# SpotiLarp

![version](https://img.shields.io/badge/version-1.1.0-blue)
![status](https://img.shields.io/badge/status-early%20WIP-orange)
![last updated](https://img.shields.io/badge/last%20updated-2026--10--18-lightgrey)
![license](https://img.shields.io/badge/license-MIT-green)
![built with](https://img.shields.io/badge/built%20with-Rust%20%2B%20Tauri%20%2B%20SvelteKit-informational)

A low-RAM Spotify client built with Rust + Tauri, with a SvelteKit frontend and
[librespot](https://github.com/librespot-org/librespot) for playback. Includes lyrics and
Discord Rich Presence support.

> [!IMPORTANT]
> I'm new to Rust and learning as I go, so bugs are to be expected this is a really early,
> work-in-progress version. Use at your own risk and expect rough edges.

> [!WARNING]
> A **Spotify Premium** account is required. Spotify's playback API does not support
> free/ad-supported accounts playback will not work without Premium.

## One-time Spotify setup

SpotiLarp talks to Spotify using your own free developer app, since it doesn't ship a
shared API key:

1. Go to the [Spotify Developer Dashboard](https://developer.spotify.com/dashboard) and log in.
2. Click **Create app** (any name/description works).
3. In the app's **Settings**, add this exact Redirect URI:
   ```
   http://127.0.0.1:8898/callback
   ```
4. Copy the app's **Client ID** you'll paste it into SpotiLarp the first time you run it.

> [!NOTE]
> This is a one-time setup per computer. Your Client ID is stored locally, not shared with anyone.
> Spotify limits how often each app can ask it things; if it holds yours back for hours, make
> another app and switch to its Client ID in Settings → Spotify app.

## Getting started

### Option 1: Download the installer (easiest, for Windows)

Grab the latest `.msi` from the [Releases](https://github.com/Exly78/SpotiLarp/releases) page, run it, and launch SpotiLarp
like any other app. No Node.js or Rust required just do the [one-time Spotify
setup](#one-time-spotify-setup) above on first launch.

### Option 2: Build from source (for developers)

Prerequisites:

- [Node.js](https://nodejs.org/) (LTS)
- [Rust](https://www.rust-lang.org/tools/install)
- The [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS

```bash
npm install
npm run tauri dev
```

This launches the desktop app with the Rust/Tauri backend, which is required for playback,
auth, and all Spotify features.

> [!WARNING]
> `npm run dev` alone only starts the SvelteKit frontend in a browser (via Vite) without the
> Tauri backend, so Spotify features won't work there it's mainly useful for quick UI/CSS
> iteration.

## Building an installer

```bash
npm run tauri build
```

Produces a release binary and installer(s) for your platform under
`src-tauri/target/release/bundle/` (e.g. `msi/` on Windows).

## Good to know

- **Playback depends on librespot.** Browsing your library uses your own Client ID, but
  playback goes through librespot, which logs in as Spotify's official desktop app (that's
  the second browser login, "Connect playback"). This isn't an official Spotify API, so a
  change on Spotify's side can break playback until librespot catches up.
- **Local Files plays songs from your computer.** Add music folders under Settings → Local
  files (MP3, FLAC, M4A, OGG and WAV). Title, artist, album and cover art come from each
  file's tags, or a `cover.jpg`/`folder.jpg` next to it. Local songs play through SpotiLarp's
  own player, not librespot, so they work at any sample rate and don't need Premium. Hit
  Rescan after adding new files.
- **Lyrics come from Spotify, or [LRCLIB](https://lrclib.net).** While the lyrics view is
  open, SpotiLarp first asks Spotify (through the playback login) for the song's own lyrics.
  If Spotify has none, or it's a local file, the song's title, artist, album and length are
  sent to lrclib.net instead. Answers are cached on disk for 30 days. Nothing is sent while
  lyrics are closed.
- **Podcasts play too.** Follow shows, browse their episodes (newest or oldest first) and find
  them in search. Spotify doesn't hear about listening in SpotiLarp, so your place in each
  episode and which ones you finished are kept on this computer (clear them in Settings →
  Storage); progress from your phone doesn't show up here. Home lists episodes you're partway
  through and new ones from the podcasts you follow (asked for once every 15 minutes at most).
  Some podcasts that Spotify streams from the publisher's own site, and video-only episodes,
  may not play.
- **Smart shuffle** (click shuffle twice, or `spotilarp shuffle smart`) keeps one artist's songs
  apart and saves the songs you played in the last day or week for the end. Podcast episodes
  always play in order.
- **Your listening history stays on your computer.** Spotify doesn't record songs played in
  SpotiLarp, so it keeps its own list (a song counts after 30 seconds, or half of a shorter
  one) and shows it next to Spotify's history under Queue → Recent. Clear it in Settings →
  Storage.
- **It gets lighter in the background.** About 30 seconds after the window is minimized or
  sent to the tray, SpotiLarp lets go of its interface and asks WebView2 to trim itself
  (in our tests about 20 MB instead of about 100 MB with the window open). Music, the queue and
  your history carry on; everything comes back when the window does.
- **Themes work like Spicetify's.** Pick one under Settings → Appearance: a few built-in
  color schemes, themes you add, and any you installed for Spicetify (they're found in
  Spicetify's Themes folder). A theme is a folder with a `color.ini` of color schemes and,
  optionally, a `user.css`. Color schemes work as they do in Spotify. A Spicetify theme's
  user.css was written for Spotify's own window, so only part of it (fonts, scrollbars, general
  rules) carries over, and it's off until you turn it on. SpotiLarp's own variables (`--text`,
  `--accent`, `--main-bg`, `--sidebar-bg`, `--player-bg` and the rest) can be set from a
  user.css too. If a theme ever leaves the window unusable, `spotilarp reset-theme` puts the
  default back.
- **Logins are stored in your OS keychain** (Windows Credential Manager, macOS Keychain,
  Secret Service on Linux). Logging out removes both the library and the playback login.

## Controlling it from the command line

Running SpotiLarp's program again with a command controls the copy that's already open,
handy for launchers, hotkey tools and Stream Deck-style buttons. On Windows the program is
`spotilarp.exe` in SpotiLarp's install folder.

```
spotilarp play-pause          spotilarp volume 40
spotilarp play | pause        spotilarp volume-up [step] | volume-down [step]
spotilarp next | previous     spotilarp mute
spotilarp seek 15             spotilarp shuffle [on|smart|off]
spotilarp seek -- -15         spotilarp repeat [off|all|one]
spotilarp seek-to 90          spotilarp like
spotilarp search <words>      spotilarp <spotify: URI or open.spotify.com link>
spotilarp reset-theme
```

`shuffle` and `repeat` cycle without an argument. A link plays the song, album,
playlist or artist. Pasting a link into the search box and pressing Enter opens it too. Press
`?` in the app for every keyboard shortcut.

Some of these features (radio, listening history, playlist folders, the command line,
taskbar buttons, lyrics matching) are adapted from
[Spotifast](https://github.com/crmne/spotifast) (MIT).

## Project structure

- `src/`  SvelteKit frontend (UI, components, API client)
- `src-tauri/`  Rust backend (Tauri commands, Spotify auth/API, librespot playback, Discord RPC)
