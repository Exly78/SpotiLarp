import * as api from "./api.js";

/** @type {{ playlists: import("./types.js").Playlist[], albums: import("./types.js").Album[], loading: boolean, error: string, userId: string|null, missingScopes: string[] }} */
export const library = $state({ playlists: [], albums: [], loading: false, error: "", userId: null, missingScopes: [] });

let loadToken = 0;

export async function loadLibrary() {
  const token = ++loadToken;
  library.loading = true;
  library.error = "";
  api
    .getSavedAlbums()
    .then((albums) => {
      if (token === loadToken) library.albums = albums;
    })
    .catch(() => {});
  try {
    const [playlists, userId] = await Promise.all([api.getPlaylists(), api.getUserId()]);
    if (token !== loadToken) return;
    library.playlists = playlists;
    library.userId = userId;
  } catch (e) {
    if (token === loadToken) library.error = `Failed to load playlists: ${e}`;
  } finally {
    if (token === loadToken) library.loading = false;
  }
  try {
    const missing = await api.missingScopes();
    if (token === loadToken) library.missingScopes = missing;
  } catch {}
}

export function clearLibrary() {
  loadToken++;
  library.playlists = [];
  library.albums = [];
  library.error = "";
  library.loading = false;
  library.userId = null;
  library.missingScopes = [];
}

/** @param {import("./types.js").Playlist} playlist */
export function isOwned(playlist) {
  if (playlist.isLikedSongs || playlist.collaborative) return true;
  return !!library.userId && playlist.owner?.id === library.userId;
}

export function ownedPlaylists() {
  return library.playlists.filter((p) => isOwned(p));
}

/** @param {string} name */
export async function createPlaylist(name) {
  const playlist = await api.createPlaylist(name);
  library.playlists.unshift(playlist);
  return playlist;
}

/**
 * @param {string} playlistId
 * @param {number} delta
 */
export function adjustTrackCount(playlistId, delta) {
  const playlist = library.playlists.find((p) => p.id === playlistId);
  if (playlist) playlist.track_count.total = Math.max(0, playlist.track_count.total + delta);
}

/**
 * @param {string} playlistId
 * @param {string} name
 */
export async function renamePlaylist(playlistId, name) {
  await api.renamePlaylist(playlistId, name);
  const playlist = library.playlists.find((p) => p.id === playlistId);
  if (playlist) playlist.name = name.trim();
}

/** @param {string} playlistId */
export async function deletePlaylist(playlistId) {
  await api.removePlaylist(playlistId);
  library.playlists = library.playlists.filter((p) => p.id !== playlistId);
}

/** @param {string|undefined} albumId */
export function isAlbumSaved(albumId) {
  return !!albumId && library.albums.some((a) => a.id === albumId);
}

/** @param {import("./types.js").Album} album */
export async function saveAlbum(album) {
  const id = album.id;
  if (!id) return;
  await api.saveAlbum(id);
  if (isAlbumSaved(id)) return;
  const { name, images, artists, release_date, album_type, total_tracks } = album;
  library.albums.unshift({ id, name, images, artists, release_date, album_type, total_tracks });
}

/** @param {string} albumId */
export async function removeAlbum(albumId) {
  await api.removeAlbum(albumId);
  library.albums = library.albums.filter((a) => a.id !== albumId);
}
