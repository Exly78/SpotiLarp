<script>
  import { untrack } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import * as api from "../api.js";
  import { menu, closeMenu } from "../contextMenu.svelte.js";
  import { addManyToQueue, playManyNext } from "../queue.svelte.js";
  import { likedIds, refreshLiked, toggleLike, likeTracks } from "../likes.svelte.js";
  import { library, ownedPlaylists, createPlaylist, adjustTrackCount } from "../library.svelte.js";
  import { openArtist, openAlbum, openRadio, openShow } from "../nav.svelte.js";
  import { isEpisode, isPlayed, setPlayed } from "../podcasts.svelte.js";
  import { notify } from "../toast.svelte.js";
  import { trackLink } from "../links.js";
  import { smallestCover } from "../utils.js";
  import { LOCAL_URI_PREFIX } from "../constants.js";
  import Icon from "./Icon.svelte";

  /** @typedef {import("../types.js").Track} Track */

  const EDGE_MARGIN = 8;

  /** @type {HTMLDivElement|undefined} */
  let el = $state();
  let width = $state(0);
  let height = $state(0);
  let page = $state("main");
  let newName = $state("");

  const left = $derived(Math.max(EDGE_MARGIN, Math.min(menu.x, window.innerWidth - width - EDGE_MARGIN)));
  const top = $derived(Math.max(EDGE_MARGIN, Math.min(menu.y, window.innerHeight - height - EDGE_MARGIN)));

  $effect(() => {
    const ids = menu.open ? menu.tracks.map((t) => t.id) : [];
    untrack(() => {
      page = "main";
      newName = "";
      if (ids.length > 0) refreshLiked(ids);
    });
  });

  /** @param {() => unknown} action */
  function run(action) {
    closeMenu();
    action();
  }

  /** @param {Track[]} tracks */
  async function copyLinks(tracks) {
    const links = tracks.map(trackLink).filter(Boolean);
    if (links.length === 0) return;
    try {
      await navigator.clipboard.writeText(links.join("\n"));
      notify(links.length === 1 ? "Link copied" : `Copied ${links.length} song links`);
    } catch (e) {
      notify(`Couldn't copy the link: ${e}`);
    }
  }

  /** @param {string} url */
  function openInSpotify(url) {
    openUrl(url).catch((e) => notify(`Couldn't open Spotify: ${e}`));
  }

  /**
   * Songs that can go in a playlist: Spotify songs and local files (not
   * podcast episodes, which SpotiLarp's playlists leave out).
   * @param {Track[]} tracks
   */
  function addable(tracks) {
    return tracks.filter((t) => !isEpisode(t) && (t.id || t.uri.startsWith(LOCAL_URI_PREFIX)));
  }

  /**
   * Opens an episode's show, asking Spotify which one it is if need be.
   * @param {Track} track
   */
  async function goToShow(track) {
    if (track.show) return openShow(track.show);
    try {
      const episode = await api.getEpisode(track.id);
      openShow(episode.show);
    } catch (e) {
      notify(`Couldn't find the podcast: ${e}`);
    }
  }

  /**
   * @param {import("../types.js").Playlist} playlist
   * @param {Track[]} tracks
   */
  async function addTo(playlist, tracks) {
    closeMenu();
    const songs = addable(tracks);
    if (songs.length === 0) return;
    try {
      await api.addToPlaylist(playlist.id, songs.map((t) => t.uri));
      adjustTrackCount(playlist.id, songs.length);
      notify(songs.length === 1 ? `Added to ${playlist.name}` : `Added ${songs.length} songs to ${playlist.name}`);
    } catch (e) {
      notify(`Couldn't add to ${playlist.name}: ${e}`);
    }
  }

  /** @param {Track[]} tracks */
  async function addToNew(tracks) {
    const name = newName.trim();
    if (!name) return;
    closeMenu();
    try {
      await addTo(await createPlaylist(name), tracks);
    } catch (e) {
      notify(`Couldn't create the playlist: ${e}`);
    }
  }

  /** @param {Track} track */
  function songRadio(track) {
    openRadio(`spotify:track:${track.id}`, track.name, smallestCover(track.album.images));
  }

  /** @param {PointerEvent} evt */
  function onWindowPointerDown(evt) {
    if (menu.open && el && !el.contains(/** @type {Node} */ (evt.target))) closeMenu();
  }

  /** @param {KeyboardEvent} evt */
  function onWindowKeydown(evt) {
    if (menu.open && evt.key === "Escape") closeMenu();
  }

  /** @param {WheelEvent} evt */
  function onWindowWheel(evt) {
    if (menu.open && el && !el.contains(/** @type {Node} */ (evt.target))) closeMenu();
  }
</script>

<svelte:window
  onpointerdown={onWindowPointerDown}
  onkeydown={onWindowKeydown}
  onwheel={onWindowWheel}
  onresize={closeMenu}
  onblur={closeMenu}
/>

