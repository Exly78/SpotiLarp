import { invoke as tauriInvoke } from "@tauri-apps/api/core";

export const LOGIN_EXPIRED = "Your Spotify login has expired, please log in again";
export const LOGIN_CANCELLED = "Login cancelled";

let onLoginExpired = () => {};

/** @param {() => void} handler */
export function setLoginExpiredHandler(handler) {
  onLoginExpired = handler;
}

/**
 * @param {string} command
 * @param {import("@tauri-apps/api/core").InvokeArgs} [args]
 * @returns {Promise<any>}
 */
function invoke(command, args) {
  return tauriInvoke(command, args).catch((e) => {
    if (e === LOGIN_EXPIRED) onLoginExpired();
    throw e;
  });
}

export const hasClientId = () => invoke("has_client_id");
/** @param {string} clientId */
export const setClientId = (clientId) => invoke("set_client_id", { clientId });
export const login = () => invoke("login");
export const cancelLogin = () => invoke("cancel_login");
export const loginStatus = () => invoke("login_status");
export const restoreSession = () => invoke("restore_session");
export const logout = () => invoke("logout");
export const hasConnectSession = () => invoke("has_connect_session");
/** @param {boolean} interactive */
export const connectPlayback = (interactive) => invoke("connect_playback", { interactive });
/**
 * @param {string} uri
 * @param {number} [positionMs]
 */
export const playTrack = (uri, positionMs = 0) => invoke("play_track", { uri, positionMs });
/** @param {string} uri */
export const preloadTrack = (uri) => invoke("preload_track", { uri });
/**
 * @param {string} seedUri
 * @param {string[]} recentUris
 * @returns {Promise<import("./types.js").Track[]>}
 */
export const getAutoplayTracks = (seedUri, recentUris) => invoke("get_autoplay_tracks", { seedUri, recentUris });
/** @returns {Promise<import("./types.js").Track[]>} */
export const search = (/** @type {string} */ query) => invoke("search", { query });
/** @returns {Promise<import("./types.js").Playlist[]>} */
export const getPlaylists = () => invoke("get_playlists");
/** @returns {Promise<import("./types.js").Track[]>} */
export const getPlaylistTracks = (/** @type {string} */ playlistId) => invoke("get_playlist_tracks", { playlistId });
/** @returns {Promise<import("./types.js").Track[]>} */
export const getLikedSongs = () => invoke("get_liked_songs");
/** @param {string} trackId */
export const isTrackLiked = (trackId) => invoke("is_track_liked", { trackId });
/** @param {string[]} trackIds */
export const areTracksLiked = (trackIds) => invoke("are_tracks_liked", { trackIds });
/** @param {string} trackId */
export const likeTrack = (trackId) => invoke("like_track", { trackId });
/** @param {string} trackId */
export const unlikeTrack = (trackId) => invoke("unlike_track", { trackId });
export const pause = () => invoke("pause");
export const resume = () => invoke("resume");
/** @param {number} positionMs */
export const seek = (positionMs) => invoke("seek", { positionMs });
/** @param {number} volume */
export const setVolume = (volume) => invoke("set_volume", { volume });
/**
 * @param {string} trackName
 * @param {string} artistName
 * @param {string} albumName
 * @param {number} durationMs
 * @returns {Promise<import("./types.js").LyricsResult | null>}
 */
export const getLyrics = (trackName, artistName, albumName, durationMs) =>
  invoke("get_lyrics", { trackName, artistName, albumName, durationMs });
/**
 * @param {string} artistId
 * @returns {Promise<import("./types.js").ArtistDetails>}
 */
export const getArtist = (artistId) => invoke("get_artist", { artistId });
/** @param {string} artistId */
export const isFollowingArtist = (artistId) => invoke("is_following_artist", { artistId });
/** @param {string} artistId */
export const followArtist = (artistId) => invoke("follow_artist", { artistId });
/** @param {string} artistId */
export const unfollowArtist = (artistId) => invoke("unfollow_artist", { artistId });
export const hasDiscordClientId = () => invoke("has_discord_client_id");
/** @param {string} clientId */
export const setDiscordClientId = (clientId) => invoke("set_discord_client_id", { clientId });
export const clearDiscordClientId = () => invoke("clear_discord_client_id");
/**
 * @param {string} query
 * @returns {Promise<import("./types.js").SearchResults>}
 */
export const searchAll = (query) => invoke("search_all", { query });
/**
 * @param {string} query
 * @param {number} offset
 * @returns {Promise<import("./types.js").Track[]>}
 */
