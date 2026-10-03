<script>
  /** @type {{ image: string|null, title: string, subtitle?: string, round?: boolean, onclick: () => void }} */
  let { image, title, subtitle = "", round = false, onclick } = $props();
</script>

<button type="button" class="card" {onclick}>
  {#if image}
    <img src={image} alt="" class="art" class:round loading="lazy" />
  {:else}
    <div class="art placeholder" class:round></div>
  {/if}
  <div class="title">{title}</div>
  {#if subtitle}
    <div class="subtitle">{subtitle}</div>
  {/if}
</button>

<style>
.card {
  display: flex;
  flex-direction: column;
  gap: 0.35em;
  min-width: 0;
  padding: 0.75em;
  border: none;
  border-radius: var(--radius-md);
  background: none;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
  transition: background-color var(--transition);
}
.card:hover {
  background-color: var(--surface-hover);
}
.art {
  width: 100%;
  aspect-ratio: 1;
  margin-bottom: 0.35em;
  border-radius: var(--radius-md);
  object-fit: cover;
  background: var(--surface-raised);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
}
.art.round {
  border-radius: 50%;
}
.art.placeholder {
  background: linear-gradient(135deg, var(--surface-raised), var(--surface-hover));
}
.title {
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.subtitle {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
