<script module>
  const NEW_EPISODES_FRESH_MS = 15 * 60 * 1000;
  /** @type {{ key: string, at: number, episodes: import("../../types.js").Track[] }} */
  let newEpisodesCache = { key: "", at: 0, episodes: [] };
  /** The last new releases found, by the artists they were looked up for, so coming back to Home doesn't flash. */
  let newReleasesCache = { key: "", releases: /** @type {{ album: import("../../types.js").Album, artist: string }[]} */ ([]) };
</script>

<script>
  import { untrack } from "svelte";
  import * as api from "../../api.js";
  import { queue, playFromList } from "../../queue.svelte.js";
  import { player, togglePlay } from "../../player.svelte.js";
  import { library } from "../../library.svelte.js";
  import { allPlays, asTrack, isPlayable } from "../../tracker.svelte.js";
  import { lastPlayed, recentContexts, viewImage, isLikedSongs } from "../../played.svelte.js";
  import { pins } from "../../pins.svelte.js";
  import { navigate, openAlbum, openRadio, viewKey } from "../../nav.svelte.js";
  import { playAlbum, playArtist, playPlaylist, playRadio } from "../../collections.js";
  import { episodeTrack, continueListening, isPlayed } from "../../podcasts.svelte.js";
  import { dominantColor } from "../../colors.js";
  import { coverAtLeast, releaseDay, releaseType, timeAgo } from "../../utils.js";
  import { cached, MINUTE } from "../../requestCache.js";
  import { LIKED_SONGS_ID } from "../../constants.js";
  import EpisodeCard from "../EpisodeCard.svelte";
  import HomeHero from "../HomeHero.svelte";
  import Icon from "../Icon.svelte";
  import MediaCard from "../MediaCard.svelte";
  import RepeatList from "../RepeatList.svelte";
  import Section from "../Section.svelte";
  import Shelf from "../Shelf.svelte";
  import WeekStats from "../WeekStats.svelte";

  /** @type {{ loggedIn: boolean, loggingIn?: boolean, onRelogin: () => void }} */
  let { loggedIn, loggingIn = false, onRelogin } = $props();

  /**
   * @typedef {import("../../types.js").Track} Track
   * @typedef {import("../../types.js").Album} Album
   * @typedef {import("../../types.js").ArtistDetails} ArtistDetails
   * @typedef {import("../../nav.svelte.js").View} View
   */

  const JUMP_BACK_IN = 18;
  /** On repeat counts the tracker's plays over this many days. */
  const REPEAT_DAYS = 30;
  const REPEAT_SHOWN = 10;
  const REPEAT_MIN_PLAYS = 2;
  /** Below this many songs on repeat, Spotify's top songs this month stand in. */
  const REPEAT_ENOUGH = 5;
  const MIXES = 8;
  /** New releases come from this many top artists (one request each, kept for hours). */
  const RELEASE_ARTISTS = 8;
  const RELEASE_DAYS = 60;
  const RELEASE_FRESH_MS = 12 * 60 * MINUTE;
  /** New episodes come from this many podcasts, most recently followed first (one request each). */
  const NEW_EPISODE_SHOWS = 8;
  const NEW_EPISODE_DAYS = 14;
  const PODCAST_EPISODES = 12;

  /** @type {Record<string, string>} */
  const KINDS = { playlist: "Playlist", album: "Album", artist: "Artist", radio: "Radio", show: "Podcast" };

  /** @type {Track[]} */
  let topTracks = $state([]);
  /** @type {ArtistDetails[]} */
  let topArtists = $state([]);
  /** @type {Track[]} */
  let newEpisodes = $state([]);
  let newEpisodesFor = "";
  /** @type {{ album: Album, artist: string }[]} */
  let newReleases = $state(newReleasesCache.releases);
  let newReleasesFor = "";
  /** Mix card colors, by artist id. @type {Record<string, string>} */
  let mixTints = $state({});

  const likedSongs = {
    id: LIKED_SONGS_ID,
    name: "Liked Songs",
    isLikedSongs: true,
    images: [],
    track_count: { total: 0 },
  };

  const greeting = $derived.by(() => {
    const hour = new Date().getHours();
    return hour < 12 ? "Good morning" : hour < 18 ? "Good afternoon" : "Good evening";
  });

  // ---- Jump back in: the pages played from lately ----

  /** @typedef {{ key: string, view: View, image: string|null, at: number }} Page `at` is 0 for never played */

  /** Your Library as pages, by view key, in the sidebar's order. */
  const libraryPages = $derived.by(() => {
    /** @type {Map<string, View>} */
    const pages = new Map();
    pages.set(`playlist:${LIKED_SONGS_ID}`, { type: "playlist", playlist: likedSongs });
    for (const playlist of library.playlists) pages.set(`playlist:${playlist.id}`, { type: "playlist", playlist });
    for (const album of library.albums) {
      if (album.id) pages.set(`album:${album.id}`, { type: "album", id: album.id, name: album.name, image: coverAtLeast(album.images, 300) });
    }
    for (const show of library.shows) {
      pages.set(`show:${show.id}`, { type: "show", id: show.id, name: show.name, image: coverAtLeast(show.images, 300) });
    }
    return pages;
  });

  /**
   * Pages remembered with their pictures, then library items played from
   * before Home remembered pages (the sidebar's "Recents" knows them by key).
   */
  const recentPages = $derived.by(() => {
    /** @type {Map<string, Page>} */
    const pages = new Map(recentContexts.items.map((item) => [item.key, item]));
    for (const [key, at] of lastPlayed) {
      const view = libraryPages.get(key);
      if (view && !pages.has(key)) pages.set(key, { key, view, image: viewImage(view), at });
    }
    return [...pages.values()].sort((a, b) => b.at - a.at);
  });

  // The hero already shows what's playing. Until enough has been played from,
  // the library fills out the row: pinned first, then the sidebar's order.
  const playingKey = $derived(queue.contextView ? viewKey(queue.contextView) : "");
  const jumpBackIn = $derived.by(() => {
    const hidden = player.track ? playingKey : "";
    const shown = recentPages.filter((page) => page.key !== hidden).slice(0, JUMP_BACK_IN);
    const seen = new Set([hidden, ...shown.map((page) => page.key)]);
    for (const key of [...pins.keys, ...libraryPages.keys()]) {
      if (shown.length >= JUMP_BACK_IN) break;
      const view = libraryPages.get(key);
      if (!view || seen.has(key)) continue;
      seen.add(key);
      shown.push({ key, view, image: viewImage(view), at: 0 });
    }
    return shown;
  });

  /** @param {Page} page */
  function pageSubtitle(page) {
    const kind = KINDS[page.view.type] ?? "";
    if (page.at) return `${kind} · ${timeAgo(page.at)}`;
    const owner = page.view.type === "playlist" ? page.view.playlist.owner?.display_name : null;
    return owner ? `By ${owner}` : kind;
  }

  /** @param {View} view */
  function pageTitle(view) {
    if (view.type === "playlist") return view.playlist.name;
    if (view.type === "radio") return `${view.name} Radio`;
    return ("name" in view && view.name) || KINDS[view.type] || "";
  }

  /**
   * @param {View} view
   * @returns {(() => void)|undefined}
   */
  function pagePlayer(view) {
    switch (view.type) {
      case "playlist":
        return () => playPlaylist(view.playlist);
      case "album":
        return () => playAlbum({ id: view.id, name: view.name ?? "", images: [] });
      case "artist":
        return () => playArtist(view);
      case "radio":
        return () => playRadio(view.seed, view.name, view.image ?? null);
    }
  }

  const heroFallback = $derived.by(() => {
    const page = recentPages[0];
    if (!page) return null;
    return {
      title: pageTitle(page.view),
      kind: KINDS[page.view.type] ?? "",
      image: page.image,
      liked: isLikedSongs(page.view),
      at: page.at,
      open: () => navigate(page.view),
      play: pagePlayer(page.view),
    };
  });

  // ---- On repeat: most played lately, from the tracker ----

  const onRepeat = $derived.by(() => {
    const since = Date.now() - REPEAT_DAYS * 86_400_000;
    /** @type {Map<string, { track: import("../../types.js").TrackerTrack, plays: number, last: number }>} */
    const counts = new Map();
    const plays = allPlays();
    for (let i = plays.length - 1; i >= 0 && plays[i].t >= since; i--) {
      const { track, t } = plays[i];
      if (!isPlayable(track)) continue;
      const seen = counts.get(track.uri);
      if (seen) seen.plays++;
      else counts.set(track.uri, { track, plays: 1, last: t });
    }
    /** @type {{ track: Track, plays: number|null }[]} */
    const items = [...counts.values()]
      .filter((item) => item.plays >= REPEAT_MIN_PLAYS)
      .sort((a, b) => b.plays - a.plays || b.last - a.last)
      .slice(0, REPEAT_SHOWN)
      .map(({ track, plays }) => ({ track: asTrack(track), plays }));
    return items;
  });
  const repeatFromSpotify = $derived(onRepeat.length < REPEAT_ENOUGH && topTracks.length > 0);
  const repeatItems = $derived(
    repeatFromSpotify ? topTracks.slice(0, REPEAT_SHOWN).map((track) => ({ track, plays: null })) : onRepeat,
  );

  // ---- Your mixes: radio from your top artists ----

  const mixes = $derived(topArtists.slice(0, MIXES));
  const artistImages = $derived(new Map(topArtists.map((artist) => [artist.id, coverAtLeast(artist.images, 160)])));

  $effect(() => {
    for (const artist of mixes) {
      if (mixTints[artist.id] !== undefined) continue;
      const id = artist.id;
      dominantColor(coverAtLeast(artist.images, 300)).then((color) => (mixTints[id] = color));
    }
  });

  /** @param {ArtistDetails} artist */
  function mixSeed(artist) {
    return `spotify:artist:${artist.id}`;
  }

  // ---- New releases from your top artists ----

  $effect(() => {
    const artists = topArtists.slice(0, RELEASE_ARTISTS);
    const key = artists.map((a) => a.id).join(",");
    if (!loggedIn || !key || key === newReleasesFor) return;
    newReleasesFor = key;
    untrack(() => loadNewReleases(artists, key));
  });

  /**
   * One artist page each, through the playback session; without one, the
   * Web API would take several requests per artist, so it waits for one.
   * @param {ArtistDetails[]} artists
   * @param {string} key
   */
  async function loadNewReleases(artists, key) {
    if (newReleasesCache.key === key) {
      newReleases = newReleasesCache.releases;
      return;
    }
    const connected = await api.hasConnectSession().catch(() => false);
    if (!connected || key !== newReleasesFor) return;
    const since = new Date(Date.now() - RELEASE_DAYS * 86_400_000).toISOString().slice(0, 10);
    /** @type {{ album: Album, artist: string }[]} */
    const found = [];
    // Two at a time, so a first Home doesn't ask for them all at once.
    for (let i = 0; i < artists.length; i += 2) {
      const batch = artists.slice(i, i + 2);
      const latest = await Promise.all(
        batch.map((artist) =>
          cached(`latest-release:${artist.id}`, RELEASE_FRESH_MS, () =>
            api.getArtistPage(artist.id).then((page) => page.latest_release),
          ).catch(() => null),
        ),
      );
      if (key !== newReleasesFor) return;
      latest.forEach((album, j) => {
        if (album?.id && (album.release_date ?? "") >= since && !found.some((f) => f.album.id === album.id)) {
          found.push({ album, artist: batch[j].name });
        }
      });
    }
    found.sort((a, b) => (b.album.release_date ?? "").localeCompare(a.album.release_date ?? ""));
    newReleases = found;
    newReleasesCache = { key, releases: found };
  }

  // ---- Podcasts: one row, started episodes first ----

  const continuing = $derived(continueListening(PODCAST_EPISODES));
  const podcastEpisodes = $derived.by(() => {
    const started = new Set(continuing.map((e) => e.uri));
    const fresh = newEpisodes.filter((e) => !isPlayed(e) && !started.has(e.uri));
    return [...continuing, ...fresh].slice(0, PODCAST_EPISODES);
  });

  /**
   * @param {Track[]} episodes
   * @param {number} index
   */
  function playEpisode(episodes, index) {
    if (player.track?.uri === episodes[index].uri) togglePlay();
    else playFromList(episodes, index, "Podcasts");
  }

  // Follows the podcasts in Your Library (loaded after Home first shows).
  $effect(() => {
    const shows = library.shows.slice(0, NEW_EPISODE_SHOWS);
    const key = shows.map((s) => s.id).join(",");
    if (!loggedIn || key === newEpisodesFor) return;
    newEpisodesFor = key;
    untrack(() => loadNewEpisodes(shows));
  });

  /** @param {import("../../types.js").Show[]} shows */
  async function loadNewEpisodes(shows) {
    if (shows.length === 0) {
      newEpisodes = [];
      return;
    }
    const key = newEpisodesFor;
    if (newEpisodesCache.key === key && Date.now() - newEpisodesCache.at < NEW_EPISODES_FRESH_MS) {
      newEpisodes = newEpisodesCache.episodes;
      return;
    }
    try {
      const ids = shows.map((s) => s.id);
      const latest = await cached(`latest-episodes:${ids.join(",")}`, 30 * MINUTE, () => api.getLatestEpisodes(ids, 3));
      if (key !== newEpisodesFor) return;
      const since = new Date(Date.now() - NEW_EPISODE_DAYS * 86400000).toISOString().slice(0, 10);
      newEpisodes = latest
        .flatMap((episodes, i) => episodes.map((episode) => episodeTrack(episode, shows[i])))
        .filter((episode) => (episode.release_date ?? "") >= since)
        .sort((a, b) => (b.release_date ?? "").localeCompare(a.release_date ?? ""));
      newEpisodesCache = { key, at: Date.now(), episodes: newEpisodes };
    } catch {}
  }

  // ---- Loading ----

  let loadedForLogin = false;
  $effect(() => {
    if (!loggedIn) {
      loadedForLogin = false;
      topTracks = [];
      topArtists = [];
      newEpisodes = [];
      newEpisodesFor = "";
      newReleases = [];
      newReleasesFor = "";
    } else if (!loadedForLogin) {
      loadedForLogin = true;
      untrack(load);
    }
  });

  // Kept a little while, so reopening the app doesn't ask for them all again.
  function load() {
    cached("top-tracks:short_term:20", 60 * MINUTE, () => api.getTopTracks("short_term", 20))
      .then((tracks) => (topTracks = tracks))
      .catch(() => {});
    cached("top-artists:medium_term:12", 60 * MINUTE, () => api.getTopArtists("medium_term", 12))
      .then((artists) => (topArtists = artists))
      .catch(() => {});
  }
