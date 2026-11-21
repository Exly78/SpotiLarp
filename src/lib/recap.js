/**
 * A year of listening looked back on, for the Recap (components/Recap.svelte),
 * worked out from the tracker's plays in one go. Plain functions, like
 * trackerStats.js.
 *
 * @typedef {import("./tracker.svelte.js").Play} Play
 * @typedef {import("./types.js").TrackerTrack} TrackerTrack
 * @typedef {"morning"|"afternoon"|"evening"|"night"} Daypart
 */

import { playsBetween, totals, topSongs, topArtists, topAlbums } from "./trackerStats.js";

/** Fewer plays than this and there isn't much of a year to look back on. */
export const RECAP_MIN_PLAYS = 25;
/** Songs saved by "Save as playlist" (Spotify's own year lists are 100 long). */
export const RECAP_PLAYLIST_SIZE = 100;
/** A song played this often in one day gets a slide of its own. */
const ON_REPEAT_MIN = 5;

/** @param {number} at */
function dayOf(at) {
  const day = new Date(at);
  day.setHours(0, 0, 0, 0);
  return day.getTime();
}

/** By the calendar, so a clock change doesn't shift it. @param {number} at @param {number} days */
function addDays(at, days) {
  const day = new Date(at);
  day.setDate(day.getDate() + days);
  return day.getTime();
}

/** @param {number} hour 0 to 23 @returns {Daypart} */
function daypartOf(hour) {
  if (hour >= 5 && hour < 12) return "morning";
  if (hour >= 12 && hour < 17) return "afternoon";
  if (hour >= 17 && hour < 22) return "evening";
  return "night";
}

/** The key with the biggest number. @template K @param {Map<K, number>} counts */
function biggest(counts) {
  /** @type {K|undefined} */
  let best;
  let most = -1;
  for (const [key, count] of counts) {
    if (count > most) {
      best = key;
      most = count;
    }
  }
  return best;
}

/**
 * @param {Play[]} plays every play, oldest first
 * @param {number} year
 * @param {number} [now]
 */
