<script>
  /** @type {{ title: string, grid?: boolean, action?: import("svelte").Snippet, children: import("svelte").Snippet }} */
  let { title, grid = false, action, children } = $props();
</script>

<section>
  <div class="heading">
    <h2>{title}</h2>
    {@render action?.()}
  </div>
  <div class:grid>
    {@render children()}
  </div>
</section>

<style>
section {
  margin-bottom: 2em;
}
.heading {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 1em;
  margin-bottom: 0.6em;
}
h2 {
  margin: 0;
  font-size: var(--fs-lg);
  font-weight: var(--fw-bold);
  letter-spacing: -0.01em;
  color: var(--text);
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 0.25em;
  margin: 0 -0.75em;
}
</style>
