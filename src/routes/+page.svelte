<script>
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import * as api from "$lib/api.js";
  import { contextLabel, playFromList, next, toggleShuffle, cycleRepeatMode, resetQueue } from "$lib/queue.svelte.js";
  import { openTrackMenu } from "$lib/contextMenu.svelte.js";
  import {
    player,
    togglePlay,
    previousOrRestart,
    seekTo,
    setVolume,
    toggleMute,
    resetPlayer,
  } from "$lib/player.svelte.js";
  import { notify } from "$lib/toast.svelte.js";
  import { nav, navigate, goBack, goForward, resetNav } from "$lib/nav.svelte.js";
  import { loadLibrary, clearLibrary } from "$lib/library.svelte.js";
  import { likedIds } from "$lib/likes.svelte.js";
  import PlaylistSidebar from "$lib/components/PlaylistSidebar.svelte";
  import MainContent from "$lib/components/MainContent.svelte";
  import { clearPlaylistCache } from "$lib/components/views/PlaylistView.svelte";
  import { clearStatsCache } from "$lib/components/views/StatsView.svelte";
  import NowPlaying from "$lib/components/NowPlaying.svelte";
  import NowPlayingPanel from "$lib/components/NowPlayingPanel.svelte";
  import QueuePanel from "$lib/components/QueuePanel.svelte";
  import TrackContextMenu from "$lib/components/TrackContextMenu.svelte";
  import MiniPlayer from "$lib/components/MiniPlayer.svelte";
  import ClientIdSetup from "$lib/components/ClientIdSetup.svelte";
  import DiscordRpcSettings from "$lib/components/DiscordRpcSettings.svelte";
  import UpdateBanner from "$lib/components/UpdateBanner.svelte";
  import WindowControls from "$lib/components/WindowControls.svelte";
  import AccountMenu from "$lib/components/AccountMenu.svelte";
  import Toasts from "$lib/components/Toasts.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import { smallestCover } from "$lib/utils.js";

  /** @type {boolean|null} */
  let clientIdConfigured = $state(null);
  let changingClientId = $state(false);

  let loggedIn = $state(false);
  let restoring = $state(false);
  let restoreFailed = $state(false);
  let status = $state("Not logged in");
  let displayName = $state("");
  let loggingIn = $state(false);
  let loggingOut = $state(false);

  let playbackConnected = $state(false);
  let playbackStatus = $state("Not connected");
  let connecting = $state(false);
  let connectingInteractive = $state(false);

  let showPanel = $state(true);
  let panelView = $state("nowPlaying");
  let miniMode = $state(false);

  let searchQuery = $state("");
  /** @type {import("$lib/types.js").Track[]} */
  let searchResults = $state([]);
  let resultsFor = $state("");
  let searching = $state(false);
  let searchError = $state("");
  let showDropdown = $state(false);
  let highlighted = $state(-1);
  /** @type {HTMLDivElement|undefined} */
  let searchBox = $state();
  /** @type {HTMLInputElement|undefined} */
  let searchInput = $state();
  /** @type {ReturnType<typeof setTimeout>|undefined} */
  let debounceHandle;
  let searchToken = 0;

  const shownResults = $derived(searchResults.slice(0, 8));
  const searchPending = $derived(searching || resultsFor !== searchQuery.trim());

  function onSearchInput() {
    clearTimeout(debounceHandle);
    showDropdown = true;
    highlighted = -1;
    if (!searchQuery.trim()) {
      searchToken++;
      searchResults = [];
      searchError = "";
      resultsFor = "";
      searching = false;
      return;
    }
    debounceHandle = setTimeout(runSearch, 300);
  }

  async function runSearch() {
    const token = ++searchToken;
    const query = searchQuery.trim();
    searching = true;
    searchError = "";
    try {
      const results = await api.search(query);
      if (token === searchToken) searchResults = results;
    } catch (e) {
      if (token === searchToken) searchError = `Search failed: ${e}`;
    } finally {
      if (token === searchToken) {
        resultsFor = query;
        searching = false;
      }
    }
  }

  /** @param {number} index */
  function pickResult(index) {
    playFromList(shownResults, index, `Search: "${searchQuery.trim()}"`);
    clearTimeout(debounceHandle);
    searchToken++;
    showDropdown = false;
    searchQuery = "";
    searchResults = [];
    resultsFor = "";
    highlighted = -1;
  }

  function toggleQueue() {
    if (showPanel && panelView === "queue") {
      panelView = "nowPlaying";
    } else {
      panelView = "queue";
      showPanel = true;
    }
  }

  function onSearchFocus() {
    if (searchQuery.trim()) showDropdown = true;
  }

  /** @param {MouseEvent} evt */
  function onWindowClick(evt) {
    if (searchBox && !searchBox.contains(/** @type {Node} */ (evt.target))) {
      showDropdown = false;
    }
  }

  /** @param {KeyboardEvent} evt */
  function onSearchKeydown(evt) {
    const count = searchPending ? 0 : shownResults.length;
    if (evt.key === "Escape") {
      showDropdown = false;
      /** @type {HTMLInputElement} */ (evt.target).blur();
    } else if (evt.key === "ArrowDown" && count > 0) {
      evt.preventDefault();
      showDropdown = true;
      highlighted = (highlighted + 1) % count;
    } else if (evt.key === "ArrowUp" && count > 0) {
      evt.preventDefault();
      showDropdown = true;
      highlighted = highlighted <= 0 ? count - 1 : highlighted - 1;
    } else if (evt.key === "Enter" && highlighted >= 0 && count > 0) {
      evt.preventDefault();
      pickResult(highlighted);
    } else if (evt.key === "Enter" && searchQuery.trim()) {
      evt.preventDefault();
      openSearchPage();
    }
  }

  function openSearchPage() {
    const query = searchQuery.trim();
    if (!query) return;
    showDropdown = false;
    highlighted = -1;
    searchInput?.blur();
    navigate({ type: "search", query });
  }

  /** @param {boolean} enabled */
  async function setMiniMode(enabled) {
    try {
      await api.setMiniMode(enabled);
      miniMode = enabled;
    } catch (e) {
      notify(`Couldn't switch the mini player: ${e}`);
    }
  }

  const VOLUME_STEP = 10;
  const SEEK_STEP_MS = 5000;

  /** @param {EventTarget|null} target */
  function isTextField(target) {
    if (target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) return true;
    if (target instanceof HTMLElement && target.isContentEditable) return true;
    return target instanceof HTMLInputElement && !["range", "checkbox", "radio", "button"].includes(target.type);
  }

  /** @param {KeyboardEvent} evt */
  function onWindowKeydown(evt) {
    if (!clientIdConfigured) return;
    const typing = isTextField(evt.target);
    const ctrl = evt.ctrlKey || evt.metaKey;
    if (evt.altKey && (evt.key === "ArrowLeft" || evt.key === "ArrowRight")) {
      evt.preventDefault();
      if (evt.key === "ArrowLeft") goBack();
      else goForward();
    } else if (ctrl && !evt.shiftKey && !evt.altKey && (evt.key === "s" || evt.key === "r")) {
      evt.preventDefault();
      if (evt.key === "s") toggleShuffle();
      else cycleRepeatMode();
    } else if (!typing && ctrl && !evt.shiftKey && !evt.altKey && evt.key.startsWith("Arrow")) {
      evt.preventDefault();
      if (evt.key === "ArrowRight") next();
      else if (evt.key === "ArrowLeft") previousOrRestart();
      else if (evt.key === "ArrowUp") setVolume(Math.min(100, player.volume + VOLUME_STEP));
      else if (evt.key === "ArrowDown") setVolume(Math.max(0, player.volume - VOLUME_STEP));
    } else if (!typing && evt.shiftKey && !ctrl && (evt.key === "ArrowLeft" || evt.key === "ArrowRight")) {
      evt.preventDefault();
      if (player.track) {
        const step = evt.key === "ArrowRight" ? SEEK_STEP_MS : -SEEK_STEP_MS;
        seekTo(Math.min(Math.max(player.positionMs + step, 0), player.track.durationMs));
      }
    } else if (!typing && !ctrl && !evt.altKey && evt.key.toLowerCase() === "m") {
      toggleMute();
    } else if (((evt.ctrlKey || evt.metaKey) && evt.key.toLowerCase() === "f") || (!typing && evt.key === "/")) {
      evt.preventDefault();
      searchInput?.focus();
      searchInput?.select();
    } else if (!typing && evt.code === "Space" && !evt.ctrlKey && !evt.metaKey && !evt.altKey) {
      evt.preventDefault();
      if (!evt.repeat) togglePlay();
    }
  }

  /** @param {MouseEvent} evt */
  function onWindowMouseUp(evt) {
    if (evt.button === 3) goBack();
    else if (evt.button === 4) goForward();
  }

  onMount(() => {
    api.setLoginExpiredHandler(onLoginExpired);
    const stopListening = listen("playback-connection", (event) => {
      playbackConnected = /** @type {boolean} */ (event.payload);
      playbackStatus = playbackConnected ? "Playback connected" : "Not connected";
    });
    api.hasClientId().then((configured) => {
      clientIdConfigured = configured;
      if (configured) restoreSessionIfPossible();
    });
    return () => {
      stopListening.then((stop) => stop());
    };
  });

  function onClientIdSaved() {
    clientIdConfigured = true;
    changingClientId = false;
    restoreSessionIfPossible();
  }

  function changeClientId() {
    changingClientId = true;
    clientIdConfigured = false;
  }

  function cancelClientIdChange() {
    changingClientId = false;
    clientIdConfigured = true;
  }

  /** @param {string} name */
  async function onLoggedIn(name) {
    loggedIn = true;
    restoreFailed = false;
    displayName = name;
    status = `Logged in as ${name}`;
    loadLibrary();
    if (!playbackConnected && !connecting && (await api.hasConnectSession())) {
      connectPlayback(false);
    }
  }

  function onLoginExpired() {
    if (!loggedIn && !restoring) return;
    loggedIn = false;
    displayName = "";
    status = "Not logged in";
    clearLibrary();
    notify(api.LOGIN_EXPIRED);
  }

  async function restoreSessionIfPossible() {
    restoreFailed = false;
    try {
      if (!(await api.loginStatus())) return;
      restoring = true;
      status = "Restoring session...";
      await onLoggedIn(await api.restoreSession());
    } catch (e) {
      if (e === api.LOGIN_EXPIRED) return;
      restoreFailed = true;
      status = "Couldn't reach Spotify";
      notify(`Couldn't restore your session: ${e}`);
    } finally {
      restoring = false;
    }
  }

  async function login() {
    const previousStatus = status;
    loggingIn = true;
    restoreFailed = false;
    status = "Waiting for the browser...";
    try {
      await onLoggedIn(await api.login());
    } catch (e) {
      status = loggedIn ? previousStatus : "Not logged in";
      if (e !== api.LOGIN_CANCELLED) notify(`Login failed: ${e}`);
    } finally {
      loggingIn = false;
    }
  }

  function cancelLogin() {
    api.cancelLogin().catch(() => {});
  }

  async function logout() {
    loggingOut = true;
    try {
      await api.logout();
      loggedIn = false;
      displayName = "";
      status = "Not logged in";
      clearLibrary();
      likedIds.clear();
      clearPlaylistCache();
      clearStatsCache();
      resetQueue();
      resetPlayer();
      resetNav();
    } catch (e) {
      notify(`Logout failed: ${e}`);
    } finally {
      loggingOut = false;
    }
  }

  function relogin() {
    login();
  }

  /** @param {boolean} interactive */
  async function connectPlayback(interactive) {
    if (connecting || playbackConnected) return;
    connecting = true;
    connectingInteractive = interactive;
    try {
      playbackStatus = (await api.hasConnectSession()) ? "Connecting..." : "Waiting for the browser...";
      await api.connectPlayback(interactive);
      playbackConnected = true;
      playbackStatus = "Playback connected";
    } catch (e) {
      playbackStatus = "Not connected";
      if (interactive && e !== api.LOGIN_CANCELLED) notify(`Couldn't connect playback: ${e}`);
    } finally {
      connecting = false;
    }
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={onWindowKeydown} onmouseup={onWindowMouseUp} />

<div class="shell">
{#if !miniMode}
  <UpdateBanner />
{/if}

{#if miniMode}
  <MiniPlayer onExpand={() => setMiniMode(false)} />
{:else if clientIdConfigured === false}
  <div class="setup-titlebar" data-tauri-drag-region="deep">
    <span class="setup-title">SpotiLarp</span>
    <WindowControls />
  </div>
  <ClientIdSetup onSaved={onClientIdSaved} onCancel={changingClientId ? cancelClientIdChange : undefined} />
{:else if clientIdConfigured === true}

<div class="app">
  <header class="topbar" data-tauri-drag-region="deep">
    <h1><span class="dot" class:on={loggedIn}></span>SpotiLarp</h1>

    <div class="search-group">
      <button type="button" class="panel-toggle" onclick={goBack} disabled={nav.back.length === 0} aria-label="Back" title="Back (Alt+Left)">
        <Icon name="chevron-left" size={18} />
      </button>
      <button type="button" class="panel-toggle" onclick={goForward} disabled={nav.forward.length === 0} aria-label="Forward" title="Forward (Alt+Right)">
        <Icon name="chevron-right" size={18} />
      </button>
      <button
        type="button"
        class="panel-toggle"
        class:active={nav.view.type === "home" && !nav.lyrics}
        onclick={() => navigate({ type: "home" })}
        aria-label="Home"
      >
        <Icon name="home" size={18} />
      </button>

      <div class="search-box" bind:this={searchBox} data-tauri-drag-region="false">
        <div class="search-bar">
          <Icon name="search" size={18} class="search-icon" />
          <input
            type="text"
            placeholder="Search (Ctrl+F)"
            bind:value={searchQuery}
            bind:this={searchInput}
            oninput={onSearchInput}
            onfocus={onSearchFocus}
            onkeydown={onSearchKeydown}
            class="search-input"
            spellcheck="false"
          />
        </div>
        {#if showDropdown && searchQuery.trim()}
          <div class="search-dropdown">
            {#if searchPending}
              <div class="dropdown-status">Searching...</div>
            {:else if searchError}
              <div class="dropdown-status error">{searchError}</div>
            {:else if shownResults.length === 0}
              <div class="dropdown-status">No results for "{searchQuery}"</div>
            {:else}
              {#each shownResults as track, i}
                <button
                  type="button"
                  class="dropdown-item"
                  class:highlighted={i === highlighted}
                  onclick={() => pickResult(i)}
                  oncontextmenu={(e) => openTrackMenu(e, track)}
                  onmouseenter={() => (highlighted = i)}
                >
                  {#if smallestCover(track.album.images)}
                    <img src={smallestCover(track.album.images)} alt="" class="dropdown-thumb" />
                  {:else}
                    <div class="dropdown-thumb placeholder"></div>
                  {/if}
                  <div class="dropdown-text">
                    <div class="dropdown-name">{track.name}</div>
                    <div class="dropdown-artist">{track.artists.map((a) => a.name).join(", ")}</div>
                  </div>
                </button>
              {/each}
              <button type="button" class="dropdown-item see-all" onclick={openSearchPage}>
                See all results for "{searchQuery.trim()}"
              </button>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <div class="topbar-right">
    <div class="account-controls">
      <span class="status-text wide-only">{status}</span>
      {#if loggingIn}
        <button type="button" onclick={cancelLogin} class="pill-button">Cancel</button>
      {:else if loggedIn}
        <button onclick={logout} disabled={loggingOut} class="pill-button wide-only">
          {loggingOut ? "Logging out..." : "Log out"}
        </button>
      {:else}
        {#if restoreFailed}
          <button onclick={restoreSessionIfPossible} disabled={restoring} class="pill-button primary">Retry</button>
        {/if}
        <button onclick={login} disabled={restoring} class="pill-button" class:primary={!restoreFailed}>
          Log in with Spotify
        </button>
        {#if !restoring}
          <button type="button" onclick={changeClientId} class="text-button wide-only">Change Client ID</button>
        {/if}
      {/if}
      {#if !playbackConnected}
        <span class="status-text wide-only">{playbackStatus}</span>
        {#if connecting && connectingInteractive}
          <button type="button" onclick={cancelLogin} class="pill-button">Cancel</button>
        {:else if connecting}
          <button class="pill-button" disabled>Connecting...</button>
        {:else}
          <button onclick={() => connectPlayback(true)} class="pill-button">Connect playback</button>
        {/if}
      {/if}
      <div class="narrow-only">
        <AccountMenu initial={loggedIn ? displayName.charAt(0) : ""} label={status}>
          <div class="menu-status">{status}</div>
          <div class="menu-status">{playbackConnected ? "Playback connected" : `Playback: ${playbackStatus}`}</div>
          {#if loggedIn}
            <button onclick={logout} disabled={loggingOut} class="pill-button">
              {loggingOut ? "Logging out..." : "Log out"}
            </button>
          {:else if !loggingIn && !restoring}
            <button type="button" onclick={changeClientId} class="text-button menu-link">Change Client ID</button>
          {/if}
        </AccountMenu>
      </div>
      <button
        type="button"
        class="panel-toggle"
        class:active={nav.lyrics}
        onclick={() => (nav.lyrics = !nav.lyrics)}
        aria-label={nav.lyrics ? "Hide lyrics" : "Show lyrics"}
        aria-pressed={nav.lyrics}
      >
        <Icon name="lyrics" size={18} />
      </button>
      <DiscordRpcSettings />
      <button
        type="button"
        class="panel-toggle"
        class:active={nav.view.type === "settings" && !nav.lyrics}
        onclick={() => navigate({ type: "settings" })}
        aria-label="Settings"
        title="Settings"
      >
        <Icon name="settings" size={18} />
      </button>
      <button
        type="button"
        class="panel-toggle"
        class:active={showPanel}
        onclick={() => (showPanel = !showPanel)}
        aria-label={showPanel ? "Hide now playing panel" : "Show now playing panel"}
        aria-pressed={showPanel}
      >
        <Icon name="panel-right" size={18} />
      </button>
    </div>
    <WindowControls />
    </div>
  </header>

  <div class="body">
    <aside class="sidebar-area">
      <PlaylistSidebar {loggedIn} />
    </aside>
    <main class="content-area">
      <MainContent {loggedIn} {loggingIn} onRelogin={relogin} />
    </main>
    <div class="panel-area" class:collapsed={!showPanel}>
      {#if panelView === "queue"}
        <QueuePanel onClose={() => (panelView = "nowPlaying")} />
      {:else}
        <NowPlayingPanel contextName={contextLabel() || null} onClose={() => (showPanel = false)} />
      {/if}
    </div>
  </div>

  <footer class="now-playing-bar">
    <NowPlaying
      queueOpen={showPanel && panelView === "queue"}
      onToggleQueue={toggleQueue}
      onMiniMode={() => setMiniMode(true)}
    />
  </footer>
</div>
{/if}
<Toasts />
<TrackContextMenu />
</div>

<style>
@font-face {
  font-family: "Gotham";
  src: url("/fonts/GothamLight.ttf") format("truetype");
  font-weight: 300;
  font-style: normal;
  font-display: swap;
}
@font-face {
  font-family: "Gotham";
  src: url("/fonts/GothamLightItalic.ttf") format("truetype");
  font-weight: 300;
  font-style: italic;
  font-display: swap;
}
@font-face {
  font-family: "Gotham";
  src: url("/fonts/GothamBook.ttf") format("truetype");
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}
@font-face {
  font-family: "Gotham";
  src: url("/fonts/GothamBookItalic.ttf") format("truetype");
  font-weight: 400;
  font-style: italic;
  font-display: swap;
}
@font-face {
  font-family: "Gotham";
  src: url("/fonts/GothamMedium.ttf") format("truetype");
  font-weight: 500;
  font-style: normal;
  font-display: swap;
}
@font-face {
  font-family: "Gotham";
  src: url("/fonts/GothamMediumItalic.ttf") format("truetype");
  font-weight: 500;
  font-style: italic;
  font-display: swap;
}
@font-face {
  font-family: "Gotham";
  src: url("/fonts/GothamBold.ttf") format("truetype");
  font-weight: 700;
  font-style: normal;
  font-display: swap;
}
@font-face {
  font-family: "Gotham";
  src: url("/fonts/GothamBoldItalic.ttf") format("truetype");
  font-weight: 700;
  font-style: italic;
  font-display: swap;
}
@font-face {
  font-family: "Gotham";
  src: url("/fonts/Gotham-Black.otf") format("opentype");
  font-weight: 800;
  font-style: normal;
  font-display: swap;
}

:global(:root) {
  
  --bg: #000000;
  --surface: #121212;
  --surface-raised: #181818;
  --surface-hover: #282828;
  --control-bg: #2a2a2a;
  --border: rgba(255, 255, 255, 0.07);
  --border-strong: rgba(255, 255, 255, 0.14);

  --text: #ffffff;
  --text-dim: #b3b3b3;
  --text-muted: #a7a7a7;

  --accent: #1ed760;
  --accent-hover: #3be477;
  --accent-text: #000000;
  --accent-bg: rgba(30, 215, 96, 0.14);
  --accent-bg-strong: rgba(30, 215, 96, 0.24);

  --danger: #f15e6c;

  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --radius-pill: 999px;

  --fs-xs: 0.75em;
  --fs-sm: 0.85em;
  --fs-md: 1.1em;
  --fs-lg: 1.35em;
  --fs-xl: 1.75em;

  --fw-medium: 500;
  --fw-semibold: 600;
  --fw-bold: 700;
  --fw-black: 800;

  --transition: 140ms ease;
  --spring: 380ms cubic-bezier(0.34, 1.56, 0.64, 1);

  --shimmer-a: var(--surface-raised);
  --shimmer-b: var(--surface-hover);

  --font-ui: "Gotham", "Circular Std", Inter, "Helvetica Neue", Helvetica, Arial, sans-serif;

  font-family: var(--font-ui);
  font-size: 16px;
  line-height: 1.4;
  font-weight: 400;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

:global(body) {
  margin: 0;
  background: var(--bg);
  color: var(--text);
}

:global(*) {
  box-sizing: border-box;
}

:global(::selection) {
  background: var(--accent-bg-strong);
  color: var(--text);
}

:global(*:focus-visible) {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

:global(::-webkit-scrollbar) {
  width: 10px;
  height: 10px;
}
:global(::-webkit-scrollbar-track) {
  background: transparent;
}
:global(::-webkit-scrollbar-thumb) {
  background: var(--surface-hover);
  border-radius: var(--radius-pill);
  border: 2px solid transparent;
  background-clip: padding-box;
}
:global(::-webkit-scrollbar-thumb:hover) {
  background: #3e3e3e;
  background-clip: padding-box;
}

:global(.skeleton) {
  border-radius: var(--radius-sm);
  background: linear-gradient(100deg, var(--shimmer-a) 30%, var(--shimmer-b) 50%, var(--shimmer-a) 70%);
  background-size: 200% 100%;
  animation: shimmer 1.6s ease-in-out infinite;
}
@keyframes shimmer {
  from { background-position: 150% 0; }
  to { background-position: -50% 0; }
}

.shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}

.app {
  display: grid;
  grid-template-rows: auto 1fr auto;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.topbar {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  gap: 1em;
  padding: 0.75em 1.25em;
  background: var(--bg);
}

h1 {
  display: flex;
  align-items: center;
  gap: 0.5em;
  font-size: 0.95em;
  margin: 0;
  font-weight: var(--fw-bold);
  letter-spacing: -0.01em;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--text);
  flex-shrink: 1;
  min-width: 0;
}

.search-group {
  display: flex;
  align-items: center;
  gap: 0.5em;
  justify-self: center;
  width: 100%;
  max-width: 620px;
  min-width: 220px;
}
.search-box {
  position: relative;
  flex: 1;
  min-width: 0;
}
.search-bar {
  display: flex;
  align-items: center;
  gap: 0.6em;
  border-radius: var(--radius-pill);
  border: 1px solid transparent;
  padding: 0.6em 1.1em;
  background-color: var(--control-bg);
  transition: border-color var(--transition), background-color var(--transition);
}
.search-bar:hover {
  background-color: #3a3a3a;
}
.search-bar:focus-within {
  border-color: var(--text);
  background-color: var(--control-bg);
}
:global(.search-icon) {
  color: var(--text-muted);
  flex-shrink: 0;
}
.search-input {
  flex: 1;
  min-width: 0;
  border: none;
  padding: 0;
  font-size: var(--fs-sm);
  font-family: inherit;
  color: var(--text);
  background: transparent;
  outline: none;
}
.search-input::placeholder {
  color: var(--text-muted);
}
.search-dropdown {
  position: absolute;
  top: calc(100% + 0.4em);
  left: 0;
  right: 0;
  background: var(--surface-raised);
  border-radius: var(--radius-md);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5);
  overflow: hidden;
  z-index: 20;
  max-height: 70vh;
  overflow-y: auto;
}
.dropdown-status {
  padding: 0.9em 1em;
  font-size: var(--fs-sm);
  color: var(--text-muted);
}
.dropdown-status.error {
  color: var(--danger);
}
.dropdown-item {
  display: flex;
  align-items: center;
  gap: 0.7em;
  width: 100%;
  padding: 0.5em 0.7em;
  border: none;
  background: none;
  cursor: pointer;
  text-align: left;
  font: inherit;
  color: inherit;
  transition: background-color var(--transition);
}
.dropdown-item:hover,
.dropdown-item.highlighted {
  background-color: var(--surface-hover);
}
.dropdown-thumb {
  width: 36px;
  height: 36px;
  border-radius: var(--radius-sm);
  object-fit: cover;
  background: var(--surface-hover);
  flex-shrink: 0;
}
.dropdown-thumb.placeholder {
  background: linear-gradient(135deg, var(--surface-hover), var(--control-bg));
}
.dropdown-text {
  min-width: 0;
}
.dropdown-name {
  font-size: var(--fs-sm);
  font-weight: var(--fw-medium);
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.dropdown-artist {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 0.1em;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
  background: var(--text-muted);
  opacity: 0.4;
  transition: background-color var(--transition), opacity var(--transition);
}
.dot.on {
  background: var(--accent);
  opacity: 1;
}

.topbar-right {
  display: flex;
  align-items: center;
  gap: 0.75em;
  justify-self: end;
  align-self: stretch;
}
.topbar-right > :global(.window-controls) {
  margin: -0.75em -1.25em -0.75em 0;
}

.account-controls {
  display: flex;
  align-items: center;
  gap: 0.75em;
  justify-content: flex-end;
}

.narrow-only {
  display: none;
}
@media (max-width: 1679px) {
  .wide-only {
    display: none;
  }
  .narrow-only {
    display: block;
  }
}

.menu-status {
  font-size: var(--fs-sm);
  color: var(--text-dim);
  white-space: nowrap;
}
.menu-link {
  align-self: flex-start;
  font-size: var(--fs-sm);
}

.setup-titlebar {
  display: flex;
  align-items: stretch;
  justify-content: space-between;
  height: 36px;
  flex-shrink: 0;
  padding-left: 1em;
  background: var(--bg);
}
.setup-title {
  align-self: center;
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  color: var(--text-muted);
}

.status-text {
  font-size: var(--fs-sm);
  color: var(--text-muted);
  white-space: nowrap;
}

.body {
  display: flex;
  overflow: hidden;
}

.sidebar-area {
  width: 280px;
  flex-shrink: 0;
  padding: 1em 0.5em;
  overflow-y: auto;
  background: var(--surface);
  border-radius: var(--radius-md);
  margin: 0.5em 0.5em 0.5em 0.5em;
}

.content-area {
  position: relative;
  flex: 1;
  min-width: 0;
  padding: 1.5em 2em;
  overflow: hidden;
  display: flex;
  background: var(--surface);
  border-radius: var(--radius-md);
  margin: 0.5em 0.5em 0.5em 0;
}

.panel-area {
  width: 340px;
  flex-shrink: 0;
  overflow: hidden;
  background: var(--surface);
  border-radius: var(--radius-md);
  margin: 0.5em 0.5em 0.5em 0;
  transition: width 260ms cubic-bezier(0.34, 1.1, 0.64, 1), margin var(--transition), opacity 200ms ease;
}
.panel-area.collapsed {
  width: 0;
  margin-left: 0;
  margin-right: 0;
  opacity: 0;
}
.panel-area > :global(*) {
  width: 340px;
}

.now-playing-bar {
  background: var(--bg);
  border-radius: var(--radius-md);
  margin: 0 0.5em 0.5em 0.5em;
}

.pill-button {
  border-radius: var(--radius-pill);
  border: 1px solid var(--border-strong);
  padding: 0.55em 1.4em;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  font-family: inherit;
  color: var(--text);
  background-color: transparent;
  cursor: pointer;
  outline: none;
  white-space: nowrap;
  transition: border-color var(--transition), background-color var(--transition), transform 100ms ease;
}

.pill-button:hover {
  border-color: var(--text);
  transform: scale(1.03);
}
.pill-button:active {
  transform: scale(0.96);
}
.pill-button:disabled {
  cursor: default;
  opacity: 0.5;
  transform: none;
}

.pill-button.primary {
  background-color: var(--accent);
  border-color: var(--accent);
  color: var(--accent-text);
}
.pill-button.primary:hover {
  background-color: var(--accent-hover);
  border-color: var(--accent-hover);
  transform: scale(1.04);
}

.text-button {
  padding: 0;
  border: none;
  background: none;
  color: var(--text-muted);
  font-family: inherit;
  font-size: var(--fs-xs);
  cursor: pointer;
  white-space: nowrap;
}
.text-button:hover {
  color: var(--text);
  text-decoration: underline;
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
.panel-toggle:disabled {
  opacity: 0.35;
  cursor: default;
  background: none;
  color: var(--text-muted);
}
.dropdown-item.see-all {
  justify-content: center;
  padding: 0.7em;
  border-top: 1px solid var(--border);
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  color: var(--text-dim);
}
.panel-toggle.active {
  color: var(--accent);
  background-color: var(--accent-bg);
}
</style>
