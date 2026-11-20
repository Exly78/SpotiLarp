<script>
  import { compactDuration } from "../utils.js";

  /**
   * Time listened in each day, week or month of a range: one bar each, the
   * busiest named under the chart, the rest on hover or keyboard focus.
   * @type {{ buckets: { start: number, end: number, ms: number, plays: number }[], bucket: "day"|"week"|"month" }}
   */
  let { buckets, bucket } = $props();

  /** At most this many labels along the bottom. */
  const AXIS_LABELS = 7;

  /** @type {number|null} */
  let hovered = $state(null);

  const formats = {
    day: new Intl.DateTimeFormat(undefined, { weekday: "short", day: "numeric", month: "short" }),
    week: new Intl.DateTimeFormat(undefined, { day: "numeric", month: "short" }),
    month: new Intl.DateTimeFormat(undefined, { month: "long", year: "numeric" }),
  };
  const axisFormats = {
    day: new Intl.DateTimeFormat(undefined, { day: "numeric", month: "short" }),
    week: new Intl.DateTimeFormat(undefined, { day: "numeric", month: "short" }),
    month: new Intl.DateTimeFormat(undefined, { month: "short" }),
  };
  const monthYear = new Intl.DateTimeFormat(undefined, { month: "short", year: "2-digit" });

  const peak = $derived(Math.max(1, ...buckets.map((b) => b.ms)));
  const busiest = $derived(buckets.reduce((best, b, i) => (b.ms > buckets[best].ms ? i : best), 0));
  // Spans of years get the year on their month labels.
  const manyYears = $derived(
    buckets.length > 0 && new Date(buckets[0].start).getFullYear() !== new Date(buckets[buckets.length - 1].start).getFullYear(),
  );
  const labelEvery = $derived(Math.max(1, Math.ceil(buckets.length / AXIS_LABELS)));

  /** @param {number} index */
  function describe(index) {
    const b = buckets[index];
    const when = bucket === "week" ? `Week of ${formats.week.format(b.start)}` : formats[bucket].format(b.start);
    return `${when}: ${compactDuration(b.ms)}, ${b.plays.toLocaleString()} ${b.plays === 1 ? "play" : "plays"}`;
  }

  /** @param {number} start */
  function axisLabel(start) {
    if (bucket === "month" && manyYears) return monthYear.format(start);
    return axisFormats[bucket].format(start);
  }
</script>

<figure class="chart">
  <div class="plot">
    <span class="peak">{compactDuration(peak)}</span>
    <ol class="bars" style:--count={buckets.length} aria-label="Time listened over time">
      {#each buckets as b, i (b.start)}
        <!-- Focusable so each bar's numbers can be read from the keyboard too. -->
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <li
          class="slot"
          class:dimmed={hovered !== null && hovered !== i}
          tabindex="0"
          aria-label={describe(i)}
          onmouseenter={() => (hovered = i)}
          onmouseleave={() => (hovered = null)}
          onfocus={() => (hovered = i)}
          onblur={() => (hovered = null)}
        >
          <span class="bar" style:height="{b.ms > 0 ? Math.max(2, (b.ms / peak) * 100) : 0}%"></span>
        </li>
      {/each}
    </ol>
  </div>
  <div class="axis" style:--count={buckets.length} aria-hidden="true">
    {#each buckets as b, i (b.start)}
      <span class="tick">{i % labelEvery === 0 ? axisLabel(b.start) : ""}</span>
    {/each}
  </div>
  <figcaption class="readout" aria-live="polite">
    {#if hovered !== null}
      {describe(hovered)}
    {:else if buckets[busiest]?.ms > 0}
      Busiest: {describe(busiest)}
    {:else}
      Nothing played yet
    {/if}
  </figcaption>
</figure>

<style>
.chart {
  margin: 0;
}
.plot {
  position: relative;
  height: 160px;
  padding-top: 1.2em;
  /* The baseline. */
  border-bottom: 1px solid color-mix(in srgb, var(--text) 18%, transparent);
}
.peak {
  position: absolute;
  top: 0;
  left: 0;
  font-size: var(--fs-xs);
  font-variant-numeric: tabular-nums;
  color: var(--text-muted);
}
/* The peak's gridline. */
.plot::before {
  content: "";
  position: absolute;
  top: 1.2em;
  left: 0;
  right: 0;
  border-top: 1px dashed color-mix(in srgb, var(--text) 10%, transparent);
}
.bars,
.axis {
  display: grid;
  grid-template-columns: repeat(var(--count), minmax(0, 1fr));
}
.bars {
  height: 100%;
  margin: 0;
  padding: 0;
  list-style: none;
}
.slot {
  display: flex;
  align-items: flex-end;
  justify-content: center;
  height: 100%;
  outline: none;
  cursor: default;
}
.bar {
  width: calc(100% - 2px);
  max-width: 28px;
  border-radius: 4px 4px 0 0;
  background: var(--accent);
  transition: opacity var(--transition), background-color var(--transition);
}
.slot:hover .bar,
.slot:focus-visible .bar {
  background: var(--accent-hover);
}
.dimmed .bar {
  opacity: 0.45;
}
.slot:focus-visible {
  box-shadow: inset 0 0 0 2px var(--text);
  border-radius: 2px;
}
.axis {
  margin-top: 0.4em;
}
.tick {
  overflow: visible;
  white-space: nowrap;
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
.readout {
  margin-top: 0.6em;
  font-size: var(--fs-sm);
  font-variant-numeric: tabular-nums;
  color: var(--text-dim);
}
</style>
