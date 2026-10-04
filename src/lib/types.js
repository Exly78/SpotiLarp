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

/** @typedef {Track & { playcount: number|null, explicit: boolean }} PopularTrack */

/** @typedef {"all"|"album"|"single"|"compilation"} ReleaseGroup */

/**
 * @typedef {Object} PlaylistCard
 * @property {string} id
 * @property {string} name
 * @property {string} description
 * @property {Image[]} images
 * @property {string|null} owner
 */

/**
 * @typedef {Object} TopCity
 * @property {string} city
 * @property {string} country
 * @property {number} listeners
 */

/**
 * @typedef {Object} ExternalLink
 * @property {string} name
 * @property {string} url
 */

/**
 * Everything on an artist's page. Without a playback session only the
 * basics (name, images, genres, followers, releases, an approximate "Popular")
 * are filled in.
 * @typedef {Object} ArtistPage
 * @property {string} id
 * @property {string} name
 * @property {Image[]} images
 * @property {Image[]} header_images
 * @property {string|null} color
 * @property {boolean} verified
 * @property {number|null} monthly_listeners
 * @property {number|null} followers
 * @property {number|null} world_rank
 * @property {string[]} genres
 * @property {string|null} biography
 * @property {Image[][]} gallery
 * @property {TopCity[]} top_cities
 * @property {ExternalLink[]} external_links
 * @property {boolean|null} following
 * @property {PopularTrack[]} top_tracks
 * @property {Album|null} latest_release
 * @property {Album[]} popular_releases
 * @property {Album[]} albums
 * @property {Album[]} singles
 * @property {Album[]} compilations
 * @property {number} album_count
 * @property {number} single_count
 * @property {number} compilation_count
 * @property {ArtistDetails[]} related_artists
 * @property {Album[]} appears_on
 * @property {PlaylistCard[]} featuring
 * @property {PlaylistCard[]} discovered_on
 * @property {PlaylistCard[]} playlists
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
 * @property {string[]} local_folders
 * @property {boolean} discord_local_covers
 */

/**
 * @typedef {Object} LocalLibrary
 * @property {string[]} folders
 * @property {Track[]} tracks
 * @property {number} skipped
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
 * @property {string} [uri]
 * @property {string} [album]
 * @property {number} [duration_ms]
 * @property {string|null} [cover_url]
 */

export {};
