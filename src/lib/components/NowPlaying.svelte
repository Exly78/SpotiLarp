<script>
  import { onMount, onDestroy } from "svelte";
  import { fade } from "svelte/transition";
  import { listen } from "@tauri-apps/api/event";
  import * as api from "../api.js";
  import { queue, next, previous, hasNext, hasPrevious, cycleRepeatMode } from "../queue.svelte.js";
  import { formatTime } from "../utils.js";
  import Icon from "./Icon.svelte";

  let trackName = $state("Nothing playing");
  let artists = $state("");
  let coverUrl = $state(null);
  let durationMs = $state(0);
  let positionMs = $state(0);
  let isPlaying = $state(false);
  let volume = $state(50);
  let volumeHoverPct = $state(-1);
  let seekHoverPct = $state(-1);
  /** @type {string|null} */
  let trackId = $state(null);
  let liked = $state(false);
  let likeBusy = $state(false);
  /** @type {string|null} */
  let checkedLikeFor = null;

  const seekPct = $derived(durationMs > 0 ? (positionMs / durationMs) * 100 : 0);

  /** @type {import("@tauri-apps/api/event").UnlistenFn|undefined} */
  let unlisten;

  onMount(async () => {
    unlisten = await listen("player-event", (event) => {
      const e = event.payload;
      switch (e.type) {
        case "TrackChanged":
          trackName = e.name;
          artists = e.artists;
          coverUrl = e.cover_url;
          durationMs = e.duration_ms;
          trackId = e.track_id;
          break;
        case "Playing":
          isPlaying = true;
          positionMs = e.position_ms;
          break;
        case "Paused":
          isPlaying = false;
          positionMs = e.position_ms;
          break;
        case "PositionChanged":
        case "Seeked":
          positionMs = e.position_ms;
          break;
        case "Stopped":
        case "EndOfTrack":
          isPlaying = false;
          break;
        case "VolumeChanged":
          volume = Math.round((e.volume / 65535) * 100);
          break;
      }
    });
  });

  onDestroy(() => {
    unlisten?.();
  });

  function togglePlay() {
    if (isPlaying) {
      api.pause();
    } else {
      api.resume();
    }
  }

  $effect(() => {
    if (trackId && trackId !== checkedLikeFor) {
      checkedLikeFor = trackId;
      api
        .isTrackLiked(trackId)
        .then((result) => {
          if (trackId === checkedLikeFor) liked = result;
        })
        .catch((e) => console.error("Failed to check liked state:", e));
    } else if (!trackId) {
      checkedLikeFor = null;
      liked = false;
    }
  });

  async function toggleLike() {
    if (!trackId || likeBusy) return;
    likeBusy = true;
    const next = !liked;
    try {
      if (next) await api.likeTrack(trackId);
      else await api.unlikeTrack(trackId);
      liked = next;
    } catch (e) {
      console.error("Failed to update liked state:", e);
    } finally {
      likeBusy = false;
    }
  }

  /** @param {Event} evt */
  function onSeek(evt) {
    const ms = Number(/** @type {HTMLInputElement} */ (evt.target).value);
    positionMs = ms;
    api.seek(ms);
  }

  /** @param {Event} evt */
  function onVolume(evt) {
    const pct = Number(/** @type {HTMLInputElement} */ (evt.target).value);
    volume = pct;
    api.setVolume(Math.round((pct / 100) * 65535));
  }

  /** @param {MouseEvent & { currentTarget: HTMLElement }} evt */
  function onVolumeHoverMove(evt) {
    const rect = evt.currentTarget.getBoundingClientRect();
    volumeHoverPct = Math.min(100, Math.max(0, ((evt.clientX - rect.left) / rect.width) * 100));
  }

  function onVolumeHoverLeave() {
    volumeHoverPct = -1;
  }

  /** @param {MouseEvent & { currentTarget: HTMLElement }} evt */
  function onSeekHoverMove(evt) {
    const rect = evt.currentTarget.getBoundingClientRect();
    seekHoverPct = Math.min(100, Math.max(0, ((evt.clientX - rect.left) / rect.width) * 100));
  }

  function onSeekHoverLeave() {
    seekHoverPct = -1;
  }
</script>

