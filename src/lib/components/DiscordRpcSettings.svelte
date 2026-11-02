<script>
  import * as api from "../api.js";
  import { setDiscordSharing } from "../jam.svelte.js";
  import { discordAccount, connectDiscord, cancelConnectDiscord, disconnectDiscord } from "../discordAccount.svelte.js";
  import Icon from "./Icon.svelte";

  /** @type {boolean|null} */
  let enabled = $state(null);
  let open = $state(false);
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

  async function enable() {
    if (saving) return;
    saving = true;
    error = "";
    try {
      await api.enableDiscordPresence();
      enabled = true;
      setDiscordSharing(true);
    } catch (e) {
      error = `${e}`;
    } finally {
      saving = false;
    }
  }

  async function disable() {
    saving = true;
    error = "";
    try {
      await api.clearDiscordClientId();
      enabled = false;
      setDiscordSharing(false);
    } catch (e) {
      error = `${e}`;
    } finally {
      saving = false;
    }
  }

  /** @param {KeyboardEvent} evt */
  function onKeydown(evt) {
    if (evt.key === "Escape") open = false;
  }

  /** @param {MouseEvent} evt */
  function onWindowClick(evt) {
    if (open && root && !root.contains(/** @type {Node} */ (evt.target))) open = false;
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={(e) => open && onKeydown(e)} />

<div class="discord-settings" bind:this={root} data-tauri-drag-region="false">
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
      <div class="account">
        {#if discordAccount.profile}
          {@const profile = discordAccount.profile}
          <img src={profile.avatar_url} alt="" class="avatar" />
          <div class="who">
            <div class="name">{profile.name}</div>
            <div class="handle">@{profile.username}</div>
          </div>
          <button type="button" class="text-button" onclick={disconnectDiscord}>Disconnect</button>
        {:else if discordAccount.connecting}
          <span class="waiting">Finish connecting in your browser...</span>
          <button type="button" class="text-button" onclick={cancelConnectDiscord}>Cancel</button>
        {:else}
          <button type="button" class="connect-button" onclick={connectDiscord}>
            <Icon name="discord" size={16} />
            Connect Discord
          </button>
        {/if}
      </div>
      {#if !discordAccount.profile && !discordAccount.connecting}
        <p class="hint">Links your Discord account to SpotiLarp, for the Discord features on the way.</p>
      {/if}
      <div class="divider"></div>
      {#if enabled}
        <p class="intro">
          Discord Rich Presence is on, your listening activity shows on your profile. Friends get a
          <strong>Listen along</strong> button to hear the same songs in their SpotiLarp.
        </p>
        <p class="intro">Discord only shows the buttons to other people, not on your own profile.</p>
        {#if error}
          <p class="error">{error}</p>
        {/if}
        <button type="button" onclick={disable} disabled={saving} class="save-button danger">
          {saving ? "Disabling..." : "Disable"}
        </button>
      {:else}
        <p class="intro">
          Show what you're listening to on your Discord profile, with a <strong>Listen along</strong>
          button so friends can hear the same songs in their SpotiLarp. Needs the Discord app running.
        </p>
        {#if error}
          <p class="error">{error}</p>
        {/if}
        <button type="button" onclick={enable} disabled={saving} class="save-button">
          {saving ? "Turning on..." : "Show on Discord"}
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
.account {
  display: flex;
  align-items: center;
  gap: 0.7em;
  min-height: 36px;
}
.avatar {
  width: 36px;
  height: 36px;
  flex-shrink: 0;
  border-radius: 50%;
  object-fit: cover;
  background: var(--control-bg);
}
.who {
  flex: 1;
  min-width: 0;
}
.name,
.handle {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.name {
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  color: var(--text);
}
.handle {
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
.waiting {
  flex: 1;
  font-size: var(--fs-sm);
  color: var(--text-dim);
}
.connect-button {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5em;
  width: 100%;
  padding: 0.55em 1em;
  border: none;
  border-radius: var(--radius-pill);
  background: #5865f2;
  color: #fff;
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
  transition: background-color var(--transition), transform 100ms ease;
}
.connect-button:hover {
  background: #4752c4;
  transform: scale(1.02);
}
.hint {
  margin: -0.3em 0 0;
  font-size: var(--fs-xs);
  color: var(--text-muted);
  line-height: 1.5;
}
.text-button {
  flex-shrink: 0;
  padding: 0;
  border: none;
  background: none;
  color: var(--text-muted);
  font-family: inherit;
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.text-button:hover {
  color: var(--text);
  text-decoration: underline;
}
.divider {
  height: 1px;
  background: var(--border);
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
