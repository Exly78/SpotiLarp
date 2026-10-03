import { listen } from "@tauri-apps/api/event";
import * as api from "./api.js";
import { notify } from "./toast.svelte.js";
import { queue, next, previous, hasPrevious, prepareNext, playCurrent } from "./queue.svelte.js";
import { setFadeHandler } from "./sleep.svelte.js";
import { dominantColor } from "./colors.js";

const VOLUME_KEY = "spotilarp.volume";
const SESSION_KEY = "spotilarp.nowPlaying";
const DEFAULT_VOLUME = 50;
const TICK_MS = 200;
const SAVE_POSITION_MS = 5000;
const RESTART_THRESHOLD_MS = 3000;

/**
 * @typedef {Object} CurrentTrack
 * @property {string} name
 * @property {string} artists
 * @property {string|null} primaryArtistId
 * @property {{ id: string|null, name: string }[]} [artistList]
 * @property {string|null} trackId
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
  api.setVolume(Math.round((player.volume / 100) * 65535 * level)).catch(() => {});
});

function saveSession() {
  try {
    if (player.track) {
      const session = { track: $state.snapshot(player.track), positionMs: Math.round(player.positionMs) };
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

setInterval(() => {
  if (player.isPlaying) saveSession();
}, SAVE_POSITION_MS);
window.addEventListener("pagehide", saveSession);

api.setVolume(Math.round((player.volume / 100) * 65535)).catch(() => {});

listen("player-event", (event) => {
  const e = /** @type {import("./types.js").PlaybackEvent} */ (event.payload);
  switch (e.type) {
    case "TrackChanged":
      player.loaded = true;
      player.track = {
        name: e.name ?? "",
        artists: e.artists ?? "",
        primaryArtistId: e.primary_artist_id ?? null,
        artistList: e.artist_list ?? [],
        trackId: e.track_id ?? null,
        album: e.album ?? "",
        durationMs: e.duration_ms ?? 0,
        coverUrl: e.cover_url ?? null,
      };
      setPosition(0);
      saveSession();
      updateColor(player.track.coverUrl);
      break;
    case "Playing":
      player.isPlaying = true;
      setPosition(e.position_ms ?? 0);
      break;
    case "Paused":
      player.isPlaying = false;
      setPosition(e.position_ms ?? 0);
      saveSession();
      break;
    case "PositionChanged":
    case "Seeked":
      setPosition(e.position_ms ?? 0);
      break;
    case "Stopped":
    case "EndOfTrack":
    case "Unavailable":
      player.isPlaying = false;
      player.loaded = false;
      if (e.type === "EndOfTrack") setPosition(0);
      break;
    case "PreloadNext":
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
  if (player.positionMs > RESTART_THRESHOLD_MS || !hasPrevious()) seekTo(0);
  else previous();
}

/** @param {number} ms */
export async function seekTo(ms) {
  setPosition(ms);
  if (!player.loaded) return saveSession();
  try {
    await api.seek(Math.round(ms));
  } catch (e) {
    notify(`${e}`);
  }
}

/** @param {number} pct */
export function setVolume(pct) {
  player.volume = pct;
  try {
    localStorage.setItem(VOLUME_KEY, String(pct));
  } catch {}
  api.setVolume(Math.round((pct / 100) * 65535 * fadeLevel)).catch((e) => notify(`${e}`));
}

export function toggleMute() {
  if (player.volume > 0) {
    volumeBeforeMute = player.volume;
    setVolume(0);
  } else {
    setVolume(volumeBeforeMute || DEFAULT_VOLUME);
  }
}
