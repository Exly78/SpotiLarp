import * as api from "./api.js";

const CACHE_KEY = "spotilarp.library";

/**
 * `folders` is the playlist order with folders, once the playback session has
 * read it. `shows` are the podcasts you follow.
 * @type {{ playlists: import("./types.js").Playlist[], albums: import("./types.js").Album[], shows: import("./types.js").Show[], folders: import("./types.js").RootlistEntry[]|null, loading: boolean, error: string, userId: string|null, missingScopes: string[] }}
 */
export const library = $state({
  playlists: [],
  albums: [],
  shows: [],
  folders: null,
  loading: false,
  error: "",
  userId: null,
  missingScopes: [],
});

let loadToken = 0;

/**
 * The last library seen, shown straight away at startup while Spotify is
 * asked again (it replaces the cache once it answers).
 * @returns {{ playlists: import("./types.js").Playlist[], albums: import("./types.js").Album[], shows?: import("./types.js").Show[], folders?: import("./types.js").RootlistEntry[]|null, userId: string|null }|null}
 */
function readCache() {
  try {
    const cached = JSON.parse(localStorage.getItem(CACHE_KEY) ?? "null");
    if (cached && Array.isArray(cached.playlists) && Array.isArray(cached.albums)) return cached;
  } catch {}
  return null;
}

function writeCache() {
  try {
    const { playlists, albums, folders, userId } = $state.snapshot(library);
    // Descriptions can be long and the sidebar doesn't show them.
    const shows = $state.snapshot(library.shows).map(({ description, ...show }) => show);
    localStorage.setItem(CACHE_KEY, JSON.stringify({ playlists, albums, shows, folders, userId }));
  } catch {}
}

export async function loadLibrary() {
  const token = ++loadToken;
  library.loading = true;
  library.error = "";
  const cached = library.playlists.length === 0 ? readCache() : null;
  if (cached) {
    library.playlists = cached.playlists;
    library.albums = cached.albums;
    library.shows = Array.isArray(cached.shows) ? cached.shows : [];
    library.folders = cached.folders ?? null;
    library.userId = cached.userId;
  }
  loadFolders();
  const albumsLoaded = Promise.all([
    api
      .getSavedAlbums()
      .then((albums) => {
        if (token === loadToken) library.albums = albums;
      })
      .catch(() => {}),
    api
      .getSavedShows()
      .then((shows) => {
        if (token === loadToken) library.shows = shows;
      })
      .catch(() => {}),
  ]);
  try {
    const [playlists, userId] = await Promise.all([api.getPlaylists(), api.getUserId()]);
    if (token !== loadToken) return;
    library.playlists = playlists;
    library.userId = userId;
    albumsLoaded.then(() => token === loadToken && writeCache());
  } catch (e) {
    // A cached library stays usable when Spotify can't be reached.
    if (token === loadToken && library.playlists.length === 0) library.error = `Failed to load playlists: ${e}`;
  } finally {
    if (token === loadToken) library.loading = false;
  }
  try {
    const missing = await api.missingScopes();
    if (token === loadToken) library.missingScopes = missing;
  } catch {}
}

/**
 * Reads the playlist folders, when playback is (or can quietly be) connected;
 * without it the library stays a flat list.
 */
export async function loadFolders() {
  const token = loadToken;
  try {
    if (!(await api.hasConnectSession())) return;
    const folders = await api.getLibraryFolders();
    if (token !== loadToken) return;
    library.folders = folders;
    writeCache();
  } catch {}
}

export function clearLibrary() {
  loadToken++;
  library.playlists = [];
  library.albums = [];
  library.shows = [];
  library.folders = null;
  library.error = "";
  library.loading = false;
  library.userId = null;
  library.missingScopes = [];
  try {
    localStorage.removeItem(CACHE_KEY);
  } catch {}
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
  writeCache();
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
  writeCache();
}

/**
 * @param {import("./types.js").Playlist} playlist the open page's copy, updated too
 * @param {{ name: string, description: string, public: boolean }} details
 */
export async function updatePlaylistDetails(playlist, details) {
  await api.updatePlaylistDetails(playlist.id, details);
  for (const target of [playlist, library.playlists.find((p) => p.id === playlist.id)]) {
    if (!target) continue;
    target.name = details.name.trim();
    if (details.description.trim()) target.description = details.description.trim();
    target.public = details.public;
  }
  writeCache();
}

/** @param {string} playlistId */
export async function deletePlaylist(playlistId) {
  await api.removePlaylist(playlistId);
  library.playlists = library.playlists.filter((p) => p.id !== playlistId);
  writeCache();
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
  writeCache();
}

/** @param {string|undefined} showId */
export function isShowSaved(showId) {
  return !!showId && library.shows.some((s) => s.id === showId);
}

/** @param {import("./types.js").Show} show */
export async function saveShow(show) {
  await api.saveShow(show.id);
  if (isShowSaved(show.id)) return;
  const { id, name, images, total_episodes } = show;
  library.shows.unshift({ id, name, images, total_episodes });
  writeCache();
}

/** @param {string} showId */
export async function removeShow(showId) {
  await api.removeShow(showId);
  library.shows = library.shows.filter((s) => s.id !== showId);
  writeCache();
}

/** @param {string} albumId */
export async function removeAlbum(albumId) {
  await api.removeAlbum(albumId);
  library.albums = library.albums.filter((a) => a.id !== albumId);
  writeCache();
}
