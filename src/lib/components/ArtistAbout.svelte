<script>
  import { fade, scale } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { openArtist, openAlbum } from "../nav.svelte.js";
  import { notify } from "../toast.svelte.js";
  import { coverAtLeast, formatCount, htmlRuns } from "../utils.js";
  import Icon from "./Icon.svelte";

  /** @type {{ page: import("../types.js").ArtistPage }} */
  let { page } = $props();

  let open = $state(false);
  let photo = $state(0);
  /** @type {HTMLButtonElement|undefined} */
  let closeButton = $state();

  const photos = $derived(page.gallery.length > 0 ? page.gallery : page.images.length > 0 ? [page.images] : []);
  const cardPhoto = $derived(photos.length > 0 ? coverAtLeast(photos[0], 640) : null);
  const bio = $derived(htmlRuns(page.biography));
  const bioText = $derived(bio.map((run) => run.text).join(""));
  const hasContent = $derived(!!bioText || !!cardPhoto || page.monthly_listeners != null);

  $effect(() => {
    if (open) closeButton?.focus();
  });

  function close() {
    open = false;
  }

  /**
   * Bio links point at other artists and albums, as `spotify:` URIs or open.spotify.com links.
   * @param {string|undefined} href
   * @param {string} name
   */
  function linkAction(href, name) {
    const match = href && /(?:spotify:|open\.spotify\.com\/)(artist|album)[:/]([A-Za-z0-9]+)/.exec(href);
    if (!match) return null;
    const [, kind, id] = match;
    return () => {
      close();
      if (kind === "artist") openArtist({ id, name });
      else openAlbum({ id, name });
    };
  }

  /** @param {string} name */
  function linkLabel(name) {
    return name ? name.charAt(0) + name.slice(1).toLowerCase() : "Website";
  }

  /** @param {number} step */
  function showPhoto(step) {
    photo = (photo + step + photos.length) % photos.length;
  }
</script>

<svelte:window onkeydown={(e) => open && e.key === "Escape" && close()} />

{#if hasContent}
  <section class="about">
    <h2>About</h2>
    <button
      type="button"
      class="card"
      class:with-photo={!!cardPhoto}
      style={cardPhoto ? `--photo: url("${cardPhoto}")` : ""}
      onclick={() => {
        photo = 0;
        open = true;
      }}
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
  <div
    class="overlay"
    role="presentation"
    transition:fade={{ duration: 150 }}
    onclick={(e) => e.target === e.currentTarget && close()}
  >
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-label={`About ${page.name}`}
      transition:scale={{ start: 0.96, duration: 180, easing: cubicOut }}
    >
      <button type="button" class="close" bind:this={closeButton} onclick={close} aria-label="Close">
        <Icon name="close" size={20} />
      </button>
      {#if photos.length > 0}
        <div class="gallery">
          <img src={coverAtLeast(photos[photo], 1000)} alt="" />
          {#if photos.length > 1}
            <button type="button" class="step prev" onclick={() => showPhoto(-1)} aria-label="Previous photo">
              <Icon name="chevron-left" size={22} />
            </button>
            <button type="button" class="step next" onclick={() => showPhoto(1)} aria-label="Next photo">
              <Icon name="chevron-right" size={22} />
            </button>
            <span class="counter">{photo + 1} / {photos.length}</span>
          {/if}
        </div>
      {/if}
      <div class="details">
        <div class="stats">
          {#if page.world_rank}
            <div class="stat"><strong>#{formatCount(page.world_rank)}</strong><span>in the world</span></div>
          {/if}
          {#if page.followers != null}
            <div class="stat"><strong>{formatCount(page.followers)}</strong><span>Followers</span></div>
          {/if}
          {#if page.monthly_listeners != null}
            <div class="stat"><strong>{formatCount(page.monthly_listeners)}</strong><span>Monthly Listeners</span></div>
          {/if}
          {#each page.top_cities as city}
            <div class="stat city">
              <strong>{city.city}{city.country ? `, ${city.country}` : ""}</strong>
              <span>{formatCount(city.listeners)} listeners</span>
            </div>
          {/each}
        </div>
        <div class="text">
          {#if bio.length > 0}
            <p class="bio">
              {#each bio as run}
                {@const action = linkAction(run.href, run.text)}
                {#if action}<button type="button" class="link" onclick={action}>{run.text}</button>{:else}{run.text}{/if}
              {/each}
            </p>
          {/if}
          {#if page.external_links.length > 0}
            <ul class="links">
              {#each page.external_links as link}
                <li>
                  <button
                    type="button"
                    class="link"
                    onclick={() => openUrl(link.url).catch((e) => notify(`Couldn't open the link: ${e}`))}
                  >
                    {linkLabel(link.name)}
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </div>
    </div>
  </div>
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

.overlay {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2em;
  background: rgba(0, 0, 0, 0.7);
}
.dialog {
  position: relative;
  width: min(820px, 100%);
  max-height: 100%;
  overflow-y: auto;
  border-radius: var(--radius-md);
  background: var(--surface-raised);
  box-shadow: 0 24px 64px rgba(0, 0, 0, 0.6);
}
.close {
  position: absolute;
  top: 0.75em;
  right: 0.75em;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border: none;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.55);
  color: #fff;
  cursor: pointer;
}
.close:hover {
  background: rgba(0, 0, 0, 0.8);
}
.gallery {
  position: relative;
  aspect-ratio: 16 / 9;
  background: #000;
}
.gallery img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  object-position: center 25%;
}
.step {
  position: absolute;
  top: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border: none;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.55);
  color: #fff;
  cursor: pointer;
  transform: translateY(-50%);
}
.step:hover {
  background: rgba(0, 0, 0, 0.8);
}
.prev {
  left: 0.75em;
}
.next {
  right: 0.75em;
}
.counter {
  position: absolute;
  right: 1em;
  bottom: 0.75em;
  padding: 0.2em 0.6em;
  border-radius: var(--radius-pill);
  background: rgba(0, 0, 0, 0.55);
  color: #fff;
  font-size: var(--fs-xs);
}
.details {
  display: grid;
  grid-template-columns: minmax(0, 200px) minmax(0, 1fr);
  gap: 2em;
  padding: 2em;
}
.stats {
  display: flex;
  flex-direction: column;
  gap: 1.25em;
}
.stat {
  display: flex;
  flex-direction: column;
  gap: 0.15em;
}
.stat strong {
  font-size: var(--fs-md);
  font-weight: var(--fw-bold);
  color: var(--text);
}
.stat.city strong {
  font-size: var(--fs-sm);
}
.stat span {
  font-size: var(--fs-sm);
  color: var(--text-muted);
}
.bio {
  margin: 0;
  font-size: var(--fs-sm);
  line-height: 1.65;
  color: var(--text-dim);
  white-space: pre-line;
}
.links {
  display: flex;
  flex-direction: column;
  gap: 0.6em;
  margin: 1.5em 0 0;
  padding: 0;
  list-style: none;
}
.link {
  padding: 0;
  border: none;
  background: none;
  color: var(--text);
  font: inherit;
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.link:hover {
  text-decoration: underline;
}
@media (max-width: 640px) {
  .details {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
