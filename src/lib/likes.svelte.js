import { SvelteSet } from "svelte/reactivity";
import * as api from "./api.js";
import { notify } from "./toast.svelte.js";

export const likedIds = new SvelteSet();
export const likeBusy = new SvelteSet();

/**
 * Spotify allows an app only so many requests, so a song's liked state is
 * asked about once and remembered for a while (your own likes and unlikes
 * update it at once).
 */
const CHECK_TTL_MS = 20 * 60_000;
/** Songs coming into view within this long are asked about together. */
const QUEUE_DELAY_MS = 150;
/** How long to stop asking after Spotify says to slow down. */
const RATE_LIMIT_PAUSE_MS = 2 * 60_000;

/** @type {Map<string, number>} when each song was last asked about */
const checkedAt = new Map();
/** @type {Set<string>} */
const queued = new Set();
/** @type {ReturnType<typeof setTimeout>|undefined} */
let queueHandle;
let pausedUntil = 0;

/**
 * Asks Spotify which of these songs are liked, leaving out ones asked about lately.
 * @param {string[]} ids
 */
export async function refreshLiked(ids) {
  const now = Date.now();
  if (now < pausedUntil) return;
  const unique = [...new Set(ids.filter(Boolean))].filter((id) => now - (checkedAt.get(id) ?? -Infinity) >= CHECK_TTL_MS);
  if (unique.length === 0) return;
  // Counted as asked straight away, so the same songs aren't asked about twice at once.
  for (const id of unique) checkedAt.set(id, now);
  try {
    const results = await api.areTracksLiked(unique);
    unique.forEach((id, i) => {
      if (likeBusy.has(id)) return;
      if (results[i]) likedIds.add(id);
      else likedIds.delete(id);
    });
  } catch (e) {
    for (const id of unique) checkedAt.delete(id);
    if (api.isRateLimited(e)) pausedUntil = Date.now() + RATE_LIMIT_PAUSE_MS;
    console.error("Failed to check liked tracks:", e);
  }
}

/**
 * For a song that came into view: asked about shortly, with any others that did.
 * @param {string} id
 */
export function queueLikedCheck(id) {
  if (!id || Date.now() - (checkedAt.get(id) ?? -Infinity) < CHECK_TTL_MS) return;
  queued.add(id);
  clearTimeout(queueHandle);
  queueHandle = setTimeout(() => {
    const ids = [...queued];
    queued.clear();
    refreshLiked(ids);
  }, QUEUE_DELAY_MS);
}

/** @param {string[]} ids */
export function markLiked(ids) {
  const now = Date.now();
  for (const id of ids) {
    if (!id) continue;
    likedIds.add(id);
    checkedAt.set(id, now);
  }
}

/** @param {string|null|undefined} id */
export async function toggleLike(id) {
  if (!id || likeBusy.has(id)) return;
  likeBusy.add(id);
  try {
    if (likedIds.has(id)) {
      await api.unlikeTrack(id);
      likedIds.delete(id);
    } else {
      await api.likeTrack(id);
      likedIds.add(id);
    }
    checkedAt.set(id, Date.now());
  } catch (e) {
    notify(`Couldn't update Liked Songs: ${e}`);
  } finally {
    likeBusy.delete(id);
  }
}

/** @param {string[]} ids */
export async function likeTracks(ids) {
  const toLike = [...new Set(ids.filter((id) => id && !likedIds.has(id)))];
  try {
    for (const id of toLike) {
      await api.likeTrack(id);
      likedIds.add(id);
      checkedAt.set(id, Date.now());
    }
    notify(ids.length === 1 ? "Added to Liked Songs" : `Added ${toLike.length} songs to Liked Songs`);
  } catch (e) {
    notify(`Couldn't update Liked Songs: ${e}`);
  }
}

export function clearLikes() {
  likedIds.clear();
  checkedAt.clear();
  queued.clear();
  pausedUntil = 0;
}
