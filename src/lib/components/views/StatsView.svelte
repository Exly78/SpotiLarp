<script module>
  /** @type {Map<string, { tracks: import("../../types.js").Track[], artists: import("../../types.js").ArtistDetails[] }>} */
  const cache = new Map();

  export function clearStatsCache() {
    cache.clear();
  }
</script>

<script>
  import * as api from "../../api.js";
  import { refreshLiked } from "../../likes.svelte.js";
  import { openArtist } from "../../nav.svelte.js";
  import { coverAtLeast } from "../../utils.js";
  import TrackList from "../TrackList.svelte";
  import MediaCard from "../MediaCard.svelte";
  import Section from "../Section.svelte";

  /** @typedef {"short_term"|"medium_term"|"long_term"} TimeRange */

  /** @type {{ value: TimeRange, label: string }[]} */
  const RANGES = [
    { value: "short_term", label: "Last 4 weeks" },
    { value: "medium_term", label: "Last 6 months" },
    { value: "long_term", label: "All time" },
  ];
  const TOP_LIMIT = 50;
  const ARTIST_CARDS = 18;
  const GENRES_SHOWN = 10;

  /** @type {TimeRange} */
  let range = $state("short_term");
  /** @type {import("../../types.js").Track[]} */
  let tracks = $state([]);
  /** @type {import("../../types.js").ArtistDetails[]} */
  let artists = $state([]);
  let loading = $state(false);
  let error = $state("");

  const genres = $derived.by(() => {
    /** @type {Map<string, number>} */
    const scores = new Map();
    artists.forEach((artist, rank) => {
      const weight = artists.length - rank;
      for (const genre of artist.genres) scores.set(genre, (scores.get(genre) ?? 0) + weight);
    });
    const ranked = [...scores.entries()].sort((a, b) => b[1] - a[1]).slice(0, GENRES_SHOWN);
    const top = ranked[0]?.[1] ?? 1;
    return ranked.map(([name, score]) => ({ name, share: score / top }));
  });

  $effect(() => {
    load(range);
  });

  /** @param {TimeRange} selected */
  async function load(selected) {
    const cached = cache.get(selected);
    if (cached) {
      tracks = cached.tracks;
      artists = cached.artists;
      loading = false;
      error = "";
      return;
    }
    loading = true;
    error = "";
    try {
      const [topTracks, topArtists] = await Promise.all([
        api.getTopTracks(selected, TOP_LIMIT),
        api.getTopArtists(selected, TOP_LIMIT),
      ]);
      cache.set(selected, { tracks: topTracks, artists: topArtists });
      if (range !== selected) return;
      tracks = topTracks;
      artists = topArtists;
      refreshLiked(topTracks.map((t) => t.id));
    } catch (e) {
      if (range === selected) error = `Couldn't load your stats: ${e}`;
    } finally {
      if (range === selected) loading = false;
    }
  }

  const rangeLabel = $derived(RANGES.find((r) => r.value === range)?.label ?? "");
</script>

<h1 class="title">Your stats</h1>

<div class="tabs" role="tablist">
  {#each RANGES as option}
    <button
      type="button"
      role="tab"
      class="tab"
      class:active={range === option.value}
      aria-selected={range === option.value}
      onclick={() => (range = option.value)}
    >
      {option.label}
    </button>
  {/each}
</div>

{#if error}
  <p class="error">{error}</p>
{:else if loading && tracks.length === 0}
  <p class="status">Loading...</p>
{:else}
  {#if artists.length > 0}
    <Section title="Top artists" grid>
      {#each artists.slice(0, ARTIST_CARDS) as artist, i (artist.id)}
        <MediaCard
          image={coverAtLeast(artist.images, 200)}
          title={`${i + 1}. ${artist.name}`}
          subtitle={artist.genres.slice(0, 2).join(", ") || "Artist"}
          round
          onclick={() => openArtist(artist)}
        />
      {/each}
    </Section>
  {/if}

  {#if genres.length > 0}
    <Section title="Top genres">
      <ol class="genres">
        {#each genres as genre}
          <li>
            <span class="genre-name">{genre.name}</span>
            <span class="bar"><span class="fill" style="width: {genre.share * 100}%"></span></span>
          </li>
        {/each}
      </ol>
    </Section>
  {/if}

  {#if tracks.length > 0}
    <Section title="Top tracks">
      <TrackList tracks={tracks} contextName={`Top tracks: ${rangeLabel}`} />
    </Section>
  {/if}

  {#if !loading && tracks.length === 0 && artists.length === 0}
    <p class="status">Not enough listening history for this period yet.</p>
  {/if}
{/if}

<style>
.title {
  margin: 0 0 0.6em;
  font-size: var(--fs-xl);
  font-weight: var(--fw-black);
  letter-spacing: -0.02em;
  color: var(--text);
}
.tabs {
  display: flex;
  gap: 0.5em;
  margin-bottom: 1.75em;
}
.tab {
  padding: 0.45em 1.1em;
  border: none;
  border-radius: var(--radius-pill);
  background: var(--control-bg);
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  cursor: pointer;
  transition: background-color var(--transition);
}
.tab:hover {
  background: #3a3a3a;
}
.tab.active {
  background: var(--text);
  color: var(--bg);
}
.status {
  font-size: var(--fs-sm);
  color: var(--text-muted);
}
.error {
  font-size: var(--fs-sm);
  color: var(--danger);
}
.genres {
  list-style: none;
  margin: 0;
  padding: 0;
  max-width: 640px;
  display: flex;
  flex-direction: column;
  gap: 0.55em;
}
.genres li {
  display: grid;
  grid-template-columns: 11em 1fr;
  align-items: center;
  gap: 1em;
}
.genre-name {
  font-size: var(--fs-sm);
  color: var(--text);
  text-transform: capitalize;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.bar {
  height: 8px;
  border-radius: var(--radius-pill);
  background: var(--surface-hover);
  overflow: hidden;
}
.fill {
  display: block;
  height: 100%;
  border-radius: var(--radius-pill);
  background: var(--accent);
}
</style>
