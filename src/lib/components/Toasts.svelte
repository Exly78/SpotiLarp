<script>
  import { fly } from "svelte/transition";
  import { toasts, dismiss } from "../toast.svelte.js";
</script>

<div class="toasts" aria-live="polite">
  {#each toasts as toast (toast.id)}
    <button
      type="button"
      class="toast"
      onclick={() => dismiss(toast.id)}
      in:fly={{ y: 12, duration: 200 }}
      out:fly={{ y: 12, duration: 150 }}
    >
      {toast.message}
    </button>
  {/each}
</div>

<style>
.toasts {
  position: fixed;
  left: 50%;
  bottom: 104px;
  transform: translateX(-50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5em;
  z-index: 50;
  pointer-events: none;
  width: max-content;
  max-width: min(560px, calc(100vw - 2em));
}
.toast {
  pointer-events: auto;
  padding: 0.7em 1.1em;
  border: none;
  border-radius: var(--radius-md);
  background: #2e77d0;
  color: var(--text);
  font: inherit;
  font-size: var(--fs-sm);
  text-align: left;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
  cursor: pointer;
  overflow-wrap: anywhere;
}
</style>
