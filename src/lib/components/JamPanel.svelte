<script>
  import { untrack } from "svelte";
  import Icon from "./Icon.svelte";
  import Toggle from "./Toggle.svelte";
  import { notify } from "../toast.svelte.js";
  import { openTrackMenu } from "../contextMenu.svelte.js";
  import { smallestCover } from "../utils.js";
  import {
    jam,
    savedName,
    startJam,
    joinJam,
    leaveJam,
    saveJam,
    acceptRequest,
    declineRequest,
    removeListener,
    inviteListener,
    invited,
    setPermission,
  } from "../jam.svelte.js";

  /** @typedef {import("../types.js").Track} Track */

  /** @type {{ displayName?: string, onClose: () => void, onShowQueue: () => void }} */
  let { displayName = "", onClose, onShowQueue } = $props();

  let name = $state(savedName());
  let code = $state("");
  let confirmingEnd = $state(false);

  const listeners = $derived(jam.members.filter((m) => !m.host));
  // Hosting only for the Discord button, with nobody listening yet: looks like no Jam.
  const idle = $derived(!jam.role || (jam.role === "host" && !jam.started && listeners.length === 0));
  const title = $derived.by(() => {
    const host = jam.hostName || "Someone";
    if (jam.role === "guest") return invited() ? `${host}'s Jam` : `Listening with ${host}`;
    if (idle) return "Jam";
    return jam.started ? "Your Jam" : "Listening with you";
  });

  // Your Spotify name until you pick one, filled in once the login has said who you are.
  $effect(() => {
    const fallback = displayName;
    untrack(() => {
      if (!name) name = fallback;
    });
  });

  /**
   * @param {string} text
   * @param {string} what
   */
  async function copy(text, what) {
    try {
      await navigator.clipboard.writeText(text);
      notify(`${what} copied`);
    } catch (e) {
      notify(`Couldn't copy the ${what.toLowerCase()}: ${e}`);
    }
  }

  /** @param {SubmitEvent} evt */
  function onJoin(evt) {
    evt.preventDefault();
    joinJam(code, name);
  }

  function end() {
    if (jam.role === "host" && listeners.length > 0 && !confirmingEnd) {
      confirmingEnd = true;
      return;
    }
    confirmingEnd = false;
    leaveJam();
  }
</script>

