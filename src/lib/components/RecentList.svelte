<script>
  import { onMount } from "svelte";
  import { tracker, recentPlays, syncSpotifyPlays, asTrack, isPlayable } from "../tracker.svelte.js";
  import { playFromList } from "../queue.svelte.js";
  import { player } from "../player.svelte.js";
  import { openTrackMenu } from "../contextMenu.svelte.js";
  import { startTrackDrag, endDrag } from "../drag.svelte.js";
  import { smallestCover, timeAgo } from "../utils.js";

  /** @typedef {import("../types.js").Track} Track */

  const SHOWN = 100;

  const origin = Symbol("recent");

  // The tracker has SpotiLarp's plays; this brings in what Spotify's other apps played since it last asked.
  onMount(() => {
    syncSpotifyPlays();
  });

  const entries = $derived(
    recentPlays(SHOWN * 2)
      .filter((play) => isPlayable(play.track))
      .slice(0, SHOWN)
      .map((play) => ({ track: asTrack(play.track), at: play.t })),
  );
  const loading = $derived(!tracker.loaded || tracker.syncing);

  // A recent row plays that song on its own (autoplay carries on after it).
  /** @param {Track} track */
  function play(track) {
    playFromList([track], 0, "Recently played");
  }
</script>

{#if entries.length === 0}
  <div class="empty">{loading ? "Loading..." : "Songs you play show up here."}</div>
{:else}
  {#each entries as { track, at }, i (`${track.uri}:${at}:${i}`)}
    <button
      type="button"
      class="row"
      draggable="true"
      ondragstart={(e) => startTrackDrag(e, [track], origin, i)}
      ondragend={endDrag}
      onclick={() => play(track)}
      oncontextmenu={(e) => openTrackMenu(e, track)}
    >
      {#if smallestCover(track.album.images)}
        <img src={smallestCover(track.album.images)} alt="" class="thumb" loading="lazy" />
      {:else}
        <div class="thumb placeholder"></div>
      {/if}
      <div class="meta">
        <div class="name" class:playing={player.track?.uri === track.uri}>{track.name}</div>
        <div class="artists">{track.artists.map((a) => a.name).join(", ")}</div>
      </div>
      <span class="when">{timeAgo(at)}</span>
    </button>
  {/each}
{/if}

<style>
.empty {
  padding: 2em 0.5em;
  font-size: var(--fs-sm);
  color: var(--text-muted);
  text-align: center;
}
.row {
  display: flex;
  align-items: center;
  gap: 0.7em;
  width: 100%;
  min-width: 0;
  padding: 0.4em 0.5em;
  border: none;
  border-radius: var(--radius-md);
  background: none;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
  transition: background-color var(--transition);
}
.row:hover {
  background-color: var(--surface-hover);
}
.thumb {
  width: 40px;
  height: 40px;
  flex-shrink: 0;
  border-radius: var(--radius-sm);
  object-fit: cover;
  background: var(--surface-raised);
}
.thumb.placeholder {
  background: linear-gradient(135deg, var(--surface-raised), var(--surface-hover));
}
.meta {
  flex: 1;
  min-width: 0;
}
.name,
.artists {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.name {
  font-size: var(--fs-sm);
  font-weight: var(--fw-medium);
  color: var(--text);
}
.artists {
  margin-top: 0.15em;
}
.name.playing {
  color: var(--accent);
}
.artists,
.when {
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
.when {
  flex-shrink: 0;
}
</style>
