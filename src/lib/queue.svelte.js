import { listen } from "@tauri-apps/api/event";
import * as api from "./api.js";
import { notify } from "./toast.svelte.js";
import { consumeEndOfTrackStop } from "./sleep.svelte.js";
import { markPlayed } from "./played.svelte.js";
import { smartOrder, shuffleInPlace } from "./smartShuffle.js";
import { isEpisode, resumeAt, setPlayed } from "./podcasts.svelte.js";

const STORAGE_KEY = "spotilarp.queue";
const SAVE_DELAY_MS = 500;

/**
 * @typedef {import("./types.js").Track} Track
 * @typedef {import("./nav.svelte.js").View} View
 * `smartShuffle` (with `shuffle`) spreads artists out and plays recent songs last.
 * @type {{ tracks: Track[], order: number[], position: number, manual: Track[], current: Track|null, fromManual: boolean, contextName: string, contextView: View|null, repeatMode: string, shuffle: boolean, smartShuffle: boolean, autoplay: boolean, autoplayStart: number }}
 */
export const queue = $state({
  tracks: [],
  order: [],
  position: -1,
  manual: [],
  current: null,
  fromManual: false,
  contextName: "",
  // The page the playing list came from, for "Playing from" links.
  contextView: null,
  repeatMode: "off",
  shuffle: false,
  smartShuffle: false,
  autoplay: true,
  autoplayStart: -1,
  ...readSaved(),
});

const REPEAT_MODES = ["off", "all", "one"];
const AUTOPLAY_RECENT = 50;

let unavailableStreak = 0;
/** @type {ReturnType<typeof setTimeout>|undefined} */
let saveHandle;
let contextVersion = 0;
/** @type {Promise<number>|null} */
let autoplayRequest = null;
/** When a song was last played (ms since 1970, 0 for never), from the listening history. */
let lastPlayedAt = (/** @type {Track} */ _track) => 0;

/** @param {(track: Track) => number} lookup */
export function setLastPlayedLookup(lookup) {
  lastPlayedAt = lookup;
}

function readSaved() {
  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null");
    if (saved && Array.isArray(saved.tracks) && Array.isArray(saved.order)) return saved;
  } catch {}
  return {};
}

/**
 * A track with just what the queue and history need to show and play it.
 * @param {Track} track
 * @returns {Track}
 */
export function slim(track) {
  const images = track.album.images;
  const smallest = images.length
    ? [images.reduce((a, b) => ((a.width ?? 0) < (b.width ?? 0) ? a : b))]
    : [];
  /** @type {Track} */
  const slimmed = {
    id: track.id,
    name: track.name,
    uri: track.uri,
    duration_ms: track.duration_ms,
    artists: track.artists.map((a) => ({ id: a.id, name: a.name })),
    album: { id: track.album.id, name: track.album.name, images: smallest },
  };
  if (track.show) slimmed.show = { id: track.show.id, name: track.show.name };
  if (track.release_date) slimmed.release_date = track.release_date;
  return slimmed;
}

function persist() {
  clearTimeout(saveHandle);
  saveHandle = setTimeout(() => {
    const snapshot = $state.snapshot(queue);
    const data = {
      ...snapshot,
      tracks: snapshot.tracks.map(slim),
      manual: snapshot.manual.map(slim),
      current: snapshot.current && slim(snapshot.current),
    };
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(data));
    } catch {
      try {
        const tracks = data.current ? [data.current] : [];
        localStorage.setItem(
          STORAGE_KEY,
          JSON.stringify({
            ...data,
            tracks,
            order: tracks.map((_, i) => i),
            position: tracks.length - 1,
            autoplayStart: -1,
          }),
        );
      } catch {}
    }
  }, SAVE_DELAY_MS);
}

export function cycleRepeatMode() {
  const i = REPEAT_MODES.indexOf(queue.repeatMode);
  setRepeatMode(REPEAT_MODES[(i + 1) % REPEAT_MODES.length]);
}

/** @param {string} mode "off", "all" or "one" */
export function setRepeatMode(mode) {
  if (!REPEAT_MODES.includes(mode)) return;
  queue.repeatMode = mode;
  persist();
}

/** Whether the playing list is shuffled: podcast episodes always play in order. */
function shuffling() {
  return queue.shuffle && !isEpisode(queue.tracks[0]);
}

/**
 * @param {Track[]} tracks
 * @param {number} firstIndex the song to play first, or -1 for any
 */
function buildOrder(tracks, firstIndex) {
  const order = Array.from({ length: tracks.length }, (_, i) => i);
  if (!shuffling()) return order;
  if (queue.smartShuffle) return smartOrder(tracks, { first: firstIndex, playedAt: lastPlayedAt });
  if (firstIndex >= 0) order.splice(firstIndex, 1);
  shuffleInPlace(order);
  if (firstIndex >= 0) order.unshift(firstIndex);
  return order;
}

/** @returns {"off"|"on"|"smart"} */
export function shuffleMode() {
  if (!queue.shuffle) return "off";
  return queue.smartShuffle ? "smart" : "on";
}

