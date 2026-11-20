import { untrack } from "svelte";
import { listen } from "@tauri-apps/api/event";
import * as api from "./api.js";
import { notify } from "./toast.svelte.js";
import { player, currentPositionMs, togglePlay, previousOrRestart, seekTo } from "./player.svelte.js";
import { queue, slim, next, addManyToQueue, upcomingContext, setJamGuest } from "./queue.svelte.js";
import { episodeTrack } from "./podcasts.svelte.js";
import { createPlaylist, adjustTrackCount } from "./library.svelte.js";
import { navigate } from "./nav.svelte.js";
import { connect } from "./connect.svelte.js";

/**
 * Jams: listening together. The host's app plays as usual and tells the
 * others what's on, where it's got to and what's coming up; theirs play the
 * same songs (each with their own Premium) and send their requests back.
 * jam.rs carries the messages between the apps.
 *
 * Host to listeners: `state` (everything a listener shows and plays),
 * `welcome` (which member you are) and `notice` (a line for one listener).
 * Listeners to host: `hello`, `add` (song URIs, looked up again by the host)
 * and `control`.
 *
 * Two ways in: the Jam code the host hands out ("<id>.<token>") makes you a
 * member who can add songs; the bare id, which the "Listen along" button on
 * the host's Discord status carries, only lets you listen until the host
 * invites you. While Discord shows your status, a session stays open for that
 * button even without a Jam started.
 */

const NAME_KEY = "spotilarp.jamName";
/** What jam.rs calls the host, on a listener's side. */
const HOST = "host";
const MAX_NAME = 32;
const UP_NEXT_LIMIT = 50;
/** Songs a listener can ask for at once. */
const MAX_ADD = 50;
/** Requests waiting for the host, at most. */
const MAX_REQUESTS = 50;
const MAX_PLAYED = 1000;
const HEARTBEAT_MS = 5000;
const BROADCAST_DELAY_MS = 100;
/** How far a listener may drift from the host before being moved back in step. */
const DRIFT_MS = 2500;
/** Near a song's end a listener whose copy finished waits for the next song. */
const END_MARGIN_MS = 3000;
const LOAD_TIMEOUT_MS = 15_000;
/** How long someone who connected has to say hello. */
const HELLO_TIMEOUT_MS = 10_000;
/** How often to try opening the listen-along session again after it couldn't. */
const SHARING_RETRY_MS = 60_000;
const TOKEN_BYTES = 12;
/** Only Spotify songs and episodes are shared; a local file is on one computer. */
const SHAREABLE_URI = /^spotify:(track|episode):[A-Za-z0-9]{22}$/;
const SPOTIFY_ID = /^[A-Za-z0-9]{22}$/;
/** Where covers another app names may come from: Spotify's image servers. */
const COVER_HOST = /(^|\.)(scdn\.co|spotifycdn\.com)$/;

/**
 * @typedef {import("./types.js").Track} Track
 * @typedef {{ id: string, name: string, host: boolean, canAdd: boolean }} JamMember canAdd: invited into the Jam, not just listening
 * @typedef {{ track: Track, by: string }} JamEntry a song coming up, and who asked for it
 * @typedef {{ id: number, peer: string, name: string, tracks: Track[] }} JamRequest
 * @typedef {{ queue: boolean, control: boolean, approval: boolean }} JamPermissions
 */

/**
 * `played` outlives the Jam, so it can still be saved once it's over.
 * @type {{ role: "host"|"guest"|null, started: boolean, busy: boolean, code: string, link: string, hostName: string, myId: string, members: JamMember[], permissions: JamPermissions, requests: JamRequest[], prompts: { peer: string, name: string }[], nowPlaying: Track|null, upNext: JamEntry[], played: Track[], saving: boolean }}
 */
