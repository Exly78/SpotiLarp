import { SvelteMap } from "svelte/reactivity";
import { viewKey } from "./nav.svelte.js";
import { coverAtLeast } from "./utils.js";

const STORAGE_KEY = "spotilarp.lastPlayed";
const RECENTS_KEY = "spotilarp.recentContexts";
const MAX_ENTRIES = 500;
const MAX_RECENTS = 40;
/** The pages played from that Home's "Jump back in" can show again. */
const RECENT_TYPES = ["playlist", "album", "artist", "radio", "show"];

/**
 * @typedef {import("./nav.svelte.js").View} View
 * @typedef {{ key: string, view: View, image: string|null, at: number }} RecentContext
 */

/** When each playlist, album, artist... was last played from, by view key. */
export const lastPlayed = new SvelteMap(readSaved());

/**
 * The pages played from lately, newest first, with what a card needs to show
 * them (a page played from a link isn't in the library to look up).
 * @type {{ items: RecentContext[] }}
 */
export const recentContexts = $state({ items: readRecents() });

/** @returns {[string, number][]} */
function readSaved() {
  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null");
    if (saved && typeof saved === "object") {
      return Object.entries(saved).filter(([, at]) => typeof at === "number");
    }
  } catch {}
  return [];
}

/** @returns {RecentContext[]} */
function readRecents() {
  try {
    const saved = JSON.parse(localStorage.getItem(RECENTS_KEY) ?? "null");
    if (Array.isArray(saved)) {
      return saved
        .filter((item) => item?.key && item.view?.type && typeof item.at === "number")
        .map((item) => (item.view.type === "playlist" ? { ...item, image: viewImage(item.view) } : item));
    }
  } catch {}
  return [];
}

/**
 * @param {View} view
 * @param {import("./types.js").Track[]} [tracks] what was played, for a picture when the page has none
 */
export function markPlayed(view, tracks = []) {
  const key = viewKey(view);
  const at = Date.now();
  lastPlayed.set(key, at);
  const newest = [...lastPlayed].sort((a, b) => b[1] - a[1]).slice(0, MAX_ENTRIES);
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(Object.fromEntries(newest)));
  } catch {}
  if (RECENT_TYPES.includes(view.type)) rememberContext(view, key, at, tracks);
}

/**
 * @param {View} view
 * @param {string} key
 * @param {number} at
 * @param {import("./types.js").Track[]} tracks
 */
function rememberContext(view, key, at, tracks) {
  // An album's songs share its cover; a playlist's first song isn't the
  // playlist (Liked Songs has no picture of its own).
  const image = viewImage(view) ?? (view.type === "album" ? coverAtLeast(tracks[0]?.album.images, 300) : null);
  /** @type {RecentContext} */
  const entry = { key, view: slimView(view), image, at };
  recentContexts.items = [entry, ...recentContexts.items.filter((item) => item.key !== key)].slice(0, MAX_RECENTS);
  try {
    localStorage.setItem(RECENTS_KEY, JSON.stringify($state.snapshot(recentContexts.items)));
  } catch {}
}

/** @param {View|null|undefined} view */
export function isLikedSongs(view) {
  return view?.type === "playlist" && !!view.playlist.isLikedSongs;
}

/**
 * @param {View} view
 * @returns {string|null}
 */
export function viewImage(view) {
  if (view.type === "playlist") return coverAtLeast(view.playlist.images, 300);
  return ("image" in view && view.image) || null;
}

/**
 * A playlist page carries the whole playlist, description and all; a card
 * only needs enough to open it again.
 * @param {View} view
 * @returns {View}
 */
function slimView(view) {
  if (view.type !== "playlist") return view;
  const { id, name, owner, isLikedSongs } = view.playlist;
  const cover = coverAtLeast(view.playlist.images, 300);
  return {
    type: "playlist",
    playlist: {
      id,
      name,
      images: cover ? [{ url: cover }] : [],
      track_count: { total: 0 },
      owner: owner ? { id: owner.id, display_name: owner.display_name } : null,
      ...(isLikedSongs ? { isLikedSongs } : {}),
    },
  };
}

/** @param {string} key a view key, like `playlist:<id>` */
export function playedAt(key) {
  return lastPlayed.get(key) ?? 0;
}
