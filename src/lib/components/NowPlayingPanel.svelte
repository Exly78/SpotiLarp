<script module>
  import * as api from "../api.js";
  import { BoundedCache } from "../boundedCache.js";

  /**
   * What the artist card shows: the "About" details when the playback session
   * can get them, otherwise the Web API's basics.
   * @typedef {Object} PanelArtist
   * @property {string} id
   * @property {string} name
   * @property {string|null} photo
   * @property {number|null} monthlyListeners
   * @property {number|null} followers
   * @property {string[]} genres
   * @property {string} bio
   * @property {import("../types.js").ArtistPage|null} page
   */

  // Songs and artists already looked up, and whether you follow them, for the ones that come round again.
  /** @type {BoundedCache<string, import("../types.js").NowPlayingInfo>} */
  const songs = new BoundedCache(50);
  /** @type {BoundedCache<string, PanelArtist>} */
  const artists = new BoundedCache(50);
  /** @type {BoundedCache<string, boolean>} */
  const followed = new BoundedCache(200);

  // Canvas is decoration; people who've asked for less motion get the cover.
  const reducedMotion = typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;

  // A video Canvas can crash the page on Linux (video.rs), so it waits for the
  // app to say it's safe; until then the cover shows.
  const video = $state({ supported: false });
  api.canPlayVideo().then((supported) => (video.supported = supported)).catch(() => {});
</script>

