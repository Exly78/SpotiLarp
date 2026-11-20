<script>
  import Icon from "./Icon.svelte";
  import { dominantColor } from "../colors.js";

  /**
   * The top of a page: its picture and title on the picture's color, which
   * runs to the panel's edges and fades out under the buttons. `sticky` (the
   * play button, usually) shows beside the title in the bar that stays at the
   * top once the header has scrolled away.
   * @type {{
   *   image?: string|null,
   *   round?: boolean,
   *   liked?: boolean,
   *   icon?: string|null,
   *   label: string,
   *   title: string,
   *   meta?: import("svelte").Snippet,
   *   actions?: import("svelte").Snippet,
   *   sticky?: import("svelte").Snippet,
   * }}
   */
  let { image = null, round = false, liked = false, icon = null, label, title, meta, actions, sticky } = $props();

  /** Roughly how wide a letter of the title is, in ems. */
  const LETTER_WIDTH = 0.58;
  const STICKY_BAR_HEIGHT = 64;

  let tint = $state("");
  let scrolledPast = $state(false);
  /** @type {HTMLElement|undefined} */
  let header = $state();

  // Sized so the title fits the space beside the picture on one line, within limits.
  const fit = $derived(`${(100 / (Math.max(title.length, 6) * LETTER_WIDTH)).toFixed(2)}cqi`);

  $effect(() => {
    const url = image;
    if (liked) {
      tint = "rgb(80, 56, 160)";
      return;
    }
    if (icon) {
      tint = "rgb(22, 110, 72)";
      return;
    }
    dominantColor(url).then((color) => {
      if (image === url) tint = color;
    });
  });

  $effect(() => {
    const element = header;
    if (!element) return;
    const observer = new IntersectionObserver(([entry]) => (scrolledPast = !entry.isIntersecting), {
      root: element.closest(".results-area"),
      rootMargin: `-${STICKY_BAR_HEIGHT}px 0px 0px 0px`,
    });
    observer.observe(element);
    return () => observer.disconnect();
  });
</script>

<div class="sticky-bar" class:shown={scrolledPast} style={tint ? `--tint: ${tint}` : ""} aria-hidden={!scrolledPast} inert={!scrolledPast}>
  {@render sticky?.()}
  <span class="sticky-title">{title}</span>
</div>
<header class="page-header" class:tinted={!!tint} style={tint ? `--tint: ${tint}` : ""} bind:this={header}>
  {#if image}
    <img src={image} alt="" class="art" class:round />
  {:else if liked}
    <div class="art liked"><Icon name="heart-filled" size={72} /></div>
  {:else if icon}
    <div class="art icon-art"><Icon name={icon} size={72} /></div>
  {:else}
    <div class="art placeholder" class:round></div>
  {/if}
  <div class="text">
    <span class="label">{label}</span>
    <h1 style={`--fit: ${fit}`}>{title}</h1>
    {#if meta}
      <div class="meta">{@render meta()}</div>
    {/if}
  </div>
</header>
<div class="fade" class:tinted={!!tint} style={tint ? `--tint: ${tint}` : ""}></div>
{#if actions}
  <div class="actions">{@render actions()}</div>
{/if}

<style>
.page-header {
  container: page-header / inline-size;
  position: relative;
  display: flex;
  align-items: flex-end;
  gap: 1.5em;
  /* Out to the panel's sides (.content-area's padding, +page.svelte); the
     sticky bar above has already brought it up to the top edge. */
  margin: 0 calc(-1 * var(--content-pad-x, 0px));
  padding: calc(var(--content-pad-y, 0px) + 2.5em) var(--content-pad-x, 0px) 1.5em;
}
/* The colors sit behind the page (.results-area is their stacking context) and fade in once known. */
.page-header::before,
.fade::before {
  content: "";
  position: absolute;
  inset: 0;
  z-index: -1;
  opacity: 0;
  transition: opacity 400ms ease;
  pointer-events: none;
}
.page-header::before {
  background: linear-gradient(transparent, rgba(0, 0, 0, 0.5)), var(--tint);
}
.fade {
  position: relative;
  height: 0;
  margin: 0 calc(-1 * var(--content-pad-x, 0px));
}
.fade::before {
  bottom: auto;
  height: 232px;
  background: linear-gradient(rgba(0, 0, 0, 0.6), var(--main-bg)), var(--tint);
}
.tinted::before {
  opacity: 1;
}
.art {
  width: 232px;
  height: 232px;
  flex-shrink: 0;
  border-radius: var(--radius-sm);
  object-fit: cover;
  background: var(--surface-raised);
  box-shadow: 0 4px 60px rgba(0, 0, 0, 0.5);
}
@container page-header (max-width: 760px) {
  .art {
    width: 176px;
    height: 176px;
  }
}
.art.round {
  border-radius: 50%;
}
.art.placeholder {
  background: linear-gradient(135deg, var(--surface-raised), var(--surface-hover));
}
.art.liked {
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #4b1fa8, #8f8ff0);
  color: #fff;
}
.art.icon-art {
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #0b4d33, #1ed760);
  color: #fff;
}
.text {
  container-type: inline-size;
  flex: 1;
  min-width: 0;
}
.label {
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  color: var(--text);
}
h1 {
  margin: 0.1em 0 0.25em;
  font-size: clamp(2em, var(--fit), 6em);
  font-weight: var(--fw-black);
  letter-spacing: -0.04em;
  line-height: 1.1;
  color: var(--text);
  overflow-wrap: anywhere;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.meta {
  font-size: var(--fs-sm);
  color: var(--text-dim);
}
.actions {
  display: flex;
  align-items: center;
  gap: 1.5em;
  padding: 1.5em 0;
}
.sticky-bar {
  position: sticky;
  /* Sticking is measured from inside the scroller's padding: this reaches the panel's top edge. */
  top: calc(-1 * var(--content-pad-y, 0px));
  z-index: 6;
  display: flex;
  align-items: center;
  gap: 1em;
  height: 64px;
  /* Takes no room in the page, and reaches the panel's edges. The header below
     starts where this does: a pull-up of its own would collapse into this -64px
     margin and be lost, leaving a strip of panel above the header's color. */
  margin: calc(-1 * var(--content-pad-y, 0px)) calc(-1 * var(--content-pad-x, 0px)) -64px;
  padding: 0 var(--content-pad-x, 0px);
  background: linear-gradient(rgba(0, 0, 0, 0.45), rgba(0, 0, 0, 0.45)), var(--tint, var(--surface-raised));
  box-shadow: 0 6px 10px rgba(0, 0, 0, 0.35);
  opacity: 0;
  pointer-events: none;
  transition: opacity 200ms ease;
}
.sticky-bar.shown {
  opacity: 1;
  pointer-events: auto;
}
.sticky-title {
  min-width: 0;
  font-size: 1.5em;
  font-weight: var(--fw-bold);
  letter-spacing: -0.02em;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
