<script module>
  /** The list keyboard shortcuts (Ctrl+A, Ctrl+C, Delete...) act on: the last one used, or a page's primary list. */
  const lists = { active: /** @type {symbol|null} */ (null) };
</script>

<script>
  import { onDestroy, untrack } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { playFromList } from "../queue.svelte.js";
  import { player } from "../player.svelte.js";
  import { likedIds, likeBusy, toggleLike, queueLikedCheck } from "../likes.svelte.js";
  import { menu, openTracksMenu } from "../contextMenu.svelte.js";
  import { openArtist, openAlbum } from "../nav.svelte.js";
  import { drag, startTrackDrag, endDrag, insertionIndex } from "../drag.svelte.js";
  import { notify } from "../toast.svelte.js";
  import { trackLink, trackUrisIn } from "../links.js";
  import { isTextField } from "../keys.js";
  import { smallestCover, formatTime, formatCount } from "../utils.js";
  import Icon from "./Icon.svelte";

  /** @typedef {import("../types.js").Track} Track */

  /**
   * Rows may carry `playcount` and `explicit` (an artist's popular tracks do).
   * Ctrl/Shift+click selects rows; a plain click plays.
   * @type {{
   *   tracks: (Track & { playcount?: number|null, explicit?: boolean })[],
   *   contextName?: string,
   *   contextView?: import("../nav.svelte.js").View|null,
   *   allTracks?: Track[],
   *   primary?: boolean,
   *   showHeader?: boolean,
   *   showArtists?: boolean,
   *   showAlbum?: boolean,
   *   showDateAdded?: boolean,
   *   showPlaycount?: boolean,
   *   showCovers?: boolean,
   *   onRemove?: (tracks: Track[]) => void,
   *   onReorder?: (from: number, insertBefore: number) => void,
   *   onPaste?: (uris: string[]) => void,
   * }}
   */
  let {
    tracks,
    contextName = "",
    // The page these rows belong to, for "Playing from" in the queue.
    contextView = null,
    // When `tracks` shows only the first few of a list, the whole list to queue.
    allTracks,
    // The page's main list, which Ctrl+A selects before any list is clicked.
    primary = false,
    showHeader = true,
    showArtists = true,
    showAlbum = true,
    showDateAdded = false,
    showPlaycount = false,
    showCovers = true,
    onRemove,
    onReorder,
    onPaste,
  } = $props();

  const origin = Symbol("track-list");
  let dropIndex = $state(-1);
  /** Indices into `tracks`. */
  const selected = new SvelteSet();
  let anchor = -1;

  const FIRST_BATCH = 120;
  const BATCH = 400;
  const ANIMATED_ROWS = 20;

  let shown = $state(FIRST_BATCH);

  // Liked hearts are asked about as rows come near the screen, so a long
  // playlist doesn't cost a request per 40 songs every time it opens.
  /** @type {IntersectionObserver|null} */
  const likeWatcher =
    typeof IntersectionObserver === "undefined"
      ? null
      : new IntersectionObserver(
          (entries) => {
            for (const entry of entries) {
              if (!entry.isIntersecting) continue;
              const el = /** @type {HTMLElement} */ (entry.target);
              queueLikedCheck(el.dataset.likeId ?? "");
              likeWatcher?.unobserve(el);
            }
          },
          { rootMargin: "800px 0px" },
        );
  onDestroy(() => likeWatcher?.disconnect());
  /** @type {HTMLUListElement|undefined} */
  let listEl = $state();

  // Rows come and go (and are reused for other songs) as the list changes.
  $effect(() => {
    visible;
    const list = listEl;
    if (!list || !likeWatcher) return;
    for (const row of list.querySelectorAll("li[data-like-id]")) likeWatcher.observe(row);
  });
  const visible = $derived(tracks.length > shown ? tracks.slice(0, shown) : tracks);

  $effect(() => {
    const total = tracks.length;
    shown = FIRST_BATCH;
    let frame = 0;
    const grow = () => {
      if (shown >= total) return;
      shown += BATCH;
      frame = requestAnimationFrame(grow);
    };
    frame = requestAnimationFrame(grow);
    return () => cancelAnimationFrame(frame);
  });

  // Filtering, sorting or reloading changes what the indices mean.
  $effect(() => {
    tracks;
    untrack(() => {
      selected.clear();
      anchor = -1;
    });
  });

  $effect(() => {
    if (primary && lists.active === null) lists.active = origin;
  });

  onDestroy(() => {
    if (lists.active === origin) lists.active = null;
  });

  const dateFormatter = new Intl.DateTimeFormat(undefined, { year: "numeric", month: "short", day: "numeric" });

  /** @param {string|null|undefined} date */
  function formatDate(date) {
    if (!date) return "";
    const parsed = new Date(date);
    return Number.isNaN(parsed.getTime()) ? "" : dateFormatter.format(parsed);
  }

  /** The selected rows, in the order shown. */
  function selectedTracks() {
    return [...selected].sort((a, b) => a - b).map((i) => tracks[i]).filter(Boolean);
  }

  /**
   * @param {MouseEvent} evt
   * @param {number} index
   */
  function onRowClick(evt, index) {
    lists.active = origin;
    if (evt.shiftKey) {
      const from = anchor >= 0 ? anchor : index;
      selected.clear();
      for (let i = Math.min(from, index); i <= Math.max(from, index); i++) selected.add(i);
      anchor = from;
      return;
    }
    if (evt.ctrlKey || evt.metaKey) {
      if (selected.has(index)) selected.delete(index);
      else selected.add(index);
      anchor = index;
      return;
    }
    selected.clear();
    anchor = index;
    playFromList(allTracks ?? tracks, index, contextName, contextView);
  }

  /** @param {MouseEvent} evt */
  function onRowMouseDown(evt) {
    // Shift+click would otherwise select the page's text.
    if (evt.shiftKey) evt.preventDefault();
  }

  /**
   * @param {DragEvent} evt
   * @param {number} index
   */
  function onDragStart(evt, index) {
    lists.active = origin;
    const dragging = selected.has(index) ? selectedTracks() : [tracks[index]];
    startTrackDrag(evt, dragging, origin, index);
  }

  /**
   * @param {DragEvent & { currentTarget: HTMLElement }} evt
   * @param {number} index
   */
  function onRowDragOver(evt, index) {
    if (!onReorder || drag.origin !== origin || drag.tracks.length !== 1) return;
    evt.preventDefault();
    if (evt.dataTransfer) evt.dataTransfer.dropEffect = "move";
    dropIndex = insertionIndex(evt, index);
  }

  /** @param {DragEvent} evt */
  function onRowDrop(evt) {
    if (!onReorder || drag.origin !== origin || dropIndex < 0) return;
    evt.preventDefault();
    const from = drag.index;
    const insertBefore = dropIndex;
    dropIndex = -1;
    if (insertBefore !== from && insertBefore !== from + 1) onReorder(from, insertBefore);
  }

  function onDragEnd() {
    dropIndex = -1;
    endDrag();
  }

  /**
   * @param {MouseEvent} evt
   * @param {number} index
   */
  function onContextMenu(evt, index) {
    lists.active = origin;
    if (!selected.has(index)) {
      selected.clear();
      anchor = index;
    }
    const chosen = selected.size > 1 ? selectedTracks() : [tracks[index]];
    openTracksMenu(evt, chosen, onRemove && (() => onRemove(chosen)));
  }

  /**
   * @param {Event} evt
   * @param {boolean} [textMayBeSelected] whether a text selection on the page should take precedence
   */
  function listShortcutsApply(evt, textMayBeSelected = false) {
    if (lists.active !== origin || menu.open || isTextField(evt.target)) return false;
    return !textMayBeSelected || !window.getSelection()?.toString();
  }

  /** @param {KeyboardEvent} evt */
  function onWindowKeydown(evt) {
    if (!listShortcutsApply(evt)) return;
    const ctrl = evt.ctrlKey || evt.metaKey;
    if (ctrl && !evt.shiftKey && !evt.altKey && evt.key.toLowerCase() === "a") {
      evt.preventDefault();
      for (let i = 0; i < tracks.length; i++) selected.add(i);
    } else if (evt.key === "Escape" && selected.size > 0) {
      selected.clear();
    } else if (evt.key === "Delete" && onRemove && selected.size > 0) {
      evt.preventDefault();
      onRemove(selectedTracks());
    }
  }

  /**
   * Ctrl+C and Ctrl+X put the selected songs' links on the clipboard, one per
   * line, ready to paste in a playlist here or in Spotify.
   * @param {ClipboardEvent} evt
   * @param {boolean} cut
   */
  function copySelection(evt, cut) {
    if (!listShortcutsApply(evt, true) || selected.size === 0) return;
    const chosen = selectedTracks();
    const links = chosen.map(trackLink).filter(Boolean);
    if (links.length === 0 || !evt.clipboardData) return;
    evt.preventDefault();
    evt.clipboardData.setData("text/plain", links.join("\n"));
    if (cut && onRemove) {
      onRemove(chosen);
    } else {
      notify(links.length === 1 ? "Link copied" : `Copied ${links.length} song links`);
    }
  }

  /** @param {ClipboardEvent} evt */
  function onWindowPaste(evt) {
    if (!onPaste || !listShortcutsApply(evt)) return;
    const uris = trackUrisIn(evt.clipboardData?.getData("text/plain") ?? "");
    if (uris.length === 0) return;
    evt.preventDefault();
    onPaste(uris);
  }
