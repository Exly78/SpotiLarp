import * as api from "./api.js";
import { navigate } from "./nav.svelte.js";
import { playFromList } from "./queue.svelte.js";
import { episodeTrack } from "./podcasts.svelte.js";
import { library } from "./library.svelte.js";
import { notify } from "./toast.svelte.js";
import { parseSpotifyLink } from "./links.js";

/**
 * Opens a Spotify link or URI: a song plays, anything else opens its page,
 * and with `play` starts playing from the top too. False when `text` isn't a
 * Spotify link.
 * @param {string} text
 * @param {{ play?: boolean }} [options]
 */
export function openSpotifyLink(text, { play = false } = {}) {
  const link = parseSpotifyLink(text);
  if (!link) return false;
  open(link, play).catch((e) => notify(`Couldn't open that link: ${e}`));
  return true;
}

/**
 * @param {import("./links.js").SpotifyLink} link
 * @param {boolean} play
 */
async function open(link, play) {
  switch (link.kind) {
    case "search":
      navigate({ type: "search", query: link.query });
      return;
    case "track": {
      const [track] = await api.getTracksByUri([link.uri]);
      if (!track) throw "Spotify doesn't have that song";
      await playFromList([track], 0, track.name);
      return;
    }
    case "album": {
      navigate({ type: "album", id: link.id });
      if (!play) return;
      const album = await api.getAlbum(link.id);
      if (album.tracks.length > 0) {
        await playFromList(album.tracks, 0, album.name, { type: "album", id: link.id, name: album.name });
      }
      return;
    }
    case "playlist": {
      const playlist =
        library.playlists.find((p) => p.id === link.id) ??
        (await api.getPlaylist(link.id).catch(() => ({ id: link.id, name: "Playlist", images: [], track_count: { total: 0 } })));
      const view = /** @type {const} */ ({ type: "playlist", playlist });
      navigate(view);
      if (!play) return;
      const tracks = await api.getPlaylistTracks(link.id);
      if (tracks.length > 0) await playFromList(tracks, 0, playlist.name, view);
      return;
    }
    case "show":
      navigate({ type: "show", id: link.id });
      return;
    case "episode": {
      const episode = await api.getEpisode(link.id);
      const track = episodeTrack(episode);
      if (track.show) navigate({ type: "show", id: track.show.id, name: track.show.name });
      await playFromList([track], 0, track.show?.name ?? episode.name);
      return;
    }
    case "artist": {
      navigate({ type: "artist", id: link.id, name: "" });
      if (!play) return;
      const page = await api.getArtistPage(link.id);
      if (page.top_tracks.length > 0) {
        await playFromList(page.top_tracks, 0, page.name, { type: "artist", id: link.id, name: page.name });
      }
    }
  }
}
