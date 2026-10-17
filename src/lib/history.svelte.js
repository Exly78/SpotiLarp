import { listen } from "@tauri-apps/api/event";
import { player } from "./player.svelte.js";
import { queue, slim, setLastPlayedLookup } from "./queue.svelte.js";

/**
 * Songs played in SpotiLarp, which Spotify's own history doesn't include
 * (librespot doesn't report plays). Kept on this computer only.
 */

const STORAGE_KEY = "spotilarp.history";
const SAVE_DELAY_MS = 1000;
const MAX_ENTRIES = 1000;
/** A play counts after this long, or half of a shorter song. Paused time and seeking don't count. */
const COUNT_AFTER_MS = 30_000;
const CHECK_EVERY_MS = 1000;

/**
 * @typedef {import("./types.js").Track} Track
 * @typedef {{ track: Track, playedAt: string }} HistoryEntry
 */

/** @type {{ entries: HistoryEntry[] }} newest first */
export const history = $state({ entries: readSaved() });

let currentUri = "";
let currentDurationMs = 0;
/** The playing song as it'll be saved, taken when it starts. @type {Track|null} */
let currentTrack = null;
let listenedMs = 0;
let playingSince = 0;
let counted = false;
/** @type {ReturnType<typeof setTimeout>|undefined} */
let saveHandle;

function readSaved() {
  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null");
    if (Array.isArray(saved)) return saved.filter((entry) => entry?.track?.uri && entry.playedAt);
  } catch {}
  return [];
}

function persist() {
  clearTimeout(saveHandle);
  saveHandle = setTimeout(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify($state.snapshot(history.entries)));
    } catch {}
  }, SAVE_DELAY_MS);
}

function stopClock() {
  if (playingSince) listenedMs += performance.now() - playingSince;
  playingSince = 0;
}

/** @returns {Track|null} */
function playingTrack() {
  if (queue.current?.uri === currentUri) return slim(queue.current);
  const t = player.track;
  if (!t || t.uri !== currentUri) return null;
  return {
    id: t.trackId ?? "",
    name: t.name,
    uri: t.uri ?? "",
    duration_ms: t.durationMs,
    artists: (t.artistList ?? []).map((a) => ({ id: a.id ?? "", name: a.name })),
    album: { name: t.album, images: t.coverUrl ? [{ url: t.coverUrl }] : [] },
  };
}

function check() {
  if (counted || !currentUri) return;
  const listened = listenedMs + (playingSince ? performance.now() - playingSince : 0);
  const duration = currentDurationMs;
  const needed = duration > 0 ? Math.min(COUNT_AFTER_MS, duration / 2) : COUNT_AFTER_MS;
  if (listened < needed) return;
  const track = currentTrack ?? playingTrack();
  if (!track) return;
  counted = true;
  history.entries = [{ track, playedAt: new Date().toISOString() }, ...history.entries].slice(0, MAX_ENTRIES);
  persist();
}

export function clearHistory() {
  history.entries = [];
  persist();
}

/** @type {{ entries: HistoryEntry[]|null, at: Map<string, number> }} */
const lastPlayed = { entries: null, at: new Map() };

// Smart shuffle plays what was heard lately last.
setLastPlayedLookup((track) => {
  if (lastPlayed.entries !== history.entries) {
    lastPlayed.entries = history.entries;
    lastPlayed.at = new Map();
    for (const entry of history.entries) {
      if (!lastPlayed.at.has(entry.track.uri)) lastPlayed.at.set(entry.track.uri, Date.parse(entry.playedAt) || 0);
    }
  }
  return lastPlayed.at.get(track.uri) ?? 0;
});

setInterval(check, CHECK_EVERY_MS);

listen("player-event", (event) => {
  const e = /** @type {import("./types.js").PlaybackEvent} */ (event.payload);
  switch (e.type) {
    case "TrackChanged":
      // The song before may have been heard long enough: in a hidden window
      // the every-second check can fall behind.
      check();
      // Every load is its own play, repeats of the same song included.
      currentUri = e.uri ?? "";
      currentDurationMs = e.duration_ms ?? 0;
      currentTrack = null;
      listenedMs = 0;
      playingSince = 0;
      counted = false;
      break;
    case "Playing":
      if (!playingSince) playingSince = performance.now();
      currentTrack ??= playingTrack();
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
