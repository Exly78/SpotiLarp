<script>
  import { untrack } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import * as api from "../api.js";
  import { menu, closeMenu } from "../contextMenu.svelte.js";
  import { addToQueue, playNext } from "../queue.svelte.js";
  import { likedIds, refreshLiked, toggleLike } from "../likes.svelte.js";
  import { library, ownedPlaylists, createPlaylist, adjustTrackCount } from "../library.svelte.js";
  import { openArtist, openAlbum } from "../nav.svelte.js";
  import { notify } from "../toast.svelte.js";
  import { LOCAL_URI_PREFIX } from "../constants.js";
  import Icon from "./Icon.svelte";

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
    const id = menu.open ? menu.track?.id : null;
    untrack(() => {
      page = "main";
      newName = "";
      if (id) refreshLiked([id]);
    });
  });

  /** @param {() => unknown} action */
  function run(action) {
    closeMenu();
    action();
  }

  /** @param {string} url */
  async function copyLink(url) {
    try {
      await navigator.clipboard.writeText(url);
      notify("Link copied");
    } catch (e) {
      notify(`Couldn't copy the link: ${e}`);
    }
  }

  /** @param {string} url */
  function openInSpotify(url) {
    openUrl(url).catch((e) => notify(`Couldn't open Spotify: ${e}`));
  }

  /**
   * @param {import("../types.js").Playlist} playlist
   * @param {import("../types.js").Track} track
   */
  async function addTo(playlist, track) {
    closeMenu();
    try {
      await api.addToPlaylist(playlist.id, [track.uri]);
      adjustTrackCount(playlist.id, 1);
      notify(`Added to ${playlist.name}`);
    } catch (e) {
      notify(`Couldn't add it to ${playlist.name}: ${e}`);
    }
  }

  /** @param {import("../types.js").Track} track */
  async function addToNew(track) {
    const name = newName.trim();
    if (!name) return;
    closeMenu();
    try {
      await addTo(await createPlaylist(name), track);
    } catch (e) {
      notify(`Couldn't create the playlist: ${e}`);
    }
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
  {@const link = track.id ? `https://open.spotify.com/track/${track.id}` : ""}
  {@const local = track.uri.startsWith(LOCAL_URI_PREFIX)}
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
        onkeydown={(e) => e.key === "Enter" && addToNew(track)}
        spellcheck="false"
        aria-label="New playlist name"
      />
      {#if newName.trim()}
        <button type="button" role="menuitem" onclick={() => addToNew(track)}>
          Create "{newName.trim()}"
        </button>
      {/if}
      <div class="playlists">
        {#each ownedPlaylists() as playlist (playlist.id)}
          <button type="button" role="menuitem" onclick={() => addTo(playlist, track)}>{playlist.name}</button>
        {:else}
          <div class="empty">{library.loading ? "Loading playlists..." : "You don't have your own playlists yet."}</div>
        {/each}
      </div>
    {:else}
      <button type="button" role="menuitem" onclick={() => run(() => playNext(track))}>Play next</button>
      <button type="button" role="menuitem" onclick={() => run(() => addToQueue(track))}>Add to queue</button>
      {#if link || local}
        <div class="divider"></div>
        <button type="button" role="menuitem" class="has-sub" onclick={() => (page = "playlists")}>
          Add to playlist <Icon name="chevron-right" size={14} />
        </button>
        {#if menu.onRemove}
          {@const remove = menu.onRemove}
          <button type="button" role="menuitem" onclick={() => run(remove)}>Remove from this playlist</button>
        {/if}
        {#if link}
          <button type="button" role="menuitem" onclick={() => run(() => toggleLike(track.id))}>
            {likedIds.has(track.id) ? "Remove from Liked Songs" : "Save to Liked Songs"}
          </button>
        {/if}
      {/if}
      {#if artist || track.album.id}
        <div class="divider"></div>
        {#if artist}
          <button type="button" role="menuitem" onclick={() => run(() => openArtist(artist))}>Go to artist</button>
        {/if}
        {#if track.album.id}
          <button type="button" role="menuitem" onclick={() => run(() => openAlbum(track.album))}>Go to album</button>
        {/if}
      {/if}
      {#if link}
        <div class="divider"></div>
        <button type="button" role="menuitem" onclick={() => run(() => copyLink(link))}>Copy song link</button>
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
  background: #3e3e3e;
}
.has-sub {
  justify-content: space-between;
}
.back {
  font-weight: var(--fw-bold);
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
