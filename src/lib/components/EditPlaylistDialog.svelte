<script>
  import { onMount, untrack } from "svelte";
  import { fade } from "svelte/transition";
  import { htmlToText } from "../utils.js";
  import Toggle from "./Toggle.svelte";

  /**
   * @type {{
   *   playlist: import("../types.js").Playlist,
   *   onSave: (details: { name: string, description: string, public: boolean }) => Promise<void>,
   *   onCancel: () => void,
   * }}
   */
  let { playlist, onSave, onCancel } = $props();

  const initial = untrack(() => ({
    name: playlist.name,
    description: htmlToText(playlist.description),
    isPublic: playlist.public ?? false,
  }));
  let name = $state(initial.name);
  let description = $state(initial.description);
  let isPublic = $state(initial.isPublic);
  let saving = $state(false);
  let error = $state("");
  /** @type {HTMLInputElement|undefined} */
  let nameInput = $state();

  onMount(() => {
    nameInput?.focus();
    nameInput?.select();
  });

  async function save() {
    if (!name.trim() || saving) return;
    saving = true;
    error = "";
    try {
      await onSave({ name: name.trim(), description: description.trim(), public: isPublic });
    } catch (e) {
      error = `Couldn't save: ${e}`;
    } finally {
      saving = false;
    }
  }

  /** @param {KeyboardEvent} evt */
  function onKeydown(evt) {
    if (evt.key === "Escape") onCancel();
    else if (evt.key === "Enter" && (evt.ctrlKey || evt.target instanceof HTMLInputElement)) save();
  }
</script>

<div class="backdrop" role="presentation" transition:fade={{ duration: 120 }} onclick={(e) => e.target === e.currentTarget && onCancel()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-label="Edit details" tabindex="-1" onkeydown={onKeydown}>
    <h2>Edit details</h2>
    <label class="field">
      <span>Name</span>
      <input bind:this={nameInput} bind:value={name} maxlength="100" spellcheck="false" disabled={saving} />
    </label>
    <label class="field">
      <span>Description</span>
      <textarea bind:value={description} rows="4" maxlength="300" placeholder="Add an optional description" disabled={saving}></textarea>
    </label>
    <p class="hint">Spotify doesn't let apps remove a description, only replace it.</p>
    <div class="toggle-row">
      <span>Public (shown on your profile)</span>
      <Toggle on={isPublic} onclick={() => (isPublic = !isPublic)} label="Public playlist" disabled={saving} />
    </div>
    {#if error}<p class="error">{error}</p>{/if}
    <div class="buttons">
      <button type="button" class="pill-button" onclick={onCancel} disabled={saving}>Cancel</button>
      <button type="button" class="pill-button primary" onclick={save} disabled={saving || !name.trim()}>
        {saving ? "Saving..." : "Save"}
      </button>
    </div>
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
  display: flex;
  flex-direction: column;
  gap: 0.9em;
  width: min(480px, calc(100vw - 2em));
  padding: 1.5em;
  border-radius: var(--radius-md);
  background: var(--surface-hover);
  box-shadow: 0 16px 24px rgba(0, 0, 0, 0.3), 0 6px 8px rgba(0, 0, 0, 0.2);
  outline: none;
}
h2 {
  margin: 0;
  font-size: var(--fs-lg);
  font-weight: var(--fw-bold);
}
.field {
  display: flex;
  flex-direction: column;
  gap: 0.35em;
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  color: var(--text-dim);
}
.field input,
.field textarea {
  padding: 0.6em 0.8em;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: var(--control-bg);
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: 400;
  outline: none;
  resize: vertical;
}
.field input:focus,
.field textarea:focus {
  border-color: var(--text);
}
.hint {
  margin: -0.4em 0 0;
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
.toggle-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1em;
  font-size: var(--fs-sm);
}
.error {
  margin: 0;
  font-size: var(--fs-sm);
  color: var(--danger);
}
.buttons {
  display: flex;
  justify-content: flex-end;
  gap: 0.6em;
}
.pill-button {
  padding: 0.55em 1.3em;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-pill);
  background: none;
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.pill-button.primary {
  border-color: transparent;
  background: var(--text);
  color: var(--main-bg);
}
.pill-button:disabled {
  opacity: 0.6;
  cursor: default;
}
</style>