<div class="now-playing">
  <div class="track-info">
    {#key coverUrl}
      {#if coverUrl}
        <img src={coverUrl} alt="" class="cover" in:fade={{ duration: 250 }} />
      {:else}
        <div class="cover placeholder"></div>
      {/if}
    {/key}
    <div class="text">
      <div class="track-name">{trackName}</div>
      <div class="artists">{artists}</div>
    </div>
    {#if trackId}
      <button
        type="button"
        class="icon-button like-button"
        class:liked
        disabled={likeBusy}
        onclick={toggleLike}
        aria-label={liked ? "Unlike" : "Like"}
      >
        <Icon name={liked ? "heart-filled" : "heart"} size={17} />
      </button>
    {/if}
  </div>

  <div class="transport">
    <div class="buttons">
      <button onclick={previous} disabled={!hasPrevious()} class="icon-button" aria-label="Previous">
        <Icon name="previous" size={18} />
      </button>
      <button onclick={togglePlay} class="icon-button play-pause" aria-label={isPlaying ? "Pause" : "Play"}>
        <Icon name={isPlaying ? "pause" : "play"} size={16} />
      </button>
      <button onclick={next} disabled={!hasNext()} class="icon-button" aria-label="Next">
        <Icon name="next" size={18} />
      </button>
      <button
        type="button"
        onclick={cycleRepeatMode}
        class="icon-button repeat-button"
        class:active={queue.repeatMode !== "off"}
        aria-label={`Repeat: ${queue.repeatMode}`}
        aria-pressed={queue.repeatMode !== "off"}
      >
        <Icon name={queue.repeatMode === "one" ? "repeat-one" : "repeat"} size={17} />
      </button>
    </div>
    <div class="seek-row">
      <span class="time">{formatTime(positionMs)}</span>
      <div
        class="slider-wrap"
        role="presentation"
        onmousemove={onSeekHoverMove}
        onmouseleave={onSeekHoverLeave}
      >
        <div class="track-base"></div>
        <div class="track-preview" style="width: {Math.max(seekHoverPct, 0)}%"></div>
        <div class="track-fill" style="width: {seekPct}%"></div>
        <input
          type="range"
          min="0"
          max={durationMs || 1}
          value={positionMs}
          oninput={onSeek}
          class="seek-slider"
        />
      </div>
      <span class="time">{formatTime(durationMs)}</span>
    </div>
  </div>

  <div class="volume-row">
    <Icon name={volume === 0 ? "volume-mute" : "volume"} size={16} class="volume-icon" />
    <div
      class="slider-wrap"
      role="presentation"
      onmousemove={onVolumeHoverMove}
      onmouseleave={onVolumeHoverLeave}
    >
      <div class="track-base"></div>
      <div class="track-preview" style="width: {Math.max(volumeHoverPct, 0)}%"></div>
      <div class="track-fill" style="width: {volume}%"></div>
      <input
        type="range"
        min="0"
        max="100"
        value={volume}
        oninput={onVolume}
        class="volume-slider"
      />
    </div>
  </div>
</div>

<style>
.now-playing {
  display: grid;
  grid-template-columns: 1fr minmax(0, 2fr) 1fr;
  align-items: center;
  gap: 1em;
  width: 100%;
  padding: 0.7em 1.25em;
}
.track-info {
  display: flex;
  align-items: center;
  gap: 0.8em;
  min-width: 0;
}
.cover {
  width: 56px;
  height: 56px;
  border-radius: var(--radius-sm);
  object-fit: cover;
  background: var(--surface-raised);
  flex-shrink: 0;
}
.cover.placeholder {
  background: linear-gradient(135deg, var(--surface-raised), var(--surface-hover));
}
.text {
  min-width: 0;
}
.track-name {
  font-size: var(--fs-sm);
  font-weight: var(--fw-semibold);
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
.transport {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.4em;
  width: 100%;
}
.buttons {
  display: flex;
  align-items: center;
  gap: 1em;
}
.icon-button {
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  color: var(--text-dim);
  cursor: pointer;
  padding: 0.3em;
  border-radius: 50%;
  transition: color var(--transition), transform var(--spring);
}
.icon-button:hover {
  color: var(--text);
  transform: scale(1.12);
}
.icon-button:active {
  transform: scale(0.9);
  transition-duration: 100ms;
}
.icon-button:disabled {
  color: var(--text-muted);
  opacity: 0.4;
  cursor: default;
  transform: none;
}
.like-button {
  flex-shrink: 0;
}
.like-button.liked {
  color: var(--accent);
}
.repeat-button.active {
  color: var(--accent);
}
.play-pause {
  width: 34px;
  height: 34px;
  background: var(--text);
  color: var(--bg);
}
.play-pause:hover {
  background: var(--text);
  color: var(--bg);
  transform: scale(1.1);
}
.seek-row {
  display: flex;
  align-items: center;
  gap: 0.6em;
  width: 100%;
}
.time {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  width: 2.5em;
  text-align: center;
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
}
.volume-row {
  display: flex;
  align-items: center;
  gap: 0.6em;
  justify-self: end;
  width: 100%;
  max-width: 160px;
}
:global(.volume-icon) {
  color: var(--text-dim);
  flex-shrink: 0;
}

.slider-wrap {
  position: relative;
  flex: 1;
  display: flex;
  align-items: center;
  height: 12px;
}

input[type="range"] {
  -webkit-appearance: none;
  appearance: none;
  height: 4px;
  border-radius: var(--radius-pill);
  background-color: #4d4d4d;
  background-repeat: no-repeat;
  outline: none;
  cursor: pointer;
  flex: 1;
  margin: 0;
}
input[type="range"]::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--text);
  opacity: 0;
  transition: opacity var(--transition);
}
.seek-row:hover input[type="range"]::-webkit-slider-thumb,
.volume-row:hover input[type="range"]::-webkit-slider-thumb {
  opacity: 1;
}

.track-base,
.track-preview,
.track-fill {
  position: absolute;
  left: 0;
  top: 50%;
  transform: translateY(-50%);
  height: 4px;
  border-radius: var(--radius-pill);
  pointer-events: none;
}
.track-base {
  width: 100%;
  background: #4d4d4d;
  z-index: 0;
}
.track-preview {
  background: var(--text);
  z-index: 1;
}
.track-fill {
  background: var(--text);
  z-index: 2;
}
.slider-wrap:hover .track-fill {
  background: var(--accent);
}

input[type="range"].volume-slider,
input[type="range"].seek-slider {
  position: relative;
  z-index: 3;
  background-color: transparent;
}
</style>
