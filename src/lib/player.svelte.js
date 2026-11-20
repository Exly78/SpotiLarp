import { listen } from "@tauri-apps/api/event";
import * as api from "./api.js";
import { notify } from "./toast.svelte.js";
import { queue, next, previous, hasPrevious, prepareNext, playCurrent, slim, updateCurrent, jamGuest } from "./queue.svelte.js";
import { isEpisodeUri, recordPosition, episodeTrack } from "./podcasts.svelte.js";
import { setFadeHandler } from "./sleep.svelte.js";
import { dominantColor } from "./colors.js";
import { LOCAL_URI_PREFIX } from "./constants.js";
import { connect, onOutputChange, applyStatus } from "./connect.svelte.js";

const VOLUME_KEY = "spotilarp.volume";
const SESSION_KEY = "spotilarp.nowPlaying";
const DEFAULT_VOLUME = 50;
const TICK_MS = 200;
const SAVE_POSITION_MS = 5000;
const RESTART_THRESHOLD_MS = 3000;
/** Another device's volume goes to Spotify once the slider settles, not on every step. */
const REMOTE_VOLUME_DELAY_MS = 250;

/**
 * @typedef {Object} CurrentTrack
 * @property {string} name
 * @property {string} artists
 * @property {string|null} primaryArtistId
 * @property {{ id: string|null, name: string }[]} [artistList]
 * @property {string|null} trackId
 * @property {string} [uri]
 * @property {string} album
 * @property {number} durationMs
 * @property {string|null} coverUrl
 */

const saved = readSavedSession();

/** @type {{ track: CurrentTrack|null, isPlaying: boolean, positionMs: number, volume: number, loaded: boolean, color: string }} */
export const player = $state({
  track: saved?.track ?? null,
  isPlaying: false,
  positionMs: saved?.positionMs ?? 0,
  volume: readSavedVolume(),
  loaded: false,
  color: "",
});

let fadeLevel = 1;

let anchorMs = player.positionMs;
let anchorAt = 0;
let volumeBeforeMute = DEFAULT_VOLUME;
/** @type {ReturnType<typeof setTimeout>|undefined} */
let remoteVolumeTimer;
/** Set while this app moves the music itself, so it isn't also treated as being moved. */
let switching = false;

function readSavedVolume() {
  try {
    const raw = localStorage.getItem(VOLUME_KEY);
    const saved = Number(raw);
    if (raw !== null && saved >= 0 && saved <= 100) return saved;
  } catch {}
  return DEFAULT_VOLUME;
}

/** @returns {{ track: CurrentTrack, positionMs: number }|null} */
function readSavedSession() {
  try {
    const session = JSON.parse(localStorage.getItem(SESSION_KEY) ?? "null");
    if (session?.track && queue.current) return session;
  } catch {}
  return null;
}

/** @param {string|null} url */
function updateColor(url) {
  dominantColor(url).then((color) => {
    if ((player.track?.coverUrl ?? null) === url) player.color = color;
  });
}
updateColor(player.track?.coverUrl ?? null);

setFadeHandler((level) => {
  fadeLevel = level;
  sendVolume(Math.round((player.volume / 100) * 65535 * level)).catch(() => {});
});

/**
 * @param {number} level 0 to 65535
 * @returns {Promise<unknown>}
 */
function sendVolume(level) {
  if (connect.output !== "remote") return api.setVolume(level);
  clearTimeout(remoteVolumeTimer);
  return new Promise((resolve, reject) => {
    remoteVolumeTimer = setTimeout(() => api.setVolume(level).then(resolve, reject), REMOTE_VOLUME_DELAY_MS);
  });
}

/** Where playback is right now (the on-screen position only moves a few times a second). */
export function currentPositionMs() {
  if (!player.isPlaying || !player.track) return player.positionMs;
  return Math.min(anchorMs + (performance.now() - anchorAt), player.track.durationMs);
}

function saveSession() {
  // Another app's song isn't SpotiLarp's to come back to.
  if (connect.output === "receiving") return;
  try {
    if (player.track) {
      const session = { track: $state.snapshot(player.track), positionMs: Math.round(currentPositionMs()) };
      localStorage.setItem(SESSION_KEY, JSON.stringify(session));
    }
  } catch {}
}

