<script>
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { queue, playFromList } from "../../queue.svelte.js";
  import { localFiles, loadLocalFiles, rescanLocalFiles, addLocalFolder } from "../../localFiles.svelte.js";
  import { navigate } from "../../nav.svelte.js";
  import { notify } from "../../toast.svelte.js";
  import { formatDuration } from "../../utils.js";
  import TrackList from "../TrackList.svelte";
  import PageHeader from "../PageHeader.svelte";
  import PlayButton from "../PlayButton.svelte";
  import Icon from "../Icon.svelte";

  const NAME = "Local Files";

  /** @typedef {import("../../types.js").Track} Track */
  /** @type {Record<string, ((a: Track, b: Track) => number) | null>} */
  const SORTS = {
    album: null,
    title: (a, b) => a.name.localeCompare(b.name),
    artist: (a, b) => (a.artists[0]?.name ?? "").localeCompare(b.artists[0]?.name ?? ""),
    added: (a, b) => (b.added_at ?? "").localeCompare(a.added_at ?? ""),
    duration: (a, b) => a.duration_ms - b.duration_ms,
  };

  let filter = $state("");
  let sort = $state("album");
  let choosingFolder = $state(false);

  const tracks = $derived(localFiles.tracks);
  const sorted = $derived.by(() => {
    const compare = SORTS[sort];
    return compare ? [...tracks].sort(compare) : tracks;
  });
  const visibleTracks = $derived.by(() => {
    const query = filter.trim().toLowerCase();
    if (!query) return sorted;
    return sorted.filter(
      (t) =>
        t.name.toLowerCase().includes(query) ||
        t.album.name.toLowerCase().includes(query) ||
        t.artists.some((a) => a.name.toLowerCase().includes(query)),
    );
  });
  const totalMs = $derived(tracks.reduce((sum, t) => sum + t.duration_ms, 0));

  onMount(loadLocalFiles);

  function playAll() {
    if (visibleTracks.length === 0) return;
    const start = queue.shuffle ? Math.floor(Math.random() * visibleTracks.length) : 0;
    playFromList(visibleTracks, start, NAME);
  }

  async function addFolder() {
    choosingFolder = true;
    try {
      await addLocalFolder();
    } catch (e) {
      notify(`Couldn't add the folder: ${e}`);
    } finally {
      choosingFolder = false;
    }
  }
</script>

<PageHeader icon="folder" label="Playlist" title={NAME}>
  {#snippet meta()}
    {#if tracks.length > 0}
      {tracks.length} {tracks.length === 1 ? "song" : "songs"} · {formatDuration(totalMs)}
    {/if}
    {#if localFiles.skipped > 0}
      · {localFiles.skipped} {localFiles.skipped === 1 ? "file" : "files"} couldn't be read
    {/if}
  {/snippet}
  {#snippet actions()}
    <PlayButton onclick={playAll} disabled={visibleTracks.length === 0} label={`Play ${NAME}`} />
    {#if localFiles.folders.length > 0}
      <button
        type="button"
        class="icon-button"
        onclick={rescanLocalFiles}
        disabled={localFiles.loading}
        aria-label="Rescan folders"
        title="Rescan folders"
      >
        <Icon name="refresh" size={18} class={localFiles.loading ? "spin" : ""} />
      </button>
      <button
        type="button"
        class="icon-button"
        onclick={() => navigate({ type: "settings" })}
        aria-label="Manage folders"
        title="Manage folders"
      >
        <Icon name="folder" size={18} />
      </button>
    {/if}
    {#if tracks.length > 1}
      <div class="tools">
        <input
          type="search"
          class="filter-input"
          placeholder="Filter"
          bind:value={filter}
          aria-label="Filter tracks"
          spellcheck="false"
        />
        <select class="sort-select" bind:value={sort} aria-label="Sort by">
          <option value="album">Album</option>
          <option value="title">Title</option>
          <option value="artist">Artist</option>
          <option value="added">Recently added</option>
          <option value="duration">Duration</option>
        </select>
      </div>
    {/if}
  {/snippet}
</PageHeader>

{#if !localFiles.loaded && !localFiles.error}
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
{:else if localFiles.error}
  <p class="status error">{localFiles.error}</p>
{:else if localFiles.folders.length === 0}
  <div class="empty-state">
    <p>Play songs from your computer alongside Spotify.</p>
    <p class="hint">
      Pick a folder and SpotiLarp finds the music in it, reading each song's title, artist, album and cover art.
      MP3, FLAC, M4A, OGG and WAV files work.
    </p>
    <button type="button" class="add-folder" onclick={addFolder} disabled={choosingFolder}>Add a folder</button>
  </div>
{:else if tracks.length === 0}
  <div class="empty-state">
    <p>{localFiles.loading ? "Looking for songs..." : "No songs found in your folders yet."}</p>
    <p class="hint">MP3, FLAC, M4A, OGG and WAV files work. Added some just now? Rescan to pick them up.</p>
  </div>
{:else if visibleTracks.length === 0}
  <div class="empty-state">No tracks match "{filter}".</div>
{:else}
  <TrackList tracks={visibleTracks} contextName={NAME} showDateAdded />
{/if}

<style>
.icon-button {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border: none;
  border-radius: 50%;
  background: none;
  color: var(--text-muted);
  cursor: pointer;
  transition: color var(--transition);
}
.icon-button:hover:not(:disabled) {
  color: var(--text);
}
.icon-button:disabled {
  cursor: default;
}
.tools {
  display: flex;
  align-items: center;
  gap: 0.6em;
  margin-left: auto;
}
.filter-input,
.sort-select {
  padding: 0.45em 0.9em;
  border: 1px solid transparent;
  border-radius: var(--radius-pill);
  background: var(--control-bg);
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  outline: none;
  transition: border-color var(--transition);
}
.filter-input {
  width: 180px;
}
.sort-select {
  cursor: pointer;
}
.filter-input:focus,
.sort-select:focus {
  border-color: var(--text);
}
.filter-input::placeholder {
  color: var(--text-muted);
}
.status {
  font-size: var(--fs-sm);
  color: var(--text-muted);
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
.empty-state p {
  margin: 0 0 0.6em;
}
.empty-state p:first-child {
  color: var(--text);
  font-size: var(--fs-md);
  font-weight: var(--fw-bold);
}
.empty-state .hint {
  max-width: 460px;
  margin: 0 auto 1.4em;
}
.add-folder {
  padding: 0.6em 1.6em;
  border: none;
  border-radius: var(--radius-pill);
  background: var(--accent);
  color: var(--accent-text);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
  transition: background-color var(--transition), transform 100ms ease;
}
.add-folder:hover:not(:disabled) {
  background: var(--accent-hover);
  transform: scale(1.04);
}
.add-folder:disabled {
  cursor: default;
  opacity: 0.5;
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
  width: 40px;
  height: 40px;
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
