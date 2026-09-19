/**
 * @typedef {Object} Image
 * @property {string} url
 * @property {number|null} [width]
 * @property {number|null} [height]
 */

/**
 * @typedef {Object} Artist
 * @property {string} id
 * @property {string} name
 */

/**
 * @typedef {Object} Album
 * @property {string} name
 * @property {Image[]} images
 */

/**
 * @typedef {Object} Track
 * @property {string} id
 * @property {string} name
 * @property {string} uri
 * @property {number} duration_ms
 * @property {Artist[]} artists
 * @property {Album} album
 */

/**
 * @typedef {Object} Followers
 * @property {number} total
 */

/**
 * @typedef {Object} ArtistDetails
 * @property {string} id
 * @property {string} name
 * @property {string[]} genres
 * @property {Image[]} images
 * @property {Followers|null} [followers]
 */

/**
 * @typedef {Object} PlaylistTrackCount
 * @property {number} total
 */

/**
 * @typedef {Object} Playlist
 * @property {string} id
 * @property {string} name
 * @property {Image[]} images
 * @property {PlaylistTrackCount} track_count
 * @property {boolean} [isLikedSongs]
 */

/**
 * @typedef {Object} LyricLine
 * @property {number} time_ms
 * @property {string} text
 */

/**
 * @typedef {Object} LyricsResult
 * @property {boolean} instrumental
 * @property {LyricLine[]|null} synced
 * @property {string|null} plain
 */

/**
 * @typedef {Object} PlaybackEvent
 * @property {string} type
 * @property {number} [position_ms]
 * @property {number} [volume]
 * @property {string} [name]
 * @property {string} [artists]
 * @property {string|null} [primary_artist_id]
 * @property {string|null} [track_id]
 * @property {string} [album]
 * @property {number} [duration_ms]
 * @property {string|null} [cover_url]
 */

export {};
