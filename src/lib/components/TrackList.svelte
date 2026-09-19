<script>
  import { fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { playFromList } from "../queue.svelte.js";
  import { smallestCover, formatTime } from "../utils.js";
  import * as api from "../api.js";
  import Icon from "./Icon.svelte";

  /** @type {{ tracks: import("../types.js").Track[] }} */
  let { tracks } = $props();

  let likedIds = $state(new Set());
  let likeBusy = $state(new Set());
  /** @type {string|null} */
  let checkedForKey = null;

  $effect(() => {
    const key = tracks.map((t) => t.id).join(",");
    if (key !== checkedForKey) {
      checkedForKey = key;
      checkLiked(tracks.map((t) => t.id));
    }
  });

  /** @param {string[]} ids */
  async function checkLiked(ids) {
    if (ids.length === 0) {
      likedIds = new Set();
      return;
    }
    try {
      const results = await api.areTracksLiked(ids);
      likedIds = new Set(ids.filter((_, i) => results[i]));
    } catch (e) {
      console.error("Failed to check liked tracks:", e);
    }
  }

  /** @param {import("../types.js").Track} track */
  async function toggleLike(track) {
    if (likeBusy.has(track.id)) return;
    likeBusy = new Set(likeBusy).add(track.id);
    const isLiked = likedIds.has(track.id);
    try {
      if (isLiked) {
        await api.unlikeTrack(track.id);
        const next = new Set(likedIds);
        next.delete(track.id);
        likedIds = next;
      } else {
        await api.likeTrack(track.id);
        likedIds = new Set(likedIds).add(track.id);
      }
    } catch (e) {
      console.error("Failed to update liked state:", e);
    } finally {
      const next = new Set(likeBusy);
      next.delete(track.id);
      likeBusy = next;
    }
  }
</script>

<ul class="track-list">
  {#each tracks as track, i (track.id)}
    <li class="result" in:fly={{ y: 8, duration: 220, delay: Math.min(i * 20, 300), easing: cubicOut }}>
      <button type="button" class="result-button" onclick={() => playFromList(tracks, i)}>
        {#if smallestCover(track.album.images)}
          <img src={smallestCover(track.album.images)} alt="" class="thumb" />
        {:else}
          <div class="thumb placeholder"></div>
        {/if}
        <div class="meta">
          <div class="name">{track.name}</div>
          <div class="artists">
            {track.artists.map((a) => a.name).join(", ")}
          </div>
        </div>
      </button>
      <button
        type="button"
        class="like-button"
        class:liked={likedIds.has(track.id)}
        disabled={likeBusy.has(track.id)}
        onclick={() => toggleLike(track)}
        aria-label={likedIds.has(track.id) ? "Unlike" : "Like"}
      >
        <Icon name={likedIds.has(track.id) ? "heart-filled" : "heart"} size={16} />
      </button>
      <div class="duration">{formatTime(track.duration_ms)}</div>
    </li>
  {/each}
</ul>

<style>
.track-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.result {
  display: flex;
  align-items: center;
  border-radius: var(--radius-md);
  transition: background-color var(--transition);
}
.result:hover {
  background-color: var(--surface-hover);
}
.result-button {
  display: flex;
  align-items: center;
  gap: 0.8em;
  flex: 1;
  min-width: 0;
  padding: 0.5em 0.6em;
  border: none;
  background: none;
  box-shadow: none;
  cursor: pointer;
  text-align: left;
  font: inherit;
  color: inherit;
}
.thumb {
  width: 44px;
  height: 44px;
  border-radius: var(--radius-sm);
  object-fit: cover;
  background: var(--surface-raised);
  flex-shrink: 0;
}
.thumb.placeholder {
  background: linear-gradient(135deg, var(--surface-raised), var(--surface-hover));
}
.meta {
  flex: 1;
  min-width: 0;
}
.name {
  font-size: var(--fs-sm);
  font-weight: var(--fw-medium);
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.artists {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 0.15em;
}
.like-button {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  width: 32px;
  height: 32px;
  background: none;
  border: none;
  border-radius: 50%;
  color: var(--text-muted);
  cursor: pointer;
  transition: color var(--transition), opacity var(--transition);
}
.like-button:hover {
  color: var(--text);
}
.like-button.liked {
  color: var(--accent);
}
.like-button:disabled {
  opacity: 0.6;
  cursor: default;
}
.duration {
  font-size: var(--fs-xs);
  font-variant-numeric: tabular-nums;
  color: var(--text-muted);
  flex-shrink: 0;
  padding-right: 0.6em;
  min-width: 2.5em;
  text-align: right;
}
</style>
