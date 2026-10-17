import { EPISODE_URI_PREFIX } from "./constants.js";

/**
 * Podcast episodes, and how far into each one you've listened. Spotify keeps
 * your place for its own apps, but librespot doesn't report listening, so
 * SpotiLarp remembers it on this computer.
 */

const STORAGE_KEY = "spotilarp.episodeProgress";
const SAVE_DELAY_MS = 1000;
const MAX_ENTRIES = 500;
/** This close to the end, an episode counts as finished (outros, ads). */
const FINISHED_WITHIN_MS = 20_000;
/** Less than this in, an episode hasn't really been started. */
const STARTED_AFTER_MS = 5_000;

/**
 * @typedef {import("./types.js").Track} Track
 * @typedef {import("./types.js").Episode} Episode
 * @typedef {import("./types.js").Show} Show
 * @typedef {{ track: Track|null, positionMs: number, durationMs: number, played: boolean, updatedAt: number }} EpisodeProgress
 */

/** @type {{ byUri: Record<string, EpisodeProgress> }} by episode URI */
export const episodes = $state({ byUri: readSaved() });

/** @type {ReturnType<typeof setTimeout>|undefined} */
let saveHandle;

function readSaved() {
  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null");
    if (saved && typeof saved === "object" && !Array.isArray(saved)) return saved;
  } catch {}
  return {};
}

function persist() {
  clearTimeout(saveHandle);
  saveHandle = setTimeout(() => {
    const entries = Object.entries($state.snapshot(episodes.byUri));
    if (entries.length > MAX_ENTRIES) {
      entries.sort((a, b) => b[1].updatedAt - a[1].updatedAt);
      episodes.byUri = Object.fromEntries(entries.slice(0, MAX_ENTRIES));
    }
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify($state.snapshot(episodes.byUri)));
    } catch {}
  }, SAVE_DELAY_MS);
}

/** @param {string|null|undefined} uri */
export function isEpisodeUri(uri) {
  return !!uri && uri.startsWith(EPISODE_URI_PREFIX);
}

/** @param {{ uri?: string }|null|undefined} track */
export function isEpisode(track) {
  return isEpisodeUri(track?.uri);
}

/**
 * An episode in the shape the queue and lists play: the show stands in for
 * the artist and the album.
 * @param {Episode} episode
 * @param {Show|null} [show] when the episode doesn't carry its own
 * @returns {Track}
 */
export function episodeTrack(episode, show = null) {
  const from = episode.show ?? show;
  const showName = from?.name ?? "";
  return {
    id: episode.id,
    name: episode.name,
    uri: episode.uri,
    duration_ms: episode.duration_ms,
    artists: showName ? [{ id: "", name: showName }] : [],
    album: { id: "", name: showName, images: episode.images.length > 0 ? episode.images : (from?.images ?? []) },
    show: from?.id ? { id: from.id, name: showName } : null,
    release_date: episode.release_date ?? null,
    description: episode.description,
    playable: episode.is_playable,
  };
}

/** @param {Track} track */
export function progressOf(track) {
  return episodes.byUri[track.uri];
}

/** @param {Track} track */
export function isPlayed(track) {
  return !!episodes.byUri[track.uri]?.played;
}

/**
 * How far in (0 to 1) a started, unfinished episode is; 0 otherwise.
 * @param {Track} track
 */
export function progressFraction(track) {
  const p = episodes.byUri[track.uri];
  const duration = p?.durationMs || track.duration_ms;
  if (!p || p.played || !duration || p.positionMs < STARTED_AFTER_MS) return 0;
  return Math.min(1, p.positionMs / duration);
}

/**
 * Where to start an episode: your place in it, unless it's finished or
 * hardly begun.
 * @param {Track} track
 */
export function resumeAt(track) {
  const p = episodes.byUri[track.uri];
  if (!p || p.played || p.positionMs < STARTED_AFTER_MS) return 0;
  const duration = p.durationMs || track.duration_ms;
  if (duration && duration - p.positionMs < FINISHED_WITHIN_MS) return 0;
  return p.positionMs;
}

/**
 * Remembers your place in an episode that's playing (or paused).
 * @param {string} uri
 * @param {number} positionMs
 * @param {number} durationMs
 * @param {Track|null} track kept for "Continue listening"
 */
export function recordPosition(uri, positionMs, durationMs, track) {
  const previous = episodes.byUri[uri];
  // Loading a finished episode again starts at 0: that isn't un-finishing it.
  if (previous?.played && positionMs < STARTED_AFTER_MS) return;
  if (!previous && positionMs < STARTED_AFTER_MS) return;
  const finished = durationMs > 0 && durationMs - positionMs < FINISHED_WITHIN_MS;
  episodes.byUri[uri] = {
    track: finished ? null : (track ?? previous?.track ?? null),
    positionMs: finished ? 0 : Math.round(positionMs),
    durationMs,
    played: finished,
    updatedAt: Date.now(),
  };
  persist();
}

/**
 * @param {Track} track
 * @param {boolean} played false forgets your place too
 */
export function setPlayed(track, played) {
  if (played) {
    episodes.byUri[track.uri] = { track: null, positionMs: 0, durationMs: track.duration_ms, played: true, updatedAt: Date.now() };
  } else {
    delete episodes.byUri[track.uri];
  }
  persist();
}

/**
 * Episodes started and not finished, most recently listened to first.
 * @param {number} limit
 * @returns {Track[]}
 */
export function continueListening(limit) {
  return Object.values(episodes.byUri)
    .filter((p) => !p.played && p.track && p.positionMs >= STARTED_AFTER_MS)
    .sort((a, b) => b.updatedAt - a.updatedAt)
    .slice(0, limit)
    .map((p) => /** @type {Track} */ (p.track));
}

/**
 * "12 min left", or the length when not started.
 * @param {Track} track
 */
export function timeLeft(track) {
  const p = episodes.byUri[track.uri];
  const duration = p?.durationMs || track.duration_ms;
  const started = p && !p.played && p.positionMs >= STARTED_AFTER_MS;
  const ms = started ? Math.max(0, duration - p.positionMs) : duration;
  const minutes = Math.max(1, Math.round(ms / 60000));
  const hours = Math.floor(minutes / 60);
  const text = hours > 0 ? `${hours} hr${minutes % 60 ? ` ${minutes % 60} min` : ""}` : `${minutes} min`;
  return started ? `${text} left` : text;
}

export function clearEpisodeProgress() {
  episodes.byUri = {};
  persist();
}
