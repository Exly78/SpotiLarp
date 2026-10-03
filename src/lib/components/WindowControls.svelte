<script>
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Icon from "./Icon.svelte";

  const appWindow = getCurrentWindow();
  let maximized = $state(false);

  onMount(() => {
    const update = () => {
      appWindow
        .isMaximized()
        .then((value) => (maximized = value))
        .catch(() => {});
    };
    update();
    const stopListening = appWindow.onResized(update);
    return () => {
      stopListening.then((stop) => stop());
    };
  });
</script>

<div class="window-controls">
  <button type="button" onclick={() => appWindow.minimize()} aria-label="Minimize" title="Minimize">
    <Icon name="window-minimize" size={16} />
  </button>
  <button
    type="button"
    onclick={() => appWindow.toggleMaximize()}
    aria-label={maximized ? "Restore" : "Maximize"}
    title={maximized ? "Restore" : "Maximize"}
  >
    <Icon name={maximized ? "window-restore" : "window-maximize"} size={16} />
  </button>
  <button type="button" class="close" onclick={() => appWindow.close()} aria-label="Close" title="Close">
    <Icon name="window-close" size={16} />
  </button>
</div>

<style>
.window-controls {
  display: flex;
  align-self: stretch;
  flex-shrink: 0;
}
button {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 46px;
  padding: 0;
  border: none;
  background: none;
  color: var(--text-dim);
  transition: background-color var(--transition), color var(--transition);
}
button:hover {
  background-color: var(--surface-hover);
  color: var(--text);
}
.close:hover {
  background-color: #e81123;
  color: #fff;
}
</style>
