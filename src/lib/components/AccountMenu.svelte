<script>
  import Icon from "./Icon.svelte";

  /** @type {{ initial: string, label: string, imageUrl?: string|null, children: import("svelte").Snippet }} */
  let { initial, label, imageUrl = null, children } = $props();

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
    {#if imageUrl}
      <img src={imageUrl} alt="" class="avatar-image" />
    {:else if initial}
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
/* A dark ring around the picture, as Spotify has. */
.avatar {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  padding: 8px;
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
  background: var(--control-hover);
  transform: scale(1.04);
}
.avatar:active {
  transform: scale(0.96);
}
.avatar-image {
  width: 100%;
  height: 100%;
  border-radius: 50%;
  object-fit: cover;
}
.popover {
  position: absolute;
  top: calc(100% + 0.5em);
  right: 0;
  z-index: 20;
  display: flex;
  flex-direction: column;
  gap: 0.6em;
  min-width: 260px;
  padding: 1em;
  border-radius: var(--radius-md);
  background: var(--surface-hover);
  box-shadow: 0 16px 24px rgba(0, 0, 0, 0.3), 0 6px 8px rgba(0, 0, 0, 0.2);
}
</style>
