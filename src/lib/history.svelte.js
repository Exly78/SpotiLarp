import { listen } from "@tauri-apps/api/event";
import { player } from "./player.svelte.js";
import { queue, setLastPlayedLookup } from "./queue.svelte.js";
import { tracker, allPlays, recordPlay } from "./tracker.svelte.js";
import { EPISODE_URI_PREFIX } from "./constants.js";

/**
 * Notices songs played in SpotiLarp, which Spotify's own history doesn't
 * include (librespot doesn't report plays), for the listening tracker. A play
 * counts after 30 seconds or half of a shorter song; paused time and seeking
 * don't count. It's saved when it counts, again when paused, and once more
 * when it ends, with how long was heard and whether it was skipped. Podcast
 * episodes keep their own progress (podcasts.svelte.js) and aren't counted.
 */

const COUNT_AFTER_MS = 30_000;
const CHECK_EVERY_MS = 1000;
/** Moving on this close to the end isn't skipping. */
const FINISHED_WITHIN_MS = 10_000;

let currentUri = "";
let currentDurationMs = 0;
/** The playing song as it'll be saved, taken when it starts. @type {import("./types.js").Track|null} */
let currentTrack = null;
/** When it first played (ms since 1970), the play's key in the tracker. */
let startedAt = 0;
let listenedMs = 0;
let playingSince = 0;
let counted = false;
let ended = false;

/** @returns {import("./types.js").Track|null} */
function playingTrack() {
  // The queue's copy has every size of the cover.
  if (queue.current?.uri === currentUri) return queue.current;
  const t = player.track;
  if (!t || t.uri !== currentUri) return null;
  return {
    id: t.trackId ?? "",
    name: t.name,
    uri: t.uri ?? "",
    duration_ms: t.durationMs,
    artists: (t.artistList ?? []).map((a) => ({ id: a.id ?? "", name: a.name })),
    album: { name: t.album, images: t.coverUrl ? [{ url: t.coverUrl }] : [] },
  };
}

function heardMs() {
  return listenedMs + (playingSince ? performance.now() - playingSince : 0);
}

function stopClock() {
  if (playingSince) listenedMs += performance.now() - playingSince;
  playingSince = 0;
}

function check() {
  if (counted || ended || !currentUri || currentUri.startsWith(EPISODE_URI_PREFIX)) return;
  const duration = currentDurationMs;
  const needed = duration > 0 ? Math.min(COUNT_AFTER_MS, duration / 2) : COUNT_AFTER_MS;
  if (heardMs() < needed) return;
  const track = currentTrack ?? playingTrack();
  if (!track) return;
  currentTrack = track;
  counted = true;
  recordPlay(track, startedAt, heardMs(), false);
}

/**
 * The play is over: saved with how long it lasted.
 * @param {boolean} finished it played to the end
 */
function end(finished) {
  // In a hidden window the every-second check can fall behind.
  check();
  stopClock();
  if (ended) return;
  ended = true;
  if (!counted || !currentTrack) return;
  const heard = heardMs();
  const skipped = !finished && heard < currentDurationMs - FINISHED_WITHIN_MS;
  recordPlay(currentTrack, startedAt, heard, skipped);
}

/** When each song was last played (ms since 1970), from every source. */
let lastPlayedVersion = -1;
/** @type {Map<string, number>} */
let lastPlayedAt = new Map();

// Smart shuffle plays what was heard lately last.
setLastPlayedLookup((track) => {
  const plays = allPlays();
  if (lastPlayedVersion !== tracker.version) {
    lastPlayedVersion = tracker.version;
    lastPlayedAt = new Map(plays.map((play) => [play.track.uri, play.t]));
  }
  return lastPlayedAt.get(track.uri) ?? 0;
});

setInterval(check, CHECK_EVERY_MS);

listen("player-event", (event) => {
  const e = /** @type {import("./types.js").PlaybackEvent} */ (event.payload);
  switch (e.type) {
    case "TrackChanged":
      end(false);
      // Every load is its own play, repeats of the same song included.
      currentUri = e.uri ?? "";
      currentDurationMs = e.duration_ms ?? 0;
      currentTrack = null;
      startedAt = 0;
      listenedMs = 0;
      playingSince = 0;
      counted = false;
      ended = false;
      break;
    case "Playing":
      if (!startedAt) startedAt = Date.now();
      if (!playingSince) playingSince = performance.now();
      currentTrack ??= playingTrack();
      break;
    case "Paused":
      check();
      stopClock();
      if (counted && !ended && currentTrack) recordPlay(currentTrack, startedAt, heardMs(), false);
      break;
    case "EndOfTrack":
      end(true);
      break;
    case "Stopped":
    case "Unavailable":
      end(false);
      break;
  }
});
