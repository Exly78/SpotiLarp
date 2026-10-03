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
 * @property {string} [id]
 * @property {string} name
 * @property {Image[]} images
 * @property {Artist[]} [artists]
 * @property {string|null} [release_date]
 * @property {string|null} [album_type]
 * @property {number|null} [total_tracks]
 */

/** @typedef {Album & { tracks: Track[] }} AlbumDetails */

/**
 * @typedef {Object} SearchResults
 * @property {Track[]} tracks
 * @property {ArtistDetails[]} artists
 * @property {Album[]} albums
 */

/**
 * @typedef {Object} Track
 * @property {string} id
 * @property {string} name
 * @property {string} uri
 * @property {number} duration_ms
 * @property {Artist[]} artists
 * @property {Album} album
 * @property {string|null} [added_at]
 * @property {number|null} [position]
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
 * @property {{ id: string, display_name?: string|null }|null} [owner]
 * @property {boolean} [collaborative]
 * @property {boolean} [isLikedSongs]
 */

/**
 * @typedef {Object} Settings
 * @property {"low"|"normal"|"high"} audio_quality
 * @property {boolean} normalize_volume
 * @property {number} cache_limit_mb
 * @property {boolean} close_to_tray
 * @property {boolean} notifications
 * @property {boolean} eq_enabled
 * @property {number[]} eq_gains
 * @property {string} eq_preset
 * @property {string|null} output_device
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
 * @property {{ id: string|null, name: string }[]} [artist_list]
 * @property {string|null} [track_id]
 * @property {string} [album]
 * @property {number} [duration_ms]
 * @property {string|null} [cover_url]
 */

export {};
