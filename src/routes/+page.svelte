<script>
  import { onMount } from "svelte";
  import * as api from "$lib/api.js";
  import PlaylistSidebar from "$lib/components/PlaylistSidebar.svelte";
  import MainContent from "$lib/components/MainContent.svelte";
  import NowPlaying from "$lib/components/NowPlaying.svelte";
  import NowPlayingPanel from "$lib/components/NowPlayingPanel.svelte";
  import ClientIdSetup from "$lib/components/ClientIdSetup.svelte";
  import DiscordRpcSettings from "$lib/components/DiscordRpcSettings.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import { smallestCover } from "$lib/utils.js";

  /** @type {boolean|null} */
  let clientIdConfigured = $state(null);

  let status = $state("Not logged in");
  let loggingIn = $state(false);
  let loggingOut = $state(false);

  let playbackStatus = $state("Not connected");
  let connecting = $state(false);

  /** @type {import("$lib/types.js").Playlist|null} */
  let selectedPlaylist = $state(null);
  /** @type {{ load: () => Promise<void> } | undefined} */
  let sidebar;
  let showPanel = $state(true);
  let showLyrics = $state(false);

  let searchQuery = $state("");
  /** @type {import("$lib/types.js").Track[]} */
  let searchResults = $state([]);
  let searching = $state(false);
  let searchError = $state("");
  let showDropdown = $state(false);
  /** @type {HTMLDivElement|undefined} */
  let searchBox;
  /** @type {ReturnType<typeof setTimeout>|undefined} */
  let debounceHandle;

  function onSearchInput() {
    clearTimeout(debounceHandle);
    showDropdown = true;
    if (!searchQuery.trim()) {
      searchResults = [];
      searchError = "";
      return;
    }
    debounceHandle = setTimeout(runSearch, 300);
  }

  async function runSearch() {
    searching = true;
    searchError = "";
    try {
      searchResults = await api.search(searchQuery);
    } catch (e) {
      searchError = `Search failed: ${e}`;
    } finally {
      searching = false;
    }
  }

  /** @param {import("$lib/types.js").Track} track */
  function pickResult(track) {
    api.playTrack(track.uri);
    showDropdown = false;
    searchQuery = "";
    searchResults = [];
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
    if (evt.key === "Escape") {
      showDropdown = false;
      /** @type {HTMLInputElement} */ (evt.target).blur();
    }
  }

  onMount(async () => {
    clientIdConfigured = await api.hasClientId();
    if (clientIdConfigured) restoreSessionIfPossible();
  });

  function onClientIdSaved() {
    clientIdConfigured = true;
    restoreSessionIfPossible();
  }

  async function restoreSessionIfPossible() {
    try {
      if (await api.loginStatus()) {
        const displayName = await api.restoreSession();
        status = `Logged in as ${displayName}`;
        
        if (await api.hasConnectSession()) {
          connectPlayback();
        }
      }
    } catch (e) {
      
    }
  }

  async function login() {
    loggingIn = true;
    status = "Opening browser...";
    try {
      const displayName = await api.login();
      status = `Logged in as ${displayName}`;
      sidebar?.load();
    } catch (e) {
      status = `Login failed: ${e}`;
    } finally {
      loggingIn = false;
    }
  }

  async function logout() {
    loggingOut = true;
    try {
      await api.logout();
      status = "Not logged in";
      sidebar?.load();
    } catch (e) {
      status = `Logout failed: ${e}`;
    } finally {
      loggingOut = false;
    }
  }

  async function connectPlayback() {
    connecting = true;
    playbackStatus = "Opening browser...";
    try {
      await api.connectPlayback();
      playbackStatus = "Connected";
    } catch (e) {
      playbackStatus = `Connect failed: ${e}`;
    } finally {
      connecting = false;
    }
  }
</script>

<svelte:window onclick={onWindowClick} />

{#if clientIdConfigured === false}
  <ClientIdSetup onSaved={onClientIdSaved} />
{:else if clientIdConfigured === true}

<div class="app">
  <header class="topbar">
    <h1><span class="dot" class:on={status.startsWith("Logged in")}></span>SpotiLarp</h1>

    <div class="search-group">
      <button
        type="button"
        class="panel-toggle"
        onclick={() => {
          showLyrics = false;
          selectedPlaylist = null;
        }}
        aria-label="Home"
      >
        <Icon name="home" size={18} />
      </button>

      <div class="search-box" bind:this={searchBox}>
        <div class="search-bar">
          <Icon name="search" size={18} class="search-icon" />
          <input
            type="text"
            placeholder="Search tracks..."
            bind:value={searchQuery}
            oninput={onSearchInput}
            onfocus={onSearchFocus}
            onkeydown={onSearchKeydown}
            class="search-input"
          />
        </div>
        {#if showDropdown && searchQuery.trim()}
          <div class="search-dropdown">
            {#if searching}
              <div class="dropdown-status">Searching...</div>
            {:else if searchError}
              <div class="dropdown-status error">{searchError}</div>
            {:else if searchResults.length === 0}
              <div class="dropdown-status">No results for "{searchQuery}"</div>
            {:else}
              {#each searchResults.slice(0, 8) as track}
                <button type="button" class="dropdown-item" onclick={() => pickResult(track)}>
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
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <div class="account-controls">
      <span class="status-text">{status}</span>
      {#if status.startsWith("Logged in")}
        <button onclick={logout} disabled={loggingOut} class="pill-button">
          {loggingOut ? "Logging out..." : "Log out"}
        </button>
      {:else}
        <button onclick={login} disabled={loggingIn} class="pill-button primary">
          {loggingIn ? "Logging in..." : "Log in with Spotify"}
        </button>
      {/if}
      <span class="status-text">{playbackStatus}</span>
      <button onclick={connectPlayback} disabled={connecting} class="pill-button">
        {connecting ? "Connecting..." : "Connect playback"}
      </button>
      <button
        type="button"
        class="panel-toggle"
        class:active={showLyrics}
        onclick={() => (showLyrics = !showLyrics)}
        aria-label={showLyrics ? "Hide lyrics" : "Show lyrics"}
        aria-pressed={showLyrics}
      >
        <Icon name="lyrics" size={18} />
      </button>
      <DiscordRpcSettings />
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
  </header>

  <div class="body">
    <aside class="sidebar-area">
      <PlaylistSidebar
        bind:this={sidebar}
        selectedId={selectedPlaylist?.id}
        onSelect={(p) => {
          selectedPlaylist = p;
          showLyrics = false;
        }}
      />
    </aside>
    <main class="content-area">
      <MainContent {selectedPlaylist} {showLyrics} />
    </main>
    <div class="panel-area" class:collapsed={!showPanel}>
      <NowPlayingPanel contextName={selectedPlaylist?.name} onClose={() => (showPanel = false)} />
    </div>
  </div>

  <footer class="now-playing-bar">
    <NowPlaying />
  </footer>
</div>
{/if}

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

.app {
  display: grid;
  grid-template-rows: auto 1fr auto;
  height: 100vh;
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
.dropdown-item:hover {
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

.account-controls {
  display: flex;
  align-items: center;
  gap: 0.75em;
  flex-wrap: wrap;
  justify-content: flex-end;
  justify-self: end;
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
