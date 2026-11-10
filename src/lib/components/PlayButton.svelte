<script>
  import Icon from "./Icon.svelte";

  /** @type {{ onclick: () => void, disabled?: boolean, playing?: boolean, size?: number, label: string }} */
  let { onclick, disabled = false, playing = false, size = 56, label } = $props();
</script>

<button type="button" class="play" style={`--size: ${size}px`} {onclick} {disabled} aria-label={playing ? "Pause" : label}>
  <Icon name={playing ? "pause" : "play"} size={Math.round(size * 0.42)} />
</button>

<style>
.play {
  display: flex;
  align-items: center;
  justify-content: center;
  width: var(--size);
  height: var(--size);
  flex-shrink: 0;
  border: none;
  border-radius: 50%;
  background: var(--accent);
  color: var(--accent-text);
  cursor: pointer;
  transition: background-color var(--transition), transform 100ms ease;
}
.play:hover:not(:disabled) {
  background: var(--accent-hover);
  transform: scale(1.04);
}
.play:active:not(:disabled) {
  transform: scale(0.96);
}
.play:disabled {
  opacity: 0.5;
  cursor: default;
}
</style>
