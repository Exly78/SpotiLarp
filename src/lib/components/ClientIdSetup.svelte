<script>
  import * as api from "../api.js";

  /** @type {{ onSaved: () => void, onCancel?: () => void }} */
  let { onSaved, onCancel } = $props();

  let clientId = $state("");
  let saving = $state(false);
  let error = $state("");

  async function save() {
    if (!clientId.trim() || saving) return;
    if (!/^[0-9a-f]{32}$/i.test(clientId.trim())) {
      error = "That doesn't look like a Client ID, it should be 32 letters and numbers.";
      return;
    }
    saving = true;
    error = "";
    try {
      await api.setClientId(clientId);
      onSaved();
    } catch (e) {
      error = `${e}`;
    } finally {
      saving = false;
    }
  }

  /** @param {KeyboardEvent} evt */
  function onKeydown(evt) {
    if (evt.key === "Enter") save();
  }
</script>

<div class="setup-screen">
  <div class="setup-card">
    <h1>One-time setup</h1>
    <p class="intro">
      This app needs its own free Spotify Developer app to talk to Spotify's Web API. Takes about a
      minute, and only has to be done once on this computer.
    </p>
    <ol class="steps">
      <li>Go to <span class="mono">developer.spotify.com/dashboard</span> and log in with your Spotify account.</li>
      <li>Click <strong>Create app</strong>, any name/description is fine.</li>
      <li>
        In the app's <strong>Settings</strong>, add this exact Redirect URI:
        <div class="mono redirect-uri">http://127.0.0.1:8898/callback</div>
      </li>
      <li>Copy the app's <strong>Client ID</strong> and paste it below.</li>
    </ol>
    <input
      type="text"
      placeholder="Client ID"
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
    <div class="actions">
      <button type="button" onclick={save} disabled={saving || !clientId.trim()} class="save-button">
        {saving ? "Saving..." : "Save & Continue"}
      </button>
      {#if onCancel}
        <button type="button" onclick={onCancel} disabled={saving} class="cancel-button">Cancel</button>
      {/if}
    </div>
  </div>
</div>

<style>
.setup-screen {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 2em;
  background: var(--bg);
}
.setup-card {
  display: flex;
  flex-direction: column;
  gap: 1em;
  width: 100%;
  max-width: 480px;
  padding: 2em;
  border-radius: var(--radius-lg);
  background: var(--surface);
}
h1 {
  margin: 0;
  font-size: var(--fs-xl);
  font-weight: var(--fw-black);
  letter-spacing: -0.01em;
  color: var(--text);
}
.intro {
  margin: 0;
  font-size: var(--fs-sm);
  color: var(--text-muted);
  line-height: 1.5;
}
.steps {
  margin: 0;
  padding-left: 1.3em;
  display: flex;
  flex-direction: column;
  gap: 0.6em;
  font-size: var(--fs-sm);
  color: var(--text-dim);
  line-height: 1.5;
}
.mono {
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
  font-size: 0.9em;
  color: var(--text);
}
.redirect-uri {
  margin-top: 0.35em;
  padding: 0.5em 0.7em;
  border-radius: var(--radius-sm);
  background: var(--surface-raised);
  width: fit-content;
  user-select: all;
}
.client-id-input {
  width: 100%;
  padding: 0.7em 0.9em;
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
  font-size: var(--fs-sm);
  color: var(--danger);
}
.save-button {
  align-self: flex-start;
  padding: 0.6em 1.4em;
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
.actions {
  display: flex;
  align-items: center;
  gap: 0.75em;
}
.cancel-button {
  padding: 0.6em 1em;
  border: none;
  background: none;
  color: var(--text-dim);
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  font-family: inherit;
  cursor: pointer;
}
.cancel-button:hover:not(:disabled) {
  color: var(--text);
}
</style>
