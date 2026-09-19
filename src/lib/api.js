import { invoke } from "@tauri-apps/api/core";

export const hasClientId = () => invoke("has_client_id");
/** @param {string} clientId */
export const setClientId = (clientId) => invoke("set_client_id", { clientId });
export const login = () => invoke("login");
export const loginStatus = () => invoke("login_status");
export const restoreSession = () => invoke("restore_session");
export const logout = () => invoke("logout");
export const hasConnectSession = () => invoke("has_connect_session");
export const connectPlayback = () => invoke("connect_playback");
/** @param {string} uri */
export const playTrack = (uri) => invoke("play_track", { uri });
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
