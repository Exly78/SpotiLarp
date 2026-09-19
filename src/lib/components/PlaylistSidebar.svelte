<script>
  import { fly, fade } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import * as api from "../api.js";
  import { smallestCover } from "../utils.js";
  import Icon from "./Icon.svelte";

  import { LIKED_SONGS_ID } from "../constants.js";

  /** @type {{ selectedId: string|undefined, onSelect: (playlist: import("../types.js").Playlist) => void }} */
  let { selectedId, onSelect } = $props();

  /** @type {import("../types.js").Playlist[]} */
  let playlists = $state([]);
  let loading = $state(false);
  let error = $state("");

  export async function load() {
    loading = true;
    error = "";
    try {
      playlists = await api.getPlaylists();
    } catch (e) {
      error = `Failed to load playlists: ${e}`;
    } finally {
      loading = false;
    }
  }

  load();
</script>

<nav class="sidebar">
  <div class="header-row">
    <h2>Playlists</h2>
    <button type="button" class="refresh" onclick={load} disabled={loading} aria-label="Refresh playlists">
      <Icon name="refresh" size={16} class={loading ? "spin" : ""} />
    </button>
  </div>
  <ul class="playlist-list pinned">
    <li>
      <button
        type="button"
        class="playlist-button"
        class:active={selectedId === LIKED_SONGS_ID}
        onclick={() =>
          onSelect({
            id: LIKED_SONGS_ID,
            name: "Liked Songs",
            isLikedSongs: true,
            images: [],
            track_count: { total: 0 },
          })}
      >
        <div class="thumb liked-thumb">
          <Icon name="heart-filled" size={20} />
        </div>
        <div class="meta">
          <div class="name">Liked Songs</div>
        </div>
      </button>
    </li>
  </ul>
  {#if loading}
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
  {:else if error}
    <p class="status error" transition:fade={{ duration: 150 }}>{error}</p>
  {:else if playlists.length === 0}
    <p class="status" transition:fade={{ duration: 150 }}>No playlists yet.</p>
  {:else}
    <ul class="playlist-list">
      {#each playlists as playlist, i (playlist.id)}
        <li in:fly={{ y: 8, duration: 220, delay: i * 25, easing: cubicOut }}>
          <button
            type="button"
            class="playlist-button"
            class:active={selectedId === playlist.id}
            onclick={() => onSelect(playlist)}
          >
            {#if smallestCover(playlist.images)}
              <img src={smallestCover(playlist.images)} alt="" class="thumb" />
            {:else}
              <div class="thumb placeholder"></div>
            {/if}
            <div class="meta">
              <div class="name">{playlist.name}</div>
              <div class="count">{playlist.track_count.total} tracks</div>
            </div>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</nav>

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
</style>
