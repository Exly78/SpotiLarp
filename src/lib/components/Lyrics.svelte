<script>
  import { onMount, onDestroy } from "svelte";
  import { fade } from "svelte/transition";
  import { listen } from "@tauri-apps/api/event";
  import * as api from "../api.js";

  /** @type {string|null} */
  let trackName = $state(null);
  let artists = $state("");
  let album = $state("");
  /** @type {string|null} */
  let coverUrl = $state(null);
  let durationMs = $state(0);
  let positionMs = $state(0);

  let loading = $state(false);
  let error = $state("");
  /** @type {import("../types.js").LyricLine[]|null} */
  let synced = $state(null);
  /** @type {string|null} */
  let plain = $state(null);
  let instrumental = $state(false);
  let loadedFor = $state("");

  let avgColor = $state("");
  let avgColorFor = "";

  /** @type {HTMLElement[]} */
  let lineEls = [];
  /** @type {import("@tauri-apps/api/event").UnlistenFn|undefined} */
  let unlisten;

  const activeIndex = $derived.by(() => {
    if (!synced) return -1;
    let idx = -1;
    for (let i = 0; i < synced.length; i++) {
      if (synced[i].time_ms <= positionMs) idx = i;
      else break;
    }
    return idx;
  });

  onMount(async () => {
    unlisten = await listen("player-event", (event) => {
      const e = event.payload;
      switch (e.type) {
        case "TrackChanged":
          trackName = e.name;
          artists = e.artists;
          album = e.album;
          coverUrl = e.cover_url;
          durationMs = e.duration_ms;
          positionMs = 0;
          break;
        case "Playing":
        case "Paused":
        case "PositionChanged":
        case "Seeked":
          positionMs = e.position_ms;
          break;
      }
    });
  });

  onDestroy(() => {
    unlisten?.();
  });

  $effect(() => {
    const key = trackName ? `${trackName}::${artists}::${album}` : null;
    if (key && key !== loadedFor) {
      loadLyrics();
    }
  });

  async function loadLyrics() {
    loadedFor = `${trackName}::${artists}::${album}`;
    loading = true;
    error = "";
    synced = null;
    plain = null;
    instrumental = false;
    try {
      const firstArtist = artists.split(",")[0]?.trim() ?? artists;
      const result = await api.getLyrics(/** @type {string} */ (trackName), firstArtist, album, durationMs);
      if (result) {
        instrumental = result.instrumental;
        synced = result.synced && result.synced.length > 0 ? result.synced : null;
        plain = result.plain;
      }
    } catch (e) {
      error = `Failed to load lyrics: ${e}`;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (activeIndex >= 0 && lineEls[activeIndex]) {
      lineEls[activeIndex].scrollIntoView({ behavior: "smooth", block: "center" });
    }
  });

  $effect(() => {
    if (coverUrl && coverUrl !== avgColorFor) {
      avgColorFor = coverUrl;
      extractAverageColor(coverUrl);
    } else if (!coverUrl) {
      avgColorFor = "";
      avgColor = "";
    }
  });

  /** @param {string} url */
  function extractAverageColor(url) {
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.onload = () => {
      try {
        const size = 32;
        const canvas = document.createElement("canvas");
        canvas.width = size;
        canvas.height = size;
        const ctx = canvas.getContext("2d");
        if (!ctx) return;
        ctx.drawImage(img, 0, 0, size, size);
        const { data } = ctx.getImageData(0, 0, size, size);

        const bucketCount = 24;
        const buckets = Array.from({ length: bucketCount }, () => ({ count: 0, hSum: 0, sSum: 0, lSum: 0 }));
        let sampled = 0;
        for (let i = 0; i < data.length; i += 4) {
          const [h, s, l] = rgbToHsl(data[i], data[i + 1], data[i + 2]);
          sampled++;
          if (l < 8 || l > 92 || s < 10) continue;
          const bucket = buckets[Math.floor(h / (360 / bucketCount)) % bucketCount];
          bucket.count++;
          bucket.hSum += h;
          bucket.sSum += s;
          bucket.lSum += l;
        }

        let best = null;
        let bestScore = 0;
        for (const bucket of buckets) {
          if (bucket.count === 0) continue;
          const avgS = bucket.sSum / bucket.count;
          const score = bucket.count * avgS;
          if (score > bestScore) {
            bestScore = score;
            best = { h: bucket.hSum / bucket.count, s: avgS, l: bucket.lSum / bucket.count, count: bucket.count };
          }
        }

        if (coverUrl === url) {
          avgColor = best && best.count / sampled >= 0.03 ? vividize(best.h, best.s, best.l) : "";
        }
      } catch {
        
        if (coverUrl === url) avgColor = "";
      }
    };
    img.onerror = () => {
      if (coverUrl === url) avgColor = "";
    };
    img.src = url;
  }

  /**
   * @param {number} h
   * @param {number} s
   * @param {number} l
   */
  function vividize(h, s, l) {
    if (s >= 35) {
      const boundedS = Math.min(62, s);
      const boundedL = Math.min(46, Math.max(l, 26));
      const [r, g, b] = hslToRgb(h, boundedS, boundedL);
      return `rgb(${r}, ${g}, ${b})`;
    }
    const boundedL = Math.min(30, Math.max(l * 0.55, 12));
    const [r, g, b] = hslToRgb(h, s, boundedL);
    return `rgb(${r}, ${g}, ${b})`;
  }

  /**
   * @param {number} r
   * @param {number} g
   * @param {number} b
   */
  function rgbToHsl(r, g, b) {
    r /= 255;
    g /= 255;
    b /= 255;
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    let h = 0;
    let s = 0;
    const l = (max + min) / 2;
    if (max !== min) {
      const d = max - min;
      s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
      switch (max) {
        case r:
          h = (g - b) / d + (g < b ? 6 : 0);
          break;
        case g:
          h = (b - r) / d + 2;
          break;
        default:
          h = (r - g) / d + 4;
      }
      h /= 6;
    }
    return [h * 360, s * 100, l * 100];
  }

  /**
   * @param {number} h
   * @param {number} s
   * @param {number} l
   */
  function hslToRgb(h, s, l) {
    h /= 360;
    s /= 100;
    l /= 100;
    if (s === 0) {
      const v = Math.round(l * 255);
      return [v, v, v];
    }
    /** @param {number} p @param {number} q @param {number} t */
    const hue2rgb = (p, q, t) => {
      if (t < 0) t += 1;
      if (t > 1) t -= 1;
      if (t < 1 / 6) return p + (q - p) * 6 * t;
      if (t < 1 / 2) return q;
      if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
      return p;
    };
    const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
    const p = 2 * l - q;
    return [
      Math.round(hue2rgb(p, q, h + 1 / 3) * 255),
      Math.round(hue2rgb(p, q, h) * 255),
      Math.round(hue2rgb(p, q, h - 1 / 3) * 255),
    ];
  }
</script>

<div class="lyrics" style={avgColor ? `--avg-color: ${avgColor}` : ""}>
  {#if !trackName}
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
        <p
          bind:this={lineEls[i]}
          class="line"
          class:active={i === activeIndex}
          class:past={i < activeIndex}
        >
          {line.text || " "}
        </p>
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