/** What the shuffle button does next, and its name for the current mode. */
export function shuffleLabels() {
  const mode = shuffleMode();
  return {
    action: mode === "off" ? "Enable shuffle" : mode === "on" ? "Enable smart shuffle" : "Disable shuffle",
    state: mode === "off" ? "Shuffle off" : mode === "on" ? "Shuffle on" : "Smart shuffle on",
  };
}

/** Off, then shuffle, then smart shuffle. */
export function toggleShuffle() {
  const mode = shuffleMode();
  setShuffleMode(mode === "off" ? "on" : mode === "on" ? "smart" : "off");
}

/** @param {string} mode "off", "on" or "smart" */
export function setShuffleMode(mode) {
  if (!["off", "on", "smart"].includes(mode) || mode === shuffleMode()) return;
  queue.shuffle = mode !== "off";
  queue.smartShuffle = mode === "smart";
  if (queue.position >= 0) {
    const current = queue.order[queue.position];
    queue.order = buildOrder(queue.tracks, current);
    queue.position = shuffling() ? 0 : current;
  }
  persist();
}

/** @param {number} position */
function contextTrackAt(position) {
  return queue.tracks[queue.order[position]];
}

function nextContextPosition() {
  if (queue.position < 0 || queue.tracks.length === 0) return -1;
  if (queue.position < queue.order.length - 1) return queue.position + 1;
  return queue.repeatMode === "all" && queue.tracks.length > 1 ? 0 : -1;
}

// Songs Spotify picks to follow a podcast episode wouldn't fit.
function canAutoplay() {
  return queue.autoplay && queue.repeatMode === "off" && queue.current !== null && !isEpisode(queue.current);
}

export function toggleAutoplay() {
  queue.autoplay = !queue.autoplay;
  persist();
}

export function contextLabel() {
  const index = queue.order[queue.position];
  const autoplaying = !queue.fromManual && queue.autoplayStart >= 0 && index >= queue.autoplayStart;
  return autoplaying ? "Autoplay" : queue.contextName;
}

function extendWithAutoplay() {
  const request = autoplayRequest ?? fetchAutoplay().finally(() => (autoplayRequest = null));
  autoplayRequest = request;
  return request;
}

async function fetchAutoplay() {
  const version = contextVersion;
  const played = queue.order
    .slice(0, queue.position + 1)
    .map((i) => queue.tracks[i])
    .reverse();
  const seed = [queue.current, ...played].find((track) => track?.id && !isEpisode(track));
  if (!seed) return -1;
  const recent = played.slice(0, AUTOPLAY_RECENT).map((track) => track.uri);
  try {
    const suggestions = await api.getAutoplayTracks(`spotify:track:${seed.id}`, recent);
    if (version !== contextVersion) return -1;
    const known = new Set(queue.tracks.map((track) => track.uri));
    const fresh = suggestions.filter((track) => !known.has(track.uri));
    if (fresh.length === 0) return -1;
    const start = queue.tracks.length;
    const firstPosition = queue.order.length;
    queue.tracks = [...queue.tracks, ...fresh];
    queue.order = [...queue.order, ...fresh.map((_, i) => start + i)];
    if (queue.autoplayStart < 0) queue.autoplayStart = start;
    persist();
    return firstPosition;
  } catch (e) {
    notify(`Couldn't find similar songs to play: ${e}`);
    return -1;
  }
}

export function hasNext() {
  return queue.manual.length > 0 || nextContextPosition() >= 0 || canAutoplay();
}

export function hasPrevious() {
  if (queue.fromManual) return queue.position >= 0;
  if (queue.position <= 0) {
    return queue.position === 0 && queue.repeatMode === "all" && queue.tracks.length > 1;
  }
  return true;
}

export function peekNext() {
  if (queue.repeatMode === "one") return undefined;
  if (queue.manual.length > 0) return queue.manual[0];
  const position = nextContextPosition();
  return position >= 0 ? contextTrackAt(position) : undefined;
}

export async function prepareNext() {
  const upcoming = peekNext();
  if (upcoming || queue.repeatMode === "one" || !canAutoplay()) return upcoming;
  return (await extendWithAutoplay()) >= 0 ? peekNext() : undefined;
}

/** @param {number} limit */
export function upcomingContext(limit) {
  /** @type {{ track: Track, position: number }[]} */
  const upcoming = [];
  if (queue.position < 0) return { upcoming, more: 0 };
  const end = Math.min(queue.order.length, queue.position + 1 + limit);
  for (let position = queue.position + 1; position < end; position++) {
    upcoming.push({ track: contextTrackAt(position), position });
  }
  return { upcoming, more: queue.order.length - end };
}

/** @param {number} [positionMs] 0 starts an episode where you left off */
export async function playCurrent(positionMs = 0) {
  const track = queue.current;
  if (!track) return;
  persist();
  const start = positionMs > 0 ? positionMs : isEpisode(track) ? resumeAt(track) : 0;
  try {
    await api.playTrack(track.uri, Math.round(start));
  } catch (e) {
    notify(`Couldn't play "${track.name}": ${e}`);
  }
}

