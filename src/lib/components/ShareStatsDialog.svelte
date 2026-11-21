<script>
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { notify } from "../toast.svelte.js";
  import { cardColor, drawCard, saveCardImage, copyCardImage, shareLink } from "../shareCard.js";
  import Icon from "./Icon.svelte";
  import Toggle from "./Toggle.svelte";

  /**
   * Shares a stats card: its picture saved or copied, or a link to a page
   * that shows it (the stats ride in the link, nothing is uploaded).
   * @type {{ card: import("../shareCard.js").ShareCard, onClose: () => void }}
   */
  let { card, onClose } = $props();

  /** @type {HTMLDivElement|undefined} */
  let dialog = $state();
  let showName = $state(true);
  let color = $state("");
  /** @type {Blob|null} */
  let png = $state(null);
  let preview = $state("");
  let error = $state("");
  /** Discord's app, on the chats you were in last. */
  const DISCORD_APP = "discord://-/channels/@me";

  /** @type {""|"discord"|"save"|"copy"|"link"} */
  let busy = $state("");

  const shown = $derived({ ...card, name: showName ? card.name : "", color });
  const label = $derived(card.kind === "recap" ? `Share your ${card.title} Recap` : "Share your stats");

  onMount(() => {
    dialog?.focus();
    cardColor(card).then((found) => (color = found));
  });

  // Drawn again whenever what it shows changes.
  $effect(() => {
    if (!color) return;
    const current = shown;
    let cancelled = false;
    png = null;
    drawCard(current)
      .then((blob) => {
        if (cancelled) return;
        png = blob;
        preview = URL.createObjectURL(blob);
        error = "";
      })
      .catch((e) => {
        if (!cancelled) error = `Couldn't make the picture: ${e}`;
      });
    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    const url = preview;
    return () => {
      if (url) URL.revokeObjectURL(url);
    };
  });

  /**
   * @param {"discord"|"save"|"copy"|"link"} action
   * @param {() => Promise<void>} run
   */
  async function act(action, run) {
    if (busy) return;
    busy = action;
    try {
      await run();
    } finally {
      busy = "";
    }
  }

  // Discord shows a pasted picture right in the chat, so the picture goes
  // on the clipboard and Discord comes up to paste it into.
  function shareOnDiscord() {
    return act("discord", async () => {
      if (!png) return;
      try {
        await copyCardImage(png);
      } catch (e) {
        notify(`Couldn't copy the picture, save it instead: ${e}`);
        return;
      }
      notify("Picture copied. Paste it into any Discord chat with Ctrl+V.");
      openUrl(DISCORD_APP).catch(() => {});
    });
  }

  function save() {
    return act("save", async () => {
      if (!png) return;
      try {
        const where = await saveCardImage(shown, png);
        if (where) notify("Picture saved");
      } catch (e) {
        notify(`${e}`);
      }
    });
  }

  function copyImage() {
    return act("copy", async () => {
      if (!png) return;
      try {
        await copyCardImage(png);
        notify("Picture copied");
      } catch (e) {
        notify(`Couldn't copy the picture, save it instead: ${e}`);
      }
    });
  }

  function copyLink() {
    return act("link", async () => {
      try {
        await navigator.clipboard.writeText(await shareLink(shown));
        notify("Link copied");
      } catch (e) {
        notify(`Couldn't copy the link: ${e}`);
      }
    });
  }

  /** @param {KeyboardEvent} evt */
  function onKeydown(evt) {
    // Ahead of the app's shortcuts and the Recap's, which wait until this closes.
    evt.stopPropagation();
    if (evt.key === "Escape") {
      evt.preventDefault();
      onClose();
    }
  }
</script>

<svelte:window onkeydowncapture={onKeydown} />

