/** @type {{ open: boolean, x: number, y: number, track: import("./types.js").Track|null, onRemove: (() => void)|null }} */
export const menu = $state({ open: false, x: 0, y: 0, track: null, onRemove: null });

/**
 * @param {MouseEvent} evt
 * @param {import("./types.js").Track} track
 * @param {(() => void)|null} [onRemove]
 */
export function openTrackMenu(evt, track, onRemove = null) {
  evt.preventDefault();
  menu.track = track;
  menu.onRemove = onRemove;
  menu.x = evt.clientX;
  menu.y = evt.clientY;
  menu.open = true;
}

export function closeMenu() {
  menu.open = false;
}
