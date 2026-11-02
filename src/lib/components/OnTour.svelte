<script>
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { notify } from "../toast.svelte.js";

  /** @type {{ concerts: import("../types.js").Concert[], total: number, artistId: string }} */
  let { concerts, total, artistId } = $props();

  const SHOWN = 3;

  const monthFormatter = new Intl.DateTimeFormat(undefined, { month: "short" });
  const timeFormatter = new Intl.DateTimeFormat(undefined, { weekday: "short", hour: "numeric", minute: "2-digit" });
  const dayFormatter = new Intl.DateTimeFormat(undefined, { weekday: "short" });

  /**
   * Spotify gives the venue's local time ("2026-11-04T20:30+08:00"); it's shown
   * as the venue has it rather than converted to yours, like the ticket says.
   * @param {string} iso
   */
  function when(iso) {
    const match = /^(\d{4})-(\d{2})-(\d{2})(?:T(\d{2}):(\d{2}))?/.exec(iso);
    if (!match) return null;
    const [, year, month, day, hours, minutes] = match;
    const hasTime = hours !== undefined;
    const at = new Date(Number(year), Number(month) - 1, Number(day), hasTime ? Number(hours) : 0, hasTime ? Number(minutes) : 0);
    if (Number.isNaN(at.getTime())) return null;
    return {
      month: monthFormatter.format(at),
      day: String(Number(day)),
      detail: hasTime ? timeFormatter.format(at) : dayFormatter.format(at),
    };
  }

  /** @param {string} url */
  function open(url) {
    openUrl(url).catch((e) => notify(`Couldn't open the link: ${e}`));
  }
</script>

<section class="on-tour">
  <div class="head">
    <h3>On tour</h3>
    {#if total > SHOWN}
      <button type="button" class="text-button" onclick={() => open(`https://open.spotify.com/artist/${artistId}/concerts`)}>
        Show all
      </button>
    {/if}
  </div>
  {#each concerts.slice(0, SHOWN) as concert}
    {@const at = when(concert.date)}
    <button
      type="button"
      class="row"
      onclick={() => open(`https://open.spotify.com/concert/${concert.id}`)}
      title="Open on Spotify"
    >
      <span class="date">
        {#if at}
          <span class="month">{at.month}</span>
          <span class="day">{at.day}</span>
        {/if}
      </span>
      <span class="meta">
        <span class="place">{concert.festival && concert.title ? concert.title : concert.city || concert.title}</span>
        <span class="venue">{[concert.venue, at?.detail].filter(Boolean).join(" · ")}</span>
      </span>
    </button>
  {/each}
</section>

<style>
.on-tour {
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
.row {
  display: flex;
  align-items: center;
  gap: 0.8em;
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
.date {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  flex-shrink: 0;
  border-radius: var(--radius-sm);
  background: var(--surface-hover);
  line-height: 1.1;
}
.month {
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  color: var(--accent);
  text-transform: uppercase;
}
.day {
  font-size: var(--fs-md);
  font-weight: var(--fw-bold);
  color: var(--text);
}
.meta {
  display: flex;
  flex-direction: column;
  gap: 0.15em;
  min-width: 0;
}
.place {
  font-size: var(--fs-sm);
  font-weight: var(--fw-medium);
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.venue {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
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
</style>