/**
 * @param {Track[]} list
 * @param {number} index the song to start with, or -1 for the first (or,
 *   shuffling, whichever the shuffle puts first)
 * @param {string} [contextName]
 * @param {View|null} [contextView] the page the list belongs to
 */
export async function playFromList(list, index, contextName = "", contextView = null) {
  contextVersion++;
  queue.contextView = contextView;
  if (contextView) markPlayed(contextView);
  queue.tracks = list;
  queue.order = buildOrder(list, index);
  queue.position = shuffling() ? 0 : Math.max(index, 0);
  queue.current = list[queue.order[queue.position]] ?? null;
  queue.fromManual = false;
  queue.contextName = contextName;
  queue.autoplayStart = -1;
  unavailableStreak = 0;
  await playCurrent();
}

export async function next() {
  if (queue.manual.length > 0) {
    queue.current = /** @type {Track} */ (queue.manual.shift());
    queue.fromManual = true;
  } else {
    let position = nextContextPosition();
    if (position < 0 && canAutoplay()) {
      const version = contextVersion;
      position = await extendWithAutoplay();
      if (version !== contextVersion) return;
    }
    if (position < 0) return;
    queue.position = position;
    queue.current = contextTrackAt(position);
    queue.fromManual = false;
  }
  await playCurrent();
}

export async function previous() {
  if (!hasPrevious()) return;
  if (!queue.fromManual) {
    queue.position = queue.position > 0 ? queue.position - 1 : queue.order.length - 1;
  }
  queue.current = contextTrackAt(queue.position);
  queue.fromManual = false;
  await playCurrent();
}

/** @param {Track} track */
export function addToQueue(track) {
  addManyToQueue([track]);
}

/** @param {Track} track */
export function playNext(track) {
  playManyNext([track]);
}

/** @param {Track[]} tracks in the order they should play */
export function addManyToQueue(tracks) {
  if (tracks.length === 0) return;
  // Not push(...tracks): a big playlist would be too many arguments.
  queue.manual = [...queue.manual, ...tracks];
  persist();
  notify(tracks.length === 1 ? "Added to queue" : `Added ${tracks.length} songs to queue`);
}

/** @param {Track[]} tracks in the order they should play */
export function playManyNext(tracks) {
  if (tracks.length === 0) return;
  queue.manual = [...tracks, ...queue.manual];
  persist();
  notify(tracks.length === 1 ? "Playing next" : `Playing ${tracks.length} songs next`);
}

/**
 * @param {number} from
 * @param {number} insertBefore
 */
export function moveInQueue(from, insertBefore) {
  if (insertBefore === from || insertBefore === from + 1) return;
  const [track] = queue.manual.splice(from, 1);
  if (!track) return;
  queue.manual.splice(insertBefore > from ? insertBefore - 1 : insertBefore, 0, track);
  persist();
}

/** @param {number} index */
export function removeFromQueue(index) {
  queue.manual.splice(index, 1);
  persist();
}

export function clearQueue() {
  queue.manual = [];
  persist();
}

export function resetQueue() {
  contextVersion++;
  queue.autoplayStart = -1;
  queue.tracks = [];
  queue.order = [];
  queue.position = -1;
  queue.manual = [];
  queue.current = null;
  queue.fromManual = false;
  queue.contextName = "";
  queue.contextView = null;
  unavailableStreak = 0;
  persist();
}

/** @param {number} index */
export async function playFromQueue(index) {
  const [track] = queue.manual.splice(0, index + 1).slice(-1);
  if (!track) return;
  queue.current = track;
  queue.fromManual = true;
  await playCurrent();
}

/**
 * Fills in what the playing item was missing (an episode found by search
 * doesn't say which show it's from).
 * @param {string} uri
 * @param {Partial<Track>} fields
 */
export function updateCurrent(uri, fields) {
  if (queue.current?.uri !== uri) return;
  queue.current = { ...queue.current, ...fields };
  const index = queue.order[queue.position];
  if (!queue.fromManual && queue.tracks[index]?.uri === uri) queue.tracks[index] = { ...queue.tracks[index], ...fields };
  persist();
}

/** @param {number} position */
export async function skipToContext(position) {
  queue.position = position;
  queue.current = contextTrackAt(position);
  queue.fromManual = false;
  await playCurrent();
}

listen("player-event", (event) => {
  const type = /** @type {import("./types.js").PlaybackEvent} */ (event.payload).type;
  if (type === "Playing") {
    unavailableStreak = 0;
  } else if (type === "Unavailable") {
    notify(`"${queue.current?.name ?? "This track"}" isn't available, skipping.`);
    unavailableStreak++;
    if (unavailableStreak < queue.tracks.length + queue.manual.length) next();
  } else if (type === "EndOfTrack") {
    if (queue.current && isEpisode(queue.current)) setPlayed(queue.current, true);
    if (consumeEndOfTrackStop()) return;
    if (queue.repeatMode === "one") {
      playCurrent();
    } else {
      next();
    }
  }
});
