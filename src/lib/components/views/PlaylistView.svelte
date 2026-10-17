<script module>
  import { BoundedCache } from "../../boundedCache.js";

  /**
   * Playlists can be thousands of songs, so only the last few opened are kept.
   * @type {BoundedCache<string, import("../../types.js").Track[]>}
   */
  const cache = new BoundedCache(6);

  export function clearPlaylistCache() {
    cache.clear();
  }

  /** @param {string} playlistId */
  export function cachedPlaylistTracks(playlistId) {
    return cache.get(playlistId);
  }

  /**
   * @param {string} playlistId
   * @param {import("../../types.js").Track[]} tracks
   */
  export function cachePlaylistTracks(playlistId, tracks) {
    cache.set(playlistId, tracks);
  }
</script>

<script>
  import { onMount, untrack } from "svelte";
  import { fade } from "svelte/transition";
  import * as api from "../../api.js";
  import { playFromList, addManyToQueue } from "../../queue.svelte.js";
  import { markLiked, likeTracks } from "../../likes.svelte.js";
  import { isOwned, adjustTrackCount, updatePlaylistDetails } from "../../library.svelte.js";
  import { openRadio } from "../../nav.svelte.js";
  import { notify } from "../../toast.svelte.js";
  import { formatDuration, coverAtLeast, htmlToText } from "../../utils.js";
  import TrackList from "../TrackList.svelte";
  import PageHeader from "../PageHeader.svelte";
  import PlayButton from "../PlayButton.svelte";
  import ShuffleButton from "../ShuffleButton.svelte";
  import MoreMenu from "../MoreMenu.svelte";
  import EditPlaylistDialog from "../EditPlaylistDialog.svelte";

  /** @type {{ playlist: import("../../types.js").Playlist }} */
  let { playlist } = $props();

  /** @typedef {import("../../types.js").Track} Track */
  /** @type {Record<string, ((a: Track, b: Track) => number) | null>} */
  const SORTS = {
    custom: null,
    title: (a, b) => a.name.localeCompare(b.name),
    artist: (a, b) => (a.artists[0]?.name ?? "").localeCompare(b.artists[0]?.name ?? ""),
    album: (a, b) => a.album.name.localeCompare(b.album.name),
    added: (a, b) => (b.added_at ?? "").localeCompare(a.added_at ?? ""),
    duration: (a, b) => a.duration_ms - b.duration_ms,
  };

  const cached = untrack(() => cache.get(playlist.id));
  /** @type {Track[]} */
  let tracks = $state(cached ?? []);
  let loading = $state(!cached);
  let error = $state("");
  let filter = $state("");
  let sort = $state("custom");
  let refreshing = $state(false);
  let editing = $state(false);

  const owned = $derived(isOwned(playlist));
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
  const view = $derived(/** @type {import("../../nav.svelte.js").View} */ ({ type: "playlist", playlist }));
  const link = $derived(playlist.isLikedSongs ? "" : `https://open.spotify.com/playlist/${playlist.id}`);
  const unreadable = $derived(!loading && !error && tracks.length === 0 && !owned && playlist.track_count.total > 0);

  onMount(load);

  async function load() {
    try {
      const result = playlist.isLikedSongs
        ? await api.getLikedSongs()
        : await api.getPlaylistTracks(playlist.id);
      tracks = result;
      cache.set(playlist.id, result);
      const ids = result.map((t) => t.id);
      if (playlist.isLikedSongs) markLiked(ids);
    } catch (e) {
      if (tracks.length === 0) error = `Failed to load playlist: ${e}`;
      else notify(`Couldn't refresh "${playlist.name}": ${e}`);
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  // Picks up changes made in other Spotify apps; the songs stay on screen meanwhile.
  function refresh() {
    if (refreshing) return;
    refreshing = true;
    load();
  }

  function playAll() {
    if (visibleTracks.length === 0) return;
    playFromList(visibleTracks, -1, playlist.name, view);
  }

  async function copyLink() {
    try {
      await navigator.clipboard.writeText(link);
      notify("Link copied");
    } catch (e) {
      notify(`Couldn't copy the link: ${e}`);
    }
  }

  /** @param {{ name: string, description: string, public: boolean }} details */
  async function saveDetails(details) {
    await updatePlaylistDetails(playlist, details);
    editing = false;
    notify("Playlist details saved");
  }

  const canReorder = $derived(owned && !playlist.isLikedSongs && sort === "custom" && !filter.trim());

  /**
   * @param {number} from
   * @param {number} insertBefore
   */
  async function reorder(from, insertBefore) {
    const moving = tracks[from];
    const fromPosition = moving?.position;
    const beforePosition =
      insertBefore < tracks.length
        ? tracks[insertBefore].position
        : (tracks[tracks.length - 1]?.position ?? -1) + 1;
    if (fromPosition == null || beforePosition == null) return;

    const previous = tracks;
    const reordered = [...tracks];
    reordered.splice(from, 1);
    reordered.splice(insertBefore > from ? insertBefore - 1 : insertBefore, 0, moving);
    tracks = reordered;
    try {
      await api.movePlaylistTrack(playlist.id, fromPosition, beforePosition);
      load();
    } catch (e) {
      tracks = previous;
      notify(`Couldn't move the song: ${e}`);
    }
  }

  /** @param {Track[]} removed every copy of each goes, as Spotify does */
  async function removeTracks(removed) {
    const uris = [...new Set(removed.map((t) => t.uri))];
    if (uris.length === 0) return;
    try {
      await api.removeFromPlaylist(playlist.id, uris);
      const before = tracks.length;
      tracks = tracks.filter((t) => !uris.includes(t.uri));
      cache.set(playlist.id, tracks);
      adjustTrackCount(playlist.id, tracks.length - before);
      notify(uris.length === 1 ? `Removed from ${playlist.name}` : `Removed ${uris.length} songs from ${playlist.name}`);
    } catch (e) {
      notify(`Couldn't remove from the playlist: ${e}`);
    }
  }

  /**
   * Ctrl+V with song links on the clipboard. Songs already here are skipped.
   * @param {string[]} uris
   */
  async function paste(uris) {
    const have = new Set(tracks.map((t) => t.uri));
    const fresh = [...new Set(uris)].filter((uri) => !have.has(uri));
    const skipped = uris.length - fresh.length;
    if (fresh.length === 0) {
      notify(uris.length === 1 ? "That song is already here" : "Those songs are already here");
      return;
    }
    try {
      if (playlist.isLikedSongs) {
        await likeTracks(fresh.map((uri) => uri.split(":")[2]));
      } else {
        await api.addToPlaylist(playlist.id, fresh);
        adjustTrackCount(playlist.id, fresh.length);
        notify(`Added ${fresh.length === 1 ? "1 song" : `${fresh.length} songs`} to ${playlist.name}${skipped > 0 ? ` (${skipped} already here)` : ""}`);
      }
      load();
    } catch (e) {
      notify(`Couldn't add the songs: ${e}`);
    }
  }
</script>

<PageHeader
  image={coverAtLeast(playlist.images, 300)}
  liked={playlist.isLikedSongs}
  label="Playlist"
  title={playlist.name}
>
  {#snippet meta()}
    {#if htmlToText(playlist.description)}<span class="description">{htmlToText(playlist.description)}</span>{/if}
    {#if playlist.owner?.display_name}<strong>{playlist.owner.display_name}</strong> · {/if}
    {#if !loading && tracks.length > 0}
      {tracks.length} {tracks.length === 1 ? "song" : "songs"} · {formatDuration(totalMs)}
    {/if}
  {/snippet}
  {#snippet actions()}
    <PlayButton onclick={playAll} disabled={loading || visibleTracks.length === 0} label={`Play ${playlist.name}`} />
    <ShuffleButton />
    <MoreMenu
      label={`More options for ${playlist.name}`}
      items={[
        { label: "Add to queue", onclick: () => addManyToQueue(visibleTracks), disabled: visibleTracks.length === 0 },
        !playlist.isLikedSongs && {
          label: "Go to playlist radio",
          onclick: () => openRadio(`spotify:playlist:${playlist.id}`, playlist.name, coverAtLeast(playlist.images, 300)),
        },
        null,
        { label: refreshing ? "Refreshing..." : "Refresh", onclick: refresh, disabled: refreshing || loading },
        owned && !playlist.isLikedSongs && { label: "Edit details", onclick: () => (editing = true) },
        null,
        !!link && { label: "Copy link to playlist", onclick: copyLink },
      ]}
    />
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
          <option value="custom">Custom order</option>
          <option value="title">Title</option>
          <option value="artist">Artist</option>
          <option value="album">Album</option>
          <option value="added">Recently added</option>
          <option value="duration">Duration</option>
        </select>
      </div>
    {/if}
  {/snippet}
</PageHeader>

{#if loading}
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
{:else if error}
  <p class="status error">{error}</p>
{:else if unreadable}
  <div class="empty-state">
    Spotify only lets this app show songs from playlists you own or collaborate on.
  </div>
{:else if tracks.length === 0}
  <div class="empty-state">This playlist is empty.</div>
{:else if visibleTracks.length === 0}
  <div class="empty-state">No tracks match "{filter}".</div>
{:else}
  <TrackList
    tracks={visibleTracks}
    contextName={playlist.name}
    contextView={view}
    primary
    showDateAdded
    onRemove={owned && !playlist.isLikedSongs ? removeTracks : undefined}
    onReorder={canReorder ? reorder : undefined}
    onPaste={owned ? paste : undefined}
  />
{/if}

{#if editing}
  <EditPlaylistDialog {playlist} onSave={saveDetails} onCancel={() => (editing = false)} />
{/if}

<style>
.description {
  display: block;
  margin-bottom: 0.3em;
  color: var(--text-muted);
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
