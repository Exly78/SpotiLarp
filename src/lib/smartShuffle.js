/**
 * Smart shuffle: a random order that spreads each artist's songs across the
 * list instead of bunching them, and plays what you heard lately last.
 *
 * The spreading follows Martin Fiedler's "The art of shuffling music": each
 * artist's songs get evenly spaced slots with a random offset and a little
 * jitter, and the list is sorted by slot.
 */

const DAY_MS = 24 * 60 * 60 * 1000;
const WEEK_MS = 7 * DAY_MS;
/** How far ahead a song is looked for to break up two in a row by one artist. */
const LOOKAHEAD = 50;
/** How far a slot may move, as a share of its artist's spacing. */
const JITTER = 0.2;

/**
 * @typedef {import("./types.js").Track} Track
 * @typedef {{ first?: number, playedAt?: (track: Track) => number, now?: number, random?: () => number }} SmartOptions
 */

/**
 * The play order, as indices into `tracks`. `first` (when 0 or more) plays
 * first; the rest go in three rounds: not heard in the last week, heard this
 * week, heard today.
 * @param {Track[]} tracks
 * @param {SmartOptions} [options] `playedAt` gives when a song was last
 *   played (ms since 1970, 0 for never)
 * @returns {number[]}
 */
export function smartOrder(tracks, { first = -1, playedAt = () => 0, now = Date.now(), random = Math.random } = {}) {
  /** @type {number[][]} */
  const tiers = [[], [], []];
  tracks.forEach((track, i) => {
    if (i !== first) tiers[tier(playedAt(track) || 0, now)].push(i);
  });
  /** @type {number[]} */
  const order = first >= 0 && first < tracks.length ? [first] : [];
  /** @type {number[]} which tier each position of `order` came from */
  const tierOf = order.map(() => -1);
  tiers.forEach((indices, t) => {
    for (const i of spread(indices, (i) => artistKey(tracks[i]), random)) {
      order.push(i);
      tierOf.push(t);
    }
  });
  separate(order, tierOf, (i) => artistKey(tracks[i]));
  return order;
}

/**
 * @param {number} playedAt when the song was last played, 0 for never
 * @param {number} now
 */
function tier(playedAt, now) {
  const age = now - playedAt;
  if (!playedAt || age >= WEEK_MS) return 0;
  return age >= DAY_MS ? 1 : 2;
}

/**
 * Who a song is by, for keeping one artist's songs apart.
 * @param {Track|undefined} track
 */
export function artistKey(track) {
  const artist = track?.artists?.[0];
  return artist?.id || artist?.name?.toLowerCase() || track?.album?.name?.toLowerCase() || "";
}

/**
 * Shuffles `indices` so each group (by `keyOf`) is spread evenly through them.
 * @param {number[]} indices
 * @param {(index: number) => string} keyOf
 * @param {() => number} random
 */
function spread(indices, keyOf, random) {
  /** @type {Map<string, number[]>} */
  const groups = new Map();
  for (const i of indices) {
    const key = keyOf(i);
    const group = groups.get(key);
    if (group) group.push(i);
    else groups.set(key, [i]);
  }
  /** @type {{ index: number, slot: number }[]} */
  const slotted = [];
  for (const group of groups.values()) {
    shuffleInPlace(group, random);
    const gap = 1 / group.length;
    const offset = random() * gap;
    group.forEach((index, n) => {
      slotted.push({ index, slot: offset + n * gap + (random() - 0.5) * JITTER * gap });
    });
  }
  return slotted.sort((a, b) => a.slot - b.slot).map((s) => s.index);
}

/**
 * Where one artist still plays twice in a row, pulls a later song from the
 * same round in between, or failing that moves the second song back to a gap
 * between two other artists (the first song stays put).
 * @param {number[]} order
 * @param {number[]} tierOf
 * @param {(index: number) => string} keyOf
 */
function separate(order, tierOf, keyOf) {
  for (let i = 1; i < order.length; i++) {
    const key = keyOf(order[i]);
    if (key !== keyOf(order[i - 1])) continue;
    let moved = false;
    const end = Math.min(order.length, i + 1 + LOOKAHEAD);
    for (let j = i + 1; j < end && tierOf[j] === tierOf[i]; j++) {
      if (keyOf(order[j]) === key) continue;
      move(order, tierOf, j, i);
      moved = true;
      break;
    }
    if (moved) continue;
    const start = Math.max(1, i - LOOKAHEAD);
    for (let p = i - 1; p >= start && tierOf[p] === tierOf[i]; p--) {
      if (keyOf(order[p - 1]) !== key && keyOf(order[p]) !== key) {
        move(order, tierOf, i, p);
        break;
      }
    }
  }
}

/**
 * @param {number[]} order
 * @param {number[]} tierOf
 * @param {number} from
 * @param {number} to
 */
function move(order, tierOf, from, to) {
  const [index] = order.splice(from, 1);
  const [t] = tierOf.splice(from, 1);
  order.splice(to, 0, index);
  tierOf.splice(to, 0, t);
}

/**
 * @param {number[]} list
 * @param {() => number} random
 */
export function shuffleInPlace(list, random = Math.random) {
  for (let i = list.length - 1; i > 0; i--) {
    const j = Math.floor(random() * (i + 1));
    [list[i], list[j]] = [list[j], list[i]];
  }
  return list;
}
