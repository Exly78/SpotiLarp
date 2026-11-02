<script>
  import { coverAtLeast, formatCount, htmlRuns } from "../utils.js";
  import ArtistAboutDialog from "./ArtistAboutDialog.svelte";

  /** @type {{ page: import("../types.js").ArtistPage }} */
  let { page } = $props();

  let open = $state(false);

  const photos = $derived(page.gallery.length > 0 ? page.gallery : page.images.length > 0 ? [page.images] : []);
  const cardPhoto = $derived(photos.length > 0 ? coverAtLeast(photos[0], 640) : null);
  const bioText = $derived(htmlRuns(page.biography).map((run) => run.text).join(""));
  const hasContent = $derived(!!bioText || !!cardPhoto || page.monthly_listeners != null);
</script>

{#if hasContent}
  <section class="about">
    <h2>About</h2>
    <button
      type="button"
      class="card"
      class:with-photo={!!cardPhoto}
      style={cardPhoto ? `--photo: url("${cardPhoto}")` : ""}
      onclick={() => (open = true)}
      aria-label={`About ${page.name}`}
    >
      {#if page.world_rank}
        <span class="rank"><strong>#{formatCount(page.world_rank)}</strong>in the world</span>
      {/if}
      <span class="card-text">
        {#if page.monthly_listeners != null}
          <span class="listeners">{formatCount(page.monthly_listeners)} monthly listeners</span>
        {/if}
        {#if bioText}
          <span class="bio-preview">{bioText}</span>
        {/if}
      </span>
    </button>
  </section>
{/if}

{#if open}
  <ArtistAboutDialog {page} onClose={() => (open = false)} />
{/if}

<style>
.about {
  margin-bottom: 2em;
}
h2 {
  margin: 0 0 0.6em;
  font-size: var(--fs-lg);
  font-weight: var(--fw-bold);
  letter-spacing: -0.01em;
  color: var(--text);
}
.card {
  position: relative;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  width: 100%;
  max-width: 680px;
  min-height: 220px;
  padding: 1.5em;
  border: none;
  border-radius: var(--radius-md);
  background: var(--surface-raised);
  color: var(--text);
  font: inherit;
  text-align: left;
  cursor: pointer;
  overflow: hidden;
  transition: transform var(--transition);
}
.card.with-photo {
  aspect-ratio: 16 / 10;
  background:
    linear-gradient(to bottom, transparent 30%, rgba(0, 0, 0, 0.75) 100%),
    var(--photo) center 25% / cover no-repeat,
    var(--surface-raised);
}
.card:hover {
  transform: scale(1.01);
}
.rank {
  position: absolute;
  top: 1.25em;
  left: 1.25em;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  width: 92px;
  height: 92px;
  border-radius: 50%;
  background: #3d91f4;
  color: #fff;
  font-size: var(--fs-xs);
  line-height: 1.2;
  text-align: center;
}
.rank strong {
  font-size: 1.6em;
  font-weight: var(--fw-bold);
}
.card-text {
  display: flex;
  flex-direction: column;
  gap: 0.5em;
  max-width: 560px;
  text-shadow: 0 1px 8px rgba(0, 0, 0, 0.4);
}
.listeners {
  font-weight: var(--fw-bold);
}
.bio-preview {
  font-size: var(--fs-sm);
  line-height: 1.5;
  color: var(--text-dim);
  display: -webkit-box;
  -webkit-line-clamp: 3;
  line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.with-photo .bio-preview {
  color: rgba(255, 255, 255, 0.9);
}
</style>
