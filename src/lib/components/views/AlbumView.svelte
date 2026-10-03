<script>
  import { onMount } from "svelte";
  import * as api from "../../api.js";
  import { queue, playFromList } from "../../queue.svelte.js";
  import { refreshLiked } from "../../likes.svelte.js";
  import { openArtist } from "../../nav.svelte.js";
  import { formatDuration, coverAtLeast, releaseYear } from "../../utils.js";
  import TrackList from "../TrackList.svelte";
  import PageHeader from "../PageHeader.svelte";
  import PlayButton from "../PlayButton.svelte";

  /** @type {{ id: string, name?: string }} */
  let { id, name = "" } = $props();

  /** @type {import("../../types.js").AlbumDetails|null} */
  let album = $state(null);
  let error = $state("");

  const totalMs = $derived.by(() => (album ? album.tracks.reduce((sum, t) => sum + t.duration_ms, 0) : 0));
  const label = $derived.by(() =>
    album?.album_type === "single" ? "Single" : album?.album_type === "compilation" ? "Compilation" : "Album",
  );

  onMount(async () => {
    try {
      album = await api.getAlbum(id);
      refreshLiked(album.tracks.map((t) => t.id));
    } catch (e) {
      error = `Failed to load album: ${e}`;
    }
  });

  function playAll() {
    if (!album || album.tracks.length === 0) return;
    const start = queue.shuffle ? Math.floor(Math.random() * album.tracks.length) : 0;
    playFromList(album.tracks, start, album.name);
  }
</script>

{#if error}
  <p class="error">{error}</p>
{:else if !album}
  <PageHeader label="Album" title={name || "Loading..."} />
{:else}
  <PageHeader image={coverAtLeast(album.images, 300)} {label} title={album.name}>
    {#snippet meta()}
      {#each album?.artists ?? [] as artist, i}
        {#if i > 0}, {/if}<button type="button" class="link" onclick={() => openArtist(artist)}>{artist.name}</button>
      {/each}
      {#if album?.release_date} · {releaseYear(album.release_date)}{/if}
      · {album?.tracks.length} {album?.tracks.length === 1 ? "song" : "songs"}, {formatDuration(totalMs)}
    {/snippet}
    {#snippet actions()}
      <PlayButton onclick={playAll} label={`Play ${album?.name}`} />
    {/snippet}
  </PageHeader>
  <TrackList tracks={album.tracks} contextName={album.name} showAlbum={false} showCovers={false} />
{/if}

<style>
.error {
  font-size: var(--fs-sm);
  color: var(--danger);
}
.link {
  padding: 0;
  border: none;
  background: none;
  color: var(--text);
  font: inherit;
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.link:hover {
  text-decoration: underline;
}
</style>
