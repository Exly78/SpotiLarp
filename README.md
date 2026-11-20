# SpotiLarp

![version](https://img.shields.io/badge/version-1.3.0-blue)
![status](https://img.shields.io/badge/status-early%20WIP-orange)
![last updated](https://img.shields.io/badge/last%20updated-2026--11--21-lightgrey)
![license](https://img.shields.io/badge/license-MIT-green)
![built with](https://img.shields.io/badge/built%20with-Rust%20%2B%20Tauri%20%2B%20SvelteKit-informational)

A low-RAM Spotify client built with Rust + Tauri, with a SvelteKit frontend and
[librespot](https://github.com/librespot-org/librespot) for playback. Includes lyrics and
Discord Rich Presence support.

> [!IMPORTANT]
> I'm new to Rust and learning as I go, so bugs are to be expected this is a really early,
> work-in-progress version. Use at your own risk and expect rough edges.

> [!WARNING]
> A **Spotify Premium** account is required for SpotiLarp's own player. Spotify's playback
> API does not support free/ad-supported accounts. Without Premium, SpotiLarp can still play
> through Spotify's desktop app on the same computer (see "Without Premium" below).

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
- On Linux, GStreamer's plugins, for the Now Playing panel's Canvas videos:
  `gst-plugins-good gst-libav` (Arch), `gstreamer1.0-plugins-good gstreamer1.0-libav`
  (Debian/Ubuntu) or `gstreamer1-plugins-good gstreamer1-plugin-openh264` (Fedora). Without
  them the panel shows the cover instead.

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

Before a release, add what's new in it to `RELEASES` in `src/lib/tourSteps.js`, under the
version in `package.json`: each entry is a title, a sentence or two, and the control it
points at. Everyone updating from an older version gets a short "What's new" tour of
those steps once (and every release they skipped), and Settings → Tour shows the latest
one again. New installs get the regular feature tour instead.

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
- **Jam: listen together.** Open Jam (the people button next to the queue, or `J`), start one
  and send friends the code; they paste it into their SpotiLarp under Jam → Join. Everyone
  hears the same songs at the same place on their own computer, so each person needs SpotiLarp
  and Spotify Premium. Listeners can add songs (playing or queueing anything adds it to the
  Jam), and the host decides whether they can, whether songs wait for the host to accept them,
  and whether listeners can pause and skip for everyone; the host can also remove people and
  end the Jam. Anyone can save what played as a playlist. With Discord Rich Presence on, your
  status gets a **Listen along on SpotiLarp** button (and a **Get SpotiLarp** one): friends who
  click it hear what you're playing straight away but can't add songs, and you get a popup to
  make it a Jam with them (or keep them just listening, or remove them). The button opens a small page (`docs/listen`, served by GitHub
  Pages) that passes the code to SpotiLarp as a `spotilarp://` link. The apps connect straight to each
  other using [iroh](https://www.iroh.computer); when a router won't allow that, the
  (encrypted) traffic goes through iroh's free public relays, which is also how a code finds
  its Jam. Nobody has to run a server. Windows may ask once whether SpotiLarp can use the
  network. Songs from Local Files aren't shared: listeners pause until the host plays a
  Spotify song again.
- **Spotify Connect works both ways.** SpotiLarp shows up in your other Spotify apps' device
  lists (your phone, Spotify's desktop app...) as "SpotiLarp" (rename it under Settings →
  Spotify Connect; turning it off there turns off both directions), and they can play on it. While one does, the player bar says so
  and its buttons control that app's queue; playing something in SpotiLarp takes over again.
  The devices button next to the queue does the opposite, playing SpotiLarp's music on another
  device: SpotiLarp's queue (smart shuffle, autoplay, a Jam) still decides what plays and sends
  each song as it comes, so there can be a short gap between songs, and the volume slider sets
  that device's volume. Local files only play on this computer, and starting something else
  on that device from a Spotify app makes SpotiLarp let go of it. Like playback, this goes
  through the playback login (as commands from SpotiLarp's own Connect device, the way
  Spotify's apps send them), not your Client ID.
- **SpotiLarp can play through your Spotify app.** Turn on Settings → Spotify desktop app →
  Play through the Spotify app, and whenever Spotify's own app is open on the same computer,
  SpotiLarp's music plays in it and SpotiLarp just controls it (picking "This computer" in the
  devices menu keeps it in SpotiLarp until it restarts). That includes Jams and Discord's
  **Listen along on SpotiLarp**: a friend who'd rather listen in Spotify installs SpotiLarp,
  turns this on, and clicks Listen along, and SpotiLarp keeps their Spotify in time with you
  from the background. Everyone still needs SpotiLarp: Spotify has no public way for its own
  Jams and SpotiLarp's to meet.
- **Without Premium**, Spotify won't take orders from other apps, so SpotiLarp reaches the
  Spotify desktop app through a small extension for [Spicetify](https://spicetify.app)
  instead. Install Spicetify, then Settings → Spotify desktop app → Set up writes SpotiLarp's extension
  into it and restarts Spotify; with "Play through the Spotify app" on, SpotiLarp (a Jam,
  listening along) then plays through it on a free account. Spotify's ads still play:
  SpotiLarp shows the song paused while one's on and carries on after it. The extension only
  talks to SpotiLarp on the same computer, with a key the two share.
- **Discord status:** the Discord menu in the top bar turns on "Listening to SpotiLarp" on your
  Discord profile, nothing to set up.
- **Connect Discord** (in the Discord menu in the top bar) links your Discord account through
  Discord's own sign-in page, for Discord features to come. SpotiLarp only asks to know who you
  are (name, @username, picture). The login is kept in your OS keychain, and Disconnect tells
  Discord to drop SpotiLarp's access too.
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
  Linux setups with no Secret Service running (no gnome-keyring, KWallet or KeePassXC, as
  on many OpenRC systems) keep them in `~/.config/SpotiLarp/logins.json` instead, readable
  only by you, and move into a Secret Service the next time they're saved with one running.

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
