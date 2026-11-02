import * as api from "./api.js";

/**
 * Which song a set of lyrics belongs to, so a slow answer for the last song
 * isn't shown for this one.
 * @param {import("./player.svelte.js").CurrentTrack} track
 */
export function lyricsKey(track) {
  return `${track.trackId ?? ""}::${track.name}::${track.artists}::${track.album}`;
}

/**
 * @param {import("./player.svelte.js").CurrentTrack} track
 * @param {boolean} [recheck] ask again even if no lyrics were found last time
 */
export function fetchLyrics(track, recheck = false) {
  const firstArtist = track.artistList?.[0]?.name ?? track.artists.split(",")[0]?.trim() ?? track.artists;
  return api.getLyrics(track.trackId, track.name, firstArtist, track.album, track.durationMs, recheck);
}

/**
 * The line playing at `positionMs`, or -1 before the first.
 * @param {import("./types.js").LyricLine[]|null} lines
 * @param {number} positionMs
 */
export function activeLine(lines, positionMs) {
  if (!lines) return -1;
  let idx = -1;
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].time_ms <= positionMs) idx = i;
    else break;
  }
  return idx;
}
