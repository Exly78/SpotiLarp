import { listen } from "@tauri-apps/api/event";
import * as api from "./api.js";
import { notify } from "./toast.svelte.js";

/** @type {{ endsAt: number|null, endOfTrack: boolean, remainingMs: number }} */
export const sleep = $state({ endsAt: null, endOfTrack: false, remainingMs: 0 });

const FADE_MS = 30_000;

let applyFade = (/** @type {number} */ _level) => {};
let fading = false;

/** @param {(level: number) => void} handler */
export function setFadeHandler(handler) {
  applyFade = handler;
}

/** @param {number} level */
function fade(level) {
  fading = level < 1;
  applyFade(level);
}

/** @param {number|"track"|null} minutes */
export function setSleepTimer(minutes) {
  if (fading) fade(1);
  sleep.endsAt = null;
  sleep.endOfTrack = minutes === "track";
  sleep.remainingMs = 0;
  if (typeof minutes !== "number" || minutes <= 0) {
    api.setClock(false).catch(() => {});
    return;
  }
  sleep.endsAt = Date.now() + minutes * 60_000;
  update();
  api.setClock(true).catch(() => {});
}

// The app ticks every second while a timer runs: the window's own timers can
// fall a minute behind while it's minimized or in the tray.
listen("clock-tick", update);

function update() {
  if (sleep.endsAt === null) return;
  sleep.remainingMs = Math.max(0, sleep.endsAt - Date.now());
  if (sleep.remainingMs === 0) {
    api.setClock(false).catch(() => {});
    sleep.endsAt = null;
    api
      .pause()
      .catch(() => {})
      .finally(() => fade(1));
    notify("Sleep timer ended, playback paused.");
  } else if (sleep.remainingMs <= FADE_MS) {
    fade(sleep.remainingMs / FADE_MS);
  }
}

export function consumeEndOfTrackStop() {
  if (!sleep.endOfTrack) return false;
  sleep.endOfTrack = false;
  notify("Sleep timer ended, playback stopped.");
  return true;
}
