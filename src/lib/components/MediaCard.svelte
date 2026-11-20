<script>
  import PlayButton from "./PlayButton.svelte";

  /**
   * `onplay` adds the play button that comes up over the picture on hover;
   * `overlay` is drawn over the picture, under the button.
   * @type {{ image: string|null, title: string, subtitle?: string, round?: boolean, onclick: () => void, onplay?: () => void, overlay?: import("svelte").Snippet }}
   */
  let { image, title, subtitle = "", round = false, onclick, onplay, overlay } = $props();
</script>

<div class="card">
  <!-- Covers the whole card, so the play button can sit on top without being inside it. -->
  <button type="button" class="open" {onclick} aria-label={subtitle ? `${title}, ${subtitle}` : title}></button>
  <div class="art-wrap">
    {#if image}
      <img src={image} alt="" class="art" class:round loading="lazy" />
    {:else}
      <div class="art placeholder" class:round></div>
    {/if}
    {#if overlay}
      <div class="overlay" class:round>{@render overlay()}</div>
    {/if}
    {#if onplay}
      <div class="play">
        <PlayButton size={48} label={`Play ${title}`} onclick={onplay} />
      </div>
    {/if}
  </div>
  <div class="title">{title}</div>
  {#if subtitle}
    <div class="subtitle">{subtitle}</div>
  {/if}
</div>

<style>
.card {
  position: relative;
  display: flex;
  flex-direction: column;
  min-width: 0;
  padding: 12px;
  border-radius: 6px;
  text-align: left;
  transition: background-color var(--transition);
}
.card:hover,
.card:focus-within {
  background-color: var(--surface-hover);
}
.open {
  position: absolute;
  inset: 0;
  z-index: 1;
  padding: 0;
  border: none;
  border-radius: inherit;
  background: none;
  cursor: pointer;
}
.art-wrap {
  position: relative;
  margin-bottom: 8px;
}
.art {
  display: block;
  width: 100%;
  aspect-ratio: 1;
  border-radius: 6px;
  object-fit: cover;
  background: var(--surface-raised);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
}
.art.round {
  border-radius: 50%;
}
.art.placeholder {
  background: linear-gradient(135deg, var(--surface-raised), var(--surface-hover));
}
.overlay {
  position: absolute;
  inset: 0;
  overflow: hidden;
  border-radius: 6px;
  pointer-events: none;
}
.overlay.round {
  border-radius: 50%;
}
.play {
  position: absolute;
  right: 8px;
  bottom: 8px;
  z-index: 2;
  border-radius: 50%;
  box-shadow: 0 8px 8px rgba(0, 0, 0, 0.3);
  opacity: 0;
  transform: translateY(8px);
  transition: opacity 200ms ease, transform 200ms ease;
}
.card:hover .play,
.card:focus-within .play {
  opacity: 1;
  transform: none;
}
.title {
  font-size: 1em;
  font-weight: var(--fw-medium);
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.subtitle {
  margin-top: 2px;
  font-size: var(--fs-sm);
  line-height: 1.35;
  color: var(--text-muted);
  overflow-wrap: anywhere;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
@media (prefers-reduced-motion: reduce) {
  .play {
    transform: none;
    transition: opacity 200ms ease;
  }
}
</style>
