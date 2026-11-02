/**
 * Spotify answers worth keeping for a while, across restarts, so opening the
 * app (or a page) again soon doesn't ask Spotify the same things again: an app
 * that asks too much gets turned away for hours.
 */

const PREFIX = "spotilarp.cache.";

export const MINUTE = 60_000;

/**
 * `load()`'s answer, or the one saved under `key` if it's younger than `maxAgeMs`.
 * @template T
 * @param {string} key
 * @param {number} maxAgeMs
 * @param {() => Promise<T>} load
 * @returns {Promise<T>}
 */
export async function cached(key, maxAgeMs, load) {
  try {
    const saved = JSON.parse(localStorage.getItem(PREFIX + key) ?? "null");
    if (saved && Date.now() - saved.at < maxAgeMs) return saved.value;
  } catch {}
  const value = await load();
  try {
    localStorage.setItem(PREFIX + key, JSON.stringify({ at: Date.now(), value }));
  } catch {}
  return value;
}

/** Forgets every saved answer (logging out: they were someone's). */
export function clearCached() {
  try {
    for (const key of Object.keys(localStorage)) {
      if (key.startsWith(PREFIX)) localStorage.removeItem(key);
    }
  } catch {}
}
