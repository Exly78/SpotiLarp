<script>
  import { openUrl } from "@tauri-apps/plugin-opener";
  import * as api from "../api.js";

  /** @type {{ onSaved: () => void, onCancel?: () => void }} */
  let { onSaved, onCancel } = $props();

  const DASHBOARD_URL = "https://developer.spotify.com/dashboard";
  const REDIRECT_URI = "http://127.0.0.1:8898/callback";
  const CLIENT_ID_LENGTH = 32;
  const COPIED_MS = 2000;

  let clientId = $state("");
  let saving = $state(false);
  let error = $state("");
  let copied = $state(false);

  const trimmed = $derived(clientId.trim());
  const valid = $derived(/^[0-9a-f]{32}$/i.test(trimmed));
  // Says what's wrong as you type, once there's enough to tell.
  const hint = $derived.by(() => {
    if (!trimmed || valid) return "";
    if (/[^0-9a-f]/i.test(trimmed)) return "A Client ID only has the numbers 0-9 and the letters a-f.";
    if (trimmed.length > CLIENT_ID_LENGTH) return `That's ${trimmed.length} characters, a Client ID has ${CLIENT_ID_LENGTH}.`;
    return "";
  });

  async function save() {
    if (!trimmed || saving) return;
    if (!valid) {
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

  function openDashboard() {
    openUrl(DASHBOARD_URL).catch(() => (error = `Couldn't open the browser. Go to ${DASHBOARD_URL} yourself.`));
  }

  async function copyRedirect() {
    try {
      await navigator.clipboard.writeText(REDIRECT_URI);
      copied = true;
      setTimeout(() => (copied = false), COPIED_MS);
    } catch {
      // Still there to select and copy by hand.
    }
  }
</script>

<div class="setup-screen">
  <div class="setup-card">
    <img src="/favicon.png" alt="" class="logo" />
    <h1>Set up SpotiLarp</h1>
    <p class="intro">
      SpotiLarp talks to Spotify through your own free developer app. It takes about a minute and
      only has to be done once on this computer.
    </p>

    <ol class="steps">
      <li>
        <span class="number">1</span>
        <div class="step">
          <div class="step-title">Open the Spotify dashboard and log in</div>
          <button type="button" class="outline-button" onclick={openDashboard}>Open developer.spotify.com</button>
        </div>
      </li>
      <li>
        <span class="number">2</span>
        <div class="step">
          <div class="step-title">Click <strong>Create app</strong></div>
          <div class="step-note">Any name and description will do.</div>
        </div>
      </li>
      <li>
        <span class="number">3</span>
        <div class="step">
          <div class="step-title">Add this Redirect URI, exactly as it is</div>
          <div class="copy-field">
            <span class="mono">{REDIRECT_URI}</span>
            <button type="button" class="copy-button" onclick={copyRedirect}>{copied ? "Copied" : "Copy"}</button>
          </div>
        </div>
      </li>
      <li>
        <span class="number">4</span>
        <div class="step">
          <div class="step-title">Paste the app's <strong>Client ID</strong> here</div>
          <input
            type="text"
            placeholder="32 letters and numbers"
            bind:value={clientId}
            onkeydown={onKeydown}
            oninput={() => (error = "")}
            disabled={saving}
            class="client-id-input"
            class:valid
            class:invalid={!!hint || !!error}
            autocomplete="off"
            spellcheck="false"
            aria-label="Client ID"
          />
          {#if error || hint}
            <p class="error">{error || hint}</p>
          {/if}
        </div>
      </li>
    </ol>

    <button type="button" onclick={save} disabled={saving || !trimmed} class="save-button">
      {saving ? "Saving..." : "Save & continue"}
    </button>
    {#if onCancel}
      <button type="button" onclick={onCancel} disabled={saving} class="cancel-button">Cancel</button>
    {/if}
    <p class="footnote">Your Client ID stays on this computer.</p>
  </div>
</div>

<style>
.setup-screen {
  display: flex;
  justify-content: center;
  width: 100%;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 2em;
  background: linear-gradient(var(--surface-hover), var(--bg) 60%);
}
.setup-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1em;
  width: 100%;
  max-width: 520px;
  /* Centered, but scrolls from the top when the window is shorter than the card. */
  margin: auto;
  padding: 2.5em;
  border-radius: var(--radius-md);
  background: var(--surface);
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.4);
}
.logo {
  width: 56px;
  height: 56px;
  border-radius: var(--radius-lg);
}
h1 {
  margin: 0;
  font-size: 2em;
  font-weight: var(--fw-black);
  letter-spacing: -0.03em;
  text-align: center;
  color: var(--text);
}
.intro {
  margin: 0;
  font-size: var(--fs-sm);
  line-height: 1.5;
  text-align: center;
  color: var(--text-dim);
}
.steps {
  display: flex;
  flex-direction: column;
  gap: 1.25em;
  width: 100%;
  margin: 0.75em 0 0.5em;
  padding: 0;
  list-style: none;
}
.steps li {
  display: flex;
  gap: 1em;
}
.number {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  border-radius: 50%;
  background: var(--surface-hover);
  color: var(--text);
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
}
.step {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.5em;
  flex: 1;
  min-width: 0;
  padding-top: 0.2em;
}
.step-title {
  font-weight: var(--fw-medium);
  color: var(--text);
}
.step-note {
  margin-top: -0.3em;
  font-size: var(--fs-sm);
  color: var(--text-muted);
}
.outline-button {
  padding: 0.5em 1.2em;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-pill);
  background: none;
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
  transition: border-color var(--transition), transform 100ms ease;
}
.outline-button:hover {
  border-color: var(--text);
  transform: scale(1.03);
}
.copy-field {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75em;
  width: 100%;
  padding: 0.35em 0.35em 0.35em 0.9em;
  border-radius: var(--radius-sm);
  background: var(--control-bg);
}
.mono {
  min-width: 0;
  overflow-wrap: anywhere;
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
  font-size: var(--fs-sm);
  color: var(--text);
  user-select: all;
}
.copy-button {
  flex-shrink: 0;
  padding: 0.4em 0.9em;
  border: none;
  border-radius: var(--radius-pill);
  background: var(--surface-hover);
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  cursor: pointer;
  transition: background-color var(--transition);
}
.copy-button:hover {
  background: var(--highlight-strong);
}
.client-id-input {
  width: 100%;
  height: 48px;
  padding: 0 1em;
  border: none;
  border-radius: var(--radius-sm);
  background: var(--control-bg);
  color: var(--text);
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
  font-size: var(--fs-sm);
  outline: none;
  box-shadow: inset 0 0 0 1px var(--border-strong);
  transition: box-shadow var(--transition);
}
.client-id-input:hover {
  box-shadow: inset 0 0 0 1px var(--text);
}
.client-id-input:focus {
  box-shadow: inset 0 0 0 2px var(--text);
}
.client-id-input.valid {
  box-shadow: inset 0 0 0 2px var(--accent);
}
.client-id-input.invalid {
  box-shadow: inset 0 0 0 2px var(--danger);
}
.client-id-input::placeholder {
  font-family: var(--font-ui);
  color: var(--text-muted);
}
.error {
  margin: 0;
  font-size: var(--fs-sm);
  color: var(--danger);
}
.save-button {
  width: 100%;
  height: 48px;
  border: none;
  border-radius: var(--radius-pill);
  background: var(--accent);
  color: var(--accent-text);
  font-family: inherit;
  font-size: 1em;
  font-weight: var(--fw-bold);
  cursor: pointer;
  transition: background-color var(--transition), transform 100ms ease;
}
.save-button:hover:not(:disabled) {
  background: var(--accent-hover);
  transform: scale(1.02);
}
.save-button:disabled {
  opacity: 0.5;
  cursor: default;
}
.cancel-button {
  padding: 0.3em 1em;
  border: none;
  background: none;
  color: var(--text-dim);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.cancel-button:hover:not(:disabled) {
  color: var(--text);
}
.footnote {
  margin: 0;
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
</style>