/** @param {number} ms */
function setPosition(ms) {
  anchorMs = ms;
  anchorAt = performance.now();
  player.positionMs = ms;
}

setInterval(() => {
  if (!player.isPlaying || !player.track) return;
  const ms = anchorMs + (performance.now() - anchorAt);
  player.positionMs = Math.min(ms, player.track.durationMs);
}, TICK_MS);

/** Remembers your place in a podcast episode. */
function recordEpisode() {
  const t = player.track;
  if (!t?.uri || !isEpisodeUri(t.uri) || !player.loaded) return;
  const track = queue.current?.uri === t.uri ? slim(queue.current) : null;
  recordPosition(t.uri, currentPositionMs(), t.durationMs, track);
}

/**
 * Search results don't say which show an episode is from, so it's asked for
 * once the episode plays.
 * @param {string} uri
 */
async function completeEpisode(uri) {
  const current = queue.current;
  if (!current || current.uri !== uri || current.show) return;
  try {
    const episode = await api.getEpisode(current.id);
    if (!episode.show) return;
    const { show, artists, album } = episodeTrack(episode);
    updateCurrent(uri, { show, artists, album });
  } catch {}
}

setInterval(() => {
  if (!player.isPlaying) return;
  saveSession();
  recordEpisode();
}, SAVE_POSITION_MS);
window.addEventListener("pagehide", () => {
  saveSession();
  recordEpisode();
});

api.setVolume(Math.round((player.volume / 100) * 65535)).catch(() => {});

listen("player-event", (event) => {
  const e = /** @type {import("./types.js").PlaybackEvent} */ (event.payload);
  switch (e.type) {
    case "TrackChanged":
      // Your place in the episode being left.
      recordEpisode();
      player.loaded = true;
      player.track = {
        name: e.name ?? "",
        artists: e.artists ?? "",
        primaryArtistId: e.primary_artist_id ?? null,
        artistList: e.artist_list ?? [],
        trackId: e.track_id ?? null,
        uri: e.uri ?? "",
        album: e.album ?? "",
        durationMs: e.duration_ms ?? 0,
        coverUrl: e.cover_url ?? null,
      };
      setPosition(0);
      saveSession();
      updateColor(player.track.coverUrl);
      if (isEpisodeUri(e.uri)) completeEpisode(e.uri ?? "");
      break;
    case "Playing":
      player.isPlaying = true;
      setPosition(e.position_ms ?? 0);
      break;
    case "Paused":
      player.isPlaying = false;
      setPosition(e.position_ms ?? 0);
      saveSession();
      recordEpisode();
      break;
    case "PositionChanged":
      setPosition(e.position_ms ?? 0);
      break;
    case "Seeked":
      setPosition(e.position_ms ?? 0);
      recordEpisode();
      break;
    case "Stopped":
    case "EndOfTrack":
    case "Unavailable":
      player.isPlaying = false;
      player.loaded = false;
      if (e.type === "EndOfTrack") setPosition(0);
      break;
    case "PreloadNext":
      if (jamGuest()) break;
      prepareNext().then((upcoming) => {
        if (upcoming) api.preloadTrack(upcoming.uri).catch(() => {});
      });
      break;
    case "VolumeChanged":
      if (fadeLevel === 1) player.volume = Math.round(((e.volume ?? 0) / 65535) * 100);
      break;
  }
});

listen("media-control", (event) => {
  const command = /** @type {{ action: string, offset_ms?: number, position_ms?: number }} */ (event.payload);
  switch (command.action) {
    case "play":
      if (!player.isPlaying) togglePlay();
      break;
    case "pause":
      if (player.isPlaying) togglePlay();
      break;
    case "toggle":
      togglePlay();
      break;
    case "next":
      next();
      break;
    case "previous":
      previousOrRestart();
      break;
    case "seek_by":
      if (player.track) {
        const target = player.positionMs + (command.offset_ms ?? 0);
        seekTo(Math.min(Math.max(target, 0), player.track.durationMs));
      }
      break;
    case "set_position":
      seekTo(command.position_ms ?? 0);
      break;
  }
});

