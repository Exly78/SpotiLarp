<script>
  import { fade } from "svelte/transition";
  import Icon from "./Icon.svelte";

  /** @typedef {{ label: string, onclick: () => unknown, disabled?: boolean }} MenuItem */

  /**
   * A page's "..." menu. `null` items draw a divider; false ones are left out.
   * @type {{ label: string, items: (MenuItem|null|false|undefined)[] }}
   */
  let { label, items } = $props();

  let open = $state(false);
  /** @type {HTMLDivElement|undefined} */
  let wrap = $state();

  const shown = $derived.by(() => {
    /** @type {(MenuItem|null)[]} */
    const list = [];
    for (const item of items) {
      if (item) list.push(item);
      else if (item === null && list.length > 0 && list[list.length - 1] !== null) list.push(null);
    }
    if (list[list.length - 1] === null) list.pop();
    return list;
  });

  /** @param {MenuItem} item */
  function choose(item) {
    open = false;
    item.onclick();
  }

  /** @param {MouseEvent} evt */
  function onWindowClick(evt) {
    if (open && !wrap?.contains(/** @type {Node} */ (evt.target))) open = false;
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={(e) => e.key === "Escape" && (open = false)} />

<div class="more-wrap" bind:this={wrap}>
  <button
    type="button"
    class="more-button"
    onclick={() => (open = !open)}
    aria-label={label}
    title={label}
    aria-haspopup="menu"
    aria-expanded={open}
  >
    <Icon name="more" size={26} />
  </button>
  {#if open}
    <div class="menu" role="menu" transition:fade={{ duration: 100 }}>
      {#each shown as item}
        {#if item}
          <button type="button" role="menuitem" disabled={item.disabled} onclick={() => choose(item)}>{item.label}</button>
        {:else}
          <div class="divider"></div>
        {/if}
      {/each}
    </div>
  {/if}
</div>

<style>
.more-wrap {
  position: relative;
}
.more-button {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: none;
  background: none;
  color: var(--text-muted);
  cursor: pointer;
  transition: color var(--transition), transform var(--transition);
}
.more-button:hover {
  color: var(--text);
  transform: scale(1.04);
}
.menu {
  position: absolute;
  top: calc(100% + 6px);
  left: 0;
  z-index: 60;
  width: 220px;
  padding: 4px;
  border-radius: var(--radius-sm);
  background: var(--surface-hover);
  box-shadow: 0 16px 24px rgba(0, 0, 0, 0.3), 0 6px 8px rgba(0, 0, 0, 0.2);
}
.menu button {
  display: block;
  width: 100%;
  padding: 0.7em 0.8em;
  border: none;
  border-radius: 2px;
  background: none;
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  text-align: left;
  cursor: pointer;
}
.menu button:hover:not(:disabled) {
  background: var(--highlight-strong);
}
.menu button:disabled {
  color: var(--text-muted);
  cursor: default;
}
.divider {
  height: 1px;
  margin: 4px 0;
  background: var(--border-strong);
}
</style>
