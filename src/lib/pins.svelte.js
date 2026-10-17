const STORAGE_KEY = "spotilarp.pins";

/**
 * Library items pinned to the top, by view key (`playlist:<id>`, `album:<id>`),
 * in the order they were pinned. Kept on this computer: Spotify has no API
 * for its own pins.
 * @type {{ keys: string[] }}
 */
export const pins = $state({ keys: readSaved() });

/** @returns {string[]} */
function readSaved() {
  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null");
    if (Array.isArray(saved)) return saved.filter((key) => typeof key === "string");
  } catch {}
  return [];
}

/** @param {string} key */
export function isPinned(key) {
  return pins.keys.includes(key);
}

/** @param {string} key */
export function togglePin(key) {
  pins.keys = isPinned(key) ? pins.keys.filter((k) => k !== key) : [...pins.keys, key];
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(pins.keys));
  } catch {}
}
