import { listen } from "@tauri-apps/api/event";
import * as api from "./api.js";
import { notify } from "./toast.svelte.js";
import { player } from "./player.svelte.js";
import { queue, jamGuest } from "./queue.svelte.js";
import { isEpisodeUri } from "./podcasts.svelte.js";
import { LOCAL_URI_PREFIX } from "./constants.js";

/**
 * Scrobbling the songs played in SpotiLarp to Last.fm (lastfm.rs sends
 * them). Spotify's own apps scrobble through Spotify's Last.fm connection;
 * SpotiLarp's plays don't reach Spotify, so they'd be missing without this.
 */

const ENABLED_KEY = "spotilarp.lastfm.enabled";
/** Last.fm's rules: songs over 30 seconds, heard for half their length or 4 minutes. */
const MIN_LENGTH_MS = 30_000;
const ENOUGH_MS = 4 * 60_000;
const CHECK_EVERY_MS = 1000;

/**
 * `enabled` pauses scrobbling without disconnecting; `queued` scrobbles are
 * waiting for Last.fm (offline, say).
 * @type {{ connected: boolean, username: string|null, hasKeys: boolean, enabled: boolean, connecting: boolean, queued: number }}
 */
export const lastfm = $state({
  connected: false,
  username: null,
  hasKeys: false,
  enabled: readEnabled(),
  connecting: false,
  queued: 0,
});

let currentUri = "";
let currentDurationMs = 0;
/** When the play started, in seconds since 1970: Last.fm's timestamp for it. */
let startedAt = 0;
let listenedMs = 0;
let playingSince = 0;
let scrobbled = false;
let announced = false;

function readEnabled() {
  try {
    return localStorage.getItem(ENABLED_KEY) !== "false";
  } catch {
    return true;
  }
}

export async function refreshLastfm() {
  try {
    const status = await api.lastfmStatus();
    lastfm.connected = status.connected;
    lastfm.username = status.username;
    lastfm.hasKeys = status.has_keys;
    lastfm.queued = status.queued;
  } catch {}
}

/**
 * @param {string} apiKey
 * @param {string} secret
 */
export async function setLastfmKeys(apiKey, secret) {
  await api.lastfmSetKeys(apiKey, secret);
  await refreshLastfm();
}

export async function connectLastfm() {
  if (lastfm.connecting) return;
  lastfm.connecting = true;
  try {
    const username = await api.lastfmConnect();
    notify(`Connected Last.fm as ${username}`);
  } catch (e) {
    if (e !== api.LOGIN_CANCELLED) notify(`Couldn't connect Last.fm: ${e}`);
  } finally {
    lastfm.connecting = false;
    await refreshLastfm();
  }
}

export function cancelConnectLastfm() {
  api.lastfmCancelConnect().catch(() => {});
}

export async function disconnectLastfm() {
  try {
    await api.lastfmDisconnect();
    notify("Disconnected Last.fm");
  } catch (e) {
    notify(`Couldn't disconnect Last.fm: ${e}`);
  }
  await refreshLastfm();
}

export function toggleLastfmScrobbling() {
  lastfm.enabled = !lastfm.enabled;
  try {
    localStorage.setItem(ENABLED_KEY, String(lastfm.enabled));
  } catch {}
}

/** Whether this play is one to send: a Spotify song you're playing yourself. */
function scrobbling() {
  return (
    lastfm.connected &&
    lastfm.enabled &&
    !jamGuest() &&
    currentUri !== "" &&
    !currentUri.startsWith(LOCAL_URI_PREFIX) &&
    !isEpisodeUri(currentUri)
  );
}

/** @returns {import("./api.js").LastfmTrack|null} */
function playingTrack() {
  const track = queue.current?.uri === currentUri ? queue.current : null;
  if (track) {
    const artist = track.artists[0]?.name;
    if (!artist) return null;
    return { artist, track: track.name, album: track.album.name || null, duration: seconds(track.duration_ms) };
  }
  const t = player.track;
  if (!t || t.uri !== currentUri) return null;
  const artist = t.artistList?.[0]?.name ?? t.artists.split(",")[0]?.trim();
  if (!artist) return null;
  return { artist, track: t.name, album: t.album || null, duration: seconds(t.durationMs) };
}

/** @param {number} ms */
function seconds(ms) {
  return ms > 0 ? Math.round(ms / 1000) : null;
}

function announce() {
  if (announced || !scrobbling()) return;
  const track = playingTrack();
  if (!track) return;
  announced = true;
  api.lastfmNowPlaying(track).catch(() => {});
}

function check() {
  if (scrobbled || !scrobbling() || currentDurationMs <= MIN_LENGTH_MS) return;
  const listened = listenedMs + (playingSince ? performance.now() - playingSince : 0);
  if (listened < Math.min(currentDurationMs / 2, ENOUGH_MS)) return;
  const track = playingTrack();
  if (!track) return;
  scrobbled = true;
  api
    .lastfmScrobble([{ ...track, timestamp: startedAt }])
    .then((waiting) => (lastfm.queued = waiting))
    .catch(() => {});
}

function stopClock() {
  if (playingSince) listenedMs += performance.now() - playingSince;
  playingSince = 0;
}

setInterval(check, CHECK_EVERY_MS);
refreshLastfm();

listen("player-event", (event) => {
  const e = /** @type {import("./types.js").PlaybackEvent} */ (event.payload);
  switch (e.type) {
    case "TrackChanged":
      // The song before may have been heard long enough by now.
      check();
      // Every load is its own play, repeats of the same song included.
      currentUri = e.uri ?? "";
      currentDurationMs = e.duration_ms ?? 0;
      startedAt = 0;
      listenedMs = 0;
      playingSince = 0;
      scrobbled = false;
      announced = false;
      break;
    case "Playing":
      if (!playingSince) playingSince = performance.now();
      // A song loaded paused (at startup, say) starts when it first plays.
      if (!startedAt) startedAt = Math.floor(Date.now() / 1000);
      announce();
      break;
    case "Paused":
    case "Stopped":
    case "EndOfTrack":
    case "Unavailable":
      check();
      stopClock();
      break;
  }
});
