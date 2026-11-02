<script>
  import { openArtist } from "../nav.svelte.js";

  /** @type {{ credits: import("../types.js").Credit[], sources: string[] }} */
  let { credits, sources } = $props();

  const SHOWN = 4;

  let expanded = $state(false);

  const shown = $derived(expanded ? credits : credits.slice(0, SHOWN));
</script>

<section class="credits">
  <div class="head">
    <h3>Credits</h3>
    {#if credits.length > SHOWN}
      <button type="button" class="text-button" onclick={() => (expanded = !expanded)}>
        {expanded ? "Show less" : "Show all"}
      </button>
    {/if}
  </div>
  <ul>
    {#each shown as credit}
      {@const id = credit.artist_id}
      <li>
        {#if id}
          <button type="button" class="name link" onclick={() => openArtist({ id, name: credit.name })}>{credit.name}</button>
        {:else}
          <span class="name">{credit.name}</span>
        {/if}
        {#if credit.roles.length}
          <span class="roles">{credit.roles.join(", ")}</span>
        {/if}
      </li>
    {/each}
  </ul>
  {#if sources.length}
    <div class="sources">Source: {sources.join(", ")}</div>
  {/if}
</section>

<style>
.credits {
  flex-shrink: 0;
  padding: 0.9em 1em 1em;
  border-radius: var(--radius-lg);
  background: var(--surface-raised);
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5em;
  margin-bottom: 0.6em;
}
h3 {
  margin: 0;
  font-size: var(--fs-md);
  font-weight: var(--fw-bold);
  color: var(--text);
}
ul {
  display: flex;
  flex-direction: column;
  gap: 0.7em;
  margin: 0;
  padding: 0;
  list-style: none;
}
li {
  display: flex;
  flex-direction: column;
  gap: 0.1em;
  min-width: 0;
}
.name {
  font-size: var(--fs-sm);
  font-weight: var(--fw-semibold);
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.link {
  align-self: flex-start;
  max-width: 100%;
  padding: 0;
  border: none;
  background: none;
  font-family: inherit;
  text-align: left;
  cursor: pointer;
}
.link:hover {
  text-decoration: underline;
}
.roles {
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
.sources {
  margin-top: 0.9em;
  font-size: var(--fs-xs);
  color: var(--text-muted);
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
