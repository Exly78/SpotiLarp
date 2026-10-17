/**
 * `tracks` holds every selected song when the menu is for a multi-selection.
 * @type {{ open: boolean, x: number, y: number, track: import("./types.js").Track|null, tracks: import("./types.js").Track[], onRemove: (() => void)|null }}
 */
export const menu = $state({ open: false, x: 0, y: 0, track: null, tracks: [], onRemove: null });

/**
 * @param {MouseEvent} evt
 * @param {import("./types.js").Track} track
 * @param {(() => void)|null} [onRemove]
 */
export function openTrackMenu(evt, track, onRemove = null) {
  openTracksMenu(evt, [track], onRemove);
}

/**
 * @param {MouseEvent} evt
 * @param {import("./types.js").Track[]} tracks
 * @param {(() => void)|null} [onRemove]
 */
export function openTracksMenu(evt, tracks, onRemove = null) {
  evt.preventDefault();
  if (tracks.length === 0) return;
  menu.track = tracks[0];
  menu.tracks = tracks;
  menu.onRemove = onRemove;
  menu.x = evt.clientX;
  menu.y = evt.clientY;
  menu.open = true;
}

export function closeMenu() {
  menu.open = false;
}
