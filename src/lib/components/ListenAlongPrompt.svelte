<script>
  import { fly } from "svelte/transition";
  import { jam, inviteListener, dismissPrompt, removeListener } from "../jam.svelte.js";

  // Someone came in from your Discord status: they're already listening.
  const prompt = $derived(jam.prompts[0] ?? null);
</script>

{#if prompt}
  {@const { peer, name } = prompt}
  <div class="prompt" role="alertdialog" aria-label={`${name} is listening along`} transition:fly={{ y: 12, duration: 160 }}>
    <div class="head">
      <span class="avatar" aria-hidden="true">{name.charAt(0).toUpperCase()}</span>
      <div class="text">
        <div class="title">{name} is listening along</div>
        <div class="sub">Make it a Jam so they can add songs too?</div>
      </div>
    </div>
    <div class="buttons">
      <button type="button" class="primary" onclick={() => inviteListener(peer)}>Make it a Jam</button>
      <button type="button" class="secondary" onclick={() => dismissPrompt(peer)}>Just listen</button>
      <button type="button" class="secondary danger" onclick={() => removeListener(peer)}>Remove</button>
    </div>
    {#if jam.prompts.length > 1}
      <div class="more">and {jam.prompts.length - 1} more listening along</div>
    {/if}
  </div>
{/if}

<style>
.prompt {
  position: fixed;
  right: 1em;
  bottom: 6.5em;
  z-index: 60;
  width: min(360px, calc(100vw - 2em));
  padding: 1em;
  border-radius: var(--radius-md);
  background: var(--surface-hover);
  box-shadow: 0 16px 24px rgba(0, 0, 0, 0.3), 0 6px 8px rgba(0, 0, 0, 0.2);
}
.head {
  display: flex;
  align-items: center;
  gap: 0.8em;
}
.avatar {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  flex-shrink: 0;
  border-radius: 50%;
  background: var(--accent-bg-strong);
  color: var(--accent);
  font-weight: var(--fw-bold);
}
.text {
  min-width: 0;
}
.title {
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.sub {
  margin-top: 0.2em;
  font-size: var(--fs-xs);
  color: var(--text-dim);
}
.buttons {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5em;
  margin-top: 0.9em;
}
.primary,
.secondary {
  padding: 0.45em 1em;
  border-radius: var(--radius-pill);
  font-family: inherit;
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  cursor: pointer;
  transition: border-color var(--transition), background-color var(--transition), transform var(--transition);
}
.primary {
  border: none;
  background: var(--accent);
  color: var(--accent-text);
}
.primary:hover {
  background: var(--accent-hover);
  transform: scale(1.03);
}
.secondary {
  border: 1px solid var(--border-strong);
  background: none;
  color: var(--text);
}
.secondary:hover {
  border-color: var(--text);
}
.secondary.danger {
  color: var(--danger);
}
.secondary.danger:hover {
  border-color: var(--danger);
}
.more {
  margin-top: 0.7em;
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
</style>