<script>
  import { untrack } from "svelte";
  import { fade } from "svelte/transition";
  import Icon from "./Icon.svelte";
  import ArtistAboutDialog from "./ArtistAboutDialog.svelte";
  import LyricsPreview from "./LyricsPreview.svelte";
  import OnTour from "./OnTour.svelte";
  import TrackCredits from "./TrackCredits.svelte";
  import UpNext from "./UpNext.svelte";
  import { likedIds, likeBusy, toggleLike } from "../likes.svelte.js";
  import { openAlbum, openArtist, openShow } from "../nav.svelte.js";
  import { player } from "../player.svelte.js";
  import { isEpisodeUri } from "../podcasts.svelte.js";
  import { queue } from "../queue.svelte.js";
  import { coverAtLeast, formatCount, htmlToText } from "../utils.js";

  /** @type {{ contextName?: string|null, visible?: boolean, onClose: () => void, onShowQueue?: () => void }} */
  let { contextName = null, visible = true, onClose, onShowQueue } = $props();

  const trackId = $derived(player.track?.trackId ?? null);
  const artistId = $derived(player.track?.primaryArtistId ?? null);
  // Only catalog songs have credits, concerts and Canvas.
  const songId = $derived(player.track?.uri?.startsWith("spotify:track:") ? trackId : null);
  const album = $derived(queue.current && queue.current.id === trackId ? queue.current.album : null);
  const show = $derived(
    isEpisodeUri(player.track?.uri) && queue.current?.uri === player.track?.uri ? (queue.current?.show ?? null) : null,
  );

  /** @type {PanelArtist|null} */
  let artistDetails = $state(null);
  /** @type {import("../types.js").NowPlayingInfo|null} */
  let info = $state(null);
  /** @type {import("../types.js").ArtistPage|null} */
  let aboutPage = $state(null);
  let artistLoading = $state(false);
  let artistError = $state("");
  let following = $state(false);
  let followBusy = $state(false);
  let followError = $state("");
  /** A Canvas that wouldn't play, so the cover shows instead. */
  let brokenCanvas = $state("");
  /** @type {HTMLVideoElement|undefined} */
  let canvasVideo = $state();
  /** @type {string|null|undefined} */
  let loadedArtistId = undefined;
  let loadedSong = "";

  const canvas = $derived.by(() => {
    const found = info?.canvas;
    const playable = found && (!found.is_video || video.supported);
    return !reducedMotion && playable && found.url !== brokenCanvas ? found : null;
  });

  // A new artist: their card from what's known so far, and whether you follow them.
  $effect(() => {
    const id = artistId;
    if (id !== loadedArtistId) untrack(() => showArtist(id));
  });

  // A new song: its credits, concerts and Canvas, with the artist's details.
  $effect(() => {
    const song = songId;
    const id = artistId;
    const key = `${song ?? ""}::${id ?? ""}`;
    if (key !== loadedSong) untrack(() => loadSong(song, id, key));
  });

  // Canvas only moves while the music does, and not behind a closed panel.
  $effect(() => {
    const video = canvasVideo;
    if (!video) return;
    if (player.isPlaying && visible) video.play().catch(() => {});
    else video.pause();
  });

  /** @param {string|null} id */
  function showArtist(id) {
    loadedArtistId = id;
    artistDetails = id ? (artists.get(id) ?? null) : null;
    artistLoading = !!id && !artistDetails;
    artistError = "";
    following = id ? (followed.get(id) ?? false) : false;
    followError = "";
    if (id && !followed.has(id)) loadFollowing(id);
  }

  /**
   * @param {string|null} song
   * @param {string|null} id
   * @param {string} key
   */
  async function loadSong(song, id, key) {
    loadedSong = key;
    info = song ? (songs.get(song) ?? null) : null;
    if (!id) return;
    let found = info;
    if (song && !found) {
      found = await api.getNowPlayingInfo(song, id).catch((e) => {
        console.warn("Couldn't load the Now Playing details:", e);
        return null;
      });
      if (found) songs.set(song, found);
      if (loadedSong !== key) return;
      info = found;
    }
    if (found) {
      const details = fromPage(id, found.artist);
      artists.set(id, details);
      artistDetails = details;
      artistLoading = false;
    } else if (!artistDetails) {
      await loadBasicArtist(id);
    }
  }

  /**
   * @param {string} id
   * @param {import("../types.js").ArtistPage} page
   * @returns {PanelArtist}
   */
  function fromPage(id, page) {
    return {
      id,
      name: page.name,
      photo: coverAtLeast(page.gallery[0] ?? page.images, 640),
      monthlyListeners: page.monthly_listeners,
      followers: page.followers,
      genres: page.genres,
      bio: htmlToText(page.biography).trim(),
      page,
    };
  }

  /** Without a playback session, the Web API's name, photo and followers. @param {string} id */
  async function loadBasicArtist(id) {
    try {
      const basic = await api.getArtist(id);
      /** @type {PanelArtist} */
      const details = {
        id,
        name: basic.name,
        photo: coverAtLeast(basic.images, 640),
        monthlyListeners: null,
        followers: basic.followers?.total ?? null,
        genres: basic.genres,
        bio: "",
        page: null,
      };
      if (!artists.has(id)) artists.set(id, details);
      if (artistId === id && !artistDetails) artistDetails = details;
    } catch (e) {
      console.error("Failed to load artist info:", e);
      if (artistId === id && !artistDetails) artistError = String(e);
    } finally {
      if (artistId === id) artistLoading = false;
    }
  }

  /** @param {string} id */
  async function loadFollowing(id) {
    try {
      const isFollowing = await api.isFollowingArtist(id);
      followed.set(id, isFollowing);
      if (artistId === id) following = isFollowing;
    } catch (e) {
      console.error("Failed to load follow status:", e);
      // Spotify slowing things down isn't worth a message under the artist.
      if (artistId === id && !api.isRateLimited(e)) followError = String(e);
    }
  }

  async function toggleFollow() {
    const id = artistId;
    if (!id || followBusy) return;
    followBusy = true;
    followError = "";
    const next = !following;
    try {
      if (next) await api.followArtist(id);
      else await api.unfollowArtist(id);
      followed.set(id, next);
      if (artistId === id) following = next;
    } catch (e) {
      console.error("Failed to update follow state:", e);
      if (artistId === id) followError = String(e);
    } finally {
      followBusy = false;
    }
  }
</script>

