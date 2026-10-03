<script>
  import { player, togglePlay, previousOrRestart } from "../player.svelte.js";
  import { next, hasNext } from "../queue.svelte.js";
  import { likedIds, likeBusy, toggleLike } from "../likes.svelte.js";
  import Icon from "./Icon.svelte";

  /** @type {{ onExpand: () => void }} */
  let { onExpand } = $props();

  const trackId = $derived(player.track?.trackId ?? null);
  const progress = $derived(
    player.track?.durationMs ? Math.min(100, (player.positionMs / player.track.durationMs) * 100) : 0,
  );
</script>

<div class="mini" style={player.color ? `--tint: ${player.color}` : ""} data-tauri-drag-region="deep">
  {#if player.track?.coverUrl}
    <img src={player.track.coverUrl} alt="" class="cover" />
  {:else}
    <div class="cover placeholder"></div>
  {/if}
  <div class="body">
    <div class="top">
      <div class="text">
        <div class="name">{player.track?.name ?? "Nothing playing"}</div>
        <div class="artists">{player.track?.artists ?? ""}</div>
      </div>
      <button type="button" class="icon-button" onclick={onExpand} aria-label="Back to full player" title="Back to full player">
        <Icon name="expand" size={15} />
      </button>
    </div>
    <div class="controls">
      {#if trackId}
        <button
          type="button"
          class="icon-button"
          class:liked={likedIds.has(trackId)}
          disabled={likeBusy.has(trackId)}
          onclick={() => toggleLike(trackId)}
          aria-label={likedIds.has(trackId) ? "Unlike" : "Like"}
        >
          <Icon name={likedIds.has(trackId) ? "heart-filled" : "heart"} size={16} />
        </button>
      {/if}
      <button type="button" class="icon-button" onclick={previousOrRestart} disabled={!player.track} aria-label="Previous">
        <Icon name="previous" size={18} />
      </button>
      <button
        type="button"
        class="icon-button play-pause"
        onclick={togglePlay}
        disabled={!player.track}
        aria-label={player.isPlaying ? "Pause" : "Play"}
      >
        <Icon name={player.isPlaying ? "pause" : "play"} size={16} />
      </button>
      <button type="button" class="icon-button" onclick={next} disabled={!hasNext()} aria-label="Next">
        <Icon name="next" size={18} />
      </button>
    </div>
    <div class="progress"><div class="fill" style="width: {progress}%"></div></div>
  </div>
</div>

<style>
.mini {
  display: flex;
  gap: 0.9em;
  flex: 1;
  min-height: 0;
  padding: 0.75em;
  background: linear-gradient(135deg, color-mix(in srgb, var(--tint, var(--surface)) 55%, var(--surface)), var(--surface));
  overflow: hidden;
}
.cover {
  height: 100%;
  aspect-ratio: 1;
  flex-shrink: 0;
  border-radius: var(--radius-sm);
  object-fit: cover;
  background: var(--surface-raised);
}
.cover.placeholder {
  background: linear-gradient(135deg, var(--surface-raised), var(--surface-hover));
}
.body {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  flex: 1;
  min-width: 0;
}
.top {
  display: flex;
  align-items: flex-start;
  gap: 0.5em;
}
.text {
  flex: 1;
  min-width: 0;
}
.name {
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.artists {
  margin-top: 0.1em;
  font-size: var(--fs-xs);
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.controls {
  display: flex;
  align-items: center;
  gap: 0.6em;
}
.icon-button {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0.3em;
  border: none;
  border-radius: 50%;
  background: none;
  color: var(--text-dim);
  cursor: pointer;
  transition: color var(--transition);
}
.icon-button:hover:not(:disabled) {
  color: var(--text);
}
.icon-button:disabled {
  opacity: 0.4;
  cursor: default;
}
.icon-button.liked {
  color: var(--accent);
}
.play-pause {
  width: 32px;
  height: 32px;
  background: var(--text);
  color: var(--bg);
}
.play-pause:hover:not(:disabled) {
  color: var(--bg);
  transform: scale(1.06);
}
.progress {
  height: 3px;
  border-radius: var(--radius-pill);
  background: #4d4d4d;
  overflow: hidden;
}
.fill {
  height: 100%;
  background: var(--text);
}
</style>
