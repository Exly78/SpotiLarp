<script>
  import { fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { playFromList } from "../queue.svelte.js";
  import { player, togglePlay } from "../player.svelte.js";
  import { openTrackMenu } from "../contextMenu.svelte.js";
  import { openShow } from "../nav.svelte.js";
  import { isPlayed, progressFraction, timeLeft } from "../podcasts.svelte.js";
  import { smallestCover, releaseDay } from "../utils.js";
  import Icon from "./Icon.svelte";

  /**
   * Podcast episodes (as tracks) with their date, length or time left, and
   * show notes. Clicking one plays from where you left off.
   * @type {{
   *   episodes: import("../types.js").Track[],
   *   contextName: string,
   *   contextView?: import("../nav.svelte.js").View|null,
   *   showShow?: boolean,
   * }}
   */
  let { episodes, contextName, contextView = null, showShow = false } = $props();

  const ANIMATED_ROWS = 12;

  /** @type {Set<string>} */
  let expanded = $state(new Set());

  /** @param {number} index */
  function play(index) {
    const episode = episodes[index];
    if (player.track?.uri === episode.uri) togglePlay();
    else playFromList(episodes, index, contextName, contextView);
  }

  /** @param {string} uri */
  function toggleNotes(uri) {
    const next = new Set(expanded);
    if (next.has(uri)) next.delete(uri);
    else next.add(uri);
    expanded = next;
  }
</script>

<ul class="episode-list">
  {#each episodes as episode, i (episode.uri)}
    {@const current = player.track?.uri === episode.uri}
    {@const played = isPlayed(episode)}
    {@const fraction = progressFraction(episode)}
    <li
      class="episode"
      class:playing={current}
      class:unplayable={episode.playable === false}
      oncontextmenu={(e) => openTrackMenu(e, episode)}
      in:fly={{ y: 8, duration: i < ANIMATED_ROWS ? 220 : 0, delay: i < ANIMATED_ROWS ? i * 20 : 0, easing: cubicOut }}
    >
      <button type="button" class="hit" onclick={() => play(i)} aria-label={`Play ${episode.name}`}></button>
      {#if smallestCover(episode.album.images)}
        <img src={smallestCover(episode.album.images)} alt="" class="thumb" loading="lazy" />
      {:else}
        <div class="thumb placeholder"><Icon name="podcast" size={22} /></div>
      {/if}
      <div class="body">
        <div class="name">{episode.name}</div>
        {#if showShow && episode.show}
          <button type="button" class="show-link" onclick={() => openShow(episode.show)}>{episode.show.name}</button>
        {/if}
        {#if episode.description}
          <p class="notes" class:open={expanded.has(episode.uri)}>{episode.description}</p>
        {/if}
        <div class="meta">
          {#if played}<span class="played"><Icon name="added" size={13} /> Played</span> ·{/if}
          {#if episode.release_date}{releaseDay(episode.release_date)} ·{/if}
          {timeLeft(episode)}
          {#if episode.playable === false} · Not available{/if}
          {#if episode.description && episode.description.length > 160}
            <button type="button" class="notes-toggle" onclick={() => toggleNotes(episode.uri)}>
              {expanded.has(episode.uri) ? "Less" : "More"}
            </button>
          {/if}
        </div>
        {#if fraction > 0}
          <div class="progress" aria-label={`${Math.round(fraction * 100)}% listened`}>
            <div class="progress-fill" style="width: {fraction * 100}%"></div>
          </div>
        {/if}
      </div>
      <span class="play-icon" aria-hidden="true">
        <Icon name={current && player.isPlaying ? "pause" : "play"} size={16} />
      </span>
    </li>
  {/each}
</ul>

<style>
.episode-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.episode {
  position: relative;
  display: grid;
  grid-template-columns: 72px minmax(0, 1fr) 36px;
  align-items: start;
  gap: 1em;
  padding: 0.8em 0.6em;
  border-radius: var(--radius-md);
  transition: background-color var(--transition);
  content-visibility: auto;
  contain-intrinsic-size: auto 120px;
}
.episode:hover {
  background-color: var(--surface-hover);
}
.episode.unplayable {
  opacity: 0.55;
}
.hit {
  position: absolute;
  inset: 0;
  border: none;
  border-radius: var(--radius-md);
  background: none;
  cursor: pointer;
}
.episode > :not(.hit) {
  position: relative;
  pointer-events: none;
}
.show-link,
.notes-toggle {
  pointer-events: auto;
}
.thumb {
  width: 72px;
  height: 72px;
  border-radius: var(--radius-sm);
  object-fit: cover;
  background: var(--surface-raised);
}
.thumb.placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  background: linear-gradient(135deg, var(--surface-raised), var(--surface-hover));
}
.body {
  display: flex;
  flex-direction: column;
  gap: 0.3em;
  min-width: 0;
}
.name {
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  color: var(--text);
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
}
.episode.playing .name {
  color: var(--accent);
}
.show-link {
  align-self: flex-start;
  padding: 0;
  border: none;
  background: none;
  color: var(--text-muted);
  font: inherit;
  font-size: var(--fs-xs);
  cursor: pointer;
}
.show-link:hover {
  color: var(--text);
  text-decoration: underline;
}
.notes {
  margin: 0;
  font-size: var(--fs-xs);
  line-height: 1.45;
  color: var(--text-muted);
  white-space: pre-line;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
}
.notes.open {
  display: block;
}
.meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.35em;
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
.played {
  display: inline-flex;
  align-items: center;
  gap: 0.25em;
  color: var(--accent);
}
.notes-toggle {
  margin-left: 0.3em;
  padding: 0;
  border: none;
  background: none;
  color: var(--text);
  font: inherit;
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.notes-toggle:hover {
  text-decoration: underline;
}
.progress {
  width: min(160px, 100%);
  height: 3px;
  margin-top: 0.2em;
  border-radius: 2px;
  background: var(--border-strong);
  overflow: hidden;
}
.progress-fill {
  height: 100%;
  background: var(--accent);
}
.play-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  align-self: center;
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: var(--text);
  color: var(--bg);
  opacity: 0.9;
}
.episode:hover .play-icon,
.episode.playing .play-icon {
  opacity: 1;
}
</style>
