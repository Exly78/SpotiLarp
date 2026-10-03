<script>
  import Icon from "./Icon.svelte";

  /** @type {{ initial: string, label: string, children: import("svelte").Snippet }} */
  let { initial, label, children } = $props();

  let open = $state(false);
  /** @type {HTMLDivElement|undefined} */
  let root;

  /** @param {MouseEvent} evt */
  function onWindowClick(evt) {
    if (open && root && !root.contains(/** @type {Node} */ (evt.target))) open = false;
  }

  /** @param {KeyboardEvent} evt */
  function onWindowKeydown(evt) {
    if (evt.key === "Escape") open = false;
  }

  /** @param {MouseEvent} evt */
  function onMenuClick(evt) {
    if (/** @type {HTMLElement} */ (evt.target).closest("button")) open = false;
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={onWindowKeydown} />

<div class="account-menu" bind:this={root} data-tauri-drag-region="false">
  <button
    type="button"
    class="avatar"
    onclick={() => (open = !open)}
    aria-label={label}
    aria-haspopup="menu"
    aria-expanded={open}
    title={label}
  >
    {#if initial}
      {initial}
    {:else}
      <Icon name="user" size={18} />
    {/if}
  </button>
  {#if open}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div class="popover" onclick={onMenuClick}>
      {@render children()}
    </div>
  {/if}
</div>

<style>
.account-menu {
  position: relative;
}
.avatar {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  padding: 0;
  border: none;
  border-radius: 50%;
  background: var(--control-bg);
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  text-transform: uppercase;
  cursor: pointer;
  transition: background-color var(--transition), transform 100ms ease;
}
.avatar:hover {
  background: var(--surface-hover);
  transform: scale(1.05);
}
.popover {
  position: absolute;
  top: calc(100% + 0.5em);
  right: 0;
  z-index: 20;
  display: flex;
  flex-direction: column;
  gap: 0.6em;
  min-width: 220px;
  padding: 1em;
  border-radius: var(--radius-md);
  background: var(--surface-raised);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5);
}
</style>
