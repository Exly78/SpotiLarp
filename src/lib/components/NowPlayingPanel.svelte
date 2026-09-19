<script>
  import { onMount, onDestroy } from "svelte";
  import { fade } from "svelte/transition";
  import { listen } from "@tauri-apps/api/event";
  import Icon from "./Icon.svelte";
  import * as api from "../api.js";

  let { contextName = null, onClose } = $props();

  let trackName = $state("Nothing playing");
  let artists = $state("");
  /** @type {string|null} */
  let coverUrl = $state(null);
  /** @type {string|null} */
  let artistId = $state(null);

  /** @type {import("../types.js").ArtistDetails|null} */
  let artistDetails = $state(null);
  let artistLoading = $state(false);
  let artistError = $state("");
  let following = $state(false);
  let followBusy = $state(false);
  let followError = $state("");
  /** @type {string|null} */
  let loadedArtistId = null;

  const followerCountFormatter = new Intl.NumberFormat();

  /** @type {import("@tauri-apps/api/event").UnlistenFn|undefined} */
  let unlisten;

  onMount(async () => {
    unlisten = await listen("player-event", (event) => {
      const e = event.payload;
      if (e.type === "TrackChanged") {
        trackName = e.name;
        artists = e.artists;
        coverUrl = e.cover_url;
        artistId = e.primary_artist_id;
      }
    });
  });

  onDestroy(() => {
    unlisten?.();
  });

  $effect(() => {
    if (artistId && artistId !== loadedArtistId) {
      loadArtist(artistId);
    } else if (!artistId) {
      loadedArtistId = null;
      artistDetails = null;
    }
  });

  /** @param {string} id */
  async function loadArtist(id) {
    loadedArtistId = id;
    artistLoading = true;
    artistDetails = null;
    artistError = "";
    
    try {
      const details = await api.getArtist(id);
      if (artistId === id) artistDetails = details;
    } catch (e) {
      console.error("Failed to load artist info:", e);
      if (artistId === id) artistError = String(e);
    } finally {
      if (artistId === id) artistLoading = false;
    }
    followError = "";
    try {
      const isFollowing = await api.isFollowingArtist(id);
      if (artistId === id) following = isFollowing;
    } catch (e) {
      console.error("Failed to load follow status:", e);
      if (artistId === id) followError = String(e);
    }
  }

  async function toggleFollow() {
    if (!artistId || followBusy) return;
    followBusy = true;
    followError = "";
    const next = !following;
    try {
      if (next) await api.followArtist(artistId);
      else await api.unfollowArtist(artistId);
      following = next;
    } catch (e) {
      console.error("Failed to update follow state:", e);
      followError = String(e);
    } finally {
      followBusy = false;
    }
  }
</script>

<div class="panel">
  <div class="panel-header">
    <span class="context-name">{contextName ?? "Now Playing"}</span>
    <button type="button" class="close-button" onclick={onClose} aria-label="Close panel">
      <Icon name="close" size={16} />
    </button>
  </div>

  <div class="cover-frame">
    {#key coverUrl}
      {#if coverUrl}
        <img src={coverUrl} alt="" class="cover" in:fade={{ duration: 250 }} />
      {:else}
        <div class="cover placeholder"></div>
      {/if}
    {/key}
  </div>

  <div class="track-meta">
    <div class="track-name">{trackName}</div>
    <div class="artists">{artists}</div>
  </div>

  {#if trackName !== "Nothing playing"}
    <div class="artist-card" transition:fade={{ duration: 200 }}>
      {#if !artistId}
        <div class="artist-card-status">No catalog artist ID for this track.</div>
      {:else if artistDetails}
        <div class="artist-card-image-wrap">
          {#if artistDetails.images[0]}
            <img src={artistDetails.images[0].url} alt="" class="artist-card-image" />
          {/if}
          <span class="artist-card-eyebrow">About the artist</span>
        </div>
        <div class="artist-card-body">
          <div class="artist-card-row">
            <div class="artist-card-name">{artistDetails.name}</div>
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
          {#if artistDetails.followers}
            <div class="artist-card-followers">
              {followerCountFormatter.format(artistDetails.followers.total)} followers
            </div>
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
.track-meta {
  min-width: 0;
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
  width: 100%;
  aspect-ratio: 16 / 9;
  background: var(--surface-hover);
}
.artist-card-image {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.artist-card-eyebrow {
  position: absolute;
  left: 0.9em;
  bottom: 0.7em;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  color: #fff;
  text-shadow: 0 1px 4px rgba(0, 0, 0, 0.6);
}
.artist-card-image-wrap::after {
  content: "";
  position: absolute;
  inset: 0;
  background: linear-gradient(to top, rgba(0, 0, 0, 0.65), rgba(0, 0, 0, 0) 55%);
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
  font-size: var(--fs-md);
  font-weight: var(--fw-black);
  color: var(--text);
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.artist-card-followers {
  margin-top: 0.3em;
  font-size: var(--fs-sm);
  color: var(--text-muted);
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
