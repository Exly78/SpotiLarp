<script>
  import { allPlays } from "../tracker.svelte.js";
  import { navigate, openArtist } from "../nav.svelte.js";
  import { compactDuration } from "../utils.js";

  /**
   * The last seven days of listening, from the tracker: time listened, how it
   * compares with the week before, a bar per day and the most played artist.
   * `artistImages` are pictures for artists, by id.
   * @type {{ artistImages?: Map<string, string|null> }}
   */
  let { artistImages = new Map() } = $props();

  const DAYS = 7;
  const weekdayFormatter = new Intl.DateTimeFormat(undefined, { weekday: "narrow" });
  const dayFormatter = new Intl.DateTimeFormat(undefined, { weekday: "short", day: "numeric", month: "short" });

  /** @type {number|null} */
  let hovered = $state(null);

  /** Midnight `back` days ago (by the calendar, so a clock change doesn't shift it). @param {number} back */
  function dayStart(back) {
    const day = new Date();
    day.setHours(0, 0, 0, 0);
    day.setDate(day.getDate() - back);
    return day.getTime();
  }

  // Time is what was really heard. Artists go by name, since imported history has no ids.
  const stats = $derived.by(() => {
    const starts = Array.from({ length: DAYS }, (_, i) => dayStart(DAYS - 1 - i));
    const days = starts.map((start) => ({ start, ms: 0, plays: 0 }));
    const weekStart = starts[0];
    const previousStart = dayStart(DAYS * 2 - 1);
    let previousMs = 0;
    /** @type {Map<string, { id: string, name: string, plays: number }>} */
    const artists = new Map();
    const plays = allPlays();
    // Oldest first, so the last fortnight is at the end.
    for (let i = plays.length - 1; i >= 0 && plays[i].t >= previousStart; i--) {
      const { t: at, ms, track } = plays[i];
      if (at < weekStart) {
        previousMs += ms;
        continue;
      }
      let index = DAYS - 1;
      while (index > 0 && at < starts[index]) index--;
      days[index].ms += ms;
      days[index].plays++;
      const artist = track.artists[0];
      if (artist?.name) {
        const key = artist.name.toLowerCase();
        const seen = artists.get(key) ?? { id: artist.id, name: artist.name, plays: 0 };
        if (!seen.id && artist.id) seen.id = artist.id;
        seen.plays++;
        artists.set(key, seen);
      }
    }
    const totalMs = days.reduce((sum, day) => sum + day.ms, 0);
    const playCount = days.reduce((sum, day) => sum + day.plays, 0);
    const top = [...artists.values()].sort((a, b) => b.plays - a.plays)[0] ?? null;
    const peak = Math.max(...days.map((day) => day.ms), 1);
    return { days, totalMs, previousMs, plays: playCount, artistCount: artists.size, top, peak };
  });

  const delta = $derived.by(() => {
    const { totalMs, previousMs } = stats;
    if (previousMs === 0 || Math.abs(totalMs - previousMs) < 60_000) return "";
    const more = totalMs > previousMs;
    return `${compactDuration(Math.abs(totalMs - previousMs))} ${more ? "more" : "less"} than the week before`;
  });

  const readout = $derived.by(() => {
    const index = hovered ?? DAYS - 1;
    const day = stats.days[index];
    const name = index === DAYS - 1 ? "Today" : dayFormatter.format(day.start);
    const plays = `${day.plays} ${day.plays === 1 ? "play" : "plays"}`;
    return `${name} · ${compactDuration(day.ms)} · ${plays}`;
  });

</script>

