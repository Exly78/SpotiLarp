<script>
  import { untrack } from "svelte";
  import { fade } from "svelte/transition";
  import * as api from "../api.js";
  import { player, seekTo } from "../player.svelte.js";

  /** @type {{ visible?: boolean }} */
  let { visible = true } = $props();

  const MANUAL_SCROLL_GRACE_MS = 3000;

  let loading = $state(false);
  let error = $state("");
  /** @type {import("../types.js").LyricLine[]|null} */
  let synced = $state(null);
  /** @type {string|null} */
  let plain = $state(null);
  let instrumental = $state(false);
  let loadedFor = "";

  /** @type {HTMLElement[]} */
  let lineEls = [];
  /** @type {HTMLDivElement|undefined} */
  let container = $state();
  let manualScrollAt = 0;
  let wasVisible = false;

  const activeIndex = $derived.by(() => {
    if (!synced) return -1;
    let idx = -1;
    for (let i = 0; i < synced.length; i++) {
      if (synced[i].time_ms <= player.positionMs) idx = i;
      else break;
    }
    return idx;
  });

  $effect(() => {
    const track = player.track;
    if (!track || !visible) return;
    const key = `${track.name}::${track.artists}::${track.album}`;
    if (key !== loadedFor) untrack(() => loadLyrics(track, key));
  });

  /**
   * @param {import("../player.svelte.js").CurrentTrack} track
   * @param {string} key
   */
  async function loadLyrics(track, key) {
    loadedFor = key;
    loading = true;
    error = "";
    synced = null;
    plain = null;
    instrumental = false;
    try {
      const firstArtist = track.artists.split(",")[0]?.trim() ?? track.artists;
      const result = await api.getLyrics(track.name, firstArtist, track.album, track.durationMs);
      if (loadedFor !== key) return;
      if (result) {
        instrumental = result.instrumental;
        synced = result.synced && result.synced.length > 0 ? result.synced : null;
        plain = result.plain;
      }
    } catch (e) {
      if (loadedFor === key) error = `Failed to load lyrics: ${e}`;
    } finally {
      if (loadedFor === key) loading = false;
    }
  }

  /** @param {ScrollBehavior} behavior */
  function scrollToActive(behavior) {
    const el = lineEls[activeIndex];
    if (!container || !el) return;
    const top = el.offsetTop - container.clientHeight / 2 + el.offsetHeight / 2;
    container.scrollTo({ top, behavior });
  }

  $effect(() => {
    const idx = activeIndex;
    const shown = visible;
    untrack(() => {
      const justShown = shown && !wasVisible;
      wasVisible = shown;
      if (!shown || idx < 0) return;
      if (justShown) {
        manualScrollAt = 0;
        scrollToActive("auto");
      } else if (performance.now() - manualScrollAt > MANUAL_SCROLL_GRACE_MS) {
        scrollToActive("smooth");
      }
    });
  });

  function onManualScroll() {
    manualScrollAt = performance.now();
  }

  /** @param {number} timeMs */
  function seekToLine(timeMs) {
    manualScrollAt = 0;
    seekTo(timeMs);
  }

</script>

<div
  class="lyrics"
  style={player.color ? `--avg-color: ${player.color}` : ""}
  bind:this={container}
  onwheel={onManualScroll}
>
  {#if !player.track}
    <div class="empty-state" transition:fade={{ duration: 150 }}>Nothing playing.</div>
  {:else if loading}
    <ul class="skeleton-list" transition:fade={{ duration: 150 }}>
      {#each { length: 8 } as _}
        <li class="skeleton line"></li>
      {/each}
    </ul>
  {:else if error}
    <p class="status error" transition:fade={{ duration: 150 }}>{error}</p>
  {:else if instrumental}
    <div class="empty-state" transition:fade={{ duration: 150 }}>Instrumental, no lyrics.</div>
  {:else if synced}
    <div class="synced-lines" transition:fade={{ duration: 150 }}>
      {#each synced as line, i}
        <button
          type="button"
          bind:this={lineEls[i]}
          class="line"
          class:active={i === activeIndex}
          class:past={i < activeIndex}
          onclick={() => seekToLine(line.time_ms)}
        >
          {line.text || " "}
        </button>
      {/each}
    </div>
  {:else if plain}
    <div class="plain-lyrics" transition:fade={{ duration: 150 }}>
      {#each plain.split("\n") as line}
        <p class="line">{line || " "}</p>
      {/each}
    </div>
  {:else}
    <div class="empty-state" transition:fade={{ duration: 150 }}>No lyrics found for this track.</div>
  {/if}
</div>

<style>
.lyrics {
  position: absolute;
  inset: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  padding: 1.5em 2em;
  border-radius: var(--radius-md);
  background-color: var(--avg-color, var(--surface-raised));
}
.empty-state {
  color: var(--text-muted);
  font-size: var(--fs-sm);
  padding: 2.5em 1em;
  text-align: center;
  margin: auto;
}
.status {
  font-size: var(--fs-sm);
  color: var(--text-muted);
}
.status.error {
  color: var(--danger);
}
.synced-lines,
.plain-lyrics {
  display: flex;
  flex-direction: column;
  gap: 0.55em;
  padding: 2em 0.5em 60vh 2em;
  margin: auto 0;
}
.synced-lines .line {
  margin: 0;
  font-size: 2.3em;
  font-weight: var(--fw-bold);
  letter-spacing: -0.01em;
  
  color: color-mix(in srgb, var(--avg-color, var(--surface-raised)) 55%, white 45%);
  transition: color var(--transition), transform var(--transition);
  transform-origin: left center;
}
.synced-lines .line {
  display: block;
  width: 100%;
  padding: 0;
  border: none;
  background: none;
  font-family: inherit;
  line-height: inherit;
  text-align: left;
  cursor: pointer;
}
.synced-lines .line:hover:not(.active) {
  color: color-mix(in srgb, var(--avg-color, var(--surface-raised)) 25%, white 75%);
}
.synced-lines .line.past {
  color: color-mix(in srgb, var(--avg-color, var(--surface-raised)) 75%, white 25%);
}
.synced-lines .line.active {
  color: var(--text);
  transform: scale(1.03);
}
.plain-lyrics .line {
  margin: 0;
  font-size: 1.5em;
  font-weight: var(--fw-medium);
  color: color-mix(in srgb, var(--avg-color, var(--surface-raised)) 55%, white 45%);
}
.skeleton-list {
  list-style: none;
  margin: auto 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 1em;
}
.skeleton-list .line {
  height: 1.2em;
  width: 70%;
  border-radius: var(--radius-sm);
}
.skeleton-list .line:nth-child(3n+2) {
  width: 45%;
}
.skeleton-list .line:nth-child(3n) {
  width: 85%;
}
</style>