export const jam = $state({
  role: null,
  // Host: a Jam was started, rather than just the listen-along session.
  started: false,
  busy: false,
  // Host: the code that lets people add songs; guest: the code joined with.
  code: "",
  // Host: the listen-only link the Discord button opens.
  link: "",
  hostName: "",
  // Which member you are.
  myId: "",
  members: [],
  permissions: defaultPermissions(),
  // Host: songs listeners asked for, waiting to be accepted.
  requests: [],
  // Host: people listening along from Discord, waiting for "make it a Jam?".
  prompts: [],
  // Listener: what the host is playing and has coming up.
  nowPlaying: null,
  upNext: [],
  played: [],
  saving: false,
});

function defaultPermissions() {
  return { queue: true, control: false, approval: false };
}

// A Jam left behind by a reloaded page would block starting a new one.
api.jamLeave().catch(() => {});

/** Your Spotify name, for when you haven't picked one for Jams. */
let defaultName = "";
/** Whether a listen-along session should be open (Discord is showing your status). */
let wantSharing = false;

/** @param {string} name */
export function setDefaultName(name) {
  defaultName = name;
}

function myName() {
  return savedName() || defaultName;
}

/**
 * The Discord status is on (or off): keep a session open for its "Listen
 * along" button.
 * @param {boolean} on
 */
export function setDiscordSharing(on) {
  wantSharing = on;
  if (on) ensureSharing();
  else if (sharingOnly() && listeners().length === 0) leaveSession();
}

/** Hosting just for the Discord button, no Jam started. */
function sharingOnly() {
  return jam.role === "host" && !jam.started;
}

function listeners() {
  return jam.members.filter((m) => !m.host);
}

async function ensureSharing() {
  if (!wantSharing || jam.role || jam.busy) return;
  jam.busy = true;
  try {
    await host(cleanName(myName(), "Host"));
  } catch {
    // Offline, say: tried again in a while.
  } finally {
    jam.busy = false;
  }
}

setInterval(ensureSharing, SHARING_RETRY_MS);

/**
 * A code as shared: "<id>.<token>", a bare id, or a listen-along link.
 * @param {string} code
 */
