<script>
  import { onMount } from "svelte";
  import { check } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";

  /** @type {import("@tauri-apps/plugin-updater").Update|null} */
  let update = $state(null);
  let installing = $state(false);
  let error = $state("");
  let dismissed = $state(false);
  let open = $state(false);
  /** @type {HTMLDivElement|undefined} */
  let root = $state();

  onMount(async () => {
    try {
      update = await check();
    } catch (e) {
    }
  });

  async function installUpdate() {
    if (!update || installing) return;
    installing = true;
    error = "";
    try {
      await update.downloadAndInstall();
      await relaunch();
    } catch (e) {
      error = `Update failed: ${e}`;
      installing = false;
    }
  }

  /** @param {MouseEvent} evt */
  function onWindowClick(evt) {
    if (open && !installing && !root?.contains(/** @type {Node} */ (evt.target))) open = false;
  }

  /** @param {KeyboardEvent} evt */
  function onWindowKeydown(evt) {
    if (evt.key === "Escape" && !installing) open = false;
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={onWindowKeydown} />

<!-- A pill in the top bar; the details and the buttons are in its popover. -->
{#if update && !dismissed}
  <div class="update" bind:this={root} data-tauri-drag-region="false">
    <button type="button" class="pill" onclick={() => (open = !open)} aria-haspopup="dialog" aria-expanded={open}>
      <span class="pill-dot"></span>
      {installing ? "Updating..." : "Update available"}
    </button>
    {#if open}
      <div class="popover" role="dialog" aria-label="Update SpotiLarp">
        <div class="title">SpotiLarp {update.version} is out</div>
        <p class="text">It downloads and installs now, then SpotiLarp restarts. Your queue comes back.</p>
        {#if error}
          <p class="error">{error}</p>
        {/if}
        <div class="buttons">
          <button type="button" onclick={installUpdate} disabled={installing} class="update-button">
            {installing ? "Updating..." : "Update & restart"}
          </button>
          {#if !installing}
            <button type="button" onclick={() => (dismissed = true)} class="later-button">Later</button>
          {/if}
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
.update {
  position: relative;
  flex-shrink: 0;
}
.pill {
  display: flex;
  align-items: center;
  gap: 0.5em;
  height: 32px;
  padding: 0 1em;
  border: none;
  border-radius: var(--radius-pill);
  background: var(--toast-bg);
  color: var(--toast-text);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  white-space: nowrap;
  cursor: pointer;
  transition: transform 100ms ease, filter var(--transition);
}
.pill:hover {
  filter: brightness(1.1);
  transform: scale(1.03);
}
.pill-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: currentColor;
}
.popover {
  position: absolute;
  top: calc(100% + 0.5em);
  right: 0;
  z-index: 20;
  display: flex;
  flex-direction: column;
  gap: 0.6em;
  width: 300px;
  padding: 1em;
  border-radius: var(--radius-md);
  background: var(--surface-hover);
  box-shadow: 0 16px 24px rgba(0, 0, 0, 0.3), 0 6px 8px rgba(0, 0, 0, 0.2);
}
.title {
  font-weight: var(--fw-bold);
  color: var(--text);
}
.text,
.error {
  margin: 0;
  font-size: var(--fs-sm);
  line-height: 1.5;
  color: var(--text-dim);
}
.error {
  color: var(--danger);
}
.buttons {
  display: flex;
  align-items: center;
  gap: 0.5em;
  margin-top: 0.2em;
}
.update-button {
  padding: 0.55em 1.3em;
  border-radius: var(--radius-pill);
  border: none;
  background: var(--accent);
  color: var(--accent-text);
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  font-family: inherit;
  cursor: pointer;
  transition: background-color var(--transition), transform 100ms ease;
}
.update-button:hover:not(:disabled) {
  background: var(--accent-hover);
  transform: scale(1.03);
}
.update-button:disabled {
  opacity: 0.6;
  cursor: default;
}
.later-button {
  padding: 0.55em 0.8em;
  border: none;
  background: none;
  color: var(--text-dim);
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  font-family: inherit;
  cursor: pointer;
}
.later-button:hover {
  color: var(--text);
}
</style>
