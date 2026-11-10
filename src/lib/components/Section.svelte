<script>
  /**
   * `onTitleClick` makes the heading a link (a shelf's "Show all"); `before`
   * sits between the heading and what's under it.
   * @type {{
   *   title: string,
   *   grid?: boolean,
   *   gridRef?: HTMLDivElement,
   *   onTitleClick?: () => void,
   *   action?: import("svelte").Snippet,
   *   before?: import("svelte").Snippet,
   *   children: import("svelte").Snippet,
   * }}
   */
  let { title, grid = false, gridRef = $bindable(), onTitleClick, action, before, children } = $props();
</script>

<section>
  <div class="heading">
    {#if onTitleClick}
      <h2><button type="button" class="title-link" onclick={onTitleClick}>{title}</button></h2>
    {:else}
      <h2>{title}</h2>
    {/if}
    {@render action?.()}
  </div>
  {@render before?.()}
  <div class:grid bind:this={gridRef}>
    {@render children()}
  </div>
</section>

<style>
section {
  margin-bottom: 2.5em;
}
.heading {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 1em;
  margin-bottom: 0.5em;
}
h2 {
  min-width: 0;
  margin: 0;
  font-size: 1.5em;
  font-weight: var(--fw-bold);
  letter-spacing: -0.02em;
  color: var(--text);
}
.title-link {
  padding: 0;
  border: none;
  background: none;
  color: inherit;
  font: inherit;
  letter-spacing: inherit;
  text-align: left;
  cursor: pointer;
}
.title-link:hover {
  text-decoration: underline;
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  /* Cards carry their own padding, so their pictures line up with the heading. */
  margin: 0 -12px;
}
</style>
