<script module>
  /** @type {Map<string, import("../../types.js").ArtistPage>} */
  const cache = new Map();
</script>

<script>
  import { onMount, untrack } from "svelte";
  import { fade } from "svelte/transition";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import * as api from "../../api.js";
  import { queue, playFromList, toggleShuffle } from "../../queue.svelte.js";
  import { player, togglePlay } from "../../player.svelte.js";
  import { refreshLiked } from "../../likes.svelte.js";
  import { openAlbum, openArtist, openDiscography, openPlaylistCard } from "../../nav.svelte.js";
  import { notify } from "../../toast.svelte.js";
  import { dominantColor, tintFromHex } from "../../colors.js";
  import { coverAtLeast, formatCount, htmlToText, releaseType, releaseYear } from "../../utils.js";
  import TrackList from "../TrackList.svelte";
  import PlayButton from "../PlayButton.svelte";
  import MediaCard from "../MediaCard.svelte";
  import Shelf from "../Shelf.svelte";
  import ArtistAbout from "../ArtistAbout.svelte";
  import Icon from "../Icon.svelte";

  /** @type {{ id: string, name: string }} */
  let { id, name } = $props();

  const POPULAR_PREVIEW = 5;

  /**
   * @typedef {import("../../types.js").Album} Album
   * @typedef {import("../../types.js").PlaylistCard} PlaylistCard
   * @typedef {"popular"|"album"|"single"|"compilation"} ReleaseTab
   */

  const cached = untrack(() => cache.get(id));
  /** @type {import("../../types.js").ArtistPage|null} */
  let page = $state(cached ?? null);
  let error = $state("");
  let following = $state(cached?.following ?? false);
  let followBusy = $state(false);
  let showAllPopular = $state(false);
  let menuOpen = $state(false);
  let extractedTint = $state("");
  /** @type {ReleaseTab|null} */
  let releaseTab = $state(null);
  /** @type {HTMLDivElement|undefined} */
  let moreWrap = $state();

  const artistName = $derived(page?.name || name);
  const avatar = $derived(coverAtLeast(page?.images, 300));
  const banner = $derived(coverAtLeast(page?.header_images, 1600));
  const metadataTint = $derived(page?.color ? tintFromHex(page.color) : "");
  const tint = $derived(metadataTint || extractedTint);
  const topTracks = $derived(page?.top_tracks ?? []);
  const hasPlaycounts = $derived(topTracks.some((track) => track.playcount != null));
  const inArtistContext = $derived(queue.contextName === artistName && !!queue.current && !queue.fromManual);
  const link = $derived(`https://open.spotify.com/artist/${id}`);

  const popularReleases = $derived.by(() => {
    if (!page) return [];
    const latest = page.latest_release;
    if (!latest) return page.popular_releases;
    return [latest, ...page.popular_releases.filter((release) => release.id !== latest.id)];
  });
  const releaseTabs = $derived.by(() => {
    if (!page) return [];
    /** @type {{ key: ReleaseTab, label: string, items: Album[] }[]} */
    const tabs = [
      { key: "popular", label: "Popular releases", items: popularReleases },
      { key: "album", label: "Albums", items: page.albums },
      { key: "single", label: "Singles and EPs", items: page.singles },
      { key: "compilation", label: "Compilations", items: page.compilations },
    ];
    return tabs.filter((tab) => tab.items.length > 0);
  });
  const activeTab = $derived(releaseTabs.find((tab) => tab.key === releaseTab) ?? releaseTabs[0]);

  onMount(() => {
    api
      .getArtistPage(id)
      .then((result) => {
        page = result;
        cache.set(id, result);
        if (result.following != null) following = result.following;
        else api.isFollowingArtist(id).then((f) => (following = f)).catch(() => {});
        refreshLiked(result.top_tracks.map((track) => track.id));
      })
      .catch((e) => {
        if (page) notify(`Couldn't refresh ${artistName}: ${e}`);
        else error = `Failed to load artist: ${e}`;
      });
  });

  $effect(() => {
    if (metadataTint) return;
    const url = avatar;
    dominantColor(url).then((color) => {
      if (avatar === url) extractedTint = color;
    });
  });

  function playArtist() {
    if (inArtistContext) {
      togglePlay();
      return;
    }
    if (topTracks.length === 0) return;
    const start = queue.shuffle ? Math.floor(Math.random() * topTracks.length) : 0;
    playFromList(topTracks, start, artistName);
  }

  async function toggleFollow() {
    if (followBusy) return;
    followBusy = true;
    try {
      if (following) await api.unfollowArtist(id);
      else await api.followArtist(id);
      following = !following;
      const entry = cache.get(id);
      if (entry) entry.following = following;
    } catch (e) {
      notify(`Couldn't update follow: ${e}`);
    } finally {
      followBusy = false;
    }
  }

  /** @param {() => unknown} action */
  function fromMenu(action) {
    menuOpen = false;
    action();
  }

  async function copyLink() {
    try {
      await navigator.clipboard.writeText(link);
      notify("Link copied");
    } catch (e) {
      notify(`Couldn't copy the link: ${e}`);
    }
  }

  /** @param {Album} release */
  function releaseSubtitle(release) {
    const type = releaseType(release.album_type);
    if (release.id && release.id === page?.latest_release?.id) return `Latest Release • ${type}`;
    const year = releaseYear(release.release_date);
    return year ? `${year} • ${type}` : type;
  }

  /** @param {PlaylistCard} playlist */
  function playlistSubtitle(playlist) {
    return htmlToText(playlist.description) || (playlist.owner ? `By ${playlist.owner}` : "Playlist");
  }

  /** @param {MouseEvent} evt */
  function onWindowClick(evt) {
    if (menuOpen && !moreWrap?.contains(/** @type {Node} */ (evt.target))) menuOpen = false;
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={(e) => e.key === "Escape" && (menuOpen = false)} />

<div class="artist-page" style={`--artist-color: ${tint || "#535353"}`}>
  <header class="hero" class:banner={!!banner}>
    {#if banner}
      <img class="hero-image" src={banner} alt="" />
    {:else if avatar}
      <img class="avatar" src={avatar} alt="" />
    {:else}
      <div class="avatar placeholder"></div>
    {/if}
    <div class="hero-text">
      {#if page?.verified}
        <span class="verified"><Icon name="verified" size={24} />Verified Artist</span>
      {/if}
      <h1 class:long={artistName.length > 18} class:very-long={artistName.length > 32}>{artistName}</h1>
      {#if page?.monthly_listeners != null}
        <span class="audience">{formatCount(page.monthly_listeners)} monthly listeners</span>
      {:else if page?.followers != null}
        <span class="audience">{formatCount(page.followers)} followers</span>
      {/if}
    </div>
  </header>

  <div class="page-body">
    <div class="action-bar">
      <PlayButton
        size={56}
        onclick={playArtist}
        playing={inArtistContext && player.isPlaying}
        disabled={topTracks.length === 0 && !inArtistContext}
        label={`Play ${artistName}`}
      />
      <button
        type="button"
        class="icon-button"
        class:active={queue.shuffle}
        onclick={toggleShuffle}
        aria-label={queue.shuffle ? "Disable shuffle" : "Enable shuffle"}
        aria-pressed={queue.shuffle}
      >
        <Icon name="shuffle" size={28} />
      </button>
      <button type="button" class="follow-button" class:following disabled={followBusy} onclick={toggleFollow}>
        {following ? "Following" : "Follow"}
      </button>
      <div class="more-wrap" bind:this={moreWrap}>
        <button
          type="button"
          class="icon-button"
          onclick={() => (menuOpen = !menuOpen)}
          aria-label={`More options for ${artistName}`}
          aria-haspopup="menu"
          aria-expanded={menuOpen}
        >
          <Icon name="more" size={26} />
        </button>
        {#if menuOpen}
          <div class="menu" role="menu" transition:fade={{ duration: 100 }}>
            <button type="button" role="menuitem" onclick={() => fromMenu(toggleFollow)}>
              {following ? "Unfollow" : "Follow"}
            </button>
            <div class="divider"></div>
            <button type="button" role="menuitem" onclick={() => fromMenu(copyLink)}>Copy link to artist</button>
            <button
              type="button"
              role="menuitem"
              onclick={() => fromMenu(() => openUrl(link).catch((e) => notify(`Couldn't open Spotify: ${e}`)))}
            >
              Open in Spotify
            </button>
          </div>
        {/if}
      </div>
    </div>

    {#if error}
      <p class="error">{error}</p>
    {:else if !page}
      <div class="skeleton-list" aria-hidden="true">
        <div class="skeleton heading"></div>
        {#each { length: POPULAR_PREVIEW } as _}
          <div class="skeleton-row">
            <div class="skeleton thumb"></div>
            <div class="skeleton line"></div>
          </div>
        {/each}
      </div>
    {:else}
      {#if topTracks.length > 0}
        <section class="popular">
          <h2>Popular</h2>
          <TrackList
            tracks={showAllPopular ? topTracks : topTracks.slice(0, POPULAR_PREVIEW)}
            allTracks={topTracks}
            contextName={artistName}
            showHeader={false}
            showArtists={false}
            showAlbum={false}
            showPlaycount={hasPlaycounts}
          />
          {#if topTracks.length > POPULAR_PREVIEW}
            <button type="button" class="see-more" onclick={() => (showAllPopular = !showAllPopular)}>
              {showAllPopular ? "Show less" : "See more"}
            </button>
          {/if}
        </section>
      {/if}

      {#if activeTab}
        <Shelf
          title="Discography"
          items={activeTab.items}
          onShowAll={() => openDiscography({ id, name: artistName }, activeTab.key === "popular" ? "all" : activeTab.key)}
        >
          {#snippet card(release)}
            <MediaCard
              image={coverAtLeast(release.images, 200)}
              title={release.name}
              subtitle={releaseSubtitle(release)}
              onclick={() => openAlbum(release)}
            />
          {/snippet}
          {#snippet controls()}
            <div class="chips" role="tablist" aria-label="Release type">
              {#each releaseTabs as tab (tab.key)}
                <button
                  type="button"
                  role="tab"
                  class="chip"
                  class:selected={tab.key === activeTab.key}
                  aria-selected={tab.key === activeTab.key}
                  onclick={() => (releaseTab = tab.key)}
                >
                  {tab.label}
                </button>
              {/each}
            </div>
          {/snippet}
        </Shelf>
      {/if}

      {#if page.featuring.length > 0}
        <Shelf title={`Featuring ${artistName}`} items={page.featuring}>
          {#snippet card(playlist)}
            <MediaCard
              image={coverAtLeast(playlist.images, 200)}
              title={playlist.name}
              subtitle={playlistSubtitle(playlist)}
              onclick={() => openPlaylistCard(playlist)}
            />
          {/snippet}
        </Shelf>
      {/if}

      {#if page.related_artists.length > 0}
        <Shelf title="Fans also like" items={page.related_artists}>
          {#snippet card(artist)}
            <MediaCard
              image={coverAtLeast(artist.images, 200)}
              title={artist.name}
              subtitle="Artist"
              round
              onclick={() => openArtist(artist)}
            />
          {/snippet}
        </Shelf>
      {/if}

      {#if page.appears_on.length > 0}
        <Shelf title="Appears On" items={page.appears_on}>
          {#snippet card(release)}
            <MediaCard
              image={coverAtLeast(release.images, 200)}
              title={release.name}
              subtitle={releaseSubtitle(release)}
              onclick={() => openAlbum(release)}
            />
          {/snippet}
        </Shelf>
      {/if}

      {#each [{ title: "Artist Playlists", items: page.playlists }, { title: "Discovered On", items: page.discovered_on }] as shelf}
        {#if shelf.items.length > 0}
          <Shelf title={shelf.title} items={shelf.items}>
            {#snippet card(playlist)}
              <MediaCard
                image={coverAtLeast(playlist.images, 200)}
                title={playlist.name}
                subtitle={playlistSubtitle(playlist)}
                onclick={() => openPlaylistCard(playlist)}
              />
            {/snippet}
          </Shelf>
        {/if}
      {/each}

      <ArtistAbout {page} />
    {/if}
  </div>
</div>

<style>
.artist-page {
  container: artist / inline-size;
  min-height: 100%;
}
.hero {
  position: relative;
  display: flex;
  align-items: flex-end;
  gap: 1.5em;
  min-height: 300px;
  padding: 3em 2em 1.5em;
  overflow: hidden;
  background:
    linear-gradient(to bottom, transparent, rgba(0, 0, 0, 0.35)),
    color-mix(in srgb, var(--artist-color) 70%, var(--surface));
  transition: background-color 400ms ease;
}
.hero.banner {
  height: clamp(300px, 40cqi, 480px);
  background: #000;
}
.hero-image {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  object-position: center 22%;
}
.hero.banner::after {
  content: "";
  position: absolute;
  inset: 0;
  background: linear-gradient(to bottom, rgba(0, 0, 0, 0.15) 0%, rgba(0, 0, 0, 0.05) 30%, rgba(0, 0, 0, 0.6) 100%);
}
.avatar {
  width: 232px;
  height: 232px;
  flex-shrink: 0;
  border-radius: 50%;
  object-fit: cover;
  background: var(--surface-raised);
  box-shadow: 0 8px 40px rgba(0, 0, 0, 0.6);
}
.avatar.placeholder {
  background: linear-gradient(135deg, var(--surface-raised), var(--surface-hover));
}
.hero-text {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  gap: 0.4em;
  min-width: 0;
  color: #fff;
}
.banner .hero-text {
  text-shadow: 0 2px 18px rgba(0, 0, 0, 0.35);
}
.verified {
  display: inline-flex;
  align-items: center;
  gap: 0.45em;
  font-size: var(--fs-sm);
}
h1 {
  margin: 0 0 0.08em;
  font-size: clamp(3rem, 9cqi, 6rem);
  font-weight: 900;
  letter-spacing: -0.04em;
  line-height: 1.02;
  overflow-wrap: anywhere;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
h1.long {
  font-size: clamp(2.5rem, 7cqi, 4.5rem);
}
h1.very-long {
  font-size: clamp(2rem, 5cqi, 3.25rem);
}
.audience {
  font-size: var(--fs-sm);
}
@container artist (max-width: 640px) {
  .avatar {
    width: 150px;
    height: 150px;
  }
}

.page-body {
  padding: 1.5em 2em 2em;
  background: linear-gradient(
    to bottom,
    color-mix(in srgb, var(--artist-color) 32%, var(--surface)) 0,
    var(--surface) 260px
  );
}
.action-bar {
  display: flex;
  align-items: center;
  gap: 1.5em;
  margin-bottom: 1.75em;
}
.icon-button {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0.25em;
  border: none;
  background: none;
  color: var(--text-muted);
  cursor: pointer;
  transition: color var(--transition), transform var(--transition);
}
.icon-button:hover {
  color: var(--text);
  transform: scale(1.04);
}
.icon-button.active {
  color: var(--accent);
}
.icon-button.active::after {
  content: "";
  position: absolute;
  bottom: -0.3em;
  left: 50%;
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: currentColor;
  transform: translateX(-50%);
}
.follow-button {
  padding: 0.45em 1.1em;
  border: 1px solid rgba(255, 255, 255, 0.5);
  border-radius: var(--radius-pill);
  background: none;
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
  transition: border-color var(--transition), transform var(--transition);
}
.follow-button:hover {
  border-color: var(--text);
  transform: scale(1.04);
}
.follow-button:disabled {
  opacity: 0.6;
  cursor: default;
}
.more-wrap {
  position: relative;
}
.menu {
  position: absolute;
  top: calc(100% + 6px);
  left: 0;
  z-index: 60;
  width: 220px;
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
.divider {
  height: 1px;
  margin: 4px 0;
  background: var(--border-strong);
}

h2 {
  margin: 0 0 0.6em;
  font-size: var(--fs-lg);
  font-weight: var(--fw-bold);
  letter-spacing: -0.01em;
  color: var(--text);
}
.popular {
  margin-bottom: 2.5em;
}
.see-more {
  margin-top: 0.75em;
  padding: 0.25em 0.75em;
  border: none;
  background: none;
  color: var(--text-muted);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.see-more:hover {
  color: var(--text);
}
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5em;
  margin-bottom: 0.75em;
}
.chip {
  padding: 0.45em 0.95em;
  border: none;
  border-radius: var(--radius-pill);
  background: var(--control-bg);
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  cursor: pointer;
  transition: background-color var(--transition);
}
.chip:hover {
  background: #333;
}
.chip.selected {
  background: var(--text);
  color: #000;
}

.error {
  font-size: var(--fs-sm);
  color: var(--danger);
}
.skeleton-list {
  display: flex;
  flex-direction: column;
  gap: 0.6em;
}
.skeleton-row {
  display: flex;
  align-items: center;
  gap: 0.8em;
  padding: 0 0.6em;
}
.skeleton {
  border-radius: var(--radius-sm);
  background: linear-gradient(90deg, var(--shimmer-a) 0%, var(--shimmer-b) 50%, var(--shimmer-a) 100%);
  background-size: 200% 100%;
  animation: shimmer 1.4s ease-in-out infinite;
}
.skeleton.heading {
  width: 140px;
  height: 1.4em;
  margin-bottom: 0.6em;
}
.skeleton.thumb {
  width: 40px;
  height: 40px;
}
.skeleton.line {
  width: min(320px, 50%);
  height: 0.9em;
}
@keyframes shimmer {
  from {
    background-position: 100% 0;
  }
  to {
    background-position: -100% 0;
  }
}
</style>
