/** @type {{ tracks: import("./types.js").Track[], origin: symbol|null, index: number }} */
export const drag = $state({ tracks: [], origin: null, index: -1 });

/**
 * @param {DragEvent} evt
 * @param {import("./types.js").Track[]} tracks
 * @param {symbol} origin
 * @param {number} index
 */
export function startTrackDrag(evt, tracks, origin, index) {
  drag.tracks = tracks;
  drag.origin = origin;
  drag.index = index;
  if (evt.dataTransfer) {
    evt.dataTransfer.effectAllowed = "copyMove";
    const links = tracks.map((t) => (t.id ? `https://open.spotify.com/track/${t.id}` : t.name));
    evt.dataTransfer.setData("text/plain", links.join("\n"));
  }
}

export function endDrag() {
  drag.tracks = [];
  drag.origin = null;
  drag.index = -1;
}

/**
 * @param {DragEvent & { currentTarget: HTMLElement }} evt
 * @param {number} index
 */
export function insertionIndex(evt, index) {
  const rect = evt.currentTarget.getBoundingClientRect();
  return evt.clientY > rect.top + rect.height / 2 ? index + 1 : index;
}
