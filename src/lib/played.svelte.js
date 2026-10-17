import { SvelteMap } from "svelte/reactivity";
import { viewKey } from "./nav.svelte.js";

const STORAGE_KEY = "spotilarp.lastPlayed";
const MAX_ENTRIES = 500;

/** When each playlist, album, artist... was last played from, by view key. */
export const lastPlayed = new SvelteMap(readSaved());

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

/** @param {import("./nav.svelte.js").View} view */
export function markPlayed(view) {
  lastPlayed.set(viewKey(view), Date.now());
  const newest = [...lastPlayed].sort((a, b) => b[1] - a[1]).slice(0, MAX_ENTRIES);
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(Object.fromEntries(newest)));
  } catch {}
}

/** @param {string} key a view key, like `playlist:<id>` */
export function playedAt(key) {
  return lastPlayed.get(key) ?? 0;
}
