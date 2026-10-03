<script>
  import { tick } from "svelte";
  import { fly, fade } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import * as api from "../api.js";
  import { smallestCover } from "../utils.js";
  import {
    library,
    loadLibrary,
    createPlaylist,
    renamePlaylist,
    deletePlaylist,
    isOwned,
    adjustTrackCount,
  } from "../library.svelte.js";
  import { likeTracks } from "../likes.svelte.js";
  import { drag } from "../drag.svelte.js";
  import { nav, navigate } from "../nav.svelte.js";
  import { notify } from "../toast.svelte.js";
  import Icon from "./Icon.svelte";

  import { LIKED_SONGS_ID } from "../constants.js";

  /** @typedef {import("../types.js").Playlist} Playlist */

  /** @type {{ loggedIn: boolean }} */
  let { loggedIn } = $props();

  const LIKED_SONGS = {
    id: LIKED_SONGS_ID,
    name: "Liked Songs",
    isLikedSongs: true,
    images: [],
    track_count: { total: 0 },
  };

  const selectedId = $derived(nav.view.type === "playlist" ? nav.view.playlist.id : null);

  let creating = $state(false);
  let newName = $state("");
  let saving = $state(false);
  /** @type {HTMLInputElement|undefined} */
  let nameInput = $state();

  /** @type {string|null} */
  let dropTargetId = $state(null);

  /** @type {{ playlist: Playlist, x: number, y: number, confirmDelete: boolean }|null} */
  let menu = $state(null);
  /** @type {HTMLDivElement|undefined} */
  let menuEl = $state();
  /** @type {string|null} */
  let renamingId = $state(null);
  let renameValue = $state("");

  /** @param {Playlist} playlist */
  function open(playlist) {
    navigate({ type: "playlist", playlist });
  }

  async function startCreating() {
    creating = true;
    newName = "";
    await tick();
    nameInput?.focus();
  }

  async function submitNew() {
    const name = newName.trim();
    if (!name || saving) return;
    saving = true;
    try {
      const playlist = await createPlaylist(name);
      creating = false;
      open(playlist);
    } catch (e) {
      notify(`Couldn't create the playlist: ${e}`);
    } finally {
      saving = false;
    }
  }

  /** @param {KeyboardEvent} evt */
  function onNameKeydown(evt) {
    if (evt.key === "Enter") submitNew();
    if (evt.key === "Escape") creating = false;
  }

  /**
   * @param {DragEvent} evt
   * @param {Playlist} playlist
   */
  function onDragOver(evt, playlist) {
    if (drag.tracks.length === 0 || !isOwned(playlist)) return;
    evt.preventDefault();
    if (evt.dataTransfer) evt.dataTransfer.dropEffect = "copy";
    dropTargetId = playlist.id;
  }

  /** @param {Playlist} playlist */
  function onDragLeave(playlist) {
    if (dropTargetId === playlist.id) dropTargetId = null;
  }

  /**
   * @param {DragEvent} evt
   * @param {Playlist} playlist
   */
  async function onDrop(evt, playlist) {
    evt.preventDefault();
    dropTargetId = null;
    const tracks = drag.tracks.filter((t) => t.id);
    if (tracks.length === 0) return;
    if (playlist.isLikedSongs) {
      await likeTracks(tracks.map((t) => t.id));
      return;
    }
    try {
      await api.addToPlaylist(playlist.id, tracks.map((t) => t.uri));
      adjustTrackCount(playlist.id, tracks.length);
      notify(`Added to ${playlist.name}`);
    } catch (e) {
      notify(`Couldn't add it to ${playlist.name}: ${e}`);
    }
  }

  /**
   * @param {MouseEvent} evt
   * @param {Playlist} playlist
   */
  function openMenu(evt, playlist) {
    evt.preventDefault();
    menu = { playlist, x: evt.clientX, y: evt.clientY, confirmDelete: false };
  }

  /** @param {PointerEvent} evt */
  function onWindowPointerDown(evt) {
    if (menu && menuEl && !menuEl.contains(/** @type {Node} */ (evt.target))) menu = null;
  }

  /** @param {Playlist} playlist */
  async function startRename(playlist) {
    menu = null;
    renamingId = playlist.id;
    renameValue = playlist.name;
    await tick();
    const input = /** @type {HTMLInputElement|null} */ (document.querySelector(".rename-input"));
    input?.focus();
    input?.select();
  }

  /** @param {Playlist} playlist */
  async function submitRename(playlist) {
    const name = renameValue.trim();
    renamingId = null;
    if (!name || name === playlist.name) return;
    try {
      await renamePlaylist(playlist.id, name);
    } catch (e) {
      notify(`Couldn't rename the playlist: ${e}`);
    }
  }

  /**
   * @param {KeyboardEvent} evt
   * @param {Playlist} playlist
   */
  function onRenameKeydown(evt, playlist) {
    if (evt.key === "Enter") submitRename(playlist);
    if (evt.key === "Escape") renamingId = null;
  }

  /** @param {Playlist} playlist */
  async function confirmDelete(playlist) {
    if (!menu) return;
    if (!menu.confirmDelete) {
      menu.confirmDelete = true;
      return;
    }
    menu = null;
    try {
      await deletePlaylist(playlist.id);
      if (selectedId === playlist.id) navigate({ type: "home" });
      notify(isOwned(playlist) ? `Deleted ${playlist.name}` : `Removed ${playlist.name} from your library`);
    } catch (e) {
      notify(`Couldn't remove the playlist: ${e}`);
    }
  }
