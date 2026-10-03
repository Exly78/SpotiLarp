<script>
  import Icon from "./Icon.svelte";

  /** @type {{ onclick: () => void, disabled?: boolean, label: string }} */
  let { onclick, disabled = false, label } = $props();
</script>

<button type="button" class="play" {onclick} {disabled} aria-label={label}>
  <Icon name="play" size={22} />
</button>

<style>
.play {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 52px;
  height: 52px;
  flex-shrink: 0;
  border: none;
  border-radius: 50%;
  background: var(--accent);
  color: var(--accent-text);
  cursor: pointer;
  transition: background-color var(--transition), transform var(--transition);
}
.play:hover:not(:disabled) {
  background: var(--accent-hover);
  transform: scale(1.06);
}
.play:disabled {
  opacity: 0.5;
  cursor: default;
}
</style>
