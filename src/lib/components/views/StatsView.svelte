<script module>
  /** Spotify's own top lists, by time range. @type {Map<string, { tracks: import("../../types.js").Track[], artists: import("../../types.js").ArtistDetails[] }>} */
  const cache = new Map();

  export function clearStatsCache() {
    cache.clear();
  }
</script>

<script>
  import { untrack } from "svelte";
  import * as api from "../../api.js";
  import { tracker, allPlays, asTrack, isPlayable, fillInTracks, importSpotifyHistory, FROM_SPOTILARP, FROM_SPOTIFY, FROM_IMPORT } from "../../tracker.svelte.js";
  import { rangeOf, playsBetween, totals, topSongs, topArtists, topAlbums, overTime, byHour, streaks, firstHeard, bySource, yearsPlayed } from "../../trackerStats.js";
  import { navigate, openAlbum, openArtist } from "../../nav.svelte.js";
  import { compactDuration, coverAtLeast, formatCount } from "../../utils.js";
  import { cached, MINUTE } from "../../requestCache.js";
  import ListeningChart from "../ListeningChart.svelte";
  import ListeningHeatmap from "../ListeningHeatmap.svelte";
  import MediaCard from "../MediaCard.svelte";
  import RepeatList from "../RepeatList.svelte";
  import Section from "../Section.svelte";
  import Shelf from "../Shelf.svelte";
  import TrackList from "../TrackList.svelte";

  /**
   * Your stats: the tracker's (every play in SpotiLarp, on Spotify's other
   * apps and from imported history) for any stretch of time, then Spotify's
   * own ranking next to them.
   * @typedef {import("../../trackerStats.js").RangeKey} RangeKey
   * @typedef {"short_term"|"medium_term"|"long_term"} SpotifyTerm
   */

  const SONGS_PREVIEW = 10;
  const SONGS_MAX = 50;
  const CARDS = 24;
  /** Artist pictures looked up one by one (Spotify's lists have most), kept for a week. */
  const PICTURE_LOOKUPS = 12;
  const PICTURE_FRESH_MS = 7 * 24 * 60 * MINUTE;
  const GENRES_SHOWN = 10;
  const SPOTIFY_TRACKS_PREVIEW = 10;

  /** @type {Partial<Record<string, SpotifyTerm>>} */
  const SPOTIFY_TERMS = { "7d": "short_term", "4w": "short_term", "6m": "medium_term" };
  /** @type {Record<SpotifyTerm, string>} */
  const TERM_LABELS = { short_term: "the last 4 weeks", medium_term: "the last 6 months", long_term: "all time" };
  /** @type {Partial<Record<string, string>>} */
  const PREVIOUS_LABELS = { "7d": "the week before", "4w": "the 4 weeks before", "6m": "the 6 months before", year: "this time last year" };

  /** @type {RangeKey} */
  let rangeKey = $state("4w");
  let showAllSongs = $state(false);
  let showAllSpotifyTracks = $state(false);

  // ---- The tracker ----

  const plays = $derived(allPlays());
  const years = $derived(yearsPlayed(plays).filter((year) => year !== new Date().getFullYear()));
  const chips = $derived(
    /** @type {{ key: RangeKey, label: string }[]} */ ([
      { key: "7d", label: "7 days" },
      { key: "4w", label: "4 weeks" },
      { key: "6m", label: "6 months" },
      { key: "year", label: "This year" },
      ...years.map((year) => ({ key: /** @type {RangeKey} */ (`y${year}`), label: String(year) })),
      { key: "all", label: "All time" },
    ]),
  );
  const range = $derived(rangeOf(rangeKey, plays));
  const inRange = $derived(playsBetween(plays, range.from, range.to));
  const sum = $derived(totals(inRange));
  const before = $derived(range.previous ? totals(playsBetween(plays, range.previous.from, range.previous.to)) : null);
  const previousLabel = $derived(PREVIOUS_LABELS[rangeKey] ?? (rangeKey.startsWith("y") ? String(Number(rangeKey.slice(1)) - 1) : ""));
  const first = $derived(firstHeard(plays));
  const newSongs = $derived(inRange.reduce((count, play) => (first.get(play.track.uri) === play.t ? count + 1 : count), 0));
  const streak = $derived(streaks(plays));
  const songs = $derived(topSongs(inRange.filter((play) => isPlayable(play.track)), SONGS_MAX));
  const artists = $derived(topArtists(inRange, CARDS));
  const albums = $derived(topAlbums(inRange, CARDS));
  const buckets = $derived(overTime(inRange, range));
  const heat = $derived(byHour(inRange));
  const sources = $derived(bySource(inRange));
  const imported = $derived(bySource(plays)[FROM_IMPORT] > 0);

  const sinceFormat = new Intl.DateTimeFormat(undefined, { month: "short", year: "numeric" });
  const songItems = $derived(
    songs.slice(0, showAllSongs ? SONGS_MAX : SONGS_PREVIEW).map((song) => ({
      track: asTrack(song.track),
      plays: song.plays,
      note: `${compactDuration(song.ms)} · since ${sinceFormat.format(first.get(song.track.uri) ?? 0)}`,
    })),
  );

  /**
   * "32% more than the 4 weeks before".
   * @param {number} now
   * @param {number|undefined} then
   */
  function change(now, then) {
    if (!then || !previousLabel) return "";
    const percent = Math.round(((now - then) / then) * 100);
    if (percent === 0) return `Same as ${previousLabel}`;
    return `${Math.abs(percent)}% ${percent > 0 ? "more" : "less"} than ${previousLabel}`;
  }

  // Imported history comes without pictures or artist links: looked up for what's on screen.
  $effect(() => {
    const wanted = [...songs.map((s) => s.track), ...artists.map((a) => a.track), ...albums.map((a) => a.track)];
    untrack(() => fillInTracks(wanted));
  });

  // ---- Artist pictures ----

  /** By artist id. @type {Record<string, string|null>} */
  let pictures = $state({});

  // After Spotify's lists, which have most of them.
  $effect(() => {
    if (!spotifyAsked) return;
    const missing = artists.filter((a) => a.id && pictures[a.id] === undefined).slice(0, PICTURE_LOOKUPS);
    for (const artist of missing) {
      const id = artist.id;
      pictures[id] = null;
      cached(`artist-picture:${id}`, PICTURE_FRESH_MS, () => api.getArtist(id).then((found) => coverAtLeast(found.images, 300)))
        .then((url) => (pictures[id] = url))
        .catch(() => {});
    }
  });

  /** @param {{ id: string, name: string }} artist */
  function openTrackerArtist(artist) {
    if (artist.id) openArtist(artist);
    else navigate({ type: "search", query: artist.name });
  }

  /** @param {{ id: string, name: string, artist: string }} album */
  function openTrackerAlbum(album) {
    if (album.id) openAlbum(album);
    else navigate({ type: "search", query: `${album.name} ${album.artist}` });
  }

  // ---- Spotify's own ranking ----

  /** @type {SpotifyTerm} */
  const term = $derived(SPOTIFY_TERMS[rangeKey] ?? "long_term");
  /** @type {import("../../types.js").Track[]} */
  let spotifyTracks = $state([]);
  /** @type {import("../../types.js").ArtistDetails[]} */
  let spotifyArtists = $state([]);
  let spotifyError = $state("");
  let spotifyAsked = $state(false);

  $effect(() => {
    const selected = term;
    untrack(() => loadSpotify(selected));
  });

  /** @param {SpotifyTerm} selected */
  async function loadSpotify(selected) {
    const saved = cache.get(selected);
    if (saved) {
      showSpotify(saved.tracks, saved.artists);
      return;
    }
    try {
      const [topTracks, topArtistList] = await Promise.all([api.getTopTracks(selected, 50), api.getTopArtists(selected, 50)]);
      cache.set(selected, { tracks: topTracks, artists: topArtistList });
      if (term === selected) showSpotify(topTracks, topArtistList);
    } catch (e) {
      if (term === selected) spotifyError = `Couldn't load Spotify's ranking: ${e}`;
    } finally {
      spotifyAsked = true;
    }
  }

  /**
   * @param {import("../../types.js").Track[]} topTracks
   * @param {import("../../types.js").ArtistDetails[]} topArtistList
   */
  function showSpotify(topTracks, topArtistList) {
    spotifyTracks = topTracks;
    spotifyArtists = topArtistList;
    spotifyError = "";
    spotifyAsked = true;
    // Spotify's pictures fill in the tracker's artists too.
    for (const artist of topArtistList) pictures[artist.id] ||= coverAtLeast(artist.images, 300);
  }

  /** Where each artist is in the tracker's list, by id, to show next to Spotify's. */
  const trackerRank = $derived(new Map(artists.map((artist, i) => [artist.id, i + 1])));

  const genres = $derived.by(() => {
    /** @type {Map<string, number>} */
    const scores = new Map();
    spotifyArtists.forEach((artist, rank) => {
      const weight = spotifyArtists.length - rank;
      for (const genre of artist.genres) scores.set(genre, (scores.get(genre) ?? 0) + weight);
    });
    const ranked = [...scores.entries()].sort((a, b) => b[1] - a[1]).slice(0, GENRES_SHOWN);
    const top = ranked[0]?.[1] ?? 1;
    return ranked.map(([name, score]) => ({ name, share: score / top }));
  });
