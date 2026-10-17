<script>
  import { onMount } from "svelte";
  import * as api from "../../api.js";
  import { openArtist, openAlbum, openShow } from "../../nav.svelte.js";
  import { episodeTrack } from "../../podcasts.svelte.js";
  import { notify } from "../../toast.svelte.js";
  import { coverAtLeast, releaseYear } from "../../utils.js";
  import TrackList from "../TrackList.svelte";
  import EpisodeList from "../EpisodeList.svelte";
  import MediaCard from "../MediaCard.svelte";
  import Section from "../Section.svelte";

  /** @type {{ query: string }} */
  let { query } = $props();

  /** @type {import("../../types.js").SearchResults|null} */
  let results = $state(null);
  let error = $state("");
  let loadingMore = $state(false);
  let moreAvailable = $state(true);

  const episodes = $derived.by(() => (results ? results.episodes.map((episode) => episodeTrack(episode)) : []));
  const view = $derived(/** @type {import("../../nav.svelte.js").View} */ ({ type: "search", query }));
  const found = $derived.by(() => {
    if (!results) return 0;
    const { tracks, artists, albums, shows, episodes } = results;
    return tracks.length + artists.length + albums.length + shows.length + episodes.length;
  });

  onMount(async () => {
    try {
      results = await api.searchAll(query);
      moreAvailable = results.tracks.length >= 10;
    } catch (e) {
      error = `Search failed: ${e}`;
    }
  });

  async function loadMore() {
    if (!results || loadingMore) return;
    loadingMore = true;
    try {
      const more = await api.searchTracksPage(query, results.tracks.length);
      results.tracks.push(...more);
      moreAvailable = more.length >= 10;
    } catch (e) {
      notify(`Couldn't load more results: ${e}`);
    } finally {
      loadingMore = false;
    }
  }
</script>

<h1 class="title">Results for "{query}"</h1>

{#if error}
  <p class="error">{error}</p>
{:else if !results}
  <p class="status">Searching...</p>
{:else if found === 0}
  <p class="status">Nothing found for "{query}".</p>
{:else}
  {#if results.tracks.length > 0}
    <Section title="Songs">
      <TrackList tracks={results.tracks} contextName={`Search: "${query}"`} contextView={view} primary />
      {#if moreAvailable}
        <button type="button" class="more" onclick={loadMore} disabled={loadingMore}>
          {loadingMore ? "Loading..." : "Show more"}
        </button>
      {/if}
    </Section>
  {/if}
  {#if results.artists.length > 0}
    <Section title="Artists" grid>
      {#each results.artists as artist (artist.id)}
        <MediaCard
          image={coverAtLeast(artist.images, 200)}
          title={artist.name}
          subtitle="Artist"
          round
          onclick={() => openArtist(artist)}
        />
      {/each}
    </Section>
  {/if}
  {#if results.albums.length > 0}
    <Section title="Albums" grid>
      {#each results.albums as album (album.id)}
        <MediaCard
          image={coverAtLeast(album.images, 200)}
          title={album.name}
          subtitle={[releaseYear(album.release_date), album.artists?.map((a) => a.name).join(", ")]
            .filter(Boolean)
            .join(" · ")}
          onclick={() => openAlbum(album)}
        />
      {/each}
    </Section>
  {/if}
  {#if results.shows.length > 0}
    <Section title="Podcasts" grid>
      {#each results.shows as show (show.id)}
        <MediaCard image={coverAtLeast(show.images, 200)} title={show.name} subtitle="Podcast" onclick={() => openShow(show)} />
      {/each}
    </Section>
  {/if}
  {#if episodes.length > 0}
    <Section title="Episodes">
      <EpisodeList {episodes} contextName={`Search: "${query}"`} contextView={view} />
    </Section>
  {/if}
{/if}

<style>
.title {
  margin: 0 0 1em;
  font-size: var(--fs-xl);
  font-weight: var(--fw-black);
  letter-spacing: -0.02em;
  color: var(--text);
}
.status {
  font-size: var(--fs-sm);
  color: var(--text-muted);
}
.error {
  font-size: var(--fs-sm);
  color: var(--danger);
}
.more {
  margin: 0.75em 0 0 0.6em;
  padding: 0.45em 1.2em;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-pill);
  background: none;
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.more:hover:not(:disabled) {
  border-color: var(--text);
}
.more:disabled {
  opacity: 0.6;
  cursor: default;
}
</style>
