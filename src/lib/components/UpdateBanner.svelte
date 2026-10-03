<script>
  import { onMount } from "svelte";
  import { check } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";

  /** @type {import("@tauri-apps/plugin-updater").Update|null} */
  let update = $state(null);
  let installing = $state(false);
  let error = $state("");
  let dismissed = $state(false);

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
</script>

{#if update && !dismissed}
  <div class="update-banner">
    <span class="text">
      {#if error}
        {error}
      {:else}
        A new version ({update.version}) is available.
      {/if}
    </span>
    <button type="button" onclick={installUpdate} disabled={installing} class="update-button">
      {installing ? "Updating..." : "Update & restart"}
    </button>
    {#if !installing}
      <button type="button" onclick={() => (dismissed = true)} class="later-button">Later</button>
    {/if}
  </div>
{/if}

<style>
.update-banner {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 1em;
  padding: 0.5em 1em;
  background: var(--accent-bg);
  color: var(--text);
  font-size: var(--fs-sm);
  border-bottom: 1px solid var(--border);
}
.text {
  color: var(--text-dim);
}
.update-button {
  padding: 0.4em 1.1em;
  border-radius: var(--radius-pill);
  border: none;
  background: var(--accent);
  color: var(--accent-text);
  font-size: var(--fs-xs);
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
  padding: 0.4em 0.6em;
  border: none;
  background: none;
  color: var(--text-dim);
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  font-family: inherit;
  cursor: pointer;
}
.later-button:hover {
  color: var(--text);
}
</style>
