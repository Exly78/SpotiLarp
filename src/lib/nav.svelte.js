/**
 * @typedef {{ type: "home" }
 *   | { type: "playlist", playlist: import("./types.js").Playlist }
 *   | { type: "artist", id: string, name: string }
 *   | { type: "album", id: string, name?: string }
 *   | { type: "search", query: string }
 *   | { type: "stats" }
 *   | { type: "settings" }} View
 */

const MAX_HISTORY = 50;

/** @type {{ view: View, back: View[], forward: View[], lyrics: boolean }} */
export const nav = $state({ view: { type: "home" }, back: [], forward: [], lyrics: false });

/** @param {View} view */
export function viewKey(view) {
  switch (view.type) {
    case "playlist":
      return `playlist:${view.playlist.id}`;
    case "artist":
    case "album":
      return `${view.type}:${view.id}`;
    case "search":
      return `search:${view.query}`;
    default:
      return view.type;
  }
}

/** @param {View} view */
export function navigate(view) {
  nav.lyrics = false;
  if (viewKey(view) === viewKey(nav.view)) return;
  nav.back.push(nav.view);
  if (nav.back.length > MAX_HISTORY) nav.back.shift();
  nav.forward = [];
  nav.view = view;
}

export function resetNav() {
  nav.view = { type: "home" };
  nav.back = [];
  nav.forward = [];
  nav.lyrics = false;
}

export function goBack() {
  const view = nav.back.pop();
  if (!view) return;
  nav.forward.push(nav.view);
  nav.view = view;
  nav.lyrics = false;
}

export function goForward() {
  const view = nav.forward.pop();
  if (!view) return;
  nav.back.push(nav.view);
  nav.view = view;
  nav.lyrics = false;
}

/** @param {{ id: string, name: string }} artist */
export function openArtist(artist) {
  if (artist.id) navigate({ type: "artist", id: artist.id, name: artist.name });
}

/** @param {{ id?: string, name?: string }} album */
export function openAlbum(album) {
  if (album.id) navigate({ type: "album", id: album.id, name: album.name });
}
