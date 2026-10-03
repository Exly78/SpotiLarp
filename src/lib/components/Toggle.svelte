<script>
  /** @type {{ on: boolean, onclick: () => void, label: string, disabled?: boolean }} */
  let { on, onclick, label, disabled = false } = $props();
</script>

<button type="button" class="switch" class:on role="switch" aria-checked={on} aria-label={label} {onclick} {disabled}>
  <span class="knob"></span>
</button>

<style>
.switch {
  position: relative;
  width: 40px;
  height: 22px;
  flex-shrink: 0;
  padding: 0;
  border: none;
  border-radius: var(--radius-pill);
  background: #535353;
  cursor: pointer;
  transition: background-color var(--transition);
}
.switch.on {
  background: var(--accent);
}
.knob {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #fff;
  transition: transform var(--transition);
}
.switch.on .knob {
  transform: translateX(18px);
}
.switch:disabled {
  opacity: 0.7;
}
</style>
