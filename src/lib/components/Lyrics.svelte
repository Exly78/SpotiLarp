<script>
  import { untrack } from "svelte";
  import { fade } from "svelte/transition";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { activeLine, fetchLyrics, lyricsKey } from "../lyrics.js";
  import { notify } from "../toast.svelte.js";
  import { player, seekTo } from "../player.svelte.js";
  import { isEpisodeUri } from "../podcasts.svelte.js";
  import { canRomanize, romanization, romanizeLabel, romanizeLines, setRomanized } from "../romanize.svelte.js";

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

  const activeIndex = $derived(activeLine(synced, player.positionMs));

  const episode = $derived(isEpisodeUri(player.track?.uri));

  /** The lines as written, before any romanizing. */
  const sourceLines = $derived.by(() => {
    if (synced) return synced.map((line) => line.text);
    return plain ? plain.split("\n") : null;
  });
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
    if (key !== loadedFor) untrack(() => loadLyrics(track, key));
  });

  /**
   * @param {import("../player.svelte.js").CurrentTrack} track
   * @param {string} key
   * @param {boolean} [recheck] ask again even if no lyrics were found last time
   */
  async function loadLyrics(track, key, recheck = false) {
    loadedFor = key;
    loading = true;
    error = "";
    synced = null;
    plain = null;
    instrumental = false;
    try {
      const result = await fetchLyrics(track, recheck);
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

  /** @param {boolean} [recheck] */
  function retry(recheck = false) {
    const track = player.track;
    if (track) loadLyrics(track, lyricsKey(track), recheck);
  }

  /** @param {string} url */
  function open(url) {
    openUrl(url).catch((e) => notify(`Couldn't open the link: ${e}`));
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
  {#if romanizable && sourceLines}
    <div class="tools">
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
    </div>
  {/if}
  {#if !player.track}
    <div class="empty-state" transition:fade={{ duration: 150 }}>Nothing playing.</div>
  {:else if episode}
    <div class="empty-state" transition:fade={{ duration: 150 }}>Podcast episodes don't have lyrics.</div>
  {:else if loading}
    <ul class="skeleton-list" transition:fade={{ duration: 150 }}>
      {#each { length: 8 } as _}
        <li class="skeleton line"></li>
      {/each}
    </ul>
  {:else if error}
    <div class="empty-state" transition:fade={{ duration: 150 }}>
      <p class="status error">{error}</p>
      <button type="button" class="retry" onclick={() => retry()}>Try again</button>
    </div>
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
          {(romanized?.[i] ?? line.text) || " "}
        </button>
      {/each}
    </div>
  {:else if plain}
    <div class="plain-lyrics" transition:fade={{ duration: 150 }}>
      {#each plain.split("\n") as line, i}
        <p class="line">{(romanized?.[i] ?? line) || " "}</p>
      {/each}
    </div>
  {:else}
    <div class="empty-state" transition:fade={{ duration: 150 }}>
      <p class="empty-title">No lyrics found for this track.</p>
      <p class="contribute">
        Know the words? Lyrics that Spotify doesn't have come from
        <button type="button" class="inline-link" onclick={() => open("https://lrclib.net")}>LRCLIB</button>, a free
        database anyone can add to. The easiest way is
        <button type="button" class="inline-link" onclick={() => open("https://github.com/tranxuanthang/lrcget")}>LRCGET</button>,
        a free app that can publish lyrics (synced to the song, too). Once they're up, they'll show here for everyone.
      </p>
      <button type="button" class="retry" onclick={() => retry(true)}>Check again</button>
    </div>
  {/if}
</div>

<style>
.lyrics {
  position: absolute;
  inset: 0;
  /* The active line's scale-up would otherwise poke out sideways and bring up a scrollbar. */
  overflow-x: hidden;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  padding: 1.5em 2em;
  border-radius: var(--radius-md);
  background-color: var(--avg-color, var(--surface-raised));
}
/* Pinned to the top corner as the lyrics scroll, without taking up a row. */
.tools {
  position: sticky;
  top: 0;
  z-index: 2;
  align-self: flex-end;
  height: 0;
}
.romanize-toggle {
  padding: 0.35em 0.9em;
  border: none;
  border-radius: var(--radius-pill);
  background: rgba(0, 0, 0, 0.3);
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
  transition: background-color var(--transition), color var(--transition);
}
.romanize-toggle:hover {
  background: rgba(0, 0, 0, 0.5);
}
.romanize-toggle.on {
  background: var(--text);
  color: #000;
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
.empty-title {
  margin: 0;
  font-size: var(--fs-md);
  font-weight: var(--fw-bold);
  color: var(--text);
}
.contribute {
  max-width: 420px;
  margin: 0.8em auto 0;
  line-height: 1.6;
}
.inline-link {
  padding: 0;
  border: none;
  background: none;
  color: var(--text);
  font: inherit;
  font-weight: var(--fw-bold);
  text-decoration: underline;
  cursor: pointer;
}
.retry {
  margin-top: 1em;
  padding: 0.5em 1.2em;
  border: 1px solid var(--text-muted);
  border-radius: var(--radius-pill);
  background: none;
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
  transition: border-color var(--transition);
}
.retry:hover {
  border-color: var(--text);
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
