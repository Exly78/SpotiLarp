import * as api from "./api.js";
import { playFromList, addManyToQueue } from "./queue.svelte.js";
import { notify } from "./toast.svelte.js";
import { cachedPlaylistTracks, cachePlaylistTracks } from "./components/views/PlaylistView.svelte";

/**
 * Playing or queueing a whole playlist or album without opening its page
 * (from the library's menus, or a double-click).
 * @typedef {import("./types.js").Playlist} Playlist
 * @typedef {import("./types.js").Album} Album
 * @typedef {import("./types.js").Track} Track
 */

/** @param {Playlist} playlist */
async function playlistTracks(playlist) {
  const cached = cachedPlaylistTracks(playlist.id);
  if (cached) return cached;
  const tracks = playlist.isLikedSongs ? await api.getLikedSongs() : await api.getPlaylistTracks(playlist.id);
  cachePlaylistTracks(playlist.id, tracks);
  return tracks;
}

/** @param {Album} album */
async function albumTracks(album) {
  return album.id ? (await api.getAlbum(album.id)).tracks : [];
}

/**
 * @param {Track[]} tracks
 * @param {string} name
 * @param {import("./nav.svelte.js").View} view
 */
function playAll(tracks, name, view) {
  if (tracks.length === 0) {
    notify(`"${name}" has no songs to play`);
    return;
  }
  playFromList(tracks, -1, name, view);
}

/** @param {Playlist} playlist */
export async function playPlaylist(playlist) {
  try {
    playAll(await playlistTracks(playlist), playlist.name, { type: "playlist", playlist });
  } catch (e) {
    notify(`Couldn't play "${playlist.name}": ${e}`);
  }
}

/** @param {Playlist} playlist */
export async function queuePlaylist(playlist) {
  try {
    addManyToQueue(await playlistTracks(playlist));
  } catch (e) {
    notify(`Couldn't queue "${playlist.name}": ${e}`);
  }
}

/** @param {Album} album */
export async function playAlbum(album) {
  try {
    playAll(await albumTracks(album), album.name, { type: "album", id: album.id ?? "", name: album.name });
  } catch (e) {
    notify(`Couldn't play "${album.name}": ${e}`);
  }
}

/** @param {Album} album */
export async function queueAlbum(album) {
  try {
    addManyToQueue(await albumTracks(album));
  } catch (e) {
    notify(`Couldn't queue "${album.name}": ${e}`);
  }
}
