<script>
  import Icon from "./Icon.svelte";
  import { player } from "../player.svelte.js";
  import {
    queue,
    contextLabel,
    upcomingContext,
    playFromQueue,
    skipToContext,
    removeFromQueue,
    clearQueue,
    addToQueue,
    moveInQueue,
  } from "../queue.svelte.js";
  import { drag, startTrackDrag, endDrag, insertionIndex } from "../drag.svelte.js";
  import { openTrackMenu } from "../contextMenu.svelte.js";
  import { smallestCover } from "../utils.js";

  /** @typedef {import("../types.js").Track} Track */

  /** @type {{ onClose: () => void }} */
  let { onClose } = $props();

  const UPCOMING_LIMIT = 100;

  const context = $derived(upcomingContext(UPCOMING_LIMIT));

  const origin = Symbol("queue");
  let dropActive = $state(false);
  let dropIndex = $state(-1);

  /** @param {DragEvent} evt */
  function onPanelDragOver(evt) {
    if (drag.tracks.length === 0 || drag.origin === origin) return;
    evt.preventDefault();
    if (evt.dataTransfer) evt.dataTransfer.dropEffect = "copy";
    dropActive = true;
  }

  /** @param {DragEvent} evt */
  function onPanelDrop(evt) {
    dropActive = false;
    if (drag.tracks.length === 0 || drag.origin === origin) return;
    evt.preventDefault();
    for (const track of drag.tracks) addToQueue(track);
  }

  /**
   * @param {DragEvent & { currentTarget: HTMLElement }} evt
   * @param {number} index
   */
  function onRowDragOver(evt, index) {
    if (drag.origin !== origin) return;
    evt.preventDefault();
    if (evt.dataTransfer) evt.dataTransfer.dropEffect = "move";
    dropIndex = insertionIndex(evt, index);
  }

  /** @param {DragEvent} evt */
  function onRowDrop(evt) {
    if (drag.origin !== origin || dropIndex < 0) return;
    evt.preventDefault();
    moveInQueue(drag.index, dropIndex);
    dropIndex = -1;
  }

  function onDragEnd() {
    dropIndex = -1;
    endDrag();
  }
</script>

{#snippet row(/** @type {Track} */ track, /** @type {boolean} */ playing)}
  {#if smallestCover(track.album.images)}
    <img src={smallestCover(track.album.images)} alt="" class="thumb" loading="lazy" />
  {:else}
    <div class="thumb placeholder"></div>
  {/if}
  <div class="meta">
    <div class="name" class:playing>{track.name}</div>
    <div class="artists">{track.artists.map((a) => a.name).join(", ")}</div>
  </div>
{/snippet}

<div
  class="panel"
  class:drop-active={dropActive}
  role="region"
  aria-label="Queue"
  ondragover={onPanelDragOver}
  ondragleave={(e) => !e.currentTarget.contains(/** @type {Node|null} */ (e.relatedTarget)) && (dropActive = false)}
  ondrop={onPanelDrop}
>
  <div class="panel-header">
    <span class="title">Queue</span>
    <button type="button" class="icon-button" onclick={onClose} aria-label="Close queue">
      <Icon name="close" size={16} />
    </button>
  </div>

  {#if !queue.current}
    <div class="empty">Nothing in the queue yet. Right-click a song and pick "Add to queue".</div>
  {:else}
    <section>
      <h3>Now playing</h3>
      <div class="row static" role="presentation" oncontextmenu={(e) => queue.current && openTrackMenu(e, queue.current)}>
        {@render row(queue.current, !!player.track)}
      </div>
    </section>

    {#if queue.manual.length > 0}
      <section>
        <div class="section-head">
          <h3>Next in queue</h3>
          <button type="button" class="text-button" onclick={clearQueue}>Clear queue</button>
        </div>
        {#each queue.manual as track, i}
          <div
            class="row-wrap"
            class:drop-before={dropIndex === i}
            class:drop-after={dropIndex === queue.manual.length && i === queue.manual.length - 1}
            role="listitem"
            draggable="true"
            ondragstart={(e) => startTrackDrag(e, [track], origin, i)}
            ondragend={onDragEnd}
            ondragover={(e) => onRowDragOver(e, i)}
            ondrop={onRowDrop}
          >
            <button
              type="button"
              class="row"
              onclick={() => playFromQueue(i)}
              oncontextmenu={(e) => openTrackMenu(e, track)}
            >
              {@render row(track, false)}
            </button>
            <button
              type="button"
              class="icon-button remove"
              onclick={() => removeFromQueue(i)}
              aria-label={`Remove ${track.name} from queue`}
            >
              <Icon name="close" size={14} />
            </button>
          </div>
        {/each}
      </section>
    {/if}

    {#if context.upcoming.length > 0}
      <section>
        <h3>Next from: {contextLabel() || "this list"}</h3>
        {#each context.upcoming as { track, position } (position)}
          <button
            type="button"
            class="row"
            onclick={() => skipToContext(position)}
            oncontextmenu={(e) => openTrackMenu(e, track)}
          >
            {@render row(track, false)}
          </button>
        {/each}
        {#if context.more > 0}
          <div class="more">and {context.more} more</div>
        {/if}
      </section>
    {/if}
  {/if}
</div>

<style>
.panel {
  display: flex;
  flex-direction: column;
  gap: 1.1em;
  width: 100%;
  height: 100%;
  padding: 1em;
  overflow-x: hidden;
  overflow-y: auto;
}
.panel-header,
.section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5em;
}
.title {
  font-size: var(--fs-md);
  font-weight: var(--fw-bold);
  color: var(--text);
}
h3 {
  margin: 0 0 0.5em;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  color: var(--text);
}
.section-head h3 {
  margin: 0;
}
.section-head {
  margin-bottom: 0.5em;
}
.empty {
  padding: 2em 0.5em;
  font-size: var(--fs-sm);
  color: var(--text-muted);
  text-align: center;
}
.row-wrap {
  position: relative;
  display: flex;
  align-items: center;
  border-radius: var(--radius-md);
}
.row-wrap:hover {
  background-color: var(--surface-hover);
}
.row-wrap.drop-before {
  box-shadow: inset 0 2px 0 var(--accent);
}
.row-wrap.drop-after {
  box-shadow: inset 0 -2px 0 var(--accent);
}
.panel.drop-active {
  box-shadow: inset 0 0 0 2px var(--accent);
  border-radius: var(--radius-md);
}
.row {
  display: flex;
  align-items: center;
  gap: 0.7em;
  flex: 1;
  min-width: 0;
  width: 100%;
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
.row-wrap .row:hover {
  background: none;
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
  background: var(--surface-raised);
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
.name.playing {
  color: var(--accent);
}
.artists {
  margin-top: 0.15em;
  font-size: var(--fs-xs);
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.icon-button {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  flex-shrink: 0;
  border: none;
  border-radius: 50%;
  background: none;
  color: var(--text-muted);
  cursor: pointer;
  transition: background-color var(--transition), color var(--transition);
}
.icon-button:hover {
  background-color: var(--surface-hover);
  color: var(--text);
}
.remove {
  margin-right: 0.3em;
  opacity: 0;
}
.row-wrap:hover .remove,
.remove:focus-visible {
  opacity: 1;
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
.more {
  padding: 0.5em;
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
</style>
