<script>
  import { fade } from "svelte/transition";
  import * as api from "../api.js";
  import { connect } from "../connect.svelte.js";
  import { playOn } from "../player.svelte.js";
  import Icon from "./Icon.svelte";

  /** @typedef {import("../api.js").ConnectDevice} ConnectDevice */

  let open = $state(false);
  let loading = $state(false);
  let error = $state("");
  /** @type {ConnectDevice[]} */
  let devices = $state([]);
  /** @type {HTMLDivElement|undefined} */
  let wrap = $state();
  /** @type {HTMLDivElement|undefined} */
  let menu = $state();
  /** How far the menu moves sideways to stay inside the window. */
  let shift = $state(0);
  const EDGE_MARGIN = 8;

  $effect(() => {
    const element = menu;
    if (!element) return;
    shift = 0;
    const frame = requestAnimationFrame(() => {
      const rect = element.getBoundingClientRect();
      if (rect.right > innerWidth - EDGE_MARGIN) shift = innerWidth - EDGE_MARGIN - rect.right;
      else if (rect.left < EDGE_MARGIN) shift = EDGE_MARGIN - rect.left;
    });
    return () => cancelAnimationFrame(frame);
  });

  // SpotiLarp's own Connect device is "This computer".
  const others = $derived(devices.filter((device) => !device.here));

  async function load() {
    loading = true;
    error = "";
    try {
      devices = await api.connectDevices();
    } catch (e) {
      error = `${e}`;
    } finally {
      loading = false;
    }
  }

  function toggle() {
    open = !open;
    if (open) load();
  }

  /** @param {ConnectDevice|null} device */
  function choose(device) {
    open = false;
    playOn(device);
  }

  /** @param {string} kind Spotify's device type, lowercased */
  function iconFor(kind) {
    if (kind === "smartphone" || kind === "tablet") return "smartphone";
    if (["tv", "castvideo", "stb", "gameconsole"].includes(kind)) return "tv";
    if (kind === "computer" || kind === "chromebook") return "computer";
    return "speaker";
  }

  /** @param {ConnectDevice} device */
  function describe(device) {
    if (connect.deviceId === device.id) return "Playing SpotiLarp";
    if (device.restricted) return "Can't be controlled from other apps";
    return device.active ? "Playing in Spotify" : "Spotify Connect";
  }

  /** @param {MouseEvent} evt */
  function onWindowClick(evt) {
    if (open && !wrap?.contains(/** @type {Node} */ (evt.target))) open = false;
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={(e) => e.key === "Escape" && (open = false)} onresize={() => (open = false)} />

<div class="device-wrap" bind:this={wrap}>
  <button
    type="button"
    class="picker-button"
    class:active={connect.output !== "here"}
    onclick={toggle}
    aria-label="Connect to a device"
    title={connect.output === "remote" ? `Playing on ${connect.device}` : "Connect to a device"}
    aria-haspopup="menu"
    aria-expanded={open}
  >
    <Icon name="devices" size={17} />
  </button>
  {#if open}
    <div class="menu" role="menu" bind:this={menu} style:translate={`${shift}px 0`} transition:fade={{ duration: 100 }}>
      <div class="heading">Connect to a device</div>
      <button type="button" role="menuitem" class="device" class:current={connect.output !== "remote"} onclick={() => choose(null)}>
        <Icon name="computer" size={22} />
        <span class="label">
          <span class="name">This computer</span>
          <span class="detail">{connect.output === "receiving" ? "Another Spotify app is playing here" : "SpotiLarp"}</span>
        </span>
      </button>
      {#each others as device (device.id)}
        <button
          type="button"
          role="menuitem"
          class="device"
          class:current={connect.deviceId === device.id}
          disabled={device.restricted}
          onclick={() => choose(device)}
        >
          <Icon name={iconFor(device.kind)} size={22} />
          <span class="label">
            <span class="name">{device.name}</span>
            <span class="detail">{describe(device)}</span>
          </span>
        </button>
      {/each}
      {#if error}
        <p class="note">{error}</p>
      {:else if loading && others.length === 0}
        <p class="note">Looking for devices...</p>
      {:else if others.length === 0}
        <p class="note">No other devices yet. Open Spotify on your phone, a speaker or another computer, signed in to the same account.</p>
      {/if}
    </div>
  {/if}
</div>

<style>
.device-wrap {
  position: relative;
  flex-shrink: 0;
}
/* As the bar's other buttons (NowPlaying.svelte). */
.picker-button {
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  color: var(--text-dim);
  cursor: pointer;
  padding: 0.3em;
  border-radius: 50%;
  transition: color var(--transition), transform var(--spring);
}
.picker-button:hover {
  color: var(--text);
  transform: scale(1.12);
}
.picker-button:active {
  transform: scale(0.9);
  transition-duration: 100ms;
}
.picker-button.active {
  color: var(--accent);
}
/* Above the bar, like MoreMenu's below a page's buttons, ending at the
   button (and moved in if that would leave the window). */
.menu {
  position: absolute;
  bottom: calc(100% + 14px);
  right: 0;
  z-index: 60;
  width: 300px;
  padding: 4px;
  border-radius: var(--radius-sm);
  background: var(--surface-hover);
  box-shadow: 0 16px 24px rgba(0, 0, 0, 0.3), 0 6px 8px rgba(0, 0, 0, 0.2);
}
.heading {
  padding: 0.8em 0.8em 0.5em;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  color: var(--text);
}
.device {
  display: flex;
  align-items: center;
  gap: 0.8em;
  width: 100%;
  padding: 0.6em 0.8em;
  border: none;
  border-radius: 2px;
  background: none;
  color: var(--text);
  font-family: inherit;
  text-align: left;
  cursor: pointer;
}
.device:hover:not(:disabled) {
  background: var(--highlight-strong);
}
.device:disabled {
  color: var(--text-muted);
  cursor: default;
}
.device.current {
  color: var(--accent);
}
.label {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.name {
  font-size: var(--fs-sm);
  font-weight: var(--fw-semibold);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.detail {
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
.device.current .detail {
  color: inherit;
}
.note {
  margin: 0;
  padding: 0.6em 0.8em 0.8em;
  font-size: var(--fs-xs);
  color: var(--text-muted);
  line-height: 1.5;
}
</style>
