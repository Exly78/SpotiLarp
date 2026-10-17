<script>
  import { queue, toggleShuffle, shuffleMode, shuffleLabels } from "../queue.svelte.js";
  import Icon from "./Icon.svelte";

  // Picks the playback order without starting anything; the next Play uses it.
</script>

<button
  type="button"
  class="shuffle-button"
  class:active={queue.shuffle}
  onclick={toggleShuffle}
  aria-label={shuffleLabels().action}
  title={`${shuffleLabels().state} (click to change)`}
  aria-pressed={queue.shuffle}
>
  <Icon name={shuffleMode() === "smart" ? "shuffle-smart" : "shuffle"} size={28} />
</button>

<style>
.shuffle-button {
  position: relative;
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
.shuffle-button:hover {
  color: var(--text);
  transform: scale(1.04);
}
.shuffle-button.active {
  color: var(--accent);
}
.shuffle-button.active::after {
  content: "";
  position: absolute;
  bottom: -0.3em;
  left: 50%;
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: currentColor;
  transform: translateX(-50%);
}
</style>
