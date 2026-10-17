import { getCurrentWebview } from "@tauri-apps/api/webview";

/** How big everything in the window is drawn, like a browser's zoom. */

const STORAGE_KEY = "spotilarp.zoom";
export const ZOOM_LEVELS = [0.5, 0.67, 0.75, 0.8, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2];
/** Trackpad pinches and smooth wheels send many small steps; this much scrolling is one zoom step. */
const WHEEL_STEP = 100;

/** `suspended` while the mini player is open: it's laid out for its fixed size. */
export const zoom = $state({ level: readSaved(), suspended: false });

let wheelTotal = 0;

function readSaved() {
  try {
    const saved = Number(localStorage.getItem(STORAGE_KEY));
    if (ZOOM_LEVELS.includes(saved)) return saved;
  } catch {}
  return 1;
}

function apply() {
  getCurrentWebview()
    .setZoom(zoom.suspended ? 1 : zoom.level)
    .catch(() => {});
}

/** @param {number} level one of `ZOOM_LEVELS` */
export function setZoom(level) {
  if (!ZOOM_LEVELS.includes(level)) return;
  zoom.level = level;
  try {
    localStorage.setItem(STORAGE_KEY, String(level));
  } catch {}
  apply();
}

/** @param {1|-1} direction */
function step(direction) {
  const i = ZOOM_LEVELS.indexOf(zoom.level);
  const next = ZOOM_LEVELS[Math.min(ZOOM_LEVELS.length - 1, Math.max(0, i + direction))];
  if (next !== zoom.level) setZoom(next);
}

export const zoomIn = () => step(1);
export const zoomOut = () => step(-1);
export const resetZoom = () => setZoom(1);

/** @param {boolean} on */
export function suspendZoom(on) {
  zoom.suspended = on;
  apply();
}

// Ctrl+scroll (and trackpad pinches, which arrive as Ctrl+scroll) zoom.
window.addEventListener(
  "wheel",
  (evt) => {
    if (!evt.ctrlKey || zoom.suspended) return;
    evt.preventDefault();
    wheelTotal += evt.deltaY;
    if (Math.abs(wheelTotal) < WHEEL_STEP && Math.abs(evt.deltaY) < WHEEL_STEP) return;
    if (wheelTotal < 0) zoomIn();
    else zoomOut();
    wheelTotal = 0;
  },
  { passive: false },
);

apply();