</script>

<svelte:window
  onpointerdown={onWindowPointerDown}
  onkeydown={(e) => e.key === "Escape" && (menu = null)}
  onwheel={() => (menu = null)}
/>

<nav class="sidebar">
  <div class="header-row">
    <h2>Playlists</h2>
    <div class="header-buttons">
      <button
        type="button"
        class="refresh"
        onclick={startCreating}
        disabled={!loggedIn}
        aria-label="Create playlist"
        title="Create playlist"
      >
        <Icon name="plus" size={16} />
      </button>
      <button
        type="button"
        class="refresh"
        onclick={loadLibrary}
        disabled={library.loading || !loggedIn}
        aria-label="Refresh playlists"
        title="Refresh playlists"
      >
        <Icon name="refresh" size={16} class={library.loading ? "spin" : ""} />
      </button>
    </div>
  </div>
  {#if creating}
    <div class="new-playlist">
      <input
        bind:this={nameInput}
        bind:value={newName}
        onkeydown={onNameKeydown}
        onblur={() => !newName.trim() && (creating = false)}
        placeholder="Playlist name"
        disabled={saving}
        spellcheck="false"
        aria-label="New playlist name"
      />
    </div>
  {/if}
  {#if !loggedIn}
    <p class="status">Log in to see your playlists.</p>
  {:else}
    <ul class="playlist-list pinned">
      <li>
        <button
          type="button"
          class="playlist-button"
          class:active={selectedId === LIKED_SONGS_ID}
          class:drop-target={dropTargetId === LIKED_SONGS_ID}
          onclick={() => open(LIKED_SONGS)}
          ondragover={(e) => onDragOver(e, LIKED_SONGS)}
          ondragleave={() => onDragLeave(LIKED_SONGS)}
          ondrop={(e) => onDrop(e, LIKED_SONGS)}
        >
          <div class="thumb liked-thumb">
            <Icon name="heart-filled" size={20} />
          </div>
          <div class="meta">
            <div class="name">Liked Songs</div>
          </div>
        </button>
      </li>
      <li>
        <button
          type="button"
          class="playlist-button"
          class:active={nav.view.type === "stats"}
          onclick={() => navigate({ type: "stats" })}
        >
          <div class="thumb stats-thumb">
            <Icon name="chart" size={20} />
          </div>
          <div class="meta">
            <div class="name">Your stats</div>
          </div>
        </button>
      </li>
    </ul>
    {#if library.loading && library.playlists.length === 0}
      <ul class="playlist-list" transition:fade={{ duration: 150 }}>
        {#each { length: 6 } as _}
          <li class="skeleton-row">
            <div class="skeleton thumb"></div>
            <div class="skeleton-lines">
              <div class="skeleton line-name"></div>
              <div class="skeleton line-count"></div>
            </div>
          </li>
        {/each}
      </ul>
    {:else if library.error}
      <p class="status error" transition:fade={{ duration: 150 }}>{library.error}</p>
    {:else if library.playlists.length === 0}
      <p class="status" transition:fade={{ duration: 150 }}>No playlists yet.</p>
    {:else}
      <ul class="playlist-list">
        {#each library.playlists as playlist, i (playlist.id)}
          <li in:fly={{ y: 8, duration: 220, delay: Math.min(i * 25, 300), easing: cubicOut }}>
            {#if renamingId === playlist.id}
              <div class="rename-row">
                <input
                  class="rename-input"
                  bind:value={renameValue}
                  onkeydown={(e) => onRenameKeydown(e, playlist)}
                  onblur={() => submitRename(playlist)}
                  spellcheck="false"
                  aria-label="Playlist name"
                />
              </div>
            {:else}
              <button
                type="button"
                class="playlist-button"
                class:active={selectedId === playlist.id}
                class:drop-target={dropTargetId === playlist.id}
                onclick={() => open(playlist)}
                oncontextmenu={(e) => openMenu(e, playlist)}
                ondragover={(e) => onDragOver(e, playlist)}
                ondragleave={() => onDragLeave(playlist)}
                ondrop={(e) => onDrop(e, playlist)}
              >
                {#if smallestCover(playlist.images)}
                  <img src={smallestCover(playlist.images)} alt="" class="thumb" />
                {:else}
                  <div class="thumb placeholder"></div>
                {/if}
                <div class="meta">
                  <div class="name">{playlist.name}</div>
                  <div class="count">
                    {playlist.track_count.total} {playlist.track_count.total === 1 ? "track" : "tracks"}
                    {#if playlist.owner?.display_name && playlist.owner.id !== library.userId}
                      · {playlist.owner.display_name}
                    {/if}
                  </div>
                </div>
              </button>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</nav>

{#if menu}
  {@const playlist = menu.playlist}
  {@const owned = isOwned(playlist)}
  <div class="menu" role="menu" bind:this={menuEl} style="left: {menu.x}px; top: {menu.y}px">
    {#if owned}
      <button type="button" role="menuitem" onclick={() => startRename(playlist)}>Rename</button>
    {/if}
    <button type="button" role="menuitem" class:danger={menu.confirmDelete} onclick={() => confirmDelete(playlist)}>
      {#if menu.confirmDelete}
        Click again to confirm
      {:else}
        {owned ? "Delete playlist" : "Remove from your library"}
      {/if}
    </button>
  </div>
{/if}

<style>
.sidebar {
  display: flex;
  flex-direction: column;
  gap: 0.75em;
  width: 100%;
  height: 100%;
  overflow-y: auto;
}
.header-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 0.4em;
}
h2 {
  font-size: var(--fs-xs);
  text-transform: uppercase;
  letter-spacing: 0.08em;
  font-weight: var(--fw-bold);
  color: var(--text-muted);
  margin: 0;
}
.refresh {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  background: none;
  border: none;
  border-radius: 50%;
  color: var(--text-muted);
  cursor: pointer;
  transition: background-color var(--transition), color var(--transition);
}
.refresh:hover {
  background-color: var(--surface-hover);
  color: var(--text);
}
.refresh:disabled {
  cursor: default;
  color: var(--text-muted);
}
:global(.spin) {
  animation: spin 1s linear infinite;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}
.status {
  font-size: var(--fs-sm);
  color: var(--text-muted);
  margin: 0;
  padding: 0.4em;
}
.status.error {
  color: var(--danger);
}
.playlist-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.skeleton-row {
  display: flex;
  align-items: center;
  gap: 0.7em;
  padding: 0.45em 0.5em;
}
.skeleton-lines {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 0.4em;
}
.line-name {
  width: 70%;
  height: 0.8em;
}
.line-count {
  width: 40%;
  height: 0.65em;
}

.playlist-button {
  display: flex;
  align-items: center;
  gap: 0.7em;
  width: 100%;
  padding: 0.45em 0.5em;
  border-radius: var(--radius-md);
  border: none;
  background: none;
  color: inherit;
  font: inherit;
  cursor: pointer;
  text-align: left;
  position: relative;
  transition: background-color var(--transition), transform var(--transition), box-shadow var(--transition);
}
.playlist-button:hover {
  background-color: var(--surface-hover);
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25);
}
.playlist-button.active {
  background-color: var(--surface-hover);
}
.playlist-button.active .name {
  color: var(--accent);
}
.thumb {
  width: 40px;
  height: 40px;
  border-radius: var(--radius-sm);
  object-fit: cover;
  background: var(--surface-raised);
  flex-shrink: 0;
}
.thumb.placeholder {
  background: linear-gradient(135deg, var(--surface-raised), var(--surface-hover));
}
.liked-thumb {
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #4b1fa8, #8f8ff0);
  color: #fff;
}
.playlist-list.pinned {
  margin-bottom: 0.5em;
  padding-bottom: 0.5em;
  border-bottom: 1px solid var(--border);
}
.meta {
  min-width: 0;
}
.name {
  font-size: var(--fs-sm);
  font-weight: var(--fw-medium);
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition: color var(--transition);
}
.count {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  margin-top: 0.15em;
}
.header-buttons {
  display: flex;
  align-items: center;
  gap: 0.2em;
}
.new-playlist input {
  width: 100%;
  padding: 0.5em 0.8em;
  border: 1px solid var(--text);
  border-radius: var(--radius-sm);
  background: var(--control-bg);
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  outline: none;
}
.count {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.playlist-button.drop-target {
  background-color: var(--accent-bg-strong);
  box-shadow: inset 0 0 0 1px var(--accent);
}
.stats-thumb {
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #1e3264, #1ed760);
  color: #fff;
}
.rename-row {
  padding: 0.35em 0.5em;
}
.rename-input {
  width: 100%;
  padding: 0.5em 0.8em;
  border: 1px solid var(--text);
  border-radius: var(--radius-sm);
  background: var(--control-bg);
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  outline: none;
}
.menu {
  position: fixed;
  z-index: 60;
  min-width: 200px;
  padding: 4px;
  border-radius: var(--radius-sm);
  background: var(--surface-hover);
  box-shadow: 0 16px 24px rgba(0, 0, 0, 0.3), 0 6px 8px rgba(0, 0, 0, 0.2);
}
.menu button {
  display: block;
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
}
.menu button:hover {
  background: #3e3e3e;
}
.menu button.danger {
  color: var(--danger);
}
</style>
