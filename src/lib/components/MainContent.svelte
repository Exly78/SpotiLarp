<script>
  import { fade } from "svelte/transition";
  import * as api from "../api.js";
  import TrackList from "./TrackList.svelte";
  import Lyrics from "./Lyrics.svelte";

  /** @type {{ selectedPlaylist: import("../types.js").Playlist|null, showLyrics?: boolean }} */
  let { selectedPlaylist, showLyrics = false } = $props();

  /** @type {import("../types.js").Track[]} */
  let playlistTracks = $state([]);
  let loadingPlaylist = $state(false);
  let playlistError = $state("");
  /** @type {string|null} */
  let loadedPlaylistId = $state(null);

  $effect(() => {
    if (selectedPlaylist && selectedPlaylist.id !== loadedPlaylistId) {
      loadPlaylistTracks(selectedPlaylist);
    }
  });

  /** @param {import("../types.js").Playlist} playlist */
  async function loadPlaylistTracks(playlist) {
    loadingPlaylist = true;
    playlistError = "";
    try {
      playlistTracks = playlist.isLikedSongs
        ? await api.getLikedSongs()
        : await api.getPlaylistTracks(playlist.id);
      loadedPlaylistId = playlist.id;
    } catch (e) {
      playlistError = `Failed to load playlist: ${e}`;
    } finally {
      loadingPlaylist = false;
    }
  }
</script>

<div class="main-content">
  <div class="results-area">
    <div class="lyrics-wrap" class:hidden={!showLyrics}>
      <Lyrics />
    </div>
    {#if !showLyrics}
      {#if selectedPlaylist}
        <h2 class="section-title">{selectedPlaylist.name}</h2>
        {#if loadingPlaylist}
          <ul class="skeleton-list" transition:fade={{ duration: 150 }}>
            {#each { length: 6 } as _}
              <li class="skeleton-row">
                <div class="skeleton thumb"></div>
                <div class="skeleton-lines">
                  <div class="skeleton line-name"></div>
                  <div class="skeleton line-sub"></div>
                </div>
              </li>
            {/each}
          </ul>
        {:else if playlistError}
          <p class="status error" transition:fade={{ duration: 150 }}>{playlistError}</p>
        {:else if playlistTracks.length === 0}
          <div class="empty-state" transition:fade={{ duration: 150 }}>This playlist is empty.</div>
        {:else}
          <TrackList tracks={playlistTracks} />
        {/if}
      {:else}
        <div class="empty-state" transition:fade={{ duration: 150 }}>Search for a track, or pick a playlist from the sidebar.</div>
      {/if}
    {/if}
  </div>
</div>

<style>
.main-content {
  display: flex;
  flex-direction: column;
  gap: 1.25em;
  width: 100%;
  height: 100%;
  min-width: 0;
}
.lyrics-wrap {
  width: 100%;
  height: 100%;
}
.lyrics-wrap.hidden {
  display: none;
}
.results-area {
  flex: 1;
  overflow-y: auto;
}
.section-title {
  font-size: var(--fs-lg);
  font-weight: var(--fw-black);
  letter-spacing: -0.01em;
  margin: 0 0 0.6em 0;
  color: var(--text);
}
.status {
  font-size: var(--fs-sm);
  color: var(--text-muted);
  margin: 0 0 0.4em 0;
}
.status.error {
  color: var(--danger);
}
.empty-state {
  color: var(--text-muted);
  font-size: var(--fs-sm);
  padding: 2.5em 1em;
  text-align: center;
}
.skeleton-list {
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
  gap: 0.8em;
  padding: 0.5em 0.6em;
}
.skeleton-row .thumb {
  width: 44px;
  height: 44px;
  border-radius: var(--radius-sm);
  flex-shrink: 0;
}
.skeleton-lines {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 0.4em;
}
.line-name {
  width: 45%;
  height: 0.8em;
}
.line-sub {
  width: 25%;
  height: 0.65em;
}
</style>
