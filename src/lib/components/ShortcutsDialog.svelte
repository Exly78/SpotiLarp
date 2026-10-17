<script>
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { SHORTCUTS } from "../keys.js";
  import Icon from "./Icon.svelte";

  /** @type {{ onClose: () => void }} */
  let { onClose } = $props();

  /** @type {HTMLDivElement|undefined} */
  let dialog = $state();

  onMount(() => dialog?.focus());
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onClose()} />

<div class="backdrop" role="presentation" transition:fade={{ duration: 120 }} onclick={(e) => e.target === e.currentTarget && onClose()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-label="Keyboard shortcuts" tabindex="-1" bind:this={dialog}>
    <div class="head">
      <h2>Keyboard shortcuts</h2>
      <button type="button" class="close" onclick={onClose} aria-label="Close">
        <Icon name="close" size={16} />
      </button>
    </div>
    <dl>
      {#each SHORTCUTS as [keys, action]}
        <dt><kbd>{keys}</kbd></dt>
        <dd>{action}</dd>
      {/each}
    </dl>
    <p class="hint">On macOS, Cmd replaces Ctrl. Shortcuts wait while you type in a text field.</p>
  </div>
</div>

<style>
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.6);
}
.dialog {
  width: min(620px, calc(100vw - 2em));
  max-height: calc(100vh - 4em);
  overflow-y: auto;
  padding: 1.5em;
  border-radius: var(--radius-md);
  background: var(--surface-hover);
  box-shadow: 0 16px 24px rgba(0, 0, 0, 0.3), 0 6px 8px rgba(0, 0, 0, 0.2);
  outline: none;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 1em;
}
h2 {
  margin: 0;
  font-size: var(--fs-lg);
  font-weight: var(--fw-bold);
}
.close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 50%;
  background: none;
  color: var(--text-muted);
  cursor: pointer;
}
.close:hover {
  background: var(--highlight-strong);
  color: var(--text);
}
dl {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: 0.55em 1.2em;
  margin: 0;
  font-size: var(--fs-sm);
}
dt {
  margin: 0;
}
dd {
  margin: 0;
  color: var(--text-dim);
}
kbd {
  font-family: inherit;
  font-weight: var(--fw-bold);
  color: var(--text);
}
.hint {
  margin: 1.2em 0 0;
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
</style>
