<script>
  import * as api from "../api.js";
  import Icon from "./Icon.svelte";

  /** @type {boolean|null} */
  let enabled = $state(null);
  let open = $state(false);
  let clientId = $state("");
  let saving = $state(false);
  let error = $state("");
  /** @type {HTMLDivElement|undefined} */
  let root;

  $effect(() => {
    api.hasDiscordClientId().then((v) => (enabled = v));
  });

  function toggleOpen() {
    open = !open;
    error = "";
  }

  async function save() {
    if (!clientId.trim() || saving) return;
    saving = true;
    error = "";
    try {
      await api.setDiscordClientId(clientId);
      enabled = true;
      clientId = "";
      open = false;
    } catch (e) {
      error = `${e}`;
    } finally {
      saving = false;
    }
  }

  async function disable() {
    saving = true;
    try {
      await api.clearDiscordClientId();
      enabled = false;
    } finally {
      saving = false;
    }
  }

  /** @param {KeyboardEvent} evt */
  function onKeydown(evt) {
    if (evt.key === "Enter") save();
    if (evt.key === "Escape") open = false;
  }

  /** @param {MouseEvent} evt */
  function onWindowClick(evt) {
    if (open && root && !root.contains(/** @type {Node} */ (evt.target))) open = false;
  }
</script>

<svelte:window onclick={onWindowClick} />

<div class="discord-settings" bind:this={root}>
  <button
    type="button"
    class="panel-toggle"
    class:active={enabled}
    onclick={toggleOpen}
    aria-label="Discord Rich Presence settings"
    aria-pressed={open}
  >
    <Icon name="discord" size={18} />
  </button>

  {#if open}
    <div class="popover">
      {#if enabled}
        <p class="intro">Discord Rich Presence is on, your listening activity shows on your profile.</p>
        <button type="button" onclick={disable} disabled={saving} class="save-button danger">
          {saving ? "Disabling..." : "Disable"}
        </button>
      {:else}
        <p class="intro">
          Show what you're listening to on your Discord profile. Needs a free Discord application,
          takes about 30 seconds.
        </p>
        <ol class="steps">
          <li>Go to <span class="mono">discord.com/developers/applications</span> and log in.</li>
          <li>Click <strong>New Application</strong>, give it any name.</li>
          <li>Copy the <strong>Application ID</strong> from the app's General Information page.</li>
        </ol>
        <input
          type="text"
          placeholder="Application ID"
          bind:value={clientId}
          onkeydown={onKeydown}
          disabled={saving}
          class="client-id-input"
          autocomplete="off"
          spellcheck="false"
        />
        {#if error}
          <p class="error">{error}</p>
        {/if}
        <button type="button" onclick={save} disabled={saving || !clientId.trim()} class="save-button">
          {saving ? "Saving..." : "Enable"}
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
.discord-settings {
  position: relative;
}
.popover {
  position: absolute;
  top: calc(100% + 0.5em);
  right: 0;
  z-index: 20;
  display: flex;
  flex-direction: column;
  gap: 0.7em;
  width: 300px;
  padding: 1em;
  border-radius: var(--radius-md);
  background: var(--surface-raised);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5);
}
.intro {
  margin: 0;
  font-size: var(--fs-sm);
  color: var(--text-dim);
  line-height: 1.5;
}
.steps {
  margin: 0;
  padding-left: 1.2em;
  display: flex;
  flex-direction: column;
  gap: 0.5em;
  font-size: var(--fs-xs);
  color: var(--text-dim);
  line-height: 1.5;
}
.mono {
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
  font-size: 0.9em;
  color: var(--text);
}
.client-id-input {
  width: 100%;
  padding: 0.6em 0.8em;
  border-radius: var(--radius-md);
  border: 1px solid var(--border-strong);
  background: var(--control-bg);
  color: var(--text);
  font-size: var(--fs-sm);
  font-family: inherit;
  outline: none;
  transition: border-color var(--transition);
}
.client-id-input:focus {
  border-color: var(--accent);
}
.client-id-input::placeholder {
  color: var(--text-muted);
}
.error {
  margin: 0;
  font-size: var(--fs-xs);
  color: var(--danger);
}
.save-button {
  align-self: flex-start;
  padding: 0.5em 1.2em;
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
.save-button:hover:not(:disabled) {
  background: var(--accent-hover);
  transform: scale(1.03);
}
.save-button:disabled {
  opacity: 0.5;
  cursor: default;
}
.save-button.danger {
  background: transparent;
  border: 1px solid var(--border-strong);
  color: var(--text);
}
.save-button.danger:hover:not(:disabled) {
  border-color: var(--danger);
  color: var(--danger);
  background: transparent;
  transform: scale(1.03);
}
.panel-toggle {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  flex-shrink: 0;
  background: none;
  border: none;
  border-radius: 50%;
  color: var(--text-muted);
  cursor: pointer;
  transition: background-color var(--transition), color var(--transition);
}
.panel-toggle:hover {
  background-color: var(--surface-hover);
  color: var(--text);
}
.panel-toggle.active {
  color: var(--accent);
  background-color: var(--accent-bg);
}
</style>
