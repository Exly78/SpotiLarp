<script>
  import PlayButton from "./PlayButton.svelte";
  import { player } from "../player.svelte.js";
  import { openTrackMenu } from "../contextMenu.svelte.js";
  import { openShow } from "../nav.svelte.js";
  import { isPlayed, progressFraction, timeLeft } from "../podcasts.svelte.js";
  import { coverAtLeast, releaseDay } from "../utils.js";

  /**
   * A podcast episode on a shelf: its show, date, time left and how far in you
   * are. Clicking it plays from where you left off.
   * @type {{ episode: import("../types.js").Track, onplay: () => void }}
   */
  let { episode, onplay } = $props();

  const current = $derived(player.track?.uri === episode.uri);
  const fraction = $derived(progressFraction(episode));
  const image = $derived(coverAtLeast(episode.album.images, 200));
</script>

<div
  class="card"
  class:playing={current}
  class:unplayable={episode.playable === false}
  oncontextmenu={(e) => openTrackMenu(e, episode)}
  role="presentation"
>
  <button type="button" class="open" onclick={onplay} aria-label={`Play ${episode.name}`}></button>
  <div class="art-wrap">
    {#if image}
      <img src={image} alt="" class="art" loading="lazy" />
    {:else}
      <div class="art placeholder"></div>
    {/if}
    <div class="play" class:shown={current && player.isPlaying}>
      <PlayButton size={48} label={`Play ${episode.name}`} playing={current && player.isPlaying} onclick={onplay} />
    </div>
  </div>
  <div class="title">{episode.name}</div>
  {#if episode.show}
    {@const show = episode.show}
    <button type="button" class="show-link" onclick={() => openShow(show)}>{show.name}</button>
  {/if}
  <div class="meta">
    {#if isPlayed(episode)}Played ·{/if}
    {#if episode.release_date}{releaseDay(episode.release_date)} ·{/if}
    {timeLeft(episode)}
  </div>
  {#if fraction > 0}
    <div class="progress" aria-label={`${Math.round(fraction * 100)}% listened`}>
      <div class="progress-fill" style="width: {fraction * 100}%"></div>
    </div>
  {/if}
</div>

<style>
.card {
  position: relative;
  display: flex;
  flex-direction: column;
  min-width: 0;
  padding: 12px;
  border-radius: 6px;
  text-align: left;
  transition: background-color var(--transition);
}
.card:hover,
.card:focus-within {
  background-color: var(--surface-hover);
}
.card.unplayable {
  opacity: 0.55;
}
.open {
  position: absolute;
  inset: 0;
  z-index: 1;
  padding: 0;
  border: none;
  border-radius: inherit;
  background: none;
  cursor: pointer;
}
.art-wrap {
  position: relative;
  margin-bottom: 8px;
}
.art {
  display: block;
  width: 100%;
  aspect-ratio: 1;
  border-radius: 6px;
  object-fit: cover;
  background: var(--surface-raised);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
}
.art.placeholder {
  background: linear-gradient(135deg, var(--surface-raised), var(--surface-hover));
}
.play {
  position: absolute;
  right: 8px;
  bottom: 8px;
  z-index: 2;
  border-radius: 50%;
  box-shadow: 0 8px 8px rgba(0, 0, 0, 0.3);
  opacity: 0;
  transform: translateY(8px);
  transition: opacity 200ms ease, transform 200ms ease;
}
.card:hover .play,
.card:focus-within .play,
.play.shown {
  opacity: 1;
  transform: none;
}
.title {
  font-size: 1em;
  font-weight: var(--fw-medium);
  line-height: 1.3;
  color: var(--text);
  overflow-wrap: anywhere;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.card.playing .title {
  color: var(--accent);
}
.show-link {
  position: relative;
  z-index: 2;
  align-self: flex-start;
  max-width: 100%;
  margin-top: 2px;
  padding: 0;
  border: none;
  background: none;
  color: var(--text-muted);
  font-family: inherit;
  font-size: var(--fs-sm);
  text-align: left;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  cursor: pointer;
}
.show-link:hover {
  color: var(--text);
  text-decoration: underline;
}
.meta {
  margin-top: 2px;
  font-size: var(--fs-xs);
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.progress {
  height: 3px;
  margin-top: 6px;
  border-radius: 2px;
  background: var(--border-strong);
  overflow: hidden;
}
.progress-fill {
  height: 100%;
  background: var(--accent);
}
@media (prefers-reduced-motion: reduce) {
  .play {
    transform: none;
    transition: opacity 200ms ease;
  }
}
</style>