</script>

<svelte:window
  onkeydown={onWindowKeydown}
  oncopy={(e) => copySelection(e, false)}
  oncut={(e) => copySelection(e, true)}
  onpaste={onWindowPaste}
/>

<ul
  class="track-list"
  bind:this={listEl}
  class:with-album={showAlbum}
  class:with-date={showDateAdded}
  class:with-plays={showPlaycount}
>
  {#if showHeader}
    <li class="row header" aria-hidden="true">
      <span class="index">#</span>
      <span class="title-cell">Title</span>
      {#if showAlbum}<span class="album-cell">Album</span>{/if}
      {#if showDateAdded}<span class="date-cell">Date added</span>{/if}
      {#if showPlaycount}<span class="plays-cell">Plays</span>{/if}
      <span></span>
      <span class="duration"><Icon name="clock" size={15} /></span>
    </li>
  {/if}
  {#each visible as track, i}
    <li
      class="row"
      class:playing={track.id ? track.id === player.track?.trackId : !!player.track?.uri && track.uri === player.track.uri}
      class:selected={selected.has(i)}
      class:drop-before={dropIndex === i}
      class:drop-after={dropIndex === tracks.length && i === tracks.length - 1}
      draggable="true"
      ondragstart={(e) => onDragStart(e, i)}
      ondragend={onDragEnd}
      ondragover={(e) => onRowDragOver(e, i)}
      ondrop={onRowDrop}
      oncontextmenu={(e) => onContextMenu(e, i)}
      data-like-id={track.id || undefined}
      in:fly={{
        y: 8,
        duration: i < ANIMATED_ROWS ? 220 : 0,
        delay: i < ANIMATED_ROWS ? i * 20 : 0,
        easing: cubicOut,
      }}
    >
      <button
        type="button"
        class="hit"
        onmousedown={onRowMouseDown}
        onclick={(e) => onRowClick(e, i)}
        aria-label={`Play ${track.name}`}
      ></button>
      <span class="index">{i + 1}</span>
      <div class="title-cell">
        {#if showCovers}
          {#if smallestCover(track.album.images)}
            <img src={smallestCover(track.album.images)} alt="" class="thumb" loading="lazy" />
          {:else}
            <div class="thumb placeholder"></div>
          {/if}
        {/if}
        <div class="meta">
          <div class="name">{track.name}</div>
          {#if showArtists}
            <div class="artists">
              {#if track.explicit}<span class="explicit" title="Explicit">E</span>{/if}
              {#each track.artists as artist, j}
                {#if j > 0},&nbsp;{/if}{#if artist.id}<button type="button" class="link" onclick={() => openArtist(artist)}>{artist.name}</button>{:else}<span>{artist.name}</span>{/if}
              {/each}
            </div>
          {:else if track.explicit}
            <div class="artists"><span class="explicit" title="Explicit">E</span></div>
          {/if}
        </div>
      </div>
      {#if showAlbum}
        <div class="album-cell">
          {#if track.album.id}
            <button type="button" class="link" onclick={() => openAlbum(track.album)}>{track.album.name}</button>
          {:else}
            <span>{track.album.name}</span>
          {/if}
        </div>
      {/if}
      {#if showDateAdded}
        <div class="date-cell">{formatDate(track.added_at)}</div>
      {/if}
      {#if showPlaycount}
        <div class="plays-cell">{track.playcount != null ? formatCount(track.playcount) : ""}</div>
      {/if}
      {#if track.id}
        <button
          type="button"
          class="like-button"
          class:liked={likedIds.has(track.id)}
          disabled={likeBusy.has(track.id)}
          onclick={() => toggleLike(track.id)}
          aria-label={likedIds.has(track.id) ? "Unlike" : "Like"}
        >
          <Icon name={likedIds.has(track.id) ? "heart-filled" : "heart"} size={16} />
        </button>
      {:else}
        <span></span>
      {/if}
      <div class="duration">{formatTime(track.duration_ms)}</div>
    </li>
  {/each}
</ul>

<style>
.track-list {
  container: tracks / inline-size;
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.row {
  position: relative;
  display: grid;
  grid-template-columns: 2.4em minmax(0, 1fr) 32px 3.6em;
  align-items: center;
  gap: 0.8em;
  min-height: 56px;
  padding: 0 0.6em;
  border-radius: var(--radius-md);
  transition: background-color var(--transition);
  content-visibility: auto;
  contain-intrinsic-size: auto 56px;
}
.with-album .row {
  grid-template-columns: 2.4em minmax(0, 1.5fr) minmax(0, 1fr) 32px 3.6em;
}
.with-date .row {
  grid-template-columns: 2.4em minmax(0, 1fr) 8em 32px 3.6em;
}
.with-album.with-date .row {
  grid-template-columns: 2.4em minmax(0, 1.5fr) minmax(0, 1fr) 8em 32px 3.6em;
}
.with-plays .row {
  grid-template-columns: 2.4em minmax(0, 4fr) minmax(0, 2fr) 32px 3.6em;
}
@container tracks (max-width: 720px) {
  .with-album .row,
  .with-date .row,
  .with-album.with-date .row {
    grid-template-columns: 2.4em minmax(0, 1fr) 32px 3.6em;
  }
  .album-cell,
  .date-cell {
    display: none;
  }
}
@container tracks (max-width: 480px) {
  .with-plays .row {
    grid-template-columns: 2.4em minmax(0, 1fr) 32px 3.6em;
  }
  .plays-cell {
    display: none;
  }
}
.row:not(.header):hover {
  background-color: var(--surface-hover);
}
.row.selected,
.row.selected:hover {
  background-color: var(--selected-row-bg);
}
.row.drop-before {
  box-shadow: inset 0 2px 0 var(--accent);
}
.row.drop-after {
  box-shadow: inset 0 -2px 0 var(--accent);
}
.row.header {
  min-height: 0;
  padding-bottom: 0.5em;
  margin-bottom: 0.4em;
  border-bottom: 1px solid var(--border);
  border-radius: 0;
  font-size: var(--fs-xs);
  color: var(--text-muted);
  content-visibility: visible;
}
.hit {
  position: absolute;
  inset: 0;
  border: none;
  border-radius: var(--radius-md);
  background: none;
  cursor: pointer;
}
.row > :not(.hit) {
  position: relative;
  pointer-events: none;
}
.row .link,
.row .like-button {
  pointer-events: auto;
}
.index {
  text-align: right;
  font-size: var(--fs-sm);
  font-variant-numeric: tabular-nums;
  color: var(--text-muted);
}
.row.playing .index,
.row.playing .name {
  color: var(--accent);
}
.title-cell {
  display: flex;
  align-items: center;
  gap: 0.8em;
  min-width: 0;
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
.artists,
.album-cell,
.date-cell,
.plays-cell {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.artists {
  margin-top: 0.15em;
}
.album-cell,
.date-cell,
.plays-cell {
  font-size: var(--fs-sm);
}
.plays-cell {
  font-variant-numeric: tabular-nums;
}
.explicit {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  width: 1.45em;
  height: 1.45em;
  margin-right: 0.45em;
  border-radius: 2px;
  background: var(--text-muted);
  color: var(--surface);
  font-size: 0.7em;
  font-weight: var(--fw-bold);
  line-height: 1;
  vertical-align: 0.1em;
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
.like-button {
  display: flex;
  align-items: center;
  justify-content: center;
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
  display: flex;
  justify-content: flex-end;
  font-size: var(--fs-xs);
  font-variant-numeric: tabular-nums;
  color: var(--text-muted);
}
</style>
