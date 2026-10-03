import { listen } from "@tauri-apps/api/event";
import * as api from "./api.js";
import { notify } from "./toast.svelte.js";
import { consumeEndOfTrackStop } from "./sleep.svelte.js";

const STORAGE_KEY = "spotilarp.queue";
const SAVE_DELAY_MS = 500;

/**
 * @typedef {import("./types.js").Track} Track
 * @type {{ tracks: Track[], order: number[], position: number, manual: Track[], current: Track|null, fromManual: boolean, contextName: string, repeatMode: string, shuffle: boolean, autoplay: boolean, autoplayStart: number }}
 */
export const queue = $state({
  tracks: [],
  order: [],
  position: -1,
  manual: [],
  current: null,
  fromManual: false,
  contextName: "",
  repeatMode: "off",
  shuffle: false,
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

function readSaved() {
  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null");
    if (saved && Array.isArray(saved.tracks) && Array.isArray(saved.order)) return saved;
  } catch {}
  return {};
}

/**
 * @param {Track} track
 * @returns {Track}
 */
function slim(track) {
  const images = track.album.images;
  const smallest = images.length
    ? [images.reduce((a, b) => ((a.width ?? 0) < (b.width ?? 0) ? a : b))]
    : [];
  return {
    id: track.id,
    name: track.name,
    uri: track.uri,
    duration_ms: track.duration_ms,
    artists: track.artists.map((a) => ({ id: a.id, name: a.name })),
    album: { name: track.album.name, images: smallest },
  };
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
  queue.repeatMode = REPEAT_MODES[(i + 1) % REPEAT_MODES.length];
  persist();
}

/**
 * @param {number} length
 * @param {number} firstIndex
 */
function buildOrder(length, firstIndex) {
  const order = Array.from({ length }, (_, i) => i);
  if (!queue.shuffle || firstIndex < 0) return order;
  order.splice(firstIndex, 1);
  for (let i = order.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [order[i], order[j]] = [order[j], order[i]];
  }
  order.unshift(firstIndex);
  return order;
}

export function toggleShuffle() {
  queue.shuffle = !queue.shuffle;
  if (queue.position >= 0) {
    const current = queue.order[queue.position];
    queue.order = buildOrder(queue.tracks.length, current);
    queue.position = queue.shuffle ? 0 : current;
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

function canAutoplay() {
  return queue.autoplay && queue.repeatMode === "off" && queue.current !== null;
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
  const seed = [queue.current, ...played].find((track) => track?.id);
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

/** @param {number} [positionMs] */
export async function playCurrent(positionMs = 0) {
  const track = queue.current;
  if (!track) return;
  persist();
  try {
    await api.playTrack(track.uri, Math.round(positionMs));
  } catch (e) {
    notify(`Couldn't play "${track.name}": ${e}`);
  }
}

/**
 * @param {Track[]} list
 * @param {number} index
 * @param {string} [contextName]
 */
export async function playFromList(list, index, contextName = "") {
  contextVersion++;
  queue.tracks = list;
  queue.order = buildOrder(list.length, index);
  queue.position = queue.shuffle ? 0 : index;
  queue.current = list[index] ?? null;
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
  queue.manual.push(track);
  persist();
  notify("Added to queue");
}

/** @param {Track} track */
export function playNext(track) {
  queue.manual.unshift(track);
  persist();
  notify("Playing next");
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
    if (consumeEndOfTrackStop()) return;
    if (queue.repeatMode === "one") {
      playCurrent();
    } else {
      next();
    }
  }
});