export const searchTracksPage = (query, offset) => invoke("search_tracks_page", { query, offset });
/**
 * @param {string} artistId
 * @returns {Promise<import("./types.js").ArtistPage>}
 */
export const getArtistPage = (artistId) => invoke("get_artist_page", { artistId });
/**
 * @param {string} artistId
 * @param {import("./types.js").ReleaseGroup} group
 * @returns {Promise<import("./types.js").Album[]>}
 */
export const getArtistDiscography = (artistId, group) => invoke("get_artist_discography", { artistId, group });
/**
 * @param {string} albumId
 * @returns {Promise<import("./types.js").AlbumDetails>}
 */
export const getAlbum = (albumId) => invoke("get_album", { albumId });
/** @returns {Promise<import("./types.js").Album[]>} */
export const getSavedAlbums = () => invoke("get_saved_albums");
/** @param {string} albumId */
export const saveAlbum = (albumId) => invoke("save_album", { albumId });
/** @param {string} albumId */
export const removeAlbum = (albumId) => invoke("remove_album", { albumId });
/** @returns {Promise<import("./types.js").Track[]>} */
export const getRecentlyPlayed = () => invoke("get_recently_played");
/**
 * @param {"short_term"|"medium_term"|"long_term"} timeRange
 * @param {number} limit
 * @returns {Promise<import("./types.js").Track[]>}
 */
export const getTopTracks = (timeRange, limit) => invoke("get_top_tracks", { timeRange, limit });
/**
 * @param {"short_term"|"medium_term"|"long_term"} timeRange
 * @param {number} limit
 * @returns {Promise<import("./types.js").ArtistDetails[]>}
 */
export const getTopArtists = (timeRange, limit) => invoke("get_top_artists", { timeRange, limit });
/**
 * @param {string} playlistId
 * @param {string[]} uris
 */
export const addToPlaylist = (playlistId, uris) => invoke("add_to_playlist", { playlistId, uris });
/**
 * @param {string} playlistId
 * @param {string} uri
 */
export const removeFromPlaylist = (playlistId, uri) => invoke("remove_from_playlist", { playlistId, uri });
/**
 * @param {string} name
 * @returns {Promise<import("./types.js").Playlist>}
 */
export const createPlaylist = (name) => invoke("create_playlist", { name });
/** @returns {Promise<string>} */
export const getUserId = () => invoke("get_user_id");
/** @returns {Promise<string[]>} */
export const missingScopes = () => invoke("missing_scopes");
/** @returns {Promise<import("./types.js").Settings>} */
export const getSettings = () => invoke("get_settings");
/**
 * @param {import("./types.js").Settings} settings
 * @returns {Promise<boolean>}
 */
export const setSettings = (settings) => invoke("set_settings", { settings });
/** @returns {Promise<number>} */
export const getCacheSize = () => invoke("get_cache_size");
export const clearCache = () => invoke("clear_cache");
/** @returns {Promise<boolean>} */
export const getAutostart = () => invoke("get_autostart");
/** @param {boolean} enabled */
export const setAutostart = (enabled) => invoke("set_autostart", { enabled });
/** @param {boolean} enabled */
export const setMiniMode = (enabled) => invoke("set_mini_mode", { enabled });
/**
 * @param {string} playlistId
 * @param {string} name
 */
export const renamePlaylist = (playlistId, name) => invoke("rename_playlist", { playlistId, name });
/** @param {string} playlistId */
export const removePlaylist = (playlistId) => invoke("remove_playlist", { playlistId });
/**
 * @param {string} playlistId
 * @param {number} from
 * @param {number} insertBefore
 */
export const movePlaylistTrack = (playlistId, from, insertBefore) =>
  invoke("move_playlist_track", { playlistId, from, insertBefore });
/** @returns {Promise<string[]>} */
export const listOutputDevices = () => invoke("list_output_devices");
/**
 * @param {boolean} enabled
 * @param {number[]} gains
 */
export const previewEqualizer = (enabled, gains) => invoke("preview_equalizer", { enabled, gains });
/** @returns {Promise<string|null>} */
export const pickFolder = () => invoke("pick_folder");
/** @returns {Promise<import("./types.js").LocalLibrary>} */
export const getLocalFiles = () => invoke("get_local_files");
/** @returns {Promise<import("./types.js").LocalLibrary>} */
export const rescanLocalFiles = () => invoke("rescan_local_files");