{#snippet header()}
  <div class="panel-header">
    <span class="context-name">{contextName ?? "Now Playing"}</span>
    <button type="button" class="close-button" onclick={onClose} aria-label="Close panel">
      <Icon name="close" size={16} />
    </button>
  </div>
{/snippet}

{#snippet trackMeta()}
  <div class="track-meta">
    <div class="track-text">
      <div class="track-name">
        {#if album?.id}
          <button type="button" class="link" onclick={() => album && openAlbum(album)}>{player.track?.name}</button>
        {:else if show?.id}
          <button type="button" class="link" onclick={() => show && openShow(show)}>{player.track?.name}</button>
        {:else}
          {player.track?.name ?? "Nothing playing"}
        {/if}
      </div>
      <div class="artists">
        {#if show?.id}
          <button type="button" class="link" onclick={() => show && openShow(show)}>{show.name}</button>
        {:else if player.track?.artistList?.length}
          {#each player.track.artistList as artist, i}
            {#if i > 0},&nbsp;{/if}{#if artist.id}<button type="button" class="link" onclick={() => artist.id && openArtist({ id: artist.id, name: artist.name })}>{artist.name}</button>{:else}{artist.name}{/if}
          {/each}
        {:else}
          {player.track?.artists ?? ""}
        {/if}
      </div>
    </div>
    {#if trackId}
      <button
        type="button"
        class="like-button"
        class:liked={likedIds.has(trackId)}
        disabled={likeBusy.has(trackId)}
        onclick={() => trackId && toggleLike(trackId)}
        aria-label={likedIds.has(trackId) ? "Unlike" : "Like"}
      >
        <Icon name={likedIds.has(trackId) ? "heart-filled" : "heart"} size={20} />
      </button>
    {/if}
  </div>
{/snippet}

<div
  class="panel"
  class:tinted={!!player.color && !canvas}
  class:has-canvas={!!canvas}
  style={player.color ? `--tint: ${player.color}` : ""}
>
  {#if canvas}
    <!-- Like Spotify's: the Canvas fills the top of the panel, with the header and song laid over it. -->
    <div class="canvas-hero">
      {#key canvas.url}
        {#if canvas.is_video}
          <video
            bind:this={canvasVideo}
            src={canvas.url}
            class="canvas-media"
            muted
            loop
            playsinline
            disablepictureinpicture
            preload="auto"
            onerror={() => canvas && (brokenCanvas = canvas.url)}
            in:fade={{ duration: 400 }}
          ></video>
        {:else}
          <img src={canvas.url} alt="" class="canvas-media" onerror={() => canvas && (brokenCanvas = canvas.url)} in:fade={{ duration: 400 }} />
        {/if}
      {/key}
      {@render header()}
      {@render trackMeta()}
    </div>
  {:else}
    {@render header()}
    <div class="cover-frame">
      {#key player.track?.coverUrl}
        {#if player.track?.coverUrl}
          <img src={player.track.coverUrl} alt="" class="cover" in:fade={{ duration: 250 }} />
        {:else}
          <div class="cover placeholder"></div>
        {/if}
      {/key}
    </div>
    {@render trackMeta()}
  {/if}

  {#if player.track}
    <div class="artist-card" transition:fade={{ duration: 200 }}>
      {#if !artistId}
        <div class="artist-card-status">
          {player.track.uri?.startsWith("spotify:local:")
            ? "Playing from a file on your computer."
            : "No catalog artist ID for this track."}
        </div>
      {:else if artistDetails}
        {@const page = artistDetails.page}
        <button
          type="button"
          class="artist-card-image-wrap"
          onclick={() => (page ? (aboutPage = page) : artistDetails && openArtist(artistDetails))}
          aria-label={`About ${artistDetails.name}`}
        >
          {#if artistDetails.photo}
            <img src={artistDetails.photo} alt="" class="artist-card-image" />
          {/if}
          <span class="artist-card-eyebrow">About the artist</span>
        </button>
        <div class="artist-card-body">
          <div class="artist-card-row">
            <button type="button" class="artist-card-name" onclick={() => artistDetails && openArtist(artistDetails)}>
              {artistDetails.name}
            </button>
            <button
              type="button"
              class="follow-button"
              class:following
              disabled={followBusy}
              onclick={toggleFollow}
            >
              {following ? "Following" : "Follow"}
            </button>
          </div>
          {#if artistDetails.monthlyListeners != null}
            <div class="artist-card-followers">{formatCount(artistDetails.monthlyListeners)} monthly listeners</div>
          {:else if artistDetails.followers}
            <div class="artist-card-followers">{formatCount(artistDetails.followers)} followers</div>
          {/if}
          {#if artistDetails.bio && page}
            <button type="button" class="artist-card-bio" onclick={() => (aboutPage = page)}>
              <span class="artist-card-bio-text">{artistDetails.bio}</span>
            </button>
          {/if}
          {#if artistDetails.genres.length}
            <div class="artist-card-genres">{artistDetails.genres.slice(0, 3).join(" · ")}</div>
          {/if}
          {#if followError}
            <div class="artist-card-status error follow-error">{followError}</div>
          {/if}
        </div>
      {:else if artistLoading}
        <div class="artist-card-skeleton"></div>
      {:else if artistError}
        <div class="artist-card-status error">{artistError}</div>
      {/if}
    </div>

    <LyricsPreview {visible} />
    {#if info?.credits.length}
      {#key songId}
        <TrackCredits credits={info.credits} sources={info.credit_sources} />
      {/key}
    {/if}
    {#if info?.concerts.length && artistId}
      <OnTour concerts={info.concerts} total={info.concert_count} {artistId} />
    {/if}
    <UpNext {onShowQueue} />
  {/if}

  <!-- In here rather than beside the panel: the page sizes everything directly in the panel area. -->
  {#if aboutPage}
    <ArtistAboutDialog page={aboutPage} onClose={() => (aboutPage = null)} />
  {/if}
</div>

<style>
.panel {
  display: flex;
  flex-direction: column;
  gap: 1em;
  width: 100%;
  height: 100%;
  padding: 1em;
  overflow-x: hidden;
  overflow-y: auto;
}
.panel.tinted {
  background: linear-gradient(to bottom, color-mix(in srgb, var(--tint) 70%, var(--surface)) 0, var(--surface) 420px);
  transition: background 400ms ease;
}
.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5em;
}
.context-name {
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.close-button {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  flex-shrink: 0;
  background: none;
  border: none;
  border-radius: 50%;
  color: var(--text-muted);
  cursor: pointer;
  transition: background-color var(--transition), color var(--transition);
}
.close-button:hover {
  background-color: var(--surface-hover);
  color: var(--text);
}
.cover-frame {
  width: 100%;
  aspect-ratio: 1;
  border-radius: var(--radius-lg);
  overflow: hidden;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45);
  flex-shrink: 0;
}
.cover {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
  background: var(--surface-raised);
}
.cover.placeholder {
  background: linear-gradient(135deg, var(--surface-raised), var(--surface-hover));
}
.panel.has-canvas {
  background: var(--surface);
}
/* Edge to edge, out through the panel's padding. Canvases are 9:16; a short
   window crops one rather than the panel scrolling past it. */
.canvas-hero {
  position: relative;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  flex-shrink: 0;
  margin: -1em -1em 0;
  padding: 1em 1em 1.4em;
  aspect-ratio: 9 / 16;
  min-height: 320px;
  max-height: 75vh;
  overflow: hidden;
}
.canvas-media {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
/* Dark at the top and bottom so the text reads, then into the panel. */
.canvas-hero::after {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  background: linear-gradient(
    to bottom,
    rgba(0, 0, 0, 0.5) 0,
    rgba(0, 0, 0, 0) 16%,
    rgba(0, 0, 0, 0) 55%,
    rgba(0, 0, 0, 0.65) 88%,
    var(--surface) 100%
  );
}
.canvas-hero .panel-header,
.canvas-hero .track-meta {
  position: relative;
  z-index: 1;
  text-shadow: 0 1px 8px rgba(0, 0, 0, 0.5);
}
.canvas-hero .context-name,
.canvas-hero .track-name {
  color: #fff;
}
.canvas-hero .artists {
  color: rgba(255, 255, 255, 0.8);
}
.canvas-hero .artists .link:hover {
  color: #fff;
}
.canvas-hero .close-button,
.canvas-hero .like-button:not(.liked) {
  color: rgba(255, 255, 255, 0.85);
}
.canvas-hero .close-button:hover {
  background-color: rgba(0, 0, 0, 0.35);
  color: #fff;
}
.track-meta {
  display: flex;
  align-items: flex-start;
  gap: 0.75em;
  min-width: 0;
}
.track-text {
  flex: 1;
  min-width: 0;
}
.like-button {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  margin-top: 0.2em;
  padding: 0.3em;
  border: none;
  border-radius: 50%;
  background: none;
  color: var(--text-dim);
  cursor: pointer;
  transition: color var(--transition), transform var(--spring);
}
.like-button:hover {
  color: var(--text);
  transform: scale(1.12);
}
.like-button:disabled {
  opacity: 0.4;
  cursor: default;
  transform: none;
}
.like-button.liked {
  color: var(--accent);
}
.link {
  padding: 0;
  border: none;
  background: none;
  color: inherit;
  font: inherit;
  letter-spacing: inherit;
  text-align: left;
  cursor: pointer;
}
.link:hover {
  text-decoration: underline;
}
.artists .link:hover {
  color: var(--text);
}
.track-name {
  font-size: var(--fs-lg);
  font-weight: var(--fw-black);
  letter-spacing: -0.01em;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
}
.artists {
  margin-top: 0.3em;
  font-size: var(--fs-sm);
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.artist-card {
  border-radius: var(--radius-lg);
  overflow: hidden;
  background: var(--surface-raised);
  flex-shrink: 0;
}
.artist-card-image-wrap {
  position: relative;
  display: block;
  width: 100%;
  aspect-ratio: 16 / 9;
  padding: 0;
  border: none;
  background: var(--surface-hover);
  overflow: hidden;
  cursor: pointer;
}
.artist-card-image {
  width: 100%;
  height: 100%;
  object-fit: cover;
  object-position: center 25%;
  display: block;
  transition: transform 400ms ease;
}
.artist-card-image-wrap:hover .artist-card-image {
  transform: scale(1.03);
}
.artist-card-eyebrow {
  position: absolute;
  z-index: 1;
  left: 0.9em;
  top: 0.7em;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  color: #fff;
  text-shadow: 0 1px 4px rgba(0, 0, 0, 0.6);
}
.artist-card-image-wrap::after {
  content: "";
  position: absolute;
  inset: 0;
  background: linear-gradient(to bottom, rgba(0, 0, 0, 0.6), rgba(0, 0, 0, 0) 45%);
}
.artist-card-body {
  padding: 0.9em 1em 1.1em;
}
.artist-card-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75em;
}
.artist-card-name {
  padding: 0;
  border: none;
  background: none;
  font-family: inherit;
  font-size: var(--fs-md);
  font-weight: var(--fw-black);
  color: var(--text);
  text-align: left;
  cursor: pointer;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.artist-card-name:hover {
  text-decoration: underline;
}
.artist-card-followers {
  margin-top: 0.3em;
  font-size: var(--fs-sm);
  color: var(--text-muted);
}
.artist-card-bio {
  display: block;
  width: 100%;
  margin-top: 0.7em;
  padding: 0;
  border: none;
  background: none;
  font-family: inherit;
  font-size: var(--fs-sm);
  line-height: 1.5;
  color: var(--text-dim);
  text-align: left;
  cursor: pointer;
}
/* Clamped inside the button: WebKit (Linux, macOS) won't clamp a button's own text. */
.artist-card-bio-text {
  display: -webkit-box;
  overflow: hidden;
  -webkit-line-clamp: 3;
  line-clamp: 3;
  -webkit-box-orient: vertical;
}
.artist-card-bio:hover {
  color: var(--text);
}
.artist-card-genres {
  margin-top: 0.2em;
  font-size: var(--fs-sm);
  color: var(--text-muted);
  text-transform: capitalize;
}
.follow-button {
  flex-shrink: 0;
  padding: 0.4em 0.9em;
  border-radius: 999px;
  border: 1px solid var(--text-muted);
  background: none;
  color: var(--text);
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
  transition: border-color var(--transition), color var(--transition), opacity var(--transition);
}
.follow-button:hover {
  border-color: var(--text);
}
.follow-button.following {
  border-color: var(--accent, var(--text));
  color: var(--accent, var(--text));
}
.follow-button:disabled {
  opacity: 0.6;
  cursor: default;
}
.artist-card-status {
  padding: 0.9em 1em;
  font-size: var(--fs-sm);
  color: var(--text-muted);
}
.artist-card-status.error {
  color: var(--danger);
}
.follow-error {
  padding: 0.5em 0 0;
}
.artist-card-skeleton {
  width: 100%;
  aspect-ratio: 16 / 9;
  background: linear-gradient(135deg, var(--surface-raised), var(--surface-hover));
}
</style>
