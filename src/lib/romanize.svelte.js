import * as api from "./api.js";
import { BoundedCache } from "./boundedCache.js";

const STORAGE_KEY = "spotilarp.romanizeLyrics";

/** Whether lyrics in another script are shown in Latin letters (the lyrics view and the panel's card share it). */
export const romanization = $state({ on: readSaved() });

function readSaved() {
  try {
    return localStorage.getItem(STORAGE_KEY) === "true";
  } catch {
    return false;
  }
}

/** @param {boolean} on */
export function setRomanized(on) {
  romanization.on = on;
  try {
    localStorage.setItem(STORAGE_KEY, String(on));
  } catch {}
}

const NON_LATIN_LETTER = /(?!\p{Script=Latin})\p{L}/u;
const KANA = /[぀-ヿ]/;

/** @param {string[]} lines */
export function canRomanize(lines) {
  return lines.some((line) => NON_LATIN_LETTER.test(line));
}

/** What the toggle's called: Japanese has its own word for it. @param {string[]} lines */
export function romanizeLabel(lines) {
  return lines.some((line) => KANA.test(line)) ? "Romaji" : "Romanized";
}

// Both the lyrics view and the panel's card ask for the same song's lines.
/** @type {BoundedCache<string, Promise<string[]>>} */
const cache = new BoundedCache(20);

/**
 * One romanized line per line given.
 * @param {string[]} lines
 */
export function romanizeLines(lines) {
  const key = lines.join("\n");
  let pending = cache.get(key);
  if (!pending) {
    pending = api.romanizeLyrics(lines);
    pending.catch(() => cache.delete(key));
    cache.set(key, pending);
  }
  return pending;
}
