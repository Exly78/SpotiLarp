import * as api from "./api.js";
import { notify } from "./toast.svelte.js";

/**
 * The listening tracker: every play heard in SpotiLarp, on Spotify's other
 * apps and in an imported Spotify data download, saved on this computer by
 * the backend (tracker.rs). The stats are worked out here from all of them.
 *
 * @typedef {import("./types.js").TrackerTrack} TrackerTrack
 * @typedef {{ t: number, ms: number, track: TrackerTrack, src: number, skip: boolean }} Play
 *   `t` is when it started, `ms` how long was heard
 */

/** Where a play was heard. */
export const FROM_SPOTILARP = 0;
export const FROM_SPOTIFY = 1;
export const FROM_IMPORT = 2;

/** Before the tracker, SpotiLarp's last 1000 plays were kept here. */
const OLD_HISTORY_KEY = "spotilarp.history";
/** Spotify only lists the last 50 plays, so it's asked often enough not to miss any. */
const SYNC_EVERY_MS = 20 * 60_000;
/** Songs whose pictures and artists are looked up at once (Spotify's limit). */
const LOOKUP_BATCH = 50;

/** `version` changes whenever the plays do: read it to follow them. */
export const tracker = $state({ loaded: false, version: 0, importing: false, syncing: false });

/** @type {Play[]} oldest first */
let plays = [];
/** @type {Map<string, TrackerTrack>} */
let tracks = new Map();
/** Songs already looked up this session, found or not. */
const lookedUp = new Set();

const ready = loadTracker();

/** Every play, oldest first. Reading it follows changes. */
export function allPlays() {
  tracker.version;
  return plays;
}

/**
 * The newest plays first.
 * @param {number} limit
 */
export function recentPlays(limit) {
  tracker.version;
  return plays.slice(-limit).reverse();
}

/** When the last play started (ms since 1970), or 0. */
export function lastPlayAt() {
  tracker.version;
  return plays[plays.length - 1]?.t ?? 0;
}

/**
 * A tracker song as the rest of the app shows and plays songs.
 * @param {TrackerTrack} track
 * @returns {import("./types.js").Track}
 */
export function asTrack(track) {
  return {
    id: track.uri.startsWith("spotify:track:") ? track.uri.slice("spotify:track:".length) : "",
    name: track.name,
    uri: track.uri,
    duration_ms: track.duration_ms,
    artists: track.artists,
    album: {
      id: track.album.id || undefined,
      name: track.album.name,
      images: track.album.image ? [{ url: track.album.image }] : [],
    },
  };
}

/** Whether a song can be played (an imported play no song was found for can't). @param {TrackerTrack} track */
export function isPlayable(track) {
  return track.uri.startsWith("spotify:track:") || track.uri.startsWith("spotify:local:");
}

async function loadTracker() {
  try {
    const data = await api.trackerLoad();
    tracks = new Map(data.tracks.map((track) => [track.uri, track]));
    plays = data.plays.map(([t, ms, index, src, skip]) => ({ t, ms, track: data.tracks[index], src, skip: skip === 1 }));
    tracker.loaded = true;
    tracker.version++;
  } catch (e) {
    console.warn("Couldn't load the listening history:", e);
  }
}

/**
 * A song as the tracker keeps it, from one the app shows. Spotify's small
 * cover links are swapped for the 300px one (same picture, other size).
 * @param {import("./types.js").Track} track
 * @returns {TrackerTrack}
 */
export function trackerTrack(track) {
  const images = track.album.images ?? [];
  const biggest = images.reduce((a, b) => ((a?.width ?? 0) >= (b.width ?? 0) ? a : b), images[0]);
  return {
    uri: track.uri,
    name: track.name,
    artists: track.artists.map((a) => ({ id: a.id ?? "", name: a.name })),
    album: { id: track.album.id ?? "", name: track.album.name, image: largerCover(biggest?.url) },
    duration_ms: track.duration_ms,
  };
}

/** @param {string|null|undefined} url */
function largerCover(url) {
  return url?.replace("ab67616d00004851", "ab67616d00001e02") ?? null;
}

/**
 * Saves a play heard in SpotiLarp. Called once it counts, then again (same
 * `startedAt`) when it ends, with how long it really lasted.
 * @param {import("./types.js").Track} track
 * @param {number} startedAt
 * @param {number} listenedMs
 * @param {boolean} skipped
 */
export async function recordPlay(track, startedAt, listenedMs, skipped) {
  await ready;
  const saved = trackerTrack(track);
  const known = tracks.get(saved.uri);
  const play = plays.find((p) => p.src === FROM_SPOTILARP && p.t === startedAt && p.track.uri === saved.uri);
  if (play) {
    play.ms = Math.round(listenedMs);
    play.skip = skipped;
  } else {
    if (!known) tracks.set(saved.uri, saved);
    plays = [...plays, { t: startedAt, ms: Math.round(listenedMs), track: known ?? saved, src: FROM_SPOTILARP, skip: skipped }];
  }
  tracker.version++;
  try {
    await api.trackerRecord([{ t: startedAt, ms: Math.round(listenedMs), src: FROM_SPOTILARP, skip: skipped, track: saved }]);
  } catch (e) {
    console.warn("Couldn't save a play:", e);
  }
}

