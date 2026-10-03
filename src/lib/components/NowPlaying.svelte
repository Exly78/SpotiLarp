<script>
  import { untrack } from "svelte";
  import { fade } from "svelte/transition";
  import { queue, next, hasNext, cycleRepeatMode, toggleShuffle } from "../queue.svelte.js";
  import {
    player,
    togglePlay,
    seekTo,
    setVolume,
    toggleMute,
    previousOrRestart,
  } from "../player.svelte.js";
  import { likedIds, likeBusy, refreshLiked, toggleLike } from "../likes.svelte.js";
  import { navigate, openArtist, openAlbum } from "../nav.svelte.js";
  import { sleep } from "../sleep.svelte.js";
  import { formatTime } from "../utils.js";
  import Icon from "./Icon.svelte";

  const VOLUME_WHEEL_STEP = 5;

  /** @type {{ queueOpen?: boolean, onToggleQueue?: () => void, onMiniMode?: () => void }} */
  let { queueOpen = false, onToggleQueue, onMiniMode } = $props();

  let volumeHoverPct = $state(-1);
  let seekHoverPct = $state(-1);
  /** @type {number|null} */
  let dragMs = $state(null);

  const trackId = $derived(player.track?.trackId ?? null);
  const album = $derived(queue.current && queue.current.id === trackId ? queue.current.album : null);
  const sleepActive = $derived(sleep.endsAt !== null || sleep.endOfTrack);
  const durationMs = $derived(player.track?.durationMs ?? 0);
  const shownMs = $derived(dragMs ?? player.positionMs);
  const seekPct = $derived(durationMs > 0 ? Math.min(100, (shownMs / durationMs) * 100) : 0);

  $effect(() => {
    const id = trackId;
    if (id) untrack(() => refreshLiked([id]));
  });

  /** @param {Event} evt */
  function onSeekInput(evt) {
    dragMs = Number(/** @type {HTMLInputElement} */ (evt.target).value);
  }

  /** @param {Event} evt */
  function onSeekCommit(evt) {
    const ms = Number(/** @type {HTMLInputElement} */ (evt.target).value);
    dragMs = null;
    seekTo(ms);
  }

  /** @param {Event} evt */
  function onVolume(evt) {
    setVolume(Number(/** @type {HTMLInputElement} */ (evt.target).value));
  }

  /** @param {MouseEvent & { currentTarget: HTMLElement }} evt */
  function onVolumeHoverMove(evt) {
    const rect = evt.currentTarget.getBoundingClientRect();
    volumeHoverPct = Math.min(100, Math.max(0, ((evt.clientX - rect.left) / rect.width) * 100));
  }

  function onVolumeHoverLeave() {
    volumeHoverPct = -1;
  }

  /** @param {WheelEvent} evt */
  function onVolumeWheel(evt) {
    const step = evt.deltaY < 0 ? VOLUME_WHEEL_STEP : -VOLUME_WHEEL_STEP;
    setVolume(Math.min(100, Math.max(0, player.volume + step)));
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
    {#key player.track?.coverUrl}
      {#if player.track?.coverUrl}
        <img src={player.track.coverUrl} alt="" class="cover" in:fade={{ duration: 250 }} />
      {:else}
        <div class="cover placeholder"></div>
      {/if}
    {/key}
    <div class="text">
      <div class="track-name">
        {#if album?.id}
          <button type="button" class="link" onclick={() => album && openAlbum(album)}>{player.track?.name}</button>
        {:else}
          {player.track?.name ?? "Nothing playing"}
        {/if}
      </div>
      <div class="artists">
        {#if player.track?.artistList?.length}
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
        class="icon-button like-button"
        class:liked={likedIds.has(trackId)}
        disabled={likeBusy.has(trackId)}
        onclick={() => toggleLike(trackId)}
        aria-label={likedIds.has(trackId) ? "Unlike" : "Like"}
      >
        <Icon name={likedIds.has(trackId) ? "heart-filled" : "heart"} size={17} />
      </button>
    {/if}
  </div>

  <div class="transport">
    <div class="buttons">
      <button
        type="button"
        onclick={toggleShuffle}
        class="icon-button toggle-button"
        class:active={queue.shuffle}
        aria-label={queue.shuffle ? "Disable shuffle" : "Enable shuffle"}
        aria-pressed={queue.shuffle}
      >
        <Icon name="shuffle" size={17} />
      </button>
      <button onclick={previousOrRestart} disabled={!player.track} class="icon-button" aria-label="Previous">
        <Icon name="previous" size={18} />
      </button>
      <button
        onclick={togglePlay}
        disabled={!player.track}
        class="icon-button play-pause"
        aria-label={player.isPlaying ? "Pause" : "Play"}
      >
        <Icon name={player.isPlaying ? "pause" : "play"} size={16} />
      </button>
      <button onclick={next} disabled={!hasNext()} class="icon-button" aria-label="Next">
        <Icon name="next" size={18} />
      </button>
      <button
        type="button"
        onclick={cycleRepeatMode}
        class="icon-button toggle-button"
        class:active={queue.repeatMode !== "off"}
        aria-label={`Repeat: ${queue.repeatMode}`}
        aria-pressed={queue.repeatMode !== "off"}
      >
        <Icon name={queue.repeatMode === "one" ? "repeat-one" : "repeat"} size={17} />
      </button>
    </div>
    <div class="seek-row">
      <span class="time">{formatTime(shownMs)}</span>
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
          value={shownMs}
          oninput={onSeekInput}
          onchange={onSeekCommit}
          disabled={!player.track}
          class="seek-slider"
          aria-label="Seek"
        />
      </div>
      <span class="time">{formatTime(durationMs)}</span>
    </div>
  </div>

  <div class="volume-row">
    {#if sleepActive}
      <button
        type="button"
        class="icon-button toggle-button active side-button"
        onclick={() => navigate({ type: "settings" })}
        aria-label="Sleep timer is on"
        title={sleep.endOfTrack ? "Sleep timer: end of song" : `Sleep timer: ${Math.ceil(sleep.remainingMs / 60000)} min left`}
      >
        <Icon name="moon" size={16} />
      </button>
    {/if}
    <button
      type="button"
      class="icon-button toggle-button queue-button"
      class:active={queueOpen}
      onclick={onToggleQueue}
      aria-label={queueOpen ? "Hide queue" : "Show queue"}
      aria-pressed={queueOpen}
    >
      <Icon name="queue" size={16} />
    </button>
    <button
      type="button"
      class="icon-button mute-button"
      onclick={toggleMute}
      aria-label={player.volume === 0 ? "Unmute" : "Mute"}
    >
      <Icon name={player.volume === 0 ? "volume-mute" : "volume"} size={16} />
    </button>
    <div
      class="slider-wrap"
      role="presentation"
      onmousemove={onVolumeHoverMove}
      onmouseleave={onVolumeHoverLeave}
      onwheel={onVolumeWheel}
    >
      <div class="track-base"></div>
      <div class="track-preview" style="width: {Math.max(volumeHoverPct, 0)}%"></div>
      <div class="track-fill" style="width: {player.volume}%"></div>
      <input
        type="range"
        min="0"
        max="100"
        value={player.volume}
        oninput={onVolume}
        class="volume-slider"
        aria-label="Volume"
      />
    </div>
    {#if onMiniMode}
      <button type="button" class="icon-button side-button" onclick={onMiniMode} aria-label="Open mini player" title="Mini player">
        <Icon name="mini-player" size={16} />
      </button>
    {/if}
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
.toggle-button.active {
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
  max-width: 260px;
}
.mute-button,
.queue-button,
.side-button {
  flex-shrink: 0;
}
.link {
  padding: 0;
  border: none;
  background: none;
  color: inherit;
  font: inherit;
  cursor: pointer;
}
.link:hover {
  color: var(--text);
  text-decoration: underline;
}
.mute-button:hover {
  transform: none;
}
input[type="range"]:disabled {
  cursor: default;
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