<article class="week">
  <div class="heading">
    <span class="label">Last 7 days</span>
    <button type="button" class="more" onclick={() => navigate({ type: "stats" })}>Your stats</button>
  </div>

  {#if stats.plays === 0}
    <p class="empty">Nothing played this week yet. Songs count once you've heard 30 seconds.</p>
  {:else}
    <div class="figure">{compactDuration(stats.totalMs)}</div>
    <div class="sub">
      {stats.plays}
      {stats.plays === 1 ? "play" : "plays"} · {stats.artistCount}
      {stats.artistCount === 1 ? "artist" : "artists"}{#if delta}<br />{delta}{/if}
    </div>

    <ol class="bars" aria-label="Time listened each day">
      {#each stats.days as day, i (day.start)}
        <!-- Focusable so the day's numbers can be read from the keyboard too. -->
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <li
          class="day"
          class:today={i === DAYS - 1}
          class:dimmed={hovered !== null && hovered !== i}
          tabindex="0"
          aria-label={`${i === DAYS - 1 ? "Today" : dayFormatter.format(day.start)}: ${compactDuration(day.ms)}, ${day.plays} plays`}
          onmouseenter={() => (hovered = i)}
          onmouseleave={() => (hovered = null)}
          onfocus={() => (hovered = i)}
          onblur={() => (hovered = null)}
        >
          <span class="track">
            <span class="bar" style:height="{day.ms > 0 ? Math.max(6, (day.ms / stats.peak) * 100) : 0}%"></span>
          </span>
          <span class="weekday">{weekdayFormatter.format(day.start)}</span>
        </li>
      {/each}
    </ol>
    <div class="readout" aria-live="polite">{readout}</div>

    {#if stats.top}
      {@const top = stats.top}
      {@const avatar = artistImages.get(top.id) ?? null}
      <button type="button" class="top" onclick={() => openArtist(top)} disabled={!top.id}>
        {#if avatar}
          <img src={avatar} alt="" class="avatar" />
        {:else}
          <span class="avatar initial">{top.name.slice(0, 1)}</span>
        {/if}
        <span class="top-text">
          <span class="top-label">Most played</span>
          <span class="top-name">{top.name}</span>
        </span>
        <span class="top-plays">{top.plays} {top.plays === 1 ? "play" : "plays"}</span>
      </button>
    {/if}
  {/if}
</article>

<style>
.week {
  display: flex;
  flex-direction: column;
  min-width: 0;
  padding: 1.25em;
  border-radius: var(--radius-lg);
  background: var(--surface-raised);
}
.heading {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 1em;
}
.label {
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-dim);
}
.more {
  padding: 0;
  border: none;
  background: none;
  color: var(--text-muted);
  font-family: inherit;
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.more:hover {
  color: var(--text);
  text-decoration: underline;
}
.empty {
  margin: 0.8em 0 0;
  font-size: var(--fs-sm);
  line-height: 1.5;
  color: var(--text-muted);
}
.figure {
  margin-top: 0.15em;
  font-size: 3.2em;
  font-weight: var(--fw-semibold);
  letter-spacing: -0.03em;
  line-height: 1.05;
  font-variant-numeric: tabular-nums;
  color: var(--text);
}
.sub {
  margin-top: 0.2em;
  font-size: var(--fs-sm);
  line-height: 1.45;
  color: var(--text-muted);
}
.bars {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  margin: 1em 0 0;
  padding: 0;
  list-style: none;
}
.day {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.35em;
  outline: none;
  cursor: default;
}
.track {
  position: relative;
  display: flex;
  align-items: flex-end;
  justify-content: center;
  width: 100%;
  height: 56px;
  /* The baseline, so a day with nothing played still has its place. */
  border-bottom: 2px solid color-mix(in srgb, var(--text) 12%, transparent);
}
.bar {
  width: min(18px, 70%);
  border-radius: 4px 4px 0 0;
  background: color-mix(in srgb, var(--text) 30%, transparent);
  transition: background-color var(--transition), opacity var(--transition);
}
.today .bar {
  background: var(--accent);
}
.day:hover .bar,
.day:focus-visible .bar {
  background: color-mix(in srgb, var(--text) 55%, transparent);
}
.today:hover .bar,
.today:focus-visible .bar {
  background: var(--accent-hover);
}
.dimmed .bar {
  opacity: 0.6;
}
.day:focus-visible .track {
  outline: 2px solid var(--text);
  outline-offset: 2px;
  border-radius: 2px;
}
.weekday {
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
.today .weekday {
  font-weight: var(--fw-bold);
  color: var(--text);
}
.readout {
  margin-top: 0.5em;
  font-size: var(--fs-xs);
  font-variant-numeric: tabular-nums;
  color: var(--text-dim);
  text-align: center;
}
.top {
  display: flex;
  align-items: center;
  gap: 0.7em;
  margin-top: auto;
  padding: 0.9em 0 0;
  border: none;
  border-top: 1px solid var(--border);
  background: none;
  color: var(--text);
  font: inherit;
  text-align: left;
  cursor: pointer;
}
.top:disabled {
  cursor: default;
}
.top:hover:not(:disabled) .top-name {
  text-decoration: underline;
}
.avatar {
  width: 36px;
  height: 36px;
  flex-shrink: 0;
  border-radius: 50%;
  object-fit: cover;
}
.avatar.initial {
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--surface-hover);
  font-weight: var(--fw-bold);
  color: var(--text-dim);
}
.top-text {
  display: flex;
  flex-direction: column;
  min-width: 0;
  flex: 1;
}
.top-label {
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
.top-name {
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.top-plays {
  flex-shrink: 0;
  font-size: var(--fs-xs);
  font-variant-numeric: tabular-nums;
  color: var(--text-muted);
}
</style>
