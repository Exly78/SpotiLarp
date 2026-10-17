const KINDS = ["track", "album", "playlist", "artist", "show", "episode"];
const ID = /^[A-Za-z0-9]{22}$/;

/**
 * @typedef {{ kind: "track"|"album"|"playlist"|"artist"|"show"|"episode", id: string, uri: string }
 *   | { kind: "search", query: string }} SpotifyLink
 */

/**
 * Reads a `spotify:` URI or an `open.spotify.com` link (with or without
 * `intl-xx/` and `?si=`), or `null` for anything else.
 * @param {string} text
 * @returns {SpotifyLink|null}
 */
export function parseSpotifyLink(text) {
  const value = text.trim();
  if (value.startsWith("spotify:")) {
    const parts = value.split(":");
    // Old playlist URIs: spotify:user:<name>:playlist:<id>
    const [kind, id] = parts[1] === "user" && parts[3] === "playlist" ? parts.slice(3, 5) : parts.slice(1, 3);
    if (kind === "search" && parts.length > 2) return { kind: "search", query: decodeQuery(parts.slice(2).join(":")) };
    return entity(kind, id);
  }
  let url;
  try {
    url = new URL(value);
  } catch {
    return null;
  }
  if (url.hostname !== "open.spotify.com" && url.hostname !== "play.spotify.com") return null;
  const segments = url.pathname.split("/").filter(Boolean);
  if (segments[0]?.startsWith("intl-")) segments.shift();
  if (segments[0] === "search") return { kind: "search", query: decodeQuery(segments.slice(1).join("/")) };
  if (segments[0] === "user" && segments[2] === "playlist") return entity("playlist", segments[3]);
  return entity(segments[0], segments[1]);
}

/**
 * @param {string|undefined} kind
 * @param {string|undefined} id
 * @returns {SpotifyLink|null}
 */
function entity(kind, id) {
  if (!kind || !id || !KINDS.includes(kind) || !ID.test(id)) return null;
  return {
    kind: /** @type {"track"|"album"|"playlist"|"artist"|"show"|"episode"} */ (kind),
    id,
    uri: `spotify:${kind}:${id}`,
  };
}

/** @param {string} query */
function decodeQuery(query) {
  try {
    return decodeURIComponent(query.replace(/\+/g, " "));
  } catch {
    return query;
  }
}

/**
 * Every song link or URI in some text (one per line, as Spotify copies them),
 * as `spotify:track:` URIs, in order.
 * @param {string} text
 */
export function trackUrisIn(text) {
  /** @type {string[]} */
  const uris = [];
  for (const word of text.split(/\s+/)) {
    const link = word && parseSpotifyLink(word);
    if (link && link.kind === "track") uris.push(link.uri);
  }
  return uris;
}

/**
 * The open.spotify.com link for a song or podcast episode, or "" for a local file.
 * @param {{ id?: string|null, uri?: string }} track
 */
export function trackLink(track) {
  if (!track.id) return "";
  const kind = track.uri?.startsWith("spotify:episode:") ? "episode" : "track";
  return `https://open.spotify.com/${kind}/${track.id}`;
}