{#snippet trackRow(/** @type {Track} */ track, /** @type {string} */ note)}
  {#if smallestCover(track.album.images)}
    <img src={smallestCover(track.album.images)} alt="" class="thumb" loading="lazy" />
  {:else}
    <div class="thumb placeholder"></div>
  {/if}
  <div class="meta">
    <div class="name">{track.name}</div>
    <div class="sub">{note || track.artists.map((a) => a.name).join(", ")}</div>
  </div>
{/snippet}

<div class="panel" role="region" aria-label="Jam">
  <div class="panel-header">
    <h2>{title}</h2>
    <button type="button" class="icon-button" onclick={onClose} aria-label="Close Jam">
      <Icon name="close" size={16} />
    </button>
  </div>

  {#if idle}
    <p class="hint">
      Listen together with friends, wherever they are. Everyone hears the same songs on their own SpotiLarp, so each
      person needs Spotify Premium. Apps connect straight to each other, through a public relay when a router won't allow it.
    </p>
    {#if jam.role === "host" && jam.link}
      <p class="hint">
        Your Discord status has a <strong>Listen along</strong> button, so friends can tune in from there (they can't add
        songs until you invite them).
        <button type="button" class="text-button" onclick={() => copy(jam.link, "Listen-along link")}>Copy the link</button>
      </p>
    {/if}

    <label class="field">
      <span class="label">Your name in the Jam</span>
      <input type="text" class="input" bind:value={name} maxlength="32" spellcheck="false" placeholder="Your name" />
    </label>

    <section>
      <button type="button" class="primary" onclick={() => startJam(name)} disabled={jam.busy}>
        {jam.busy ? "Starting..." : "Start a Jam"}
      </button>
    </section>

    <form class="join" onsubmit={onJoin}>
      <span class="label">Or join one with its code</span>
      <div class="code-row">
        <input type="text" class="input" bind:value={code} spellcheck="false" placeholder="Paste a Jam code" aria-label="Jam code" />
        <button type="submit" class="secondary" disabled={jam.busy || !code.trim()}>
          {jam.busy ? "Joining..." : "Join"}
        </button>
      </div>
    </form>

    {#if jam.played.length > 0}
      <section class="actions">
        <button type="button" class="secondary" onclick={saveJam} disabled={jam.saving}>
          {jam.saving ? "Saving..." : `Save the last Jam as a playlist (${jam.played.length})`}
        </button>
      </section>
    {/if}
  {:else}
    {#if jam.role === "host" || invited()}
      <section>
        <h3>{jam.role === "host" ? "Invite people" : "Invite others"}</h3>
        <div class="code-row">
          <code class="code" title={jam.code}>{jam.code}</code>
          <button type="button" class="secondary small" onclick={() => copy(jam.code, "Jam code")}>Copy</button>
        </div>
        <p class="hint">
          Send them this code. They paste it in their SpotiLarp under Jam, and can add songs.
          {#if jam.role === "host" && jam.link}
            To let someone only listen,
            <button type="button" class="text-button" onclick={() => copy(jam.link, "Listen-along link")}>copy the listen-along link</button>
            instead.
          {/if}
        </p>
      </section>
    {/if}

    {#if jam.role === "guest"}
      <section>
        <div class="section-head">
          <h3>Now playing</h3>
          <button type="button" class="text-button" onclick={onShowQueue}>See what's next</button>
        </div>
        {#if jam.nowPlaying}
          {@const track = jam.nowPlaying}
          <div class="row" role="presentation" oncontextmenu={(e) => openTrackMenu(e, track)}>
            {@render trackRow(track, "")}
          </div>
        {:else}
          <p class="hint">Nothing's playing in the Jam right now.</p>
        {/if}
        <p class="hint">
          {#if !invited()}
            You're listening along. {jam.hostName || "The host"} can invite you into the Jam to add songs.
            Pausing only pauses it for you.
          {:else}
            {#if jam.permissions.queue}
              Play or queue a song anywhere to add it to the Jam{jam.permissions.approval ? " (the host accepts it first)" : ""}.
            {:else}
              The host has turned off adding songs.
            {/if}
            {jam.permissions.control ? "You can pause and skip for everyone." : "Pausing only pauses it for you."}
          {/if}
        </p>
      </section>
    {/if}

    {#if jam.role === "host" && jam.requests.length > 0}
      <section>
        <h3>Waiting for you ({jam.requests.length})</h3>
        {#each jam.requests as request (request.id)}
          <div class="request">
            {#if request.tracks.length === 1}
              {@const track = request.tracks[0]}
              <div class="row" role="presentation" oncontextmenu={(e) => openTrackMenu(e, track)}>
                {@render trackRow(track, `${request.name} wants to add this`)}
              </div>
            {:else}
              <div class="row">
                {@render trackRow(request.tracks[0], `${request.name} wants to add ${request.tracks.length} songs`)}
              </div>
            {/if}
            <button type="button" class="icon-button accept" onclick={() => acceptRequest(request.id)} aria-label={`Add what ${request.name} asked for`} title="Add to queue">
              <Icon name="added" size={18} />
            </button>
            <button type="button" class="icon-button" onclick={() => declineRequest(request.id)} aria-label={`Turn down what ${request.name} asked for`} title="Turn down">
              <Icon name="close" size={14} />
            </button>
          </div>
        {/each}
      </section>
    {/if}

    <section>
      <h3>Listening ({jam.members.length})</h3>
      {#each jam.members as member (member.id)}
        <div class="member">
          <span class="avatar" aria-hidden="true">{member.name.charAt(0).toUpperCase()}</span>
          <span class="member-name">{member.name}{member.id === jam.myId ? " (you)" : ""}</span>
          {#if member.host}
            <span class="badge">Host</span>
          {:else if !member.canAdd && jam.role === "host"}
            <button type="button" class="text-button" onclick={() => inviteListener(member.id)} title="Let them add songs">Invite to Jam</button>
          {:else if !member.canAdd}
            <span class="badge quiet">Listening</span>
          {/if}
          {#if !member.host && jam.role === "host"}
            <button type="button" class="icon-button remove" onclick={() => removeListener(member.id)} aria-label={`Remove ${member.name}`} title="Remove">
              <Icon name="close" size={14} />
            </button>
          {/if}
        </div>
      {/each}
      {#if jam.role === "host" && listeners.length === 0}
        <p class="hint">Nobody's joined yet.</p>
      {/if}
    </section>

    {#if jam.role === "host" && jam.started}
      <section>
        <h3>What people in the Jam can do</h3>
        <div class="setting">
          <span class="label">Add songs to the queue</span>
          <Toggle on={jam.permissions.queue} onclick={() => setPermission("queue", !jam.permissions.queue)} label="Listeners can add songs" />
        </div>
        <div class="setting">
          <span class="label">Ask you before songs are added</span>
          <Toggle
            on={jam.permissions.approval}
            onclick={() => setPermission("approval", !jam.permissions.approval)}
            label="Accept songs before they're added"
            disabled={!jam.permissions.queue}
          />
        </div>
        <div class="setting">
          <span class="label">Pause, skip and seek for everyone</span>
          <Toggle on={jam.permissions.control} onclick={() => setPermission("control", !jam.permissions.control)} label="Listeners can control playback" />
        </div>
      </section>
    {/if}

    <section class="actions">
      <button type="button" class="secondary" onclick={saveJam} disabled={jam.saving || jam.played.length === 0}>
        {jam.saving ? "Saving..." : `Save as playlist${jam.played.length ? ` (${jam.played.length})` : ""}`}
      </button>
      <button type="button" class="secondary danger" onclick={end}>
        {#if jam.role === "guest"}
          {invited() ? "Leave Jam" : "Stop listening"}
        {:else if confirmingEnd}
          {jam.started ? "End it for everyone?" : "Stop everyone listening?"}
        {:else}
          {jam.started ? "End Jam" : "Stop sharing"}
        {/if}
      </button>
    </section>
  {/if}
</div>

<style>
.panel {
  display: flex;
  flex-direction: column;
  gap: 1.1em;
  width: 100%;
  height: 100%;
  padding: 1em;
  overflow-x: hidden;
  overflow-y: auto;
}
.panel-header,
.section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5em;
}
.section-head {
  margin-bottom: 0.5em;
}
h2 {
  margin: 0;
  font-size: var(--fs-md);
  font-weight: var(--fw-bold);
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
h3 {
  margin: 0 0 0.5em;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  color: var(--text);
}
.section-head h3 {
  margin: 0;
}
.hint {
  margin: 0.4em 0 0;
  font-size: var(--fs-xs);
  line-height: 1.5;
  color: var(--text-muted);
}
.label {
  font-size: var(--fs-sm);
  color: var(--text);
}
.field,
.join {
  display: flex;
  flex-direction: column;
  gap: 0.5em;
}
.input {
  flex: 1;
  min-width: 0;
  padding: 0.55em 1em;
  border: 1px solid transparent;
  border-radius: var(--radius-pill);
  background: var(--control-bg);
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  outline: none;
}
.input:focus {
  border-color: var(--text);
}
.code-row {
  display: flex;
  align-items: center;
  gap: 0.5em;
}
.code {
  flex: 1;
  min-width: 0;
  padding: 0.55em 0.8em;
  border-radius: var(--radius-sm);
  background: var(--surface-raised);
  color: var(--text);
  font-size: var(--fs-xs);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  user-select: all;
}
.primary,
.secondary {
  padding: 0.55em 1.2em;
  border-radius: var(--radius-pill);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
  transition: border-color var(--transition), background-color var(--transition), transform var(--transition);
}
.primary {
  width: 100%;
  border: none;
  background: var(--accent);
  color: var(--accent-text);
}
.primary:hover:not(:disabled) {
  background: var(--accent-hover);
  transform: scale(1.02);
}
.secondary {
  flex-shrink: 0;
  border: 1px solid var(--border-strong);
  background: none;
  color: var(--text);
}
.secondary:hover:not(:disabled) {
  border-color: var(--text);
}
.secondary.small {
  padding: 0.4em 0.9em;
  font-size: var(--fs-xs);
}
.secondary.danger {
  color: var(--danger);
}
.secondary.danger:hover:not(:disabled) {
  border-color: var(--danger);
}
.primary:disabled,
.secondary:disabled {
  opacity: 0.5;
  cursor: default;
}
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.6em;
}
.row {
  display: flex;
  align-items: center;
  gap: 0.7em;
  flex: 1;
  min-width: 0;
  padding: 0.4em 0.5em;
  border-radius: var(--radius-md);
}
.request {
  display: flex;
  align-items: center;
  gap: 0.2em;
  border-radius: var(--radius-md);
}
.request:hover {
  background-color: var(--surface-hover);
}
.thumb {
  width: 40px;
  height: 40px;
  border-radius: var(--radius-sm);
  object-fit: cover;
  background: var(--surface-raised);
  flex-shrink: 0;
}
.thumb.placeholder {
  background: linear-gradient(135deg, var(--surface-raised), var(--surface-hover));
}
.meta {
  min-width: 0;
}
.name {
  font-size: var(--fs-sm);
  font-weight: var(--fw-medium);
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.sub {
  margin-top: 0.15em;
  font-size: var(--fs-xs);
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.member {
  display: flex;
  align-items: center;
  gap: 0.7em;
  min-height: 36px;
  padding: 0.2em 0.5em;
  border-radius: var(--radius-md);
}
.member:hover {
  background-color: var(--surface-hover);
}
.avatar {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  border-radius: 50%;
  background: var(--highlight-strong);
  color: var(--text);
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
}
.member-name {
  flex: 1;
  min-width: 0;
  font-size: var(--fs-sm);
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.badge {
  padding: 0.15em 0.6em;
  border-radius: var(--radius-pill);
  background: var(--accent-bg);
  color: var(--accent);
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
}
.badge.quiet {
  background: var(--surface-raised);
  color: var(--text-muted);
}
.setting {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1em;
  padding: 0.45em 0;
}
.icon-button {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  flex-shrink: 0;
  border: none;
  border-radius: 50%;
  background: none;
  color: var(--text-muted);
  cursor: pointer;
  transition: background-color var(--transition), color var(--transition);
}
.icon-button:hover {
  background-color: var(--surface-hover);
  color: var(--text);
}
.accept {
  color: var(--accent);
}
.accept:hover {
  color: var(--accent-hover);
}
.remove {
  opacity: 0;
}
.member:hover .remove,
.remove:focus-visible {
  opacity: 1;
}
.text-button {
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
</style>
