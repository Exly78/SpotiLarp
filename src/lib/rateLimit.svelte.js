import * as api from "./api.js";

/** When Spotify will take SpotiLarp's requests again, if it's turning them away (ms since epoch, 0 if not). */
export const rateLimit = $state({ until: 0 });

/** @type {ReturnType<typeof setTimeout>|undefined} */
let timer;
/** @type {Set<() => void>} */
const endListeners = new Set();

/**
 * Runs `callback` whenever a limit runs out, to pick up what couldn't load.
 * @param {() => void} callback
 * @returns {() => void} stops listening
 */
export function onRateLimitEnd(callback) {
  endListeners.add(callback);
  return () => endListeners.delete(callback);
}

/** Asks the app (not Spotify) whether a limit is on; call after anything was turned away. */
export async function checkRateLimit() {
  const wasLimited = rateLimit.until > 0;
  try {
    const seconds = await api.rateLimitRemaining();
    rateLimit.until = seconds ? Date.now() + seconds * 1000 : 0;
  } catch {
    return;
  }
  if (wasLimited && !rateLimit.until) for (const callback of endListeners) callback();
  clearTimeout(timer);
  // Rechecked every minute, so the countdown moves and the notice goes when it's over.
  if (rateLimit.until) timer = setTimeout(checkRateLimit, 60_000);
}

/** "about 4 hours", "12 minutes". @param {number} until */
export function timeLeft(until) {
  const minutes = Math.max(1, Math.ceil((until - Date.now()) / 60_000));
  if (minutes >= 90) return `about ${Math.round(minutes / 60)} hours`;
  return minutes === 1 ? "1 minute" : `${minutes} minutes`;
}

api.setRateLimitedHandler(checkRateLimit);
checkRateLimit();
