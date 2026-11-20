<script>
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { notify } from "../toast.svelte.js";
  import {
    lastfm,
    refreshLastfm,
    setLastfmKeys,
    connectLastfm,
    cancelConnectLastfm,
    disconnectLastfm,
    toggleLastfmScrobbling,
  } from "../lastfm.svelte.js";
  import Toggle from "./Toggle.svelte";

  const CREATE_ACCOUNT_URL = "https://www.last.fm/api/account/create";

  let apiKey = $state("");
  let secret = $state("");
  let editingKeys = $state(false);
  let saving = $state(false);
  let error = $state("");

  $effect(() => {
    refreshLastfm();
  });

  /** @param {string} url */
  function open(url) {
    openUrl(url).catch(() => notify(`Couldn't open the browser. Go to ${url} yourself.`));
  }

  /** @param {SubmitEvent} event */
  async function saveKeys(event) {
    event.preventDefault();
    if (saving) return;
    saving = true;
    error = "";
    try {
      await setLastfmKeys(apiKey, secret);
      apiKey = "";
      secret = "";
      editingKeys = false;
      connectLastfm();
    } catch (e) {
      error = `${e}`;
    } finally {
      saving = false;
    }
  }
</script>

<section>
  <h2>Last.fm</h2>
  {#if lastfm.connected}
    <div class="row">
      <div>
        <div class="label">
          Connected as
          <button type="button" class="link" onclick={() => open(`https://www.last.fm/user/${lastfm.username}`)}>
            {lastfm.username}
          </button>
        </div>
        <div class="hint">
          Songs you play in SpotiLarp go to your Last.fm profile once you've heard half of them (or 4 minutes).
          {#if lastfm.queued > 0}{lastfm.queued} {lastfm.queued === 1 ? "scrobble is" : "scrobbles are"} waiting to send.{/if}
        </div>
      </div>
      <button type="button" class="secondary" onclick={disconnectLastfm}>Disconnect</button>
    </div>
    <div class="row">
      <div>
        <div class="label">Scrobble songs</div>
        <div class="hint">
          Pauses scrobbling without disconnecting. Podcasts, local files and other people's Jams aren't scrobbled.
        </div>
      </div>
      <Toggle on={lastfm.enabled} onclick={toggleLastfmScrobbling} label="Scrobble to Last.fm" />
    </div>
  {:else if lastfm.hasKeys && !editingKeys}
    <div class="row">
      <div>
        <div class="label">Scrobble to Last.fm</div>
        <div class="hint">
          {#if lastfm.connecting}
            Waiting for you to approve in your browser...
          {:else}
            Sends the songs you play in SpotiLarp to your Last.fm profile. Your API account is saved; connect your
            Last.fm account to start.
          {/if}
        </div>
      </div>
      <div class="buttons">
        {#if lastfm.connecting}
          <button type="button" class="secondary" onclick={cancelConnectLastfm}>Cancel</button>
        {:else}
          <button type="button" class="secondary" onclick={() => (editingKeys = true)}>Change keys...</button>
          <button type="button" class="primary" onclick={connectLastfm}>Connect</button>
        {/if}
      </div>
    </div>
  {:else}
    <div class="row">
      <div>
        <div class="label">Scrobble to Last.fm</div>
        <div class="hint">
          Sends the songs you play in SpotiLarp to your Last.fm profile. Last.fm wants an API account of your own for
          that, and it's free:
          <button type="button" class="link" onclick={() => open(CREATE_ACCOUNT_URL)}>create one</button>
          (any name, leave the callback URL empty), then paste its API key and shared secret here. Spotify's own apps
          scrobble by themselves if you've connected Last.fm in Spotify.
        </div>
      </div>
    </div>
    <form class="keys" onsubmit={saveKeys}>
      <input type="text" placeholder="API key" bind:value={apiKey} spellcheck="false" autocomplete="off" aria-label="Last.fm API key" />
      <input type="password" placeholder="Shared secret" bind:value={secret} autocomplete="off" aria-label="Last.fm shared secret" />
      <div class="buttons">
        {#if lastfm.hasKeys}
          <button type="button" class="secondary" onclick={() => (editingKeys = false)}>Cancel</button>
        {/if}
        <button type="submit" class="primary" disabled={saving || !apiKey.trim() || !secret.trim()}>
          {saving ? "Saving..." : "Save and connect"}
        </button>
      </div>
    </form>
    {#if error}
      <p class="hint error-text">{error}</p>
    {/if}
  {/if}
</section>

<style>
/* The settings page's own look (SettingsView.svelte), whose styles don't reach in here. */
section {
  max-width: 760px;
  margin-bottom: 2em;
}
h2 {
  margin: 0 0 0.6em;
  font-size: var(--fs-md);
  font-weight: var(--fw-bold);
  color: var(--text);
}
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 2em;
  padding: 0.7em 0;
}
.label {
  font-size: var(--fs-sm);
  color: var(--text);
}
.hint {
  margin-top: 0.2em;
  font-size: var(--fs-xs);
  line-height: 1.5;
  color: var(--text-muted);
}
.error-text {
  margin: 0;
  color: var(--danger);
}
.buttons {
  display: flex;
  flex-shrink: 0;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.5em;
}
.secondary,
.primary {
  padding: 0.45em 1em;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-pill);
  background: none;
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  cursor: pointer;
  transition: border-color var(--transition), background-color var(--transition);
}
.secondary:hover:not(:disabled) {
  border-color: var(--text);
}
.primary {
  border-color: transparent;
  background: var(--accent);
  color: var(--accent-text);
  font-weight: var(--fw-bold);
}
.primary:hover:not(:disabled) {
  background: var(--accent-hover);
}
.secondary:disabled,
.primary:disabled {
  opacity: 0.5;
  cursor: default;
}
.link {
  padding: 0;
  border: none;
  background: none;
  color: var(--text);
  font: inherit;
  text-decoration: underline;
  cursor: pointer;
}
.label .link {
  font-weight: var(--fw-bold);
}
.keys {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.6em;
  padding: 0.2em 0 0.7em;
}
.keys input {
  flex: 1 1 200px;
  min-width: 0;
  padding: 0.5em 0.9em;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: var(--control-bg);
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  outline: none;
}
.keys input:focus {
  border-color: var(--text);
}
</style>
