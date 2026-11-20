<script>
  import { fade } from "svelte/transition";

  /**
   * The steps (tourSteps.js), and for a what's-new tour its name ("New in 1.3.0").
   * @type {{ steps: import("../tourSteps.js").TourStep[], label?: string, onClose: () => void }}
   */
  let { steps, label = "", onClose } = $props();

  const CARD_WIDTH = 320;
  /** A release's list of changes gets a little more room. */
  const LIST_WIDTH = 400;
  const GAP = 14;
  const EDGE = 12;
  /** Room around the control inside the ring. */
  const RING_PAD = 6;

  let index = $state(0);
  /** @type {DOMRect|null} */
  let rect = $state(null);
  let cardHeight = $state(0);
  let innerWidth = $state(0);
  let innerHeight = $state(0);
  /** @type {HTMLButtonElement|undefined} */
  let nextButton = $state();

  const step = $derived(steps[index]);
  const last = $derived(index === steps.length - 1);

  // Below the control when there's room, otherwise above it.
  const cardPosition = $derived.by(() => {
    if (!rect) return null;
    const below = rect.bottom + GAP + cardHeight <= innerHeight - EDGE;
    const top = below ? rect.bottom + GAP : Math.max(EDGE, rect.top - GAP - cardHeight);
    const left = Math.min(Math.max(EDGE, rect.left + rect.width / 2 - CARD_WIDTH / 2), innerWidth - CARD_WIDTH - EDGE);
    return { top, left };
  });

  function locate() {
    const target = step.target ? document.querySelector(step.target) : null;
    const found = target?.getBoundingClientRect();
    rect = found && found.width > 0 && found.height > 0 ? found : null;
  }

  $effect(() => {
    index;
    locate();
    nextButton?.focus();
  });

  function next() {
    if (last) onClose();
    else index += 1;
  }

  function back() {
    if (index > 0) index -= 1;
  }

  /** @param {KeyboardEvent} evt */
  function onKeydown(evt) {
    // Ahead of the app's own shortcuts, which wait until the tour's done.
    evt.stopPropagation();
    if (evt.key === "Escape") onClose();
    else if (evt.key === "ArrowRight") next();
    else if (evt.key === "ArrowLeft") back();
    // Tab, Enter and Space still work the tour's buttons.
    else return;
    evt.preventDefault();
  }
</script>

<svelte:window bind:innerWidth bind:innerHeight onresize={locate} onkeydowncapture={onKeydown} />

<!-- Keeps clicks off the app while the tour is up. -->
<div class="catcher" class:dim={!rect} transition:fade={{ duration: 150 }}></div>
{#if rect}
  <div
    class="ring"
    style:top="{rect.top - RING_PAD}px"
    style:left="{rect.left - RING_PAD}px"
    style:width="{rect.width + RING_PAD * 2}px"
    style:height="{rect.height + RING_PAD * 2}px"
  ></div>
{/if}
<div
  class="card"
  class:centered={!cardPosition}
  style:top={cardPosition ? `${cardPosition.top}px` : null}
  style:left={cardPosition ? `${cardPosition.left}px` : null}
  style:width="{step.list ? LIST_WIDTH : CARD_WIDTH}px"
  role="dialog"
  aria-modal="true"
  aria-labelledby="tour-title"
  bind:clientHeight={cardHeight}
  transition:fade={{ duration: 150 }}
>
  <div class="count">
    {#if label}<span class="label">{label}</span>{" · "}{/if}{index + 1} of {steps.length}
  </div>
  <h2 id="tour-title">{step.title}</h2>
  {#if step.text}
    <p>{step.text}</p>
  {/if}
  {#if step.list}
    <div class="lists">
      {#each step.list as group (group.heading)}
        <h3>{group.heading}</h3>
        <ul>
          {#each group.items as item, i (i)}
            <li>{item}</li>
          {/each}
        </ul>
      {/each}
    </div>
  {/if}
  <div class="buttons">
    {#if !last}
      <button type="button" class="skip" onclick={onClose}>{label ? "Skip" : "Skip tour"}</button>
    {/if}
    {#if index > 0}
      <button type="button" class="back" onclick={back}>Back</button>
    {/if}
    <button type="button" class="next" onclick={next} bind:this={nextButton}>{last ? "Done" : "Next"}</button>
  </div>
</div>

<style>
.catcher {
  position: fixed;
  inset: 0;
  z-index: 90;
}
.catcher.dim {
  background: rgba(0, 0, 0, 0.62);
}
/* The dimming around the control is the ring's own shadow, so the control stays lit. */
.ring {
  position: fixed;
  z-index: 90;
  border-radius: var(--radius-lg);
  box-shadow: 0 0 0 2px var(--accent), 0 0 0 9999px rgba(0, 0, 0, 0.62);
  pointer-events: none;
  transition: top 250ms ease, left 250ms ease, width 250ms ease, height 250ms ease;
}
.card {
  position: fixed;
  z-index: 91;
  display: flex;
  flex-direction: column;
  gap: 0.5em;
  padding: 1.1em 1.2em 1em;
  border-radius: var(--radius-md);
  background: var(--surface-hover);
  box-shadow: 0 16px 24px rgba(0, 0, 0, 0.3), 0 6px 8px rgba(0, 0, 0, 0.2);
  transition: top 250ms ease, left 250ms ease;
}
.card.centered {
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
}
.count {
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
.count .label {
  color: var(--accent);
  font-weight: var(--fw-bold);
}
h2 {
  margin: 0;
  font-size: 1.15em;
  font-weight: var(--fw-bold);
  color: var(--text);
}
p {
  margin: 0;
  font-size: var(--fs-sm);
  line-height: 1.5;
  color: var(--text-dim);
}
/* Long releases scroll inside the card. */
.lists {
  max-height: min(55vh, 420px);
  overflow-y: auto;
  margin-right: -0.4em;
  padding-right: 0.4em;
}
h3 {
  margin: 0.6em 0 0.25em;
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--text-muted);
}
h3:first-child {
  margin-top: 0;
}
ul {
  margin: 0;
  padding-left: 1.1em;
}
li {
  font-size: var(--fs-sm);
  line-height: 1.5;
  color: var(--text-dim);
}
.buttons {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 0.5em;
  margin-top: 0.4em;
}
.buttons button {
  padding: 0.5em 1.1em;
  border: none;
  border-radius: var(--radius-pill);
  background: none;
  color: var(--text-dim);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.buttons button:hover {
  color: var(--text);
}
.buttons .skip {
  margin-right: auto;
  padding-left: 0;
}
.buttons .next {
  background: var(--text);
  color: var(--bg);
  transition: transform 100ms ease;
}
.buttons .next:hover {
  transform: scale(1.04);
}
@media (prefers-reduced-motion: reduce) {
  .ring,
  .card {
    transition: none;
  }
}
</style>