export function recapOf(plays, year, now = Date.now()) {
  const from = new Date(year, 0, 1).getTime();
  const to = new Date(year + 1, 0, 1).getTime();
  const inYear = playsBetween(plays, from, to);
  const sum = totals(inYear);

  // A year still going is held up against the same stretch of the one before.
  const lastYearSameDay = new Date(Math.min(now, to));
  lastYearSameDay.setFullYear(year - 1);
  const before = totals(playsBetween(plays, new Date(year - 1, 0, 1).getTime(), Math.min(from, lastYearSameDay.getTime())));

  const months = new Array(12).fill(0);
  const hours = new Array(24).fill(0);
  const weekdays = new Array(7).fill(0);
  /** @type {Map<Daypart, number>} */
  const dayparts = new Map();
  /** Time heard each day. @type {Map<number, number>} */
  const days = new Map();
  /** Plays of each song each day, by "day uri". @type {Map<string, number>} */
  const songDays = new Map();
  for (const play of inYear) {
    const at = new Date(play.t);
    months[at.getMonth()] += play.ms;
    hours[at.getHours()] += play.ms;
    weekdays[at.getDay()] += play.ms;
    const part = daypartOf(at.getHours());
    dayparts.set(part, (dayparts.get(part) ?? 0) + play.ms);
    const day = dayOf(play.t);
    days.set(day, (days.get(day) ?? 0) + play.ms);
    const key = `${day} ${play.track.uri}`;
    songDays.set(key, (songDays.get(key) ?? 0) + 1);
  }

  // The day with the most music, and who was on most.
  const biggestDayAt = biggest(days);
  let biggestDay = null;
  if (biggestDayAt !== undefined) {
    const dayPlays = playsBetween(inYear, biggestDayAt, addDays(biggestDayAt, 1));
    biggestDay = { day: biggestDayAt, ms: days.get(biggestDayAt) ?? 0, plays: dayPlays.length, artist: topArtists(dayPlays, 1)[0]?.name ?? "" };
  }

  // The most times one song was played in a day.
  const repeatKey = biggest(songDays);
  let onRepeat = null;
  if (repeatKey !== undefined && (songDays.get(repeatKey) ?? 0) >= ON_REPEAT_MIN) {
    const [day, uri] = [Number(repeatKey.slice(0, repeatKey.indexOf(" "))), repeatKey.slice(repeatKey.indexOf(" ") + 1)];
    const track = inYear.find((play) => play.track.uri === uri)?.track;
    if (track) onRepeat = { day, track, plays: songDays.get(repeatKey) ?? 0 };
  }

  // What was heard for the first time ever this year.
  /** @type {Map<string, number>} */
  const songFirst = new Map();
  /** By name, as the stats count artists. @type {Map<string, number>} */
  const artistFirst = new Map();
  for (const play of plays) {
    if (play.t >= to) break;
    if (!songFirst.has(play.track.uri)) songFirst.set(play.track.uri, play.t);
    for (const artist of play.track.artists) {
      const key = artist.name.toLowerCase();
      if (!artistFirst.has(key)) artistFirst.set(key, play.t);
    }
  }
  let newSongs = 0;
  for (const at of songFirst.values()) if (at >= from) newSongs++;
  let newArtists = 0;
  for (const at of artistFirst.values()) if (at >= from) newArtists++;

  // The longest run of days in a row, within the year.
  const sortedDays = [...days.keys()].sort((a, b) => a - b);
  let streak = { days: 0, from: 0, to: 0 };
  let runStart = 0;
  let run = 0;
  for (let i = 0; i < sortedDays.length; i++) {
    const day = sortedDays[i];
    if (run > 0 && addDays(sortedDays[i - 1], 1) === day) run++;
    else {
      run = 1;
      runStart = day;
    }
    if (run > streak.days) streak = { days: run, from: runStart, to: day };
  }

  const daypart = biggest(dayparts) ?? "evening";
  // Imported plays no song was found for count above, but can't be shown or played.
  const playable = inYear.filter((play) => !play.track.uri.startsWith("spotilarp:unmatched:"));
  const artists = topArtists(inYear, 10).map((artist) => ({ ...artist, firstHeard: artistFirst.get(artist.name.toLowerCase()) ?? from }));

  return {
    year,
    from,
    to,
    ms: sum.ms,
    plays: sum.plays,
    skipped: sum.skipped,
    songCount: sum.songs,
    artistCount: sum.artists,
    previousMs: before.ms,
    months,
    hours,
    peakHour: hours.indexOf(Math.max(...hours)),
    /** Sunday first, as `Date.getDay()` counts. */
    peakWeekday: weekdays.indexOf(Math.max(...weekdays)),
    daypart,
    daypartShare: sum.ms > 0 ? (dayparts.get(daypart) ?? 0) / sum.ms : 0,
    firstPlay: playable[0] ?? null,
    songs: topSongs(playable, RECAP_PLAYLIST_SIZE),
    artists,
    album: topAlbums(inYear, 1)[0] ?? null,
    biggestDay,
    onRepeat,
    newSongs,
    newArtists,
    streak,
  };
}

/** @typedef {ReturnType<typeof recapOf>} Recap */

/**
 * The year a Recap opens on: the one picked on the Stats page, else this
 * year, or last year early on before this one has much in it.
 * @param {Play[]} plays every play, oldest first
 * @param {number|null} picked
 * @param {number} [now]
 */
export function recapYear(plays, picked, now = Date.now()) {
  if (picked) return picked;
  const thisYear = new Date(now).getFullYear();
  const start = new Date(thisYear, 0, 1).getTime();
  if (playsBetween(plays, start, now).length >= RECAP_MIN_PLAYS) return thisYear;
  const lastYear = playsBetween(plays, new Date(thisYear - 1, 0, 1).getTime(), start).length;
  return lastYear >= RECAP_MIN_PLAYS ? thisYear - 1 : thisYear;
}
