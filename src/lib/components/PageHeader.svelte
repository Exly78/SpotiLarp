<script>
  import Icon from "./Icon.svelte";
  import { dominantColor } from "../colors.js";

  /**
   * @type {{
   *   image?: string|null,
   *   round?: boolean,
   *   liked?: boolean,
   *   label: string,
   *   title: string,
   *   meta?: import("svelte").Snippet,
   *   actions?: import("svelte").Snippet,
   * }}
   */
  let { image = null, round = false, liked = false, label, title, meta, actions } = $props();

  let tint = $state("");

  $effect(() => {
    const url = image;
    if (liked) {
      tint = "rgb(80, 56, 160)";
      return;
    }
    dominantColor(url).then((color) => {
      if (image === url) tint = color;
    });
  });
</script>

<header class="page-header" class:tinted={!!tint} style={tint ? `--tint: ${tint}` : ""}>
  {#if image}
    <img src={image} alt="" class="art" class:round />
  {:else if liked}
    <div class="art liked"><Icon name="heart-filled" size={64} /></div>
  {:else}
    <div class="art placeholder" class:round></div>
  {/if}
  <div class="text">
    <span class="label">{label}</span>
    <h1 class:long={title.length > 28}>{title}</h1>
    {#if meta}
      <div class="meta">{@render meta()}</div>
    {/if}
  </div>
</header>
{#if actions}
  <div class="actions">{@render actions()}</div>
{/if}

<style>
.page-header {
  display: flex;
  align-items: flex-end;
  gap: 1.5em;
  margin-bottom: 1.5em;
  padding: 1.25em;
  border-radius: var(--radius-md);
  transition: background 400ms ease;
}
.page-header.tinted {
  background: linear-gradient(to bottom, color-mix(in srgb, var(--tint) 85%, transparent), color-mix(in srgb, var(--tint) 15%, transparent));
}
.art {
  width: 192px;
  height: 192px;
  flex-shrink: 0;
  border-radius: var(--radius-md);
  object-fit: cover;
  background: var(--surface-raised);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
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
.text {
  min-width: 0;
}
.label {
  font-size: var(--fs-sm);
  font-weight: var(--fw-medium);
  color: var(--text);
}
h1 {
  margin: 0.15em 0 0.3em;
  font-size: 3.5em;
  font-weight: var(--fw-black);
  letter-spacing: -0.03em;
  line-height: 1.05;
  color: var(--text);
  overflow-wrap: anywhere;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
h1.long {
  font-size: 2.4em;
}
.meta {
  font-size: var(--fs-sm);
  color: var(--text-dim);
}
.actions {
  display: flex;
  align-items: center;
  gap: 1em;
  margin-bottom: 1.25em;
}
</style>
