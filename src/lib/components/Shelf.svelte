<script>
  import Section from "./Section.svelte";

  /**
   * A titled row of cards that shows one line until "Show all" (Spotify's shelves).
   * @type {{
   *   title: string,
   *   items: any[],
   *   card: import("svelte").Snippet<[any, number]>,
   *   controls?: import("svelte").Snippet,
   *   onShowAll?: () => void,
   * }}
   */
  let { title, items, card, controls, onShowAll } = $props();

  /** @type {HTMLDivElement|undefined} */
  let grid = $state();
  let columns = $state(6);
  let expanded = $state(false);

  const shown = $derived(expanded ? items : items.slice(0, columns));

  $effect(() => {
    const element = grid;
    if (!element) return;
    // auto-fill keeps empty tracks, so this is the column count even when there are few items.
    const measure = () => {
      columns = Math.max(1, getComputedStyle(element).gridTemplateColumns.split(" ").length);
    };
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    measure();
    return () => observer.disconnect();
  });
</script>

<Section {title}>
  {#snippet action()}
    {#if onShowAll}
      <button type="button" class="show-all" onclick={onShowAll}>Show all</button>
    {:else if items.length > columns}
      <button type="button" class="show-all" onclick={() => (expanded = !expanded)}>
        {expanded ? "Show less" : "Show all"}
      </button>
    {/if}
  {/snippet}
  {@render controls?.()}
  <div class="grid" bind:this={grid}>
    {#each shown as item, i}
      {@render card(item, i)}
    {/each}
  </div>
</Section>

<style>
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 0.25em;
  margin: 0 -0.75em;
}
.show-all {
  padding: 0;
  border: none;
  background: none;
  color: var(--text-muted);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.show-all:hover {
  color: var(--text);
  text-decoration: underline;
}
</style>