{#if menu.open && menu.track}
  {@const track = menu.track}
  {@const tracks = menu.tracks}
  {@const many = tracks.length > 1}
  {@const link = many ? "" : trackLink(track)}
  {@const canAdd = addable(tracks).length > 0}
  {@const likeable = tracks.filter((t) => t.id && !isEpisode(t))}
  {@const episode = !many && isEpisode(track)}
  {@const allLiked = likeable.length > 0 && likeable.every((t) => likedIds.has(t.id))}
  {@const artist = track.artists.find((a) => a.id)}
  <div
    class="menu"
    role="menu"
    bind:this={el}
    bind:offsetWidth={width}
    bind:offsetHeight={height}
    style="left: {left}px; top: {top}px"
  >
    {#if page === "playlists"}
      <button type="button" role="menuitem" class="back" onclick={() => (page = "main")}>
        <Icon name="chevron-left" size={14} /> Add to playlist
      </button>
      <div class="divider"></div>
      <input
        class="new-name"
        placeholder="New playlist name"
        bind:value={newName}
        onkeydown={(e) => e.key === "Enter" && addToNew(tracks)}
        spellcheck="false"
        aria-label="New playlist name"
      />
      {#if newName.trim()}
        <button type="button" role="menuitem" onclick={() => addToNew(tracks)}>
          Create "{newName.trim()}"
        </button>
      {/if}
      <div class="playlists">
        {#each ownedPlaylists() as playlist (playlist.id)}
          <button type="button" role="menuitem" onclick={() => addTo(playlist, tracks)}>{playlist.name}</button>
        {:else}
          <div class="empty">{library.loading ? "Loading playlists..." : "You don't have your own playlists yet."}</div>
        {/each}
      </div>
    {:else}
      {#if many}
        <div class="heading">{tracks.length} songs selected</div>
      {/if}
      <button type="button" role="menuitem" onclick={() => run(() => playManyNext(tracks))}>Play next</button>
      <button type="button" role="menuitem" onclick={() => run(() => addManyToQueue(tracks))}>Add to queue</button>
      {#if canAdd}
        <div class="divider"></div>
        <button type="button" role="menuitem" class="has-sub" onclick={() => (page = "playlists")}>
          Add to playlist <Icon name="chevron-right" size={14} />
        </button>
        {#if menu.onRemove}
          {@const remove = menu.onRemove}
          <button type="button" role="menuitem" onclick={() => run(remove)}>Remove from this playlist</button>
        {/if}
        {#if many && likeable.length > 0 && !allLiked}
          <button type="button" role="menuitem" onclick={() => run(() => likeTracks(likeable.map((t) => t.id)))}>
            Save to Liked Songs
          </button>
        {:else if !many && track.id}
          <button type="button" role="menuitem" onclick={() => run(() => toggleLike(track.id))}>
            {likedIds.has(track.id) ? "Remove from Liked Songs" : "Save to Liked Songs"}
          </button>
        {/if}
      {/if}
      {#if episode}
        <div class="divider"></div>
        <button type="button" role="menuitem" onclick={() => run(() => setPlayed(track, !isPlayed(track)))}>
          {isPlayed(track) ? "Mark as unplayed" : "Mark as played"}
        </button>
        {#if track.id}
          <button type="button" role="menuitem" onclick={() => run(() => goToShow(track))}>Go to podcast</button>
        {/if}
      {:else if !many && (artist || track.album.id || track.id)}
        <div class="divider"></div>
        {#if track.id}
          <button type="button" role="menuitem" onclick={() => run(() => songRadio(track))}>Go to song radio</button>
        {/if}
        {#if artist}
          <button type="button" role="menuitem" onclick={() => run(() => openArtist(artist))}>Go to artist</button>
        {/if}
        {#if track.album.id}
          <button type="button" role="menuitem" onclick={() => run(() => openAlbum(track.album))}>Go to album</button>
        {/if}
      {/if}
      {#if many && likeable.length > 0}
        <div class="divider"></div>
        <button type="button" role="menuitem" onclick={() => run(() => copyLinks(tracks))}>Copy song links</button>
      {:else if link}
        <div class="divider"></div>
        <button type="button" role="menuitem" onclick={() => run(() => copyLinks([track]))}>
          {episode ? "Copy episode link" : "Copy song link"}
        </button>
        <button type="button" role="menuitem" onclick={() => run(() => openInSpotify(link))}>Open in Spotify</button>
      {/if}
    {/if}
  </div>
{/if}

<style>
.menu {
  position: fixed;
  z-index: 60;
  width: 240px;
  padding: 4px;
  border-radius: var(--radius-sm);
  background: var(--surface-hover);
  box-shadow: 0 16px 24px rgba(0, 0, 0, 0.3), 0 6px 8px rgba(0, 0, 0, 0.2);
}
.menu button {
  display: flex;
  align-items: center;
  gap: 0.4em;
  width: 100%;
  padding: 0.7em 0.8em;
  border: none;
  border-radius: 2px;
  background: none;
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  text-align: left;
  cursor: pointer;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.menu button:hover {
  background: var(--highlight-strong);
}
.has-sub {
  justify-content: space-between;
}
.back {
  font-weight: var(--fw-bold);
}
.heading {
  padding: 0.5em 0.8em 0.3em;
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  color: var(--text-muted);
}
.divider {
  height: 1px;
  margin: 4px 0;
  background: var(--border-strong);
}
.new-name {
  width: 100%;
  margin: 2px 0 4px;
  padding: 0.6em 0.8em;
  border: 1px solid var(--border-strong);
  border-radius: 2px;
  background: var(--control-bg);
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  outline: none;
}
.new-name:focus {
  border-color: var(--text);
}
.playlists {
  max-height: 280px;
  overflow-y: auto;
}
.empty {
  padding: 0.7em 0.8em;
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
</style>