export async function togglePlay() {
  const guest = jamGuest();
  if (guest) return guest.control("toggle");
  if (!player.track) return;
  try {
    if (!player.loaded) {
      await playCurrent(player.positionMs);
    } else if (player.isPlaying) {
      await api.pause();
    } else {
      await api.resume();
    }
  } catch (e) {
    notify(`${e}`);
  }
}

export function restartAfterPlayerRebuild() {
  const wasPlaying = player.isPlaying;
  player.loaded = false;
  player.isPlaying = false;
  if (wasPlaying) playCurrent(player.positionMs);
}

// Spotify dropped the playback session and the backend connected again: a
// Spotify song carries on from where it was (a local file never stopped).
listen("playback-reconnected", () => {
  // Another device carries on regardless.
  if (connect.output === "remote") return;
  if (!player.track || player.track.uri?.startsWith(LOCAL_URI_PREFIX)) return;
  const wasPlaying = player.isPlaying;
  restartAfterPlayerRebuild();
  if (wasPlaying) notify("Reconnected to Spotify");
});

export function resetPlayer() {
  player.track = null;
  player.isPlaying = false;
  player.loaded = false;
  player.color = "";
  setPosition(0);
  try {
    localStorage.removeItem(SESSION_KEY);
  } catch {}
}

export function previousOrRestart() {
  const guest = jamGuest();
  if (guest) return guest.control("previous");
  if (connect.output === "receiving") return api.connectControl("previous").catch((e) => notify(`${e}`));
  if (player.positionMs > RESTART_THRESHOLD_MS || !hasPrevious()) seekTo(0);
  else previous();
}

/** @param {number} ms */
export async function seekTo(ms) {
  const guest = jamGuest();
  if (guest) return guest.control("seek", ms);
  setPosition(ms);
  if (!player.loaded) return saveSession();
  try {
    await api.seek(Math.round(ms));
  } catch (e) {
    notify(`${e}`);
  }
}

/** @param {number} ms forward, or back when negative */
export function seekBy(ms) {
  if (!player.track) return;
  seekTo(Math.min(Math.max(player.positionMs + ms, 0), player.track.durationMs));
}

/** @param {number} pct */
export function setVolume(pct) {
  player.volume = pct;
  // Another device's volume is its own; this computer's is kept for coming back to.
  if (connect.output === "here") {
    try {
      localStorage.setItem(VOLUME_KEY, String(pct));
    } catch {}
  }
  sendVolume(Math.round((pct / 100) * 65535 * fadeLevel)).catch((e) => notify(`${e}`));
}

export function toggleMute() {
  if (player.volume > 0) {
    volumeBeforeMute = player.volume;
    setVolume(0);
  } else {
    setVolume(volumeBeforeMute || DEFAULT_VOLUME);
  }
}

onOutputChange((from, to) => {
  if (from === "receiving") {
    // The other app's song goes, and SpotiLarp's own comes back as it was left.
    const session = readSavedSession();
    player.track = session?.track ?? null;
    setPosition(session?.positionMs ?? 0);
    updateColor(player.track?.coverUrl ?? null);
  }
  if (to === "here") {
    if (!switching) {
      // Nothing is loaded here: playing again starts SpotiLarp's song here.
      player.isPlaying = false;
      player.loaded = false;
    }
    setVolume(readSavedVolume());
  }
});

/**
 * Plays on another Spotify Connect device, or back on this computer (`null`),
 * carrying on from the same place in the song.
 * @param {import("./api.js").ConnectDevice|null} device
 */
export async function playOn(device) {
  const wasPlaying = player.isPlaying;
  const positionMs = currentPositionMs();
  const from = connect.output;
  switching = true;
  try {
    applyStatus(await api.connectSelect(device?.id ?? null, device?.name));
  } catch (e) {
    notify(`${e}`);
    return;
  } finally {
    switching = false;
  }
  // Another app's music was playing here: Spotify moves it there itself.
  if (from === "receiving") {
    if (device) notify(`Playing on ${device.name}`);
    return;
  }
  player.loaded = false;
  player.isPlaying = false;
  if (wasPlaying || jamGuest()) await playCurrent(positionMs);
}
