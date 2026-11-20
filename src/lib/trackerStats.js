/**
 * Stats from the listening tracker's plays (tracker.svelte.js), for the Stats
 * page. Plain functions over the plays, oldest first.
 *
 * @typedef {import("./tracker.svelte.js").Play} Play
 * @typedef {import("./types.js").TrackerTrack} TrackerTrack
 * @typedef {"7d"|"4w"|"6m"|"year"|"all"|`y${number}`} RangeKey
 * @typedef {"day"|"week"|"month"} Bucket
 * @typedef {{ key: RangeKey, label: string, from: number, to: number, previous: { from: number, to: number }|null, bucket: Bucket }} Range
 */

/** @param {number} at */
function startOfDay(at) {
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

/** @param {number} at @param {number} months */
function addMonths(at, months) {
  const day = new Date(at);
  day.setMonth(day.getMonth() + months);
  return day.getTime();
}

/** @param {number} year */
function startOfYear(year) {
  return new Date(year, 0, 1).getTime();
}

/** The years with plays in them, newest first. @param {Play[]} plays */
export function yearsPlayed(plays) {
  if (plays.length === 0) return [];
  const first = new Date(plays[0].t).getFullYear();
  const last = new Date(plays[plays.length - 1].t).getFullYear();
  return Array.from({ length: last - first + 1 }, (_, i) => last - i);
}

/**
 * @param {RangeKey} key
 * @param {Play[]} plays
 * @param {number} [now]
 * @returns {Range}
 */
export function rangeOf(key, plays, now = Date.now()) {
  const today = startOfDay(now);
  const thisYear = new Date(now).getFullYear();
  switch (key) {
    case "7d": {
      const from = addDays(today, -6);
      return { key, label: "the last 7 days", from, to: now, previous: { from: addDays(from, -7), to: from }, bucket: "day" };
    }
    case "4w": {
      const from = addDays(today, -27);
      return { key, label: "the last 4 weeks", from, to: now, previous: { from: addDays(from, -28), to: from }, bucket: "day" };
    }
    case "6m": {
      const from = addMonths(today, -6);
      return { key, label: "the last 6 months", from, to: now, previous: { from: addMonths(from, -6), to: from }, bucket: "week" };
    }
    case "year": {
      const from = startOfYear(thisYear);
      // The same stretch of last year, so January isn't compared with a whole year.
      const lastYear = new Date(now);
      lastYear.setFullYear(thisYear - 1);
      return { key, label: `${thisYear} so far`, from, to: now, previous: { from: startOfYear(thisYear - 1), to: lastYear.getTime() }, bucket: "month" };
    }
    case "all": {
      const from = plays.length ? startOfDay(plays[0].t) : today;
      return { key, label: "all time", from, to: now, previous: null, bucket: "month" };
    }
    default: {
      const year = Number(key.slice(1));
      return {
        key,
        label: String(year),
        from: startOfYear(year),
        to: startOfYear(year + 1),
        previous: { from: startOfYear(year - 1), to: startOfYear(year) },
        bucket: "month",
      };
    }
  }
}

/** The first play at or after `at`. @param {Play[]} plays @param {number} at */
function firstIndexFrom(plays, at) {
  let low = 0;
  let high = plays.length;
  while (low < high) {
    const mid = (low + high) >> 1;
    if (plays[mid].t < at) low = mid + 1;
    else high = mid;
  }
  return low;
}

/** The plays from `from` up to `to`. @param {Play[]} plays @param {number} from @param {number} to */
export function playsBetween(plays, from, to) {
  return plays.slice(firstIndexFrom(plays, from), firstIndexFrom(plays, to));
}

/** @param {Play[]} plays */
export function totals(plays) {
  const songs = new Set();
  const artists = new Set();
  let ms = 0;
  let skipped = 0;
  for (const play of plays) {
    ms += play.ms;
    if (play.skip) skipped++;
    songs.add(play.track.uri);
    for (const artist of play.track.artists) artists.add(artist.name.toLowerCase());
  }
  return { ms, plays: plays.length, songs: songs.size, artists: artists.size, skipped };
}

/**
 * Most played songs, by plays then time heard.
 * @param {Play[]} plays
 * @param {number} limit
 */
export function topSongs(plays, limit) {
  /** @type {Map<string, { track: TrackerTrack, plays: number, ms: number }>} */
  const songs = new Map();
  for (const play of plays) {
    const song = songs.get(play.track.uri) ?? { track: play.track, plays: 0, ms: 0 };
    song.plays++;
    song.ms += play.ms;
    songs.set(play.track.uri, song);
  }
  return [...songs.values()].sort((a, b) => b.plays - a.plays || b.ms - a.ms).slice(0, limit);
}

/**
 * Most played artists. Every artist on a song gets its play; they go by name,
 * since imported history only names them. `track` is their most played song.
 * @param {Play[]} plays
 * @param {number} limit
 */
export function topArtists(plays, limit) {
  /** @type {Map<string, { id: string, name: string, plays: number, ms: number, songs: Map<string, { track: TrackerTrack, plays: number }> }>} */
  const artists = new Map();
  for (const play of plays) {
    for (const credit of play.track.artists) {
      const key = credit.name.toLowerCase();
      const artist = artists.get(key) ?? { id: "", name: credit.name, plays: 0, ms: 0, songs: new Map() };
      if (!artist.id && credit.id) artist.id = credit.id;
      artist.plays++;
      artist.ms += play.ms;
      const song = artist.songs.get(play.track.uri) ?? { track: play.track, plays: 0 };
      song.plays++;
      artist.songs.set(play.track.uri, song);
      artists.set(key, artist);
    }
  }
  return [...artists.values()]
    .sort((a, b) => b.plays - a.plays || b.ms - a.ms)
    .slice(0, limit)
    .map(({ songs, ...artist }) => ({ ...artist, track: [...songs.values()].sort((a, b) => b.plays - a.plays)[0].track }));
}

/**
 * Most played albums, by the album's link, or its name and artist when there's none.
 * @param {Play[]} plays
 * @param {number} limit
 */
export function topAlbums(plays, limit) {
  /** @type {Map<string, { id: string, name: string, artist: string, image: string|null, plays: number, ms: number, track: TrackerTrack }>} */
  const albums = new Map();
  for (const play of plays) {
    const { album, artists } = play.track;
    if (!album.name) continue;
    const artist = artists[0]?.name ?? "";
    const key = album.id || `${album.name.toLowerCase()}\u001f${artist.toLowerCase()}`;
    // `track` is one of its songs, for looking up a cover it came without.
    const entry = albums.get(key) ?? { id: album.id, name: album.name, artist, image: album.image ?? null, plays: 0, ms: 0, track: play.track };
    entry.image ??= album.image ?? null;
    entry.plays++;
    entry.ms += play.ms;
    albums.set(key, entry);
  }
  return [...albums.values()].sort((a, b) => b.plays - a.plays || b.ms - a.ms).slice(0, limit);
}

/**
 * Time heard in each stretch of the range (a day, a week or a month).
 * @param {Play[]} plays in the range
 * @param {Range} range
 */
export function overTime(plays, range) {
  /** @type {{ start: number, end: number, ms: number, plays: number }[]} */
  const buckets = [];
  let start = range.bucket === "month" ? new Date(new Date(range.from).getFullYear(), new Date(range.from).getMonth(), 1).getTime() : range.from;
  while (start < range.to) {
    const end = range.bucket === "day" ? addDays(start, 1) : range.bucket === "week" ? addDays(start, 7) : addMonths(start, 1);
    buckets.push({ start, end, ms: 0, plays: 0 });
    start = end;
  }
  let index = 0;
  for (const play of plays) {
    while (index < buckets.length - 1 && play.t >= buckets[index].end) index++;
    buckets[index].ms += play.ms;
    buckets[index].plays++;
  }
  return buckets;
}

/**
 * Time heard by weekday (Monday first) and hour, on this computer's clock.
 * @param {Play[]} plays
 */
export function byHour(plays) {
  const grid = Array.from({ length: 7 }, () => new Array(24).fill(0));
  for (const play of plays) {
    const at = new Date(play.t);
    grid[(at.getDay() + 6) % 7][at.getHours()] += play.ms;
  }
  return grid;
}

/**
 * Days in a row with something played: the run going on now (counting
 * yesterday's, so it isn't lost before today's first song) and the longest.
 * @param {Play[]} plays all of them
 * @param {number} [now]
 */
export function streaks(plays, now = Date.now()) {
  const days = new Set(plays.map((play) => startOfDay(play.t)));
  let longest = 0;
  let run = 0;
  let previous = 0;
  for (const day of [...days].sort((a, b) => a - b)) {
    run = previous && addDays(previous, 1) === day ? run + 1 : 1;
    longest = Math.max(longest, run);
    previous = day;
  }
  const today = startOfDay(now);
  let current = 0;
  let day = days.has(today) ? today : addDays(today, -1);
  while (days.has(day)) {
    current++;
    day = addDays(day, -1);
  }
  return { current, longest };
}

/** When each song was first heard. @param {Play[]} plays all of them, oldest first */
export function firstHeard(plays) {
  /** @type {Map<string, number>} */
  const first = new Map();
  for (const play of plays) if (!first.has(play.track.uri)) first.set(play.track.uri, play.t);
  return first;
}

/** How many plays came from each source. @param {Play[]} plays */
export function bySource(plays) {
  const counts = [0, 0, 0];
  for (const play of plays) counts[play.src]++;
  return counts;
}