<div class="backdrop" role="presentation" transition:fade={{ duration: 120 }} onclick={(e) => e.target === e.currentTarget && onClose()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="share-title" tabindex="-1" bind:this={dialog}>
    <div class="preview" style:background={color || "var(--surface-raised)"}>
      {#if png && preview}
        <img src={preview} alt="Your stats card: {shown.title}" />
      {:else if error}
        <p class="error">{error}</p>
      {:else}
        <p class="making">Making the picture...</p>
      {/if}
    </div>

    <div class="side">
      <div class="head">
        <h2 id="share-title">{label}</h2>
        <button type="button" class="close" onclick={onClose} aria-label="Close">
          <Icon name="close" size={16} />
        </button>
      </div>
      <p class="lede">Post the picture anywhere, or send a link that opens the card in any browser.</p>

      {#if card.name}
        <div class="option">
          <span>Show my name</span>
          <Toggle on={showName} onclick={() => (showName = !showName)} label="Show my name" />
        </div>
      {/if}

      <div class="actions">
        <button type="button" class="pill primary" onclick={shareOnDiscord} disabled={!png || busy !== ""}>
          <Icon name="discord" size={16} />
          {busy === "discord" ? "Copying..." : "Share on Discord"}
        </button>
        <button type="button" class="pill" onclick={copyImage} disabled={!png || busy !== ""}>
          {busy === "copy" ? "Copying..." : "Copy picture"}
        </button>
        <button type="button" class="pill" onclick={save} disabled={!png || busy !== ""}>
          {busy === "save" ? "Saving..." : "Save picture"}
        </button>
        <button type="button" class="pill" onclick={copyLink} disabled={!color || busy !== ""}>
          {busy === "link" ? "Copying..." : "Copy link"}
        </button>
      </div>

      <p class="note">
        The link holds the stats on the card itself, so they aren't uploaded anywhere. Anyone you send it to can see
        them.
      </p>
    </div>
  </div>
</div>

<style>
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 90;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.6);
}
.dialog {
  display: flex;
  gap: 1.5em;
  width: min(680px, calc(100vw - 2em));
  max-height: calc(100vh - 4em);
  overflow-y: auto;
  padding: 1.5em;
  border-radius: var(--radius-md);
  background: var(--surface-hover);
  box-shadow: 0 16px 24px rgba(0, 0, 0, 0.3), 0 6px 8px rgba(0, 0, 0, 0.2);
  outline: none;
}
.preview {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 252px;
  aspect-ratio: 9 / 16;
  border-radius: var(--radius-md);
  overflow: hidden;
  transition: background-color 300ms ease;
}
.preview img {
  display: block;
  width: 100%;
  height: 100%;
}
.making,
.error {
  margin: 0;
  padding: 1em;
  font-size: var(--fs-sm);
  text-align: center;
}
.making {
  color: rgba(255, 255, 255, 0.8);
}
.error {
  color: var(--danger);
}
.side {
  display: flex;
  flex-direction: column;
  min-width: 0;
  flex: 1;
}
.head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1em;
}
h2 {
  margin: 0;
  font-size: var(--fs-lg);
  font-weight: var(--fw-bold);
}
.close {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 50%;
  background: none;
  color: var(--text-muted);
  cursor: pointer;
}
.close:hover {
  background: var(--highlight-strong);
  color: var(--text);
}
.lede {
  margin: 0.5em 0 1.4em;
  font-size: var(--fs-sm);
  line-height: 1.5;
  color: var(--text-dim);
}
.option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1em;
  margin-bottom: 1.4em;
  font-size: var(--fs-sm);
}
.actions {
  display: flex;
  flex-direction: column;
  gap: 0.6em;
}
.pill {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5em;
  padding: 0.65em 1.3em;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-pill);
  background: none;
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
  transition: border-color var(--transition), background-color var(--transition);
}
.pill:hover:not(:disabled) {
  border-color: var(--text);
}
.pill.primary {
  border-color: transparent;
  background: var(--accent);
  color: var(--accent-text);
}
.pill.primary:hover:not(:disabled) {
  background: var(--accent-hover);
}
.pill:disabled {
  opacity: 0.5;
  cursor: default;
}
.note {
  margin: auto 0 0;
  padding-top: 1.4em;
  font-size: var(--fs-xs);
  line-height: 1.5;
  color: var(--text-muted);
}
</style>