function splitCode(code) {
  let trimmed = code.trim().replace(/^["'`]+|["'`]+$/g, "");
  const hash = trimmed.lastIndexOf("#");
  // A listen-along link has the song after the code: "#<code>/track/<id>".
  if (hash >= 0) trimmed = trimmed.slice(hash + 1).split("/")[0];
  trimmed = trimmed.replace(/^spotilarp:\/\/listen\//, "").replace(/\/+$/, "");
  const dot = trimmed.lastIndexOf(".");
  return dot < 0 ? { id: trimmed, token: "" } : { id: trimmed.slice(0, dot), token: trimmed.slice(dot + 1) };
}

function newToken() {
  const bytes = crypto.getRandomValues(new Uint8Array(TOKEN_BYTES));
  return btoa(String.fromCharCode(...bytes)).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

export function savedName() {
  try {
    return localStorage.getItem(NAME_KEY) ?? "";
  } catch {
    return "";
  }
}

/** @param {unknown} raw */
function cleanName(raw, fallback = "Listener") {
  const name = typeof raw === "string" ? raw.replace(/\s+/g, " ").trim().slice(0, MAX_NAME) : "";
  return name || fallback;
}

/**
 * @param {unknown} value
 * @param {number} [max]
 */
function text(value, max = 200) {
  return typeof value === "string" ? value.slice(0, max) : "";
}

/** @param {unknown} value */
function spotifyId(value) {
  return typeof value === "string" && SPOTIFY_ID.test(value) ? value : "";
}

/** @param {unknown} url */
function coverUrl(url) {
  try {
    const parsed = new URL(/** @type {string} */ (url));
    return parsed.protocol === "https:" && COVER_HOST.test(parsed.hostname) ? parsed.href : null;
  } catch {
    return null;
  }
}

/**
 * A song as another app described it, if it looks like one: just what's
 * needed to show and play it.
 * @param {any} raw
 * @returns {Track|null}
 */
function cleanTrack(raw) {
  if (!raw || typeof raw !== "object" || typeof raw.uri !== "string" || !SHAREABLE_URI.test(raw.uri)) return null;
  const cover = coverUrl(raw.album?.images?.[0]?.url);
  /** @type {Track} */
  const track = {
    id: raw.uri.split(":")[2],
    name: text(raw.name) || "Unknown",
    uri: raw.uri,
    duration_ms: Number.isFinite(raw.duration_ms) ? Math.max(0, raw.duration_ms) : 0,
    artists: Array.isArray(raw.artists)
      ? raw.artists.slice(0, 10).map((/** @type {any} */ a) => ({ id: spotifyId(a?.id), name: text(a?.name) }))
      : [],
    album: {
      id: spotifyId(raw.album?.id),
      name: text(raw.album?.name),
      images: cover ? [{ url: cover }] : [],
    },
  };
  if (raw.show && typeof raw.show === "object") track.show = { id: spotifyId(raw.show.id), name: text(raw.show.name) };
  return track;
}

/** @param {Track[]} tracks */
function describe(tracks) {
  return tracks.length === 1 ? `"${tracks[0].name}"` : `${tracks.length} songs`;
}

/**
 * @param {string|null} peer
 * @param {object} message
 */
function send(peer, message) {
  api.jamSend(peer, message).catch(() => {});
}

/**
 * A line for one listener's screen.
 * @param {string} peer
 * @param {string} line
 */
function tell(peer, line) {
  send(peer, { type: "notice", text: line });
}

/** @param {Track} track */
function addPlayed(track) {
  if (jam.played.length >= MAX_PLAYED || jam.played.some((t) => t.uri === track.uri)) return;
  jam.played.push(track);
}

// ---- Hosting ----

/** The song listeners are told is on: taken when it starts, so a new song isn't sent with the old one's place. @type {Track|null} */
let sharedTrack = null;
let requestId = 0;
/** Listeners the host sent away, kept out for the rest of the Jam. */
let removed = new Set();
/** The secret in the Jam code that makes someone a member rather than a listener. */
let inviteToken = "";
/** This app's own code while hosting. */
let hostedId = "";
/** @type {Map<string, ReturnType<typeof setTimeout>>} */
const helloTimers = new Map();
/** @type {ReturnType<typeof setTimeout>|undefined} */
let broadcastHandle;
/** @type {ReturnType<typeof setInterval>|undefined} */
let heartbeat;

/** @param {string} name what listeners see you as */
export async function startJam(name) {
  if (jam.role === "guest" || jam.busy) return;
  const me = cleanName(name, "Host");
  rememberName(me);
  // The listen-along session becomes the Jam: whoever's listening stays.
  if (jam.role === "host") {
    jam.started = true;
    jam.hostName = me;
    jam.members[0].name = me;
    jam.played = sharedTrack ? [sharedTrack] : [];
    return;
  }
  jam.busy = true;
  try {
    await host(me);
    jam.started = true;
  } catch (e) {
    notify(`${e}`);
  } finally {
    jam.busy = false;
  }
}

/** @param {string} me */
async function host(me) {
  const hosted = await api.jamHost();
  hostedId = hosted.code;
  removed = new Set();
  inviteToken = newToken();
  sharedTrack = queue.current && player.track ? shareable(queue.current) : null;
  Object.assign(jam, {
    role: "host",
    started: false,
    code: `${hosted.code}.${inviteToken}`,
    link: hosted.link,
    hostName: me,
    myId: HOST,
    members: [{ id: HOST, name: me, host: true, canAdd: true }],
    permissions: defaultPermissions(),
    requests: [],
    prompts: [],
    nowPlaying: null,
    upNext: [],
    played: sharedTrack ? [sharedTrack] : [],
  });
  heartbeat = setInterval(broadcast, HEARTBEAT_MS);
}

/** @param {Track} track */
function shareable(track) {
  return SHAREABLE_URI.test(track.uri) ? slim(track) : null;
}

function hostUpNext() {
  if (queue.repeatMode === "one") return [];
  const coming = [...queue.manual.slice(0, UP_NEXT_LIMIT), ...upcomingContext(UP_NEXT_LIMIT).upcoming.map(({ track }) => track)];
  return coming
    .filter((track) => SHAREABLE_URI.test(track.uri))
    .slice(0, UP_NEXT_LIMIT)
    .map((track) => ({ track: slim(track), by: track.addedBy ?? "" }));
}

function hostState() {
  return {
    type: "state",
    track: sharedTrack,
    playing: !!sharedTrack && player.isPlaying,
    positionMs: sharedTrack ? Math.round(currentPositionMs()) : 0,
    upNext: hostUpNext(),
    members: $state.snapshot(jam.members),
    permissions: $state.snapshot(jam.permissions),
  };
}

function broadcast() {
  // Nobody to tell until someone has said hello.
  if (jam.role !== "host" || jam.members.length < 2) return;
  send(null, hostState());
}

function broadcastSoon() {
  if (jam.role !== "host") return;
  clearTimeout(broadcastHandle);
  broadcastHandle = setTimeout(broadcast, BROADCAST_DELAY_MS);
}

// What's coming up, who's here and what they may do go out as they change.
$effect.root(() => {
  $effect(() => {
    if (jam.role !== "host") return;
    JSON.stringify([hostUpNext().map(({ track, by }) => [track.uri, by]), jam.members, jam.permissions]);
    untrack(broadcastSoon);
  });
});

/** @param {import("./types.js").PlaybackEvent} e */
function onHostPlayback(e) {
  if (e.type === "TrackChanged") {
    // Another Spotify app playing on SpotiLarp isn't the host's queue to share.
    sharedTrack = queue.current && connect.output !== "receiving" ? shareable(queue.current) : null;
    if (sharedTrack) addPlayed(sharedTrack);
    broadcastSoon();
  } else if (["Playing", "Paused", "Seeked", "Stopped"].includes(e.type)) {
    broadcastSoon();
  }
}

/** @param {{ type: string, peer: string, reason?: string, message?: any }} e */
function onHostJamEvent(e) {
  if (e.type === "joined") {
    if (removed.has(e.peer)) {
      api.jamRemove(e.peer).catch(() => {});
      return;
    }
    helloTimers.set(e.peer, setTimeout(() => api.jamRemove(e.peer).catch(() => {}), HELLO_TIMEOUT_MS));
  } else if (e.type === "left") {
    clearTimeout(helloTimers.get(e.peer));
    helloTimers.delete(e.peer);
    const member = jam.members.find((m) => m.id === e.peer);
    if (!member) return;
    jam.members = jam.members.filter((m) => m.id !== e.peer);
    jam.requests = jam.requests.filter((r) => r.peer !== e.peer);
    dismissPrompt(e.peer);
    notify(member.canAdd ? `${member.name} left the Jam` : `${member.name} stopped listening along`);
    // Discord was turned off while they listened: nobody left to keep it open for.
    if (!wantSharing && sharingOnly() && listeners().length === 0) leaveSession();
  } else if (e.type === "message" && e.message && typeof e.message === "object") {
    onListenerMessage(e.peer, e.message);
  }
}

/**
 * @param {string} peer
 * @param {any} message
 */
function onListenerMessage(peer, message) {
  const member = jam.members.find((m) => m.id === peer);
  // Sent away, though the hello was already on its way.
  if (removed.has(peer)) return;
  if (message.type === "hello") {
    clearTimeout(helloTimers.get(peer));
    helloTimers.delete(peer);
    // Back on a new connection before the old one timed out.
    if (member) {
      send(peer, { type: "welcome", id: peer });
      return broadcastSoon();
    }
    const name = uniqueName(cleanName(message.name));
    const invited = typeof message.token === "string" && message.token === inviteToken;
    jam.members.push({ id: peer, name, host: false, canAdd: invited });
    send(peer, { type: "welcome", id: peer });
    if (invited) {
      notify(`${name} joined the Jam`);
    } else {
      // Listening from Discord: they hear it straight away, and you choose whether they can join in.
      jam.prompts.push({ peer, name });
      api.jamNotify(`${name} is listening along with you`).catch(() => {});
    }
  } else if (!member) {
    return;
  } else if (message.type === "add") {
    addFromListener(member, message.uris);
  } else if (message.type === "control") {
    controlFromListener(member, message.action, message.positionMs);
  }
}

/** @param {string} name */
function uniqueName(name) {
  const taken = new Set(jam.members.map((m) => m.name));
  let unique = name;
  for (let n = 2; taken.has(unique); n++) unique = `${name} (${n})`;
  return unique;
}

/**
 * @param {JamMember} member
 * @param {unknown} uris
 */
async function addFromListener(member, uris) {
  if (!member.canAdd) return tell(member.id, `You're listening along. Ask ${jam.hostName} to invite you to add songs`);
  if (!jam.permissions.queue) return tell(member.id, "The host has turned off adding songs");
  if (!Array.isArray(uris)) return;
  const wanted = uris.filter((uri) => typeof uri === "string" && SHAREABLE_URI.test(uri)).slice(0, MAX_ADD);
  if (wanted.length === 0) return;
  if (jam.permissions.approval && jam.requests.length >= MAX_REQUESTS) {
    return tell(member.id, "The host has a lot of songs to go through. Try again in a bit");
  }
  let tracks;
  try {
    tracks = await lookUp(wanted);
  } catch (e) {
    return tell(member.id, `The host's app couldn't find that: ${e}`);
  }
  // Gone while the songs were looked up.
  if (jam.role !== "host" || !jam.members.some((m) => m.id === member.id) || tracks.length === 0) return;
  const named = tracks.map((track) => ({ ...track, addedBy: member.name }));
  const what = describe(named);
  if (jam.permissions.approval) {
    jam.requests.push({ id: ++requestId, peer: member.id, name: member.name, tracks: named });
    notify(`${member.name} wants to add ${what}`);
    tell(member.id, `Asked the host to add ${what}`);
  } else {
    addManyToQueue(named, false);
    notify(`${member.name} added ${what}`);
    tell(member.id, `Added ${what} to the Jam`);
  }
}

/**
 * The host's own copy of each song, rather than trusting a listener's description of it.
 * @param {string[]} uris
 * @returns {Promise<Track[]>}
 */
async function lookUp(uris) {
  const songs = uris.filter((uri) => uri.startsWith("spotify:track:"));
  const episodeIds = uris.filter((uri) => uri.startsWith("spotify:episode:")).map((uri) => uri.split(":")[2]);
  const [tracks, episodes] = await Promise.all([
    songs.length > 0 ? api.getTracksByUri(songs) : [],
    Promise.all(episodeIds.map((id) => api.getEpisode(id).then(episodeTrack, () => null))),
  ]);
  return [...tracks, ...episodes.filter((episode) => episode !== null)];
}

/**
 * @param {JamMember} member
 * @param {unknown} action
 * @param {unknown} positionMs
 */
function controlFromListener(member, action, positionMs) {
  if (!member.canAdd || !jam.permissions.control) return tell(member.id, "Only the host can control playback here");
  if (action === "pause" && player.isPlaying) {
    togglePlay();
    notify(`${member.name} paused`);
  } else if (action === "resume" && !player.isPlaying) {
    togglePlay();
    notify(`${member.name} pressed play`);
  } else if (action === "next") {
    next();
    notify(`${member.name} skipped the song`);
  } else if (action === "previous") {
    previousOrRestart();
  } else if (action === "seek" && typeof positionMs === "number" && Number.isFinite(positionMs) && player.track) {
    seekTo(Math.min(Math.max(positionMs, 0), player.track.durationMs));
  }
}

/** @param {number} id */
export function acceptRequest(id) {
  const request = jam.requests.find((r) => r.id === id);
  if (!request) return;
  jam.requests = jam.requests.filter((r) => r.id !== id);
  addManyToQueue($state.snapshot(request.tracks));
  tell(request.peer, `The host added ${describe(request.tracks)}`);
}

/** @param {number} id */
export function declineRequest(id) {
  const request = jam.requests.find((r) => r.id === id);
  if (!request) return;
  jam.requests = jam.requests.filter((r) => r.id !== id);
  tell(request.peer, `The host passed on ${describe(request.tracks)}`);
}

/**
 * @param {keyof JamPermissions} key
 * @param {boolean} on
 */
export function setPermission(key, on) {
  if (jam.role === "host") jam.permissions[key] = on;
}

/** @param {string} peer */
export function removeListener(peer) {
  const member = jam.members.find((m) => m.id === peer && !m.host);
  if (jam.role !== "host" || !member) return;
  removed.add(peer);
  jam.members = jam.members.filter((m) => m.id !== peer);
  jam.requests = jam.requests.filter((r) => r.peer !== peer);
  dismissPrompt(peer);
  api.jamRemove(peer).catch(() => {});
  notify(`Removed ${member.name}`);
}

/**
 * "Make it a Jam": someone listening along can add songs now (and pause and
 * skip, if the Jam allows that).
 * @param {string} peer
 */
export function inviteListener(peer) {
  const member = jam.members.find((m) => m.id === peer && !m.host);
  dismissPrompt(peer);
  if (jam.role !== "host" || !member || member.canAdd) return;
  member.canAdd = true;
  jam.started = true;
  tell(peer, `${jam.hostName} invited you into the Jam. You can add songs now`);
}

/** @param {string} peer */
export function dismissPrompt(peer) {
  jam.prompts = jam.prompts.filter((p) => p.peer !== peer);
}

// ---- Listening along ----

/** The host's last word on what's playing, and when it came. @type {{ track: Track|null, playing: boolean, positionMs: number }|null} */
let hostNow = null;
let hostNowAt = 0;
/** The host's song this app last started, and when, until it plays or fails. */
let loadedUri = "";
let loadingSince = 0;
/** A host song that won't play here (not in this country, say). */
let failedUri = "";
/** Paused on this computer only, while the host's music goes on. */
let pausedHere = false;

/**
 * Someone's "Listen along" button on Discord.
 * @param {string} code
 */
export function listenAlong(code) {
  joinJam(code, myName());
}

/**
 * @param {string} code
 * @param {string} name what the others see you as
 */
export async function joinJam(code, name) {
  if (jam.busy || !code.trim()) return;
  if (jam.role === "host" && !sharingOnly()) return notify("End your Jam first to join someone else's");
  if (jam.role === "host" && listeners().length > 0) {
    return notify("People are listening along with you. Remove them first to join someone else");
  }
  const { id, token } = splitCode(code);
  if (jam.role === "host" && id === hostedId) return notify("That's your own listen-along link");
  const me = cleanName(name);
  if (name.trim()) rememberName(me);
  jam.busy = true;
  try {
    // Only one session at a time: the listen-along one, or another Jam.
    if (jam.role) await leaveSession();
    await api.jamJoin(id);
    hostNow = null;
    loadedUri = failedUri = "";
    loadingSince = 0;
    pausedHere = false;
    Object.assign(jam, {
      role: "guest",
      started: false,
      code: token ? `${id}.${token}` : id,
      link: "",
      hostName: "",
      myId: "",
      members: [],
      permissions: defaultPermissions(),
      requests: [],
      prompts: [],
      nowPlaying: null,
      upNext: [],
      played: [],
    });
    setJamGuest(listener);
    await api.jamSend(HOST, { type: "hello", name: me, token });
  } catch (e) {
    if (jam.role) await leaveSession();
    notify(`${e}`);
  } finally {
    jam.busy = false;
  }
  ensureSharing();
}

/** Whether the host has let you in to add songs, rather than just listen. */
export function invited() {
  return jam.members.find((m) => m.id === jam.myId)?.canAdd === true;
}

/** @type {import("./queue.svelte.js").JamGuest} */
const listener = {
  add(tracks) {
    if (!invited()) return notify(`You're listening along. Ask ${jam.hostName || "the host"} to invite you to add songs`);
    if (!jam.permissions.queue) return notify("The host has turned off adding songs");
    const uris = tracks.map((track) => track.uri).filter((uri) => SHAREABLE_URI.test(uri));
    if (uris.length === 0) return notify("Songs on your computer can't be shared in a Jam");
    if (uris.length > MAX_ADD) notify(`Only the first ${MAX_ADD} songs were sent`);
    send(HOST, { type: "add", uris: uris.slice(0, MAX_ADD) });
  },
  control(action, positionMs) {
    if (!jam.permissions.control || !invited()) {
      // Without the host's say-so, play and pause are for this computer only.
      if (action !== "toggle") return notify("Only the host can control playback here");
      if (player.isPlaying) {
        pausedHere = true;
        api.pause().catch(() => {});
      } else {
        pausedHere = false;
        follow(true);
      }
      return;
    }
    pausedHere = false;
    const wanted = action === "toggle" ? (hostNow?.playing ? "pause" : "resume") : action;
    send(HOST, { type: "control", action: wanted, positionMs: Math.round(positionMs ?? 0) });
  },
  resync() {
    follow(true);
  },
  canControl: () => jam.permissions.control && invited(),
};

/** Where the host has got to by now. */
function hostPositionMs() {
  if (!hostNow) return 0;
  const ms = hostNow.positionMs + (hostNow.playing ? performance.now() - hostNowAt : 0);
  const duration = hostNow.track?.duration_ms ?? 0;
  return duration > 0 ? Math.min(ms, duration) : ms;
}

/** @param {any} m */
function applyHostState(m) {
  const track = cleanTrack(m.track);
  const positionMs = typeof m.positionMs === "number" && Number.isFinite(m.positionMs) ? Math.max(0, m.positionMs) : 0;
  hostNow = { track, playing: m.playing === true && track !== null, positionMs };
  hostNowAt = performance.now();
  jam.nowPlaying = track;
  if (track) addPlayed(track);
  jam.upNext = (Array.isArray(m.upNext) ? m.upNext.slice(0, UP_NEXT_LIMIT) : [])
    .map((/** @type {any} */ entry) => ({ track: cleanTrack(entry?.track), by: cleanName(entry?.by, "") }))
    .filter((/** @type {any} */ entry) => entry.track !== null);
  jam.members = (Array.isArray(m.members) ? m.members.slice(0, 20) : []).map((/** @type {any} */ member) => ({
    id: text(member?.id, 80),
    name: cleanName(member?.name),
    host: member?.host === true,
    canAdd: member?.canAdd === true,
  }));
  const firstWord = !jam.hostName;
  jam.hostName = jam.members.find((member) => member.host)?.name ?? jam.hostName;
  if (firstWord && jam.hostName) notify(invited() ? `Joined ${jam.hostName}'s Jam` : `Listening along with ${jam.hostName}`);
  const permissions = m.permissions ?? {};
  jam.permissions = {
    queue: permissions.queue !== false,
    control: permissions.control === true,
    approval: permissions.approval === true,
  };
  follow();
}

/**
 * Plays what the host is playing, where they've got to.
 * @param {boolean} [force] start it again even if it's already on
 */
async function follow(force = false) {
  if (jam.role !== "guest" || !hostNow || pausedHere) return;
  const { track, playing } = hostNow;
  if (!track) {
    loadedUri = "";
    if (player.isPlaying) api.pause().catch(() => {});
    return;
  }
  const loading = loadingSince > 0 && performance.now() - loadingSince < LOAD_TIMEOUT_MS;
  const expected = hostPositionMs();
  // This app's copy finished a moment before the host's: the next song is on its way.
  const ending = track.duration_ms > 0 && expected > track.duration_ms - END_MARGIN_MS;
  const start = force || track.uri !== loadedUri || (!player.loaded && !loading && !ending && track.uri !== failedUri);
  if (start) {
    loadedUri = track.uri;
    failedUri = "";
    loadingSince = performance.now();
    try {
      await api.playTrack(track.uri, Math.round(expected), !playing);
    } catch (e) {
      loadingSince = 0;
      notify(`Couldn't play "${track.name}": ${e}`);
    }
    return;
  }
  if (loading || !player.loaded) return;
  if (playing && !player.isPlaying) api.resume().catch(() => {});
  else if (!playing && player.isPlaying) api.pause().catch(() => {});
  if (Math.abs(currentPositionMs() - expected) > DRIFT_MS) api.seek(Math.round(expected)).catch(() => {});
}

/** @param {import("./types.js").PlaybackEvent} e */
function onGuestPlayback(e) {
  if (e.type === "TrackChanged" || e.type === "Playing" || e.type === "Paused") {
    loadingSince = 0;
  } else if (e.type === "Unavailable") {
    loadingSince = 0;
    failedUri = loadedUri;
    notify(`"${hostNow?.track?.name ?? "This song"}" can't play here. You'll join back in on the next one.`);
  }
}

/** @param {{ type: string, peer: string, reason?: string, message?: any }} e */
function onGuestJamEvent(e) {
  if (e.peer !== HOST) return;
  if (e.type === "left") {
    const host = jam.hostName || "The host";
    const member = invited();
    leaveJam();
    if (e.reason === "ended") notify(member ? `${host} ended the Jam` : `${host} stopped sharing their music`);
    else if (e.reason === "removed") notify(`${host} removed you`);
    else if (e.reason === "full") notify("That Jam is full");
    else notify(member ? "Lost the connection to the Jam" : `Lost the connection to ${host}`);
  } else if (e.type === "message" && e.message && typeof e.message === "object") {
    if (e.message.type === "state") applyHostState(e.message);
    else if (e.message.type === "welcome") jam.myId = text(e.message.id, 80);
    else if (e.message.type === "notice" && typeof e.message.text === "string") notify(e.message.text);
  }
}

// ---- Either side ----

/** Leaves the Jam, or ends it for everyone when hosting. */
export async function leaveJam() {
  await leaveSession();
  ensureSharing();
}

async function leaveSession() {
  const role = jam.role;
  if (!role) return;
  clearInterval(heartbeat);
  clearTimeout(broadcastHandle);
  for (const timer of helloTimers.values()) clearTimeout(timer);
  helloTimers.clear();
  setJamGuest(null);
  hostNow = null;
  sharedTrack = null;
  Object.assign(jam, { role: null, started: false, code: "", link: "", members: [], requests: [], prompts: [], nowPlaying: null, upNext: [] });
  // A listener's music stops with the Jam; their own queue is where they left it.
  if (role === "guest" && player.isPlaying) api.pause().catch(() => {});
  await api.jamLeave().catch(() => {});
}

/** Everything played in the Jam as a new playlist. */
export async function saveJam() {
  if (jam.saving || jam.played.length === 0) return;
  jam.saving = true;
  try {
    const when = new Date().toLocaleDateString();
    const playlist = await createPlaylist(jam.hostName ? `${jam.hostName}'s Jam (${when})` : `Jam (${when})`);
    const uris = jam.played.map((track) => track.uri);
    await api.addToPlaylist(playlist.id, uris);
    adjustTrackCount(playlist.id, uris.length);
    notify(`Saved as "${playlist.name}"`);
    navigate({ type: "playlist", playlist });
  } catch (e) {
    notify(`Couldn't save the Jam: ${e}`);
  } finally {
    jam.saving = false;
  }
}

/** @param {string} name */
function rememberName(name) {
  try {
    localStorage.setItem(NAME_KEY, name);
  } catch {}
}

listen("player-event", (event) => {
  const e = /** @type {import("./types.js").PlaybackEvent} */ (event.payload);
  if (jam.role === "host") onHostPlayback(e);
  else if (jam.role === "guest") onGuestPlayback(e);
});

listen("jam-event", (event) => {
  const e = /** @type {{ type: string, peer: string, reason?: string, message?: any }} */ (event.payload);
  if (jam.role === "host") onHostJamEvent(e);
  else if (jam.role === "guest") onGuestJamEvent(e);
});
