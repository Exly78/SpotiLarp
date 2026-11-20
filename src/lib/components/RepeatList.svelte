<script>
  import { playFromList } from "../queue.svelte.js";
  import { player } from "../player.svelte.js";
  import { openTrackMenu } from "../contextMenu.svelte.js";
  import { startTrackDrag, endDrag } from "../drag.svelte.js";
  import { smallestCover } from "../utils.js";

  /**
   * A numbered list of songs in two columns, with how often each was played
   * when that's known, and a `note` under it. A row plays the list from that song.
   * @type {{ items: { track: import("../types.js").Track, plays: number|null, note?: string }[], contextName: string }}
   */
  let { items, contextName } = $props();

  const origin = Symbol("repeat");
  const tracks = $derived(items.map((item) => item.track));
</script>

<ol class="list">
  {#each items as { track, plays, note }, i (track.uri)}
    <li>
      <button
        type="button"
        class="row"
        draggable="true"
        ondragstart={(e) => startTrackDrag(e, [track], origin, i)}
        ondragend={endDrag}
        onclick={() => playFromList(tracks, i, contextName)}
        oncontextmenu={(e) => openTrackMenu(e, track)}
      >
        <span class="rank">{i + 1}</span>
        {#if smallestCover(track.album.images)}
          <img src={smallestCover(track.album.images)} alt="" class="thumb" loading="lazy" />
        {:else}
          <span class="thumb placeholder"></span>
        {/if}
        <span class="meta">
          <span class="name" class:playing={player.track?.uri === track.uri}>{track.name}</span>
          <span class="artists">{track.artists.map((a) => a.name).join(", ")}</span>
        </span>
        {#if plays != null}
          <span class="plays">
            {plays.toLocaleString()} {plays === 1 ? "play" : "plays"}
            {#if note}<span class="note">{note}</span>{/if}
          </span>
        {/if}
      </button>
    </li>
  {/each}
</ol>

<style>
.list {
  columns: 2 320px;
  column-gap: 1.5em;
  margin: 0 -0.5em;
  padding: 0;
  list-style: none;
}
li {
  break-inside: avoid;
}
.row {
  display: flex;
  align-items: center;
  gap: 0.8em;
  width: 100%;
  min-width: 0;
  padding: 0.4em 0.5em;
  border: none;
  border-radius: var(--radius-sm);
  background: none;
  color: var(--text);
  font: inherit;
  text-align: left;
  cursor: pointer;
  transition: background-color var(--transition);
}
.row:hover {
  background: var(--surface-hover);
}
.rank {
  width: 1.4em;
  flex-shrink: 0;
  font-size: var(--fs-sm);
  font-variant-numeric: tabular-nums;
  color: var(--text-muted);
  text-align: right;
}
.thumb {
  width: 44px;
  height: 44px;
  flex-shrink: 0;
  border-radius: var(--radius-sm);
  object-fit: cover;
}
.thumb.placeholder {
  background: var(--surface-hover);
}
.meta {
  display: flex;
  flex-direction: column;
  min-width: 0;
  flex: 1;
}
.name,
.artists {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.name {
  font-size: 0.95em;
  font-weight: var(--fw-medium);
}
.name.playing {
  color: var(--accent);
}
.artists {
  font-size: var(--fs-sm);
  color: var(--text-muted);
}
.plays {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  flex-shrink: 0;
  font-size: var(--fs-sm);
  font-variant-numeric: tabular-nums;
  color: var(--text-muted);
}
.note {
  font-size: var(--fs-xs);
}
</style>
