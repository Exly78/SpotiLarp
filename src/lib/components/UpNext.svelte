<script>
  import { fade } from "svelte/transition";
  import { queue, upcomingContext, playFromQueue, skipToContext } from "../queue.svelte.js";
  import { openTrackMenu } from "../contextMenu.svelte.js";
  import { jam } from "../jam.svelte.js";
  import { smallestCover } from "../utils.js";

  /** @typedef {import("../types.js").Track} Track */

  /** @type {{ onShowQueue?: () => void }} */
  let { onShowQueue } = $props();

  const LIMIT = 3;

  /** @type {{ track: Track, by?: string, play: (() => void) | null }[]} */
  const upcoming = $derived.by(() => {
    // In a Jam the host's app picks what's next; a guest only gets to look.
    if (jam.role === "guest") return jam.upNext.slice(0, LIMIT).map(({ track, by }) => ({ track, by, play: null }));
    const manual = queue.manual.slice(0, LIMIT).map((track, i) => ({ track, play: () => playFromQueue(i) }));
    const context = upcomingContext(LIMIT - manual.length).upcoming.map(({ track, position }) => ({
      track,
      play: () => skipToContext(position),
    }));
    return [...manual, ...context];
  });
</script>

{#if upcoming.length > 0}
  <section class="up-next" transition:fade={{ duration: 200 }}>
    <div class="head">
      <h3>{jam.role === "guest" ? "Next in the Jam" : "Next in queue"}</h3>
      {#if onShowQueue}
        <button type="button" class="text-button" onclick={onShowQueue}>Open queue</button>
      {/if}
    </div>
    {#each upcoming as { track, by, play }}
      {@const addedBy = by ?? track.addedBy}
      {@const cover = smallestCover(track.album.images)}
      <button
        type="button"
        class="row"
        class:static={!play}
        onclick={() => play?.()}
        oncontextmenu={(e) => openTrackMenu(e, track)}
        title={play ? `Play ${track.name}` : undefined}
      >
        {#if cover}
          <img src={cover} alt="" class="thumb" loading="lazy" />
        {:else}
          <div class="thumb placeholder"></div>
        {/if}
        <div class="meta">
          <div class="name">{track.name}</div>
          <div class="artists">{track.artists.map((a) => a.name).join(", ")}{#if addedBy} · Added by {addedBy}{/if}</div>
        </div>
      </button>
    {/each}
  </section>
{/if}

<style>
.up-next {
  flex-shrink: 0;
  padding: 0.9em 0.5em 0.5em;
  border-radius: var(--radius-lg);
  background: var(--surface-raised);
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5em;
  margin: 0 0.5em 0.4em;
}
h3 {
  margin: 0;
  font-size: var(--fs-md);
  font-weight: var(--fw-bold);
  color: var(--text);
}
.text-button {
  padding: 0;
  border: none;
  background: none;
  color: var(--text-muted);
  font-family: inherit;
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.text-button:hover {
  color: var(--text);
  text-decoration: underline;
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
}
.row:hover {
  background-color: var(--surface-hover);
}
.row.static {
  cursor: default;
}
.row.static:hover {
  background: none;
}
.thumb {
  width: 40px;
  height: 40px;
  border-radius: var(--radius-sm);
  object-fit: cover;
  background: var(--surface-hover);
  flex-shrink: 0;
}
.thumb.placeholder {
  background: linear-gradient(135deg, var(--surface-raised), var(--surface-hover));
}
.meta {
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
  margin-top: 0.15em;
  font-size: var(--fs-xs);
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
