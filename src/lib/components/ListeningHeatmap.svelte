<script>
  import { compactDuration } from "../utils.js";

  /**
   * When you listen: time heard by weekday and hour, darker to brighter in
   * the accent color. The busiest hour is named under it; any other shows on
   * hover or keyboard focus.
   * @type {{ grid: number[][] }} weekday (Monday first) × hour, in ms
   */
  let { grid } = $props();

  /** Shades, from a little to the most (none is the empty cell). */
  const STEPS = [0.18, 0.36, 0.56, 0.78, 1];

  const weekdays = Array.from({ length: 7 }, (_, i) =>
    new Intl.DateTimeFormat(undefined, { weekday: "short" }).format(new Date(2024, 0, 1 + i)),
  );
  const longWeekdays = Array.from({ length: 7 }, (_, i) =>
    new Intl.DateTimeFormat(undefined, { weekday: "long" }).format(new Date(2024, 0, 1 + i)),
  );
  // "22:00" or "10:00 PM", whichever this computer uses.
  const hourFormat = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" });

  /** @type {{ day: number, hour: number }|null} */
  let hovered = $state(null);

  const peak = $derived(Math.max(1, ...grid.flat()));
  const busiest = $derived.by(() => {
    let best = { day: 0, hour: 0, ms: 0 };
    grid.forEach((hours, day) => hours.forEach((ms, hour) => ms > best.ms && (best = { day, hour, ms })));
    return best;
  });

  /** @param {number} ms */
  function step(ms) {
    if (ms <= 0) return 0;
    return STEPS[Math.min(STEPS.length - 1, Math.floor((ms / peak) * STEPS.length))];
  }

  /** @param {number} hour */
  function hourLabel(hour) {
    return hourFormat.format(new Date(2024, 0, 1, hour));
  }

  /** @param {number} day @param {number} hour */
  function describe(day, hour) {
    return `${longWeekdays[day]}s, ${hourLabel(hour)}: ${compactDuration(grid[day][hour])}`;
  }
</script>

<figure class="heatmap">
  <div class="grid" role="group" aria-label="Time listened by weekday and hour">
    <span></span>
    {#each { length: 24 } as _, hour}
      <span class="hour" aria-hidden="true">{hour % 6 === 0 ? hourLabel(hour) : ""}</span>
    {/each}
    {#each grid as hours, day}
      <span class="weekday" aria-hidden="true">{weekdays[day]}</span>
      {#each hours as ms, hour}
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <span
          class="cell"
          role="img"
          tabindex="0"
          aria-label={describe(day, hour)}
          style:--shade={step(ms)}
          onmouseenter={() => (hovered = { day, hour })}
          onmouseleave={() => (hovered = null)}
          onfocus={() => (hovered = { day, hour })}
          onblur={() => (hovered = null)}
        ></span>
      {/each}
    {/each}
  </div>
  <figcaption class="footer">
    <span class="readout" aria-live="polite">
      {#if hovered}
        {describe(hovered.day, hovered.hour)}
      {:else if busiest.ms > 0}
        Most often {longWeekdays[busiest.day]}s around {hourLabel(busiest.hour)}
      {/if}
    </span>
    <span class="legend" aria-hidden="true">
      Less
      <span class="cell swatch" style:--shade={0}></span>
      {#each STEPS as shade}
        <span class="cell swatch" style:--shade={shade}></span>
      {/each}
      More
    </span>
  </figcaption>
</figure>

<style>
.heatmap {
  margin: 0;
}
.grid {
  display: grid;
  grid-template-columns: auto repeat(24, minmax(0, 1fr));
  gap: 2px;
  align-items: center;
}
.hour,
.weekday {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  white-space: nowrap;
}
.hour {
  overflow: visible;
}
.weekday {
  padding-right: 0.6em;
}
.cell {
  aspect-ratio: 1;
  min-height: 10px;
  border-radius: 3px;
  /* None is the faint track; more is brighter accent. */
  background: color-mix(in srgb, var(--accent) calc(var(--shade) * 100%), color-mix(in srgb, var(--text) 6%, transparent));
  outline: none;
}
.cell:hover,
.cell:focus-visible {
  box-shadow: 0 0 0 2px var(--text);
}
.footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1em;
  flex-wrap: wrap;
  margin-top: 0.7em;
}
.readout {
  font-size: var(--fs-sm);
  font-variant-numeric: tabular-nums;
  color: var(--text-dim);
}
.legend {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
.swatch {
  width: 12px;
  min-height: 0;
}
.legend .swatch:first-of-type {
  margin-left: 4px;
}
.legend .swatch:last-of-type {
  margin-right: 4px;
}
</style>
