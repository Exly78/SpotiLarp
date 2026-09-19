# SpotiLarp

![version](https://img.shields.io/badge/version-0.1.2-blue)
![status](https://img.shields.io/badge/status-early%20WIP-orange)
![last updated](https://img.shields.io/badge/last%20updated-2026--09--19-lightgrey)
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

## Project structure

- `src/`  SvelteKit frontend (UI, components, API client)
- `src-tauri/`  Rust backend (Tauri commands, Spotify auth/API, librespot playback, Discord RPC)