</script>

<div class="stats">
  <header class="header">
    <h1 class="title">Your stats</h1>
    <button type="button" class="secondary" onclick={importSpotifyHistory} disabled={tracker.importing}>
      {tracker.importing ? "Importing..." : "Import Spotify history"}
    </button>
  </header>

  <div class="tabs" role="tablist">
    {#each chips as chip (chip.key)}
      <button
        type="button"
        role="tab"
        class="tab"
        class:active={rangeKey === chip.key}
        aria-selected={rangeKey === chip.key}
        onclick={() => (rangeKey = chip.key)}
      >
        {chip.label}
      </button>
    {/each}
  </div>

  {#if tracker.loaded && !imported}
    <div class="note">
      <p>
        Your stats start when SpotiLarp started counting. To go back to when you joined Spotify, ask for your data:
        Spotify account → Privacy → Download your data → <b>Extended streaming history</b> (it takes a few days to
        arrive). Unzip it, then import the folder.
      </p>
    </div>
  {/if}

  {#if !tracker.loaded}
    <p class="status">Loading...</p>
  {:else if sum.plays === 0}
    <p class="status">Nothing played in {range.label}.</p>
  {:else}
    <dl class="kpis">
      <div class="kpi lead">
        <dt>Time listened</dt>
        <dd class="figure">{compactDuration(sum.ms)}</dd>
        <dd class="sub">{change(sum.ms, before?.ms) || `In ${range.label}`}</dd>
      </div>
      <div class="kpi">
        <dt>Plays</dt>
        <dd class="value">{formatCount(sum.plays)}</dd>
        <dd class="sub">{Math.round((sum.skipped / sum.plays) * 100)}% skipped</dd>
      </div>
      <div class="kpi">
        <dt>Songs</dt>
        <dd class="value">{formatCount(sum.songs)}</dd>
        <dd class="sub">{formatCount(newSongs)} heard for the first time</dd>
      </div>
      <div class="kpi">
        <dt>Artists</dt>
        <dd class="value">{formatCount(sum.artists)}</dd>
        <dd class="sub">{change(sum.artists, before?.artists) || (artists[0] ? `Most played: ${artists[0].name}` : "")}</dd>
      </div>
      <div class="kpi">
        <dt>Streak</dt>
        <dd class="value">{streak.current} {streak.current === 1 ? "day" : "days"}</dd>
        <dd class="sub">Longest: {streak.longest} {streak.longest === 1 ? "day" : "days"}</dd>
      </div>
    </dl>

    <div class="charts">
      <section class="panel">
        <h2 class="panel-title">Listening over time</h2>
        <ListeningChart {buckets} bucket={range.bucket} />
      </section>
      <section class="panel">
        <h2 class="panel-title">When you listen</h2>
        <ListeningHeatmap grid={heat} />
      </section>
    </div>

    {#if songItems.length > 0}
      <Section title="Top songs">
        {#snippet action()}
          {#if songs.length > SONGS_PREVIEW}
            <button type="button" class="show-all" onclick={() => (showAllSongs = !showAllSongs)}>
              {showAllSongs ? "Show less" : `Show ${songs.length}`}
            </button>
          {/if}
        {/snippet}
        <RepeatList items={songItems} contextName={`Top songs: ${range.label}`} />
      </Section>
    {/if}

    {#if artists.length > 0}
      <Shelf title="Top artists" items={artists}>
        {#snippet card(artist, i)}
          <MediaCard
            image={pictures[artist.id] ?? null}
            title={`${i + 1}. ${artist.name}`}
            subtitle={`${formatCount(artist.plays)} plays · ${compactDuration(artist.ms)}`}
            round
            onclick={() => openTrackerArtist(artist)}
          />
        {/snippet}
      </Shelf>
    {/if}

    {#if albums.length > 0}
      <Shelf title="Top albums" items={albums}>
        {#snippet card(album, i)}
          <MediaCard
            image={album.image}
            title={`${i + 1}. ${album.name}`}
            subtitle={`${album.artist} · ${formatCount(album.plays)} plays`}
            onclick={() => openTrackerAlbum(album)}
          />
        {/snippet}
      </Shelf>
    {/if}

    <p class="sources">
      From {formatCount(sum.plays)} plays: {formatCount(sources[FROM_SPOTILARP])} in SpotiLarp,
      {formatCount(sources[FROM_SPOTIFY])} on Spotify's other apps{#if sources[FROM_IMPORT] > 0}, {formatCount(sources[FROM_IMPORT])} imported{/if}.
    </p>
  {/if}

  <section class="spotify">
    <div class="spotify-heading">
      <h2>Spotify's ranking</h2>
      <p class="caption">
        What Spotify counts for {TERM_LABELS[term]}, from every app you use. It ranks by its own rules and doesn't say
        how many plays.
      </p>
    </div>

    {#if spotifyError}
      <p class="error">{spotifyError}</p>
    {:else}
      {#if spotifyArtists.length > 0}
        <Shelf title="Top artists on Spotify" items={spotifyArtists.slice(0, CARDS)}>
          {#snippet card(artist, i)}
            {@const yours = trackerRank.get(artist.id)}
            <MediaCard
              image={coverAtLeast(artist.images, 300)}
              title={`${i + 1}. ${artist.name}`}
              subtitle={yours ? `#${yours} in your stats` : artist.genres.slice(0, 2).join(", ") || "Artist"}
              round
              onclick={() => openArtist(artist)}
            />
          {/snippet}
        </Shelf>
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

      {#if spotifyTracks.length > 0}
        <Section title="Top songs on Spotify">
          {#snippet action()}
            <button type="button" class="show-all" onclick={() => (showAllSpotifyTracks = !showAllSpotifyTracks)}>
              {showAllSpotifyTracks ? "Show less" : `Show ${spotifyTracks.length}`}
            </button>
          {/snippet}
          <TrackList
            tracks={showAllSpotifyTracks ? spotifyTracks : spotifyTracks.slice(0, SPOTIFY_TRACKS_PREVIEW)}
            allTracks={spotifyTracks}
            contextName={`Spotify's top songs: ${TERM_LABELS[term]}`}
            contextView={{ type: "stats" }}
            primary
          />
        </Section>
      {/if}
    {/if}
  </section>
</div>

<style>
.stats {
  container: stats / inline-size;
}
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1em;
  margin-bottom: 0.6em;
}
.title {
  margin: 0;
  font-size: var(--fs-xl);
  font-weight: var(--fw-black);
  letter-spacing: -0.02em;
  color: var(--text);
}
.secondary {
  flex-shrink: 0;
  padding: 0.45em 1.1em;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-pill);
  background: none;
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
  transition: border-color var(--transition);
}
.secondary:hover:not(:disabled) {
  border-color: var(--text);
}
.secondary:disabled {
  opacity: 0.5;
  cursor: default;
}
.tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5em;
  margin-bottom: 1.5em;
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
  background: var(--control-hover);
}
.tab.active {
  background: var(--text);
  color: var(--bg);
}
.note {
  margin-bottom: 1.5em;
  padding: 0.8em 1em;
  border-radius: var(--radius-md);
  background: var(--surface-raised);
}
.note p {
  margin: 0;
  max-width: 80ch;
  font-size: var(--fs-sm);
  line-height: 1.5;
  color: var(--text-dim);
}
.note b {
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
.kpis {
  display: grid;
  grid-template-columns: minmax(0, 1.6fr) repeat(4, minmax(0, 1fr));
  gap: 1em;
  margin: 0 0 1em;
}
.kpi {
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  min-width: 0;
  padding: 1em 1.1em;
  border-radius: var(--radius-lg);
  background: var(--surface-raised);
}
.kpi dt {
  order: -1;
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-dim);
}
.kpi dd {
  margin: 0;
}
.figure {
  margin-top: 0.1em;
  font-size: 3.2em;
  font-weight: var(--fw-semibold);
  letter-spacing: -0.03em;
  line-height: 1.05;
  font-variant-numeric: tabular-nums;
  color: var(--text);
}
.value {
  margin-top: 0.3em;
  font-size: 1.6em;
  font-weight: var(--fw-semibold);
  letter-spacing: -0.02em;
  font-variant-numeric: tabular-nums;
  color: var(--text);
}
.sub {
  margin-top: 0.3em;
  font-size: var(--fs-sm);
  color: var(--text-muted);
}
.charts {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  gap: 1em;
  margin-bottom: 2.5em;
}
.panel {
  min-width: 0;
  padding: 1.1em 1.2em;
  border-radius: var(--radius-lg);
  background: var(--surface-raised);
}
.panel-title {
  margin: 0 0 0.8em;
  font-size: var(--fs-md);
  font-weight: var(--fw-bold);
  color: var(--text);
}
@container stats (max-width: 980px) {
  .kpis {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .lead {
    grid-column: 1 / -1;
  }
  .charts {
    grid-template-columns: minmax(0, 1fr);
  }
}
.show-all {
  flex-shrink: 0;
  padding: 0;
  border: none;
  background: none;
  color: var(--text-muted);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.show-all:hover {
  color: var(--text);
  text-decoration: underline;
}
.sources {
  margin: -1em 0 2.5em;
  font-size: var(--fs-sm);
  color: var(--text-muted);
}
.spotify {
  padding-top: 2em;
  border-top: 1px solid var(--border);
}
.spotify-heading {
  margin-bottom: 1.5em;
}
.spotify-heading h2 {
  margin: 0;
  font-size: var(--fs-lg);
  font-weight: var(--fw-black);
  letter-spacing: -0.02em;
  color: var(--text);
}
.caption {
  margin: 0.3em 0 0;
  max-width: 80ch;
  font-size: var(--fs-sm);
  color: var(--text-muted);
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