</script>

<div class="home">
  <h1 class="greeting">{greeting}</h1>

  {#if !loggedIn && loggingIn}
    <p class="status">
      Finish logging in in your browser. If Spotify shows an error like <span class="mono">INVALID_CLIENT</span>,
      check your Client ID and that your app's Redirect URI is exactly
      <span class="mono">http://127.0.0.1:8898/callback</span>, then cancel and try again.
    </p>
  {:else if !loggedIn}
    <p class="status">
      Log in with Spotify to see your music here, or play your own in
      <button type="button" class="inline-link" onclick={() => navigate({ type: "local" })}>Local Files</button>.
    </p>
  {:else}
    {#if library.missingScopes.length > 0}
      <div class="banner">
        <span>Log in again to see your listening history and edit your playlists.</span>
        <button type="button" class="banner-button" onclick={onRelogin}>Log in again</button>
      </div>
    {/if}

    <div class="dashboard">
      <HomeHero fallback={heroFallback} />
      <WeekStats {artistImages} />
    </div>

    {#if jumpBackIn.length > 0}
      <Shelf title="Jump back in" items={jumpBackIn}>
        {#snippet card(page)}
          {@const view = /** @type {View} */ (page.view)}
          {@const liked = isLikedSongs(view)}
          <MediaCard
            image={page.image}
            title={pageTitle(view)}
            subtitle={pageSubtitle(page)}
            round={view.type === "artist"}
            onclick={() => navigate(view)}
            onplay={pagePlayer(view)}
            overlay={liked ? likedCover : undefined}
          />
        {/snippet}
      </Shelf>
    {/if}

    {#if repeatItems.length > 0}
      <Section title="On repeat">
        {#snippet before()}
          <p class="caption">
            {repeatFromSpotify
              ? "Your top songs on Spotify this month, until the tracker has heard enough of yours."
              : `Your most played over the last ${REPEAT_DAYS} days.`}
          </p>
        {/snippet}
        <RepeatList items={repeatItems} contextName="On repeat" />
      </Section>
    {/if}

    {#if mixes.length > 0}
      <Shelf title="Your mixes" items={mixes}>
        {#snippet card(artist)}
          {@const image = coverAtLeast(artist.images, 300)}
          <MediaCard
            {image}
            title={`${artist.name} Radio`}
            subtitle={artist.genres.slice(0, 2).join(", ") || "Artist radio"}
            onclick={() => openRadio(mixSeed(artist), artist.name, image)}
            onplay={() => playRadio(mixSeed(artist), artist.name, image)}
          >
            {#snippet overlay()}
              <div class="mix" style:--mix-tint={mixTints[artist.id] || null}>
                <span class="mix-label">Mix</span>
                <span class="mix-name">{artist.name}</span>
              </div>
            {/snippet}
          </MediaCard>
        {/snippet}
      </Shelf>
    {/if}

    {#if newReleases.length > 0}
      <Shelf title="New releases" items={newReleases}>
        {#snippet card(release)}
          <MediaCard
            image={coverAtLeast(release.album.images, 300)}
            title={release.album.name}
            subtitle={`${releaseType(release.album.album_type)} · ${release.artist} · ${releaseDay(release.album.release_date)}`}
            onclick={() => openAlbum(release.album)}
            onplay={() => playAlbum(release.album)}
          />
        {/snippet}
      </Shelf>
    {/if}

    {#if podcastEpisodes.length > 0}
      <Shelf title="Podcasts" items={podcastEpisodes}>
        {#snippet card(episode, i)}
          <EpisodeCard {episode} onplay={() => playEpisode(podcastEpisodes, i)} />
        {/snippet}
      </Shelf>
    {/if}
  {/if}
</div>

{#snippet likedCover()}
  <div class="liked"><Icon name="heart-filled" size={56} /></div>
{/snippet}

<style>
.home {
  container: home / inline-size;
}
.greeting {
  margin: 0 0 0.6em;
  font-size: var(--fs-xl);
  font-weight: var(--fw-black);
  letter-spacing: -0.02em;
  color: var(--text);
}
.status {
  max-width: 60ch;
  font-size: var(--fs-sm);
  line-height: 1.5;
  color: var(--text-muted);
}
.mono {
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
  color: var(--text);
}
.inline-link {
  padding: 0;
  border: none;
  background: none;
  color: var(--text);
  font: inherit;
  text-decoration: underline;
  cursor: pointer;
}
.banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1em;
  margin-bottom: 1.5em;
  padding: 0.8em 1em;
  border-radius: var(--radius-md);
  background: var(--accent-bg);
  font-size: var(--fs-sm);
  color: var(--text);
}
.banner-button {
  flex-shrink: 0;
  padding: 0.45em 1.1em;
  border: none;
  border-radius: var(--radius-pill);
  background: var(--accent);
  color: var(--accent-text);
  font-family: inherit;
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.dashboard {
  display: grid;
  grid-template-columns: minmax(0, 2fr) minmax(260px, 1fr);
  gap: 1em;
  margin-bottom: 2.5em;
}
@container home (max-width: 760px) {
  .dashboard {
    grid-template-columns: minmax(0, 1fr);
  }
}
.caption {
  margin: -0.2em 0 0.8em;
  font-size: var(--fs-sm);
  color: var(--text-muted);
}
.mix {
  --mix-tint: var(--surface-hover);
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  gap: 0.15em;
  padding: 0.8em;
  background: linear-gradient(to top, var(--mix-tint) 0%, color-mix(in srgb, var(--mix-tint) 60%, transparent) 32%, transparent 62%);
  /* A strip down the side, so a mix reads as one even before its picture loads. */
  box-shadow: inset 6px 0 0 var(--mix-tint);
  color: #fff;
}
.mix-label {
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  letter-spacing: 0.12em;
  text-transform: uppercase;
  opacity: 0.85;
}
.mix-name {
  font-size: 1.3em;
  font-weight: var(--fw-black);
  letter-spacing: -0.02em;
  line-height: 1.1;
  text-shadow: 0 1px 8px rgba(0, 0, 0, 0.35);
  overflow-wrap: anywhere;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.liked {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #4b1fa8, #8f8ff0);
  color: #fff;
}
</style>
