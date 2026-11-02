<script>
  import { untrack } from "svelte";
  import { fade } from "svelte/transition";
  import { nav } from "../nav.svelte.js";
  import { player, seekTo } from "../player.svelte.js";
  import { isEpisodeUri } from "../podcasts.svelte.js";
  import { activeLine, fetchLyrics, lyricsKey } from "../lyrics.js";
  import { canRomanize, romanization, romanizeLabel, romanizeLines, setRomanized } from "../romanize.svelte.js";
  import Icon from "./Icon.svelte";

  /** @type {{ visible?: boolean }} */
  let { visible = true } = $props();

  /** Room left above the line before the playing one when it scrolls up. */
  const SCROLL_MARGIN_PX = 6;
  const PLAIN_LINES = 8;

  /** @type {import("../types.js").LyricLine[]|null} */
  let synced = $state(null);
  /** @type {string[]|null} */
  let plain = $state(null);
  let loadedFor = "";

  /** @type {HTMLElement[]} */
  let lineEls = [];
  /** @type {HTMLDivElement|undefined} */
  let container = $state();
  /** @type {HTMLDivElement|undefined} */
  let scrolled;

  const episode = $derived(isEpisodeUri(player.track?.uri));
  const activeIndex = $derived(activeLine(synced, player.positionMs));
  // The lyrics view is already showing them.
  const shown = $derived(!!player.track && !episode && !nav.lyrics && (synced !== null || plain !== null));

  /** The lines as written, before any romanizing. */
  const sourceLines = $derived.by(() => (synced ? synced.map((line) => line.text) : plain));
  const romanizable = $derived(!!sourceLines && canRomanize(sourceLines));
  /** @type {string[]|null} */
  let romanized = $state(null);

  $effect(() => {
    const lines = sourceLines;
    romanized = null;
    if (!lines || !romanizable || !romanization.on) return;
    romanizeLines(lines)
      .then((result) => {
        if (sourceLines === lines) romanized = result;
      })
      .catch((e) => console.warn("Couldn't romanize the lyrics:", e));
  });

  $effect(() => {
    const track = player.track;
    if (!track || !visible || episode) return;
    const key = lyricsKey(track);
    if (key !== loadedFor) untrack(() => load(track, key));
  });

  /**
   * @param {import("../player.svelte.js").CurrentTrack} track
   * @param {string} key
   */
  async function load(track, key) {
    loadedFor = key;
    synced = null;
    plain = null;
    try {
      const result = await fetchLyrics(track);
      if (loadedFor !== key || !result || result.instrumental) return;
      if (result.synced?.length) synced = result.synced;
      else if (result.plain?.trim()) plain = result.plain.split("\n").slice(0, PLAIN_LINES);
    } catch (e) {
      // The lyrics view says what went wrong; the card just stays away.
      console.warn("Couldn't load lyrics for the Now Playing panel:", e);
    }
  }

  // Keeps the playing line second from the top, so the one before it stays in view.
  $effect(() => {
    const idx = activeIndex;
    const box = container;
    // Romanized lines wrap differently, so the scroll is redone when they arrive.
    void romanized;
    if (!box || !synced) return;
    untrack(() => {
      const target = lineEls[Math.max(idx - 1, 0)];
      if (!target) return;
      box.scrollTo({ top: target.offsetTop - SCROLL_MARGIN_PX, behavior: box === scrolled ? "smooth" : "auto" });
      scrolled = box;
    });
  });
</script>

{#if shown}
  <section class="lyrics-card" style={player.color ? `--avg-color: ${player.color}` : ""} transition:fade={{ duration: 200 }}>
    <div class="head">
      <h3>Lyrics</h3>
      {#if romanizable && sourceLines}
        <button
          type="button"
          class="romanize-toggle"
          class:on={romanization.on}
          aria-pressed={romanization.on}
          onclick={() => setRomanized(!romanization.on)}
          title={romanization.on ? "Show the lyrics as written" : "Show the lyrics in Latin letters"}
        >
          {romanizeLabel(sourceLines)}
        </button>
      {/if}
      <button type="button" class="open-button" onclick={() => (nav.lyrics = true)} aria-label="Show lyrics" title="Show lyrics (L)">
        <Icon name="expand" size={14} />
      </button>
    </div>
    {#if synced}
      <div class="lines" bind:this={container}>
        {#each synced as line, i}
          <button
            type="button"
            bind:this={lineEls[i]}
            class="line"
            class:active={i === activeIndex}
            class:past={i < activeIndex}
            onclick={() => seekTo(line.time_ms)}
          >
            {(romanized?.[i] ?? line.text) || "♪"}
          </button>
        {/each}
      </div>
    {:else if plain}
      <button type="button" class="lines plain" onclick={() => (nav.lyrics = true)}>
        {#each plain as line, i}
          <span class="line">{(romanized?.[i] ?? line) || " "}</span>
        {/each}
      </button>
    {/if}
  </section>
{/if}

<style>
.lyrics-card {
  flex-shrink: 0;
  padding: 0.9em 1em 0.6em;
  border-radius: var(--radius-lg);
  background: var(--avg-color, var(--surface-raised));
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5em;
  margin-bottom: 0.5em;
}
h3 {
  margin: 0;
  font-size: var(--fs-md);
  font-weight: var(--fw-bold);
  color: var(--text);
}
.romanize-toggle {
  margin-left: auto;
  padding: 0.25em 0.75em;
  border: none;
  border-radius: var(--radius-pill);
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  cursor: pointer;
  transition: background-color var(--transition), color var(--transition);
}
.romanize-toggle:hover {
  background: rgba(0, 0, 0, 0.45);
}
.romanize-toggle.on {
  background: var(--text);
  color: #000;
}
.open-button {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  border: none;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  cursor: pointer;
  transition: background-color var(--transition), transform var(--spring);
}
.open-button:hover {
  background: rgba(0, 0, 0, 0.45);
  transform: scale(1.08);
}
.lines {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 0.35em;
  height: 9.5em;
  overflow: hidden;
  padding: 0.3em 0 2em;
  -webkit-mask-image: linear-gradient(to bottom, transparent 0, #000 0.6em, #000 70%, transparent 100%);
  mask-image: linear-gradient(to bottom, transparent 0, #000 0.6em, #000 70%, transparent 100%);
}
.line {
  display: block;
  flex-shrink: 0;
  width: 100%;
  margin: 0;
  padding: 0;
  border: none;
  background: none;
  font-family: inherit;
  font-size: var(--fs-md);
  font-weight: var(--fw-bold);
  line-height: 1.3;
  text-align: left;
  color: color-mix(in srgb, var(--avg-color, var(--surface-raised)) 55%, white 45%);
  cursor: pointer;
  transition: color var(--transition);
}
.line:hover:not(.active) {
  color: color-mix(in srgb, var(--avg-color, var(--surface-raised)) 25%, white 75%);
}
.line.past {
  color: color-mix(in srgb, var(--avg-color, var(--surface-raised)) 75%, white 25%);
}
.line.active {
  color: var(--text);
}
.plain {
  width: 100%;
  border: none;
  background: none;
  font: inherit;
  text-align: left;
  cursor: pointer;
}
.plain .line {
  font-weight: var(--fw-medium);
}
</style>