/**
 * Adds what Spotify's other apps played lately (Spotify's history leaves out
 * SpotiLarp's own plays, so they're not counted twice). The backend skips any
 * it already has. Its times are when each song ended; it doesn't say how
 * much was heard, so the whole song counts.
 */
export async function syncSpotifyPlays() {
  await ready;
  if (tracker.syncing) return;
  tracker.syncing = true;
  try {
    const recent = await api.getRecentlyPlayed();
    const found = recent
      .filter((track) => track.added_at && track.uri.startsWith("spotify:track:"))
      .map((track) => ({
        t: Date.parse(/** @type {string} */ (track.added_at)) - track.duration_ms,
        ms: track.duration_ms,
        src: FROM_SPOTIFY,
        skip: false,
        track: trackerTrack(track),
      }));
    if (found.length > 0 && (await api.trackerRecord(found)) > 0) await loadTracker();
  } catch {
    // Logged out or offline: the next sync tries again.
  } finally {
    tracker.syncing = false;
  }
}
setInterval(syncSpotifyPlays, SYNC_EVERY_MS);

/** Asks for Spotify's data download and adds its plays. */
export async function importSpotifyHistory() {
  if (tracker.importing) return;
  tracker.importing = true;
  try {
    const result = await api.trackerImport();
    if (!result) return;
    await loadTracker();
    const parts = [`Added ${result.added.toLocaleString()} plays from ${result.files} ${result.files === 1 ? "file" : "files"}`];
    if (result.duplicates > 0) parts.push(`${result.duplicates.toLocaleString()} were already here`);
    if (result.unmatched > 0) parts.push(`${result.unmatched.toLocaleString()} have no link to the song, so they count but can't play`);
    notify(`${parts.join(". ")}.`);
  } catch (e) {
    notify(`${e}`);
  } finally {
    tracker.importing = false;
  }
}

export async function clearTracker() {
  try {
    await api.trackerClear();
    plays = [];
    tracks = new Map();
    tracker.version++;
  } catch (e) {
    notify(`${e}`);
  }
}

/**
 * Looks up pictures and artists for songs that came without them (imported
 * history), so the stats can show them. Each song is asked about once a session.
 * @param {TrackerTrack[]} wanted
 */
export async function fillInTracks(wanted) {
  const missing = wanted
    .filter((track) => !lookedUp.has(track.uri) && track.uri.startsWith("spotify:track:"))
    .filter((track) => !track.album.image || !track.artists.some((a) => a.id));
  if (missing.length === 0) return;
  for (const track of missing) lookedUp.add(track.uri);
  for (let i = 0; i < missing.length; i += LOOKUP_BATCH) {
    try {
      const found = await api.getTracksByUri(missing.slice(i, i + LOOKUP_BATCH).map((track) => track.uri));
      const details = found.map(trackerTrack);
      for (const detail of details) {
        const track = tracks.get(detail.uri);
        if (!track) continue;
        // In place, so every play of it sees the change.
        if (!track.album.image) track.album = { ...track.album, ...detail.album, name: track.album.name || detail.album.name };
        if (!track.artists.some((a) => a.id)) track.artists = detail.artists;
        if (!track.duration_ms) track.duration_ms = detail.duration_ms;
      }
      if (details.length > 0) {
        tracker.version++;
        await api.trackerUpdateTracks(details);
      }
    } catch {
      // No playback session yet: the stats show them without pictures.
      return;
    }
  }
}

// The plays SpotiLarp kept before the tracker, moved in once.
ready.then(async () => {
  let entries;
  try {
    entries = JSON.parse(localStorage.getItem(OLD_HISTORY_KEY) ?? "null");
  } catch {}
  if (!Array.isArray(entries) || entries.length === 0) return;
  // They were saved half a minute in (or halfway through a short song).
  const old = entries
    .filter((entry) => entry?.track?.uri && entry.playedAt)
    .map((entry) => ({
      t: Math.round(Date.parse(entry.playedAt) - Math.min(30_000, (entry.track.duration_ms || 0) / 2)),
      ms: entry.track.duration_ms || 0,
      src: FROM_SPOTILARP,
      skip: false,
      track: trackerTrack(entry.track),
    }));
  try {
    await api.trackerRecord(old);
    localStorage.removeItem(OLD_HISTORY_KEY);
    await loadTracker();
  } catch (e) {
    console.warn("Couldn't move the old listening history over:", e);
  }
});
