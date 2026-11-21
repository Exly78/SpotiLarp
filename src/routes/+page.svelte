<script>
  import { onMount, untrack } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import * as api from "$lib/api.js";
  import "$lib/themes.svelte.js";
  import {
    queue,
    contextLabel,
    autoplaying,
    playFromList,
    next,
    toggleShuffle,
    shuffleLabels,
    cycleRepeatMode,
    resetQueue,
  } from "$lib/queue.svelte.js";
  import { openTrackMenu, closeMenu } from "$lib/contextMenu.svelte.js";
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
  import { nav, navigate, goBack, goForward, resetNav, openArtist, openAlbum, openShow, openProfile } from "$lib/nav.svelte.js";
  import { library, loadLibrary, loadFolders, clearLibrary } from "$lib/library.svelte.js";
  import { likedIds, toggleLike, clearLikes } from "$lib/likes.svelte.js";
  import { isTextField } from "$lib/keys.js";
  import { openSpotifyLink } from "$lib/openLink.js";
  import { parseSpotifyLink } from "$lib/links.js";
  import { runLaunchCommands } from "$lib/remote.js";
  import { zoom, zoomIn, zoomOut, resetZoom, suspendZoom } from "$lib/zoom.svelte.js";
  import "$lib/history.svelte.js";
  import "$lib/lastfm.svelte.js";
  import { syncSpotifyPlays } from "$lib/tracker.svelte.js";
  import { LIKED_SONGS_ID } from "$lib/constants.js";
  import PlaylistSidebar from "$lib/components/PlaylistSidebar.svelte";
  import MainContent from "$lib/components/MainContent.svelte";
  import { clearPlaylistCache } from "$lib/components/views/PlaylistView.svelte";
  import { clearStatsCache } from "$lib/components/views/StatsView.svelte";
  import { clearRadioCache } from "$lib/components/views/RadioView.svelte";
  import ShortcutsDialog from "$lib/components/ShortcutsDialog.svelte";
  import Recap from "$lib/components/Recap.svelte";
  import ShareStatsDialog from "$lib/components/ShareStatsDialog.svelte";
  import { sharing, closeRecap, closeShareCard } from "$lib/sharing.svelte.js";
  import FeatureTour from "$lib/components/FeatureTour.svelte";
  import { tour, startTour, startWhatsNew, finishTour, tourSeen, pendingWhatsNew } from "$lib/tour.svelte.js";
  import NowPlaying from "$lib/components/NowPlaying.svelte";
  import NowPlayingPanel from "$lib/components/NowPlayingPanel.svelte";
  import QueuePanel from "$lib/components/QueuePanel.svelte";
  import JamPanel from "$lib/components/JamPanel.svelte";
  import ListenAlongPrompt from "$lib/components/ListenAlongPrompt.svelte";
  import { jam, leaveJam, setDefaultName, setDiscordSharing } from "$lib/jam.svelte.js";
  import TrackContextMenu from "$lib/components/TrackContextMenu.svelte";
  import MiniPlayer from "$lib/components/MiniPlayer.svelte";
  import ClientIdSetup from "$lib/components/ClientIdSetup.svelte";
  import DiscordRpcSettings from "$lib/components/DiscordRpcSettings.svelte";
  import UpdateBanner from "$lib/components/UpdateBanner.svelte";
  import WindowControls from "$lib/components/WindowControls.svelte";
  import AccountMenu from "$lib/components/AccountMenu.svelte";
  import Toasts from "$lib/components/Toasts.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import { coverAtLeast, formatCount, smallestCover } from "$lib/utils.js";
  import { discordAccount } from "$lib/discordAccount.svelte.js";
  import { onRateLimitEnd, rateLimit, timeLeft } from "$lib/rateLimit.svelte.js";

  /** @type {boolean|null} */
  let clientIdConfigured = $state(null);
  let changingClientId = $state(false);
  const LOCAL_ONLY_KEY = "spotilarp.localOnly";
  /** Setting up Spotify was skipped, to play your own files (ClientIdSetup's skip). */
  let localOnly = $state(readLocalOnly());
  // The app opens once Spotify is set up, or without it for your own files.
  const appOpen = $derived(clientIdConfigured === true || (clientIdConfigured === false && localOnly && !changingClientId));

  let loggedIn = $state(false);
  let restoring = $state(false);
  let restoreFailed = $state(false);
  let status = $state("Not logged in");
  let displayName = $state("");
  /** @type {import("$lib/types.js").Profile|null} */
  let profile = $state(null);
  let loggingIn = $state(false);
  let loggingOut = $state(false);

  let playbackConnected = $state(false);
  let playbackStatus = $state("Not connected");
  let connecting = $state(false);
  let connectingInteractive = $state(false);

  let showPanel = $state(true);
  let panelView = $state("nowPlaying");
  let miniMode = $state(false);

  /** How long the window is out of sight before the interface is let go. */
  const ASLEEP_AFTER_MS = 30_000;
  // While minimized or in the tray for a while, the interface (lists, covers,
  // pages) is let go to save memory; playback, the queue and history carry
  // on, and it all comes back when the window does.
  let uiAsleep = $state(false);

  // The feature tour, once: when the app first shows for real, logged in or
  // with your own files, so it doesn't compete with setting up or logging in.
  // After an update, what's new instead, once (tourSteps.js).
  $effect(() => {
    if (!appOpen || uiAsleep || miniMode || !(loggedIn || localOnly) || tour.open) return;
    if (!tourSeen()) {
      const handle = setTimeout(startTour, 1200);
      return () => clearTimeout(handle);
    }
    if (!tour.version) return;
    const news = untrack(pendingWhatsNew);
    if (news.length === 0) return;
    const handle = setTimeout(() => startWhatsNew(news), 1200);
    return () => clearTimeout(handle);
  });
  /** @type {ReturnType<typeof setTimeout>|undefined} */
  let asleepHandle;

  /**
   * @param {boolean} hidden
   * @param {boolean} [now] at launch, hidden from the start
   */
  function onBackground(hidden, now = false) {
    clearTimeout(asleepHandle);
    if (!hidden) uiAsleep = false;
    else if (now) fallAsleep();
    else asleepHandle = setTimeout(fallAsleep, ASLEEP_AFTER_MS);
  }

  function fallAsleep() {
    // A menu left open would come back on its own when the window does.
    closeMenu();
    uiAsleep = true;
  }
  let showSidebar = $state(true);
  const LIBRARY_COLLAPSED_KEY = "spotilarp.libraryCollapsed";
  /** Your Library as a strip of covers. */
  let libraryCollapsed = $state(readLibraryCollapsed());

  function readLibraryCollapsed() {
    try {
      return localStorage.getItem(LIBRARY_COLLAPSED_KEY) === "true";
    } catch {
      return false;
    }
  }

  function toggleLibraryCollapsed() {
    libraryCollapsed = !libraryCollapsed;
    try {
      localStorage.setItem(LIBRARY_COLLAPSED_KEY, String(libraryCollapsed));
    } catch {}
  }
  let showShortcuts = $state(false);
  let lyricsFullscreen = $state(false);
  let launchCommandsRun = false;

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

  function clearSearch() {
    clearTimeout(debounceHandle);
    searchToken++;
    showDropdown = false;
    searchQuery = "";
    searchResults = [];
    resultsFor = "";
    highlighted = -1;
    searchInput?.focus();
  }

  /** @param {number} index */
  function pickResult(index) {
    const query = searchQuery.trim();
    playFromList(shownResults, index, `"${query}"`, { type: "search", query });
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

  function toggleJam() {
    if (showPanel && panelView === "jam") {
      panelView = "nowPlaying";
    } else {
      panelView = "jam";
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
      if (parseSpotifyLink(searchQuery)) openLinkFromSearch();
      else openSearchPage();
    }
  }

  // A pasted Spotify link opens its song, album, playlist or artist.
  function openLinkFromSearch() {
    const link = searchQuery;
    clearTimeout(debounceHandle);
    searchToken++;
    showDropdown = false;
    highlighted = -1;
    searchQuery = "";
    searchResults = [];
    resultsFor = "";
    searchInput?.blur();
    openSpotifyLink(link);
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
    // The mini player is laid out for its fixed size, so it isn't zoomed.
    if (enabled) suspendZoom(true);
    try {
      await api.setMiniMode(enabled);
      miniMode = enabled;
    } catch (e) {
      notify(`Couldn't switch the mini player: ${e}`);
    }
    if (!miniMode) suspendZoom(false);
  }

  const VOLUME_STEP = 10;
  const SEEK_STEP_MS = 5000;

  const LIKED_SONGS = { id: LIKED_SONGS_ID, name: "Liked Songs", isLikedSongs: true, images: [], track_count: { total: 0 } };

  function toggleLyrics() {
    if (lyricsFullscreen) setLyricsFullscreen(false);
    else nav.lyrics = !nav.lyrics;
  }

  /** @param {boolean} on */
  async function setLyricsFullscreen(on) {
    if (miniMode) return;
    lyricsFullscreen = on;
    if (on) nav.lyrics = true;
    try {
      await getCurrentWindow().setFullscreen(on);
    } catch {}
  }

  // Going anywhere but lyrics (Ctrl+H, a link...) leaves full screen.
  $effect(() => {
    if (lyricsFullscreen && !nav.lyrics) setLyricsFullscreen(false);
  });

  async function likePlaying() {
    const id = player.track?.trackId;
    if (!id) return;
    const wasLiked = likedIds.has(id);
    await toggleLike(id);
    if (likedIds.has(id) !== wasLiked) notify(wasLiked ? "Removed from Liked Songs" : "Saved to Liked Songs");
  }

  // Three modes look alike on the button, so a key press says which one it picked.
  function cycleShuffle() {
    toggleShuffle();
    notify(shuffleLabels().state);
  }

  function goToPlayingArtist() {
    const t = player.track;
    if (t?.primaryArtistId) openArtist({ id: t.primaryArtistId, name: t.artistList?.[0]?.name ?? "" });
  }

  function goToPlayingAlbum() {
    if (queue.current?.uri !== player.track?.uri) return;
    if (queue.current?.album.id) openAlbum(queue.current.album);
    else if (queue.current?.show) openShow(queue.current.show);
  }

  /** @param {KeyboardEvent} evt */
  function onWindowKeydown(evt) {
    const ctrl = evt.ctrlKey || evt.metaKey;
    if (ctrl && !evt.altKey && ["=", "+", "-", "_", "0"].includes(evt.key)) {
      evt.preventDefault();
      if (miniMode) return;
      if (evt.key === "0") resetZoom();
      else if (evt.key === "-" || evt.key === "_") zoomOut();
      else zoomIn();
      notify(`Zoom ${Math.round(zoom.level * 100)}%`);
      return;
    }
    if (!appOpen) return;
    const typing = isTextField(evt.target);
    const plain = !typing && !ctrl && !evt.altKey;
    const key = evt.key.toLowerCase();
    if (showShortcuts) return;
    if (evt.key === "Escape" && lyricsFullscreen) {
      setLyricsFullscreen(false);
    } else if (evt.altKey && (evt.key === "ArrowLeft" || evt.key === "ArrowRight")) {
      evt.preventDefault();
      if (evt.key === "ArrowLeft") goBack();
      else goForward();
    } else if (ctrl && !evt.shiftKey && !evt.altKey && (key === "s" || key === "r")) {
      evt.preventDefault();
      if (key === "s") cycleShuffle();
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
    } else if ((ctrl && key === "f") || (!typing && evt.key === "/" && !ctrl)) {
      evt.preventDefault();
      if (lyricsFullscreen) setLyricsFullscreen(false);
      searchInput?.focus();
      searchInput?.select();
    } else if ((ctrl && evt.key === "/") || (plain && evt.key === "?")) {
      evt.preventDefault();
      showShortcuts = true;
    } else if (ctrl && !evt.shiftKey && !evt.altKey && ["h", "l", "b", ",", "q"].includes(key)) {
      evt.preventDefault();
      if (key === "h") navigate({ type: "home" });
      else if (key === "l") navigate({ type: "playlist", playlist: LIKED_SONGS });
      else if (key === "b") showSidebar = !showSidebar;
      else if (key === ",") navigate({ type: "settings" });
      else api.quitApp().catch(() => {});
    } else if (ctrl && evt.shiftKey && !evt.altKey && (key === "a" || key === "b")) {
      evt.preventDefault();
      if (key === "a") goToPlayingArtist();
      else goToPlayingAlbum();
    } else if (plain && !evt.shiftKey && !evt.repeat && ["m", "b", "s", "r", "q", "j", "l", "f"].includes(key)) {
      if (key === "m") toggleMute();
      else if (key === "b") likePlaying();
      else if (key === "s") cycleShuffle();
      else if (key === "r") cycleRepeatMode();
      else if (key === "q") toggleQueue();
      else if (key === "j") toggleJam();
      else if (key === "l") toggleLyrics();
      else setLyricsFullscreen(!lyricsFullscreen);
    } else if (!typing && evt.code === "Space" && !ctrl && !evt.altKey) {
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
      // Folders come through the playback session.
      if (playbackConnected && loggedIn && !library.folders) loadFolders();
    });
    api.hasClientId().then((configured) => {
      clientIdConfigured = configured;
      if (configured) restoreSessionIfPossible();
      else if (localOnly) navigate({ type: "local" });
    });
    const stopBackground = listen("app-background", (event) => onBackground(/** @type {boolean} */ (event.payload)));
    // Started minimized to the tray (launch on startup): nothing to draw yet.
    api
      .windowHidden()
      .then((hidden) => hidden && onBackground(true, true))
      .catch(() => {});
    return () => {
      stopListening.then((stop) => stop());
      stopBackground.then((stop) => stop());
      clearTimeout(asleepHandle);
    };
  });

  async function onClientIdSaved() {
    clientIdConfigured = true;
    changingClientId = false;
    setLocalOnly(false);
    // A different Client ID ends the old app's login (playback's stays):
    // log in again with the new one straight away.
    if (loggedIn && !(await api.loginStatus().catch(() => false))) {
      loggedIn = false;
      displayName = "";
      status = "Not logged in";
      clearLibrary();
      clearLikes();
      clearPlaylistCache();
      clearStatsCache();
      clearRadioCache();
      login();
      return;
    }
    restoreSessionIfPossible();
  }

  function changeClientId() {
    changingClientId = true;
    clientIdConfigured = false;
  }

  function cancelClientIdChange() {
    changingClientId = false;
    // Back to your own files if Spotify was never set up.
    if (!localOnly) clientIdConfigured = true;
  }

  /** No Spotify for now: straight to your own files. */
  function skipSetup() {
    setLocalOnly(true);
    navigate({ type: "local" });
  }

  function readLocalOnly() {
    try {
      return localStorage.getItem(LOCAL_ONLY_KEY) === "true";
    } catch {
      return false;
    }
  }

  /** @param {boolean} on */
  function setLocalOnly(on) {
    localOnly = on;
    try {
      if (on) localStorage.setItem(LOCAL_ONLY_KEY, "true");
      else localStorage.removeItem(LOCAL_ONLY_KEY);
    } catch {}
  }

  const regionNames = new Intl.DisplayNames(undefined, { type: "region" });

  /** "GB" as "United Kingdom". @param {string|null} code */
  function countryName(code) {
    if (!code) return "";
    try {
      return regionNames.of(code) ?? code;
    } catch {
      return code;
    }
  }

  // Whichever way you were logged out, your picture and details go with it.
  $effect(() => {
    if (!loggedIn) profile = null;
  });

  /** Your name as last seen, for when Spotify can't be asked (cleared with the other saved answers on logout). */
  const SAVED_NAME_KEY = "spotilarp.cache.display-name";

  /** @param {string} name */
  async function onLoggedIn(name) {
    try {
      localStorage.setItem(SAVED_NAME_KEY, name);
    } catch {}
    api.getProfile().then((found) => (profile = found)).catch((e) => console.warn("Couldn't load your profile:", e));
    loggedIn = true;
    restoreFailed = false;
    displayName = name;
    status = `Logged in as ${name}`;
    setDefaultName(name);
    // Friends can listen along from the Discord status whenever it's showing.
    api.hasDiscordClientId().then(setDiscordSharing).catch(() => {});
    loadLibrary();
    // What Spotify's other apps played while SpotiLarp was closed, for the tracker.
    syncSpotifyPlays();
    if (!launchCommandsRun) {
      launchCommandsRun = true;
      runLaunchCommands();
    }
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

  /**
   * Turned away for asking too often isn't logged out: the login is fine, so
   * carry on with what's saved and let the notice say when it's over.
   */
  async function carryOnRateLimited() {
    let saved = null;
    try {
      saved = localStorage.getItem(SAVED_NAME_KEY);
    } catch {}
    await onLoggedIn(saved || "you");
  }

  // When it's over, ask again for what couldn't load (your name, profile and library).
  $effect(() => onRateLimitEnd(() => restoreSessionIfPossible()));

  async function restoreSessionIfPossible() {
    restoreFailed = false;
    try {
      if (!(await api.loginStatus())) return;
      restoring = true;
      status = "Restoring session...";
      await onLoggedIn(await api.restoreSession());
    } catch (e) {
      if (e === api.LOGIN_EXPIRED) return;
      if (api.isRateLimited(e)) {
        await carryOnRateLimited();
        return;
      }
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
      // The browser part worked (the login is saved); only asking who you are was turned away.
      if (api.isRateLimited(e) && (await api.loginStatus().catch(() => false))) {
        await carryOnRateLimited();
        return;
      }
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
      setDiscordSharing(false);
      await leaveJam();
      await api.logout();
      loggedIn = false;
      displayName = "";
      status = "Not logged in";
      clearLibrary();
      clearLikes();
      clearPlaylistCache();
      clearStatsCache();
      clearRadioCache();
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

{#if miniMode}
  <MiniPlayer onExpand={() => setMiniMode(false)} />
{:else if clientIdConfigured === false && !appOpen}
  <div class="setup-titlebar" data-tauri-drag-region="deep">
    <span class="setup-title">SpotiLarp</span>
    <WindowControls />
  </div>
  <ClientIdSetup
    onSaved={onClientIdSaved}
    onCancel={changingClientId ? cancelClientIdChange : undefined}
    onSkip={changingClientId ? undefined : skipSetup}
  />
{:else if appOpen && !uiAsleep}

<div class="app" class:lyrics-fullscreen={lyricsFullscreen}>
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
        class="home-button"
        class:active={nav.view.type === "home" && !nav.lyrics}
        onclick={() => navigate({ type: "home" })}
        aria-label="Home"
        title="Home"
      >
        <Icon name="home" size={24} />
      </button>

      <div class="search-box" bind:this={searchBox} data-tauri-drag-region="false">
        <div class="search-bar">
          <Icon name="search" size={22} class="search-icon" />
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
          {#if searchQuery}
            <button type="button" class="search-clear" onclick={clearSearch} aria-label="Clear search" title="Clear">
              <Icon name="close" size={16} />
            </button>
          {/if}
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
      <UpdateBanner />
      {#if rateLimit.until > 0}
        <span
          class="rate-limit-pill"
          title="Spotify is turning away SpotiLarp's requests for a while (it got too many in a short time). Playback still works, and everything refreshes once it's over."
        >
          Spotify limit · back in {timeLeft(rateLimit.until)}
        </span>
      {/if}
      {#if loggingIn}
        <span class="status-text wide-only">{status}</span>
        <button type="button" onclick={cancelLogin} class="pill-button">Cancel</button>
      {:else if !loggedIn && clientIdConfigured === false}
        <button type="button" onclick={changeClientId} class="pill-button">Set up Spotify</button>
      {:else if !loggedIn}
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
      {#if !playbackConnected && clientIdConfigured}
        <span class="status-text wide-only">{playbackStatus}</span>
        {#if connecting && connectingInteractive}
          <button type="button" onclick={cancelLogin} class="pill-button">Cancel</button>
        {:else if connecting}
          <button class="pill-button" disabled>Connecting...</button>
        {:else}
          <button onclick={() => connectPlayback(true)} class="pill-button">Connect playback</button>
        {/if}
      {/if}
      <AccountMenu
        initial={loggedIn ? displayName.charAt(0) : ""}
        label={status}
        imageUrl={loggedIn && profile ? smallestCover(profile.images) : null}
      >
        {#if loggedIn}
          {@const picture = profile ? coverAtLeast(profile.images, 120) : null}
          <div class="menu-account">
            {#if picture}
              <img src={picture} alt="" class="menu-avatar" />
            {:else}
              <div class="menu-avatar placeholder">{displayName.charAt(0)}</div>
            {/if}
            <div class="menu-account-text">
              <div class="menu-name">{profile?.name ?? displayName}</div>
              {#if profile}
                <div class="menu-detail">
                  {[
                    profile.product === "premium" ? "Spotify Premium" : profile.product ? "Spotify Free" : "",
                    countryName(profile.country),
                  ]
                    .filter(Boolean)
                    .join(" · ")}
                </div>
                {#if profile.followers != null}
                  <div class="menu-detail">{formatCount(profile.followers)} {profile.followers === 1 ? "follower" : "followers"}</div>
                {/if}
              {/if}
            </div>
          </div>
          {#if profile}
            {@const id = profile.id}
            {@const name = profile.name}
            <button type="button" class="menu-row" onclick={() => openProfile({ id, name })}>Profile</button>
          {/if}
        {:else}
          <div class="menu-status">{status}</div>
        {/if}
        <div class="menu-status">
          <span class="menu-dot" class:on={playbackConnected}></span>
          {playbackConnected ? "Playback connected" : `Playback: ${playbackStatus}`}
        </div>
        {#if discordAccount.profile}
          {@const discord = discordAccount.profile}
          <div class="menu-divider"></div>
          <div class="menu-account discord">
            <img src={discord.avatar_url} alt="" class="menu-avatar small" />
            <div class="menu-account-text">
              <div class="menu-name small">{discord.name}</div>
              <div class="menu-detail">@{discord.username}</div>
            </div>
            <span class="menu-discord-badge" title="Connected to Discord"><Icon name="discord" size={14} /></span>
          </div>
        {/if}
        {#if loggedIn}
          <div class="menu-divider"></div>
          <button onclick={logout} disabled={loggingOut} class="pill-button">
            {loggingOut ? "Logging out..." : "Log out"}
          </button>
        {:else if !loggingIn && !restoring}
          <button type="button" onclick={changeClientId} class="text-button menu-link">
            {clientIdConfigured ? "Change Client ID" : "Set up Spotify"}
          </button>
        {/if}
      </AccountMenu>
      <button
        type="button"
        class="panel-toggle"
        class:active={nav.lyrics}
        onclick={toggleLyrics}
        aria-label={nav.lyrics ? "Hide lyrics" : "Show lyrics"}
        title="Lyrics (L, F for full screen)"
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
    <aside class="sidebar-area" class:collapsed={!showSidebar} class:rail={libraryCollapsed}>
      <PlaylistSidebar {loggedIn} collapsed={libraryCollapsed} onToggleCollapsed={toggleLibraryCollapsed} />
    </aside>
    <main class="content-area">
      <MainContent {loggedIn} {loggingIn} onRelogin={relogin} onChangeClientId={changeClientId} />
      {#if lyricsFullscreen}
        <button type="button" class="panel-toggle exit-fullscreen" onclick={() => setLyricsFullscreen(false)} aria-label="Leave full screen" title="Leave full screen (Esc)">
          <Icon name="close" size={18} />
        </button>
      {/if}
    </main>
    <div class="panel-area" class:collapsed={!showPanel}>
      {#if panelView === "queue"}
        <QueuePanel onClose={() => (panelView = "nowPlaying")} />
      {:else if panelView === "jam"}
        <JamPanel {displayName} onClose={() => (panelView = "nowPlaying")} onShowQueue={() => (panelView = "queue")} />
      {:else}
        <NowPlayingPanel
          contextName={jam.role === "guest" ? `${jam.hostName || "Someone"}'s Jam` : contextLabel() || null}
          contextView={jam.role === "guest" || autoplaying() ? null : queue.contextView}
          visible={showPanel}
          onClose={() => (showPanel = false)}
          onShowQueue={() => (panelView = "queue")}
        />
      {/if}
    </div>
  </div>

  <footer class="now-playing-bar">
    <NowPlaying
      queueOpen={showPanel && panelView === "queue"}
      onToggleQueue={toggleQueue}
      jamOpen={showPanel && panelView === "jam"}
      onToggleJam={toggleJam}
      onMiniMode={() => setMiniMode(true)}
    />
  </footer>
</div>
{/if}
<Toasts />
{#if !miniMode && !uiAsleep}
  <ListenAlongPrompt />
{/if}
<TrackContextMenu />
{#if showShortcuts}
  <ShortcutsDialog onClose={() => (showShortcuts = false)} />
{/if}
{#if tour.open && appOpen && !miniMode && !uiAsleep}
  <FeatureTour steps={tour.steps} label={tour.label} onClose={finishTour} />
{/if}
{#if sharing.recapYear !== null && appOpen && !miniMode && !uiAsleep}
  <Recap year={sharing.recapYear} onClose={closeRecap} />
{/if}
{#if sharing.card && appOpen && !miniMode && !uiAsleep}
  <ShareStatsDialog card={sharing.card} onClose={closeShareCard} />
{/if}
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
  /* Native controls (WebKitGTK's selects especially) are drawn light otherwise. */
  color-scheme: dark;

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

  /* Panels, so themes can color them apart (Spicetify's main, sidebar, player). */
  --main-bg: var(--surface);
  --sidebar-bg: var(--surface);
  --player-bg: var(--bg);
  --highlight-strong: #3e3e3e;
  --control-hover: #333333;
  --selected-row-bg: #3e3e3e;
  --track-bg: #4d4d4d;
  --toggle-off: #535353;
  --toast-bg: #2e77d0;
  --toast-text: #ffffff;

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
  background: var(--highlight-strong);
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
  padding: 0.5em 1.25em;
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
  gap: 0.75em;
  height: 48px;
  border-radius: var(--radius-pill);
  padding: 0 0.75em 0 0.9em;
  background-color: var(--control-bg);
  /* The outline is drawn inside, so the bar doesn't change size with it. */
  box-shadow: inset 0 0 0 1px transparent;
  transition: box-shadow var(--transition), background-color var(--transition);
}
.search-bar:hover {
  background-color: var(--control-hover);
  box-shadow: inset 0 0 0 1px var(--border-strong);
}
.search-bar:focus-within {
  background-color: var(--control-bg);
  box-shadow: inset 0 0 0 2px var(--text);
}
.search-clear {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  padding: 0;
  border: none;
  border-radius: 50%;
  background: none;
  color: var(--text-muted);
  cursor: pointer;
  transition: color var(--transition), background-color var(--transition);
}
.search-clear:hover {
  color: var(--text);
  background-color: var(--surface-hover);
}
.home-button {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  flex-shrink: 0;
  padding: 0;
  border: none;
  border-radius: 50%;
  background: var(--control-bg);
  color: var(--text-muted);
  cursor: pointer;
  transition: background-color var(--transition), color var(--transition), transform 100ms ease;
}
.home-button:hover {
  background: var(--control-hover);
  color: var(--text);
  transform: scale(1.04);
}
.home-button:active {
  transform: scale(0.96);
}
.home-button.active {
  color: var(--text);
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
  font-size: 1em;
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
  padding: 4px;
  background: var(--surface-hover);
  border-radius: var(--radius-md);
  box-shadow: 0 16px 24px rgba(0, 0, 0, 0.3), 0 6px 8px rgba(0, 0, 0, 0.2);
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
  border-radius: var(--radius-sm);
  background: none;
  cursor: pointer;
  text-align: left;
  font: inherit;
  color: inherit;
  transition: background-color var(--transition);
}
.dropdown-item:hover,
.dropdown-item.highlighted {
  background-color: var(--highlight-strong);
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
  margin: -0.5em -1.25em -0.5em 0;
}

.account-controls {
  display: flex;
  align-items: center;
  gap: 0.5em;
  justify-content: flex-end;
}
/* The account picture comes last, after the buttons, set a little apart. */
.account-controls > :global(.account-menu) {
  order: 1;
  margin-left: 0.25em;
}

@media (max-width: 1679px) {
  .wide-only {
    display: none;
  }
}

.rate-limit-pill {
  flex-shrink: 0;
  padding: 0.35em 0.8em;
  border-radius: var(--radius-pill);
  background: color-mix(in srgb, var(--danger) 18%, transparent);
  color: var(--text);
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  white-space: nowrap;
  cursor: help;
}
.menu-status {
  display: flex;
  align-items: center;
  gap: 0.5em;
  font-size: var(--fs-sm);
  color: var(--text-dim);
  white-space: nowrap;
}
.menu-dot {
  width: 8px;
  height: 8px;
  flex-shrink: 0;
  border-radius: 50%;
  background: var(--text-muted);
}
.menu-dot.on {
  background: var(--accent);
}
.menu-account {
  display: flex;
  align-items: center;
  gap: 0.8em;
  min-width: 0;
}
.menu-avatar {
  width: 52px;
  height: 52px;
  flex-shrink: 0;
  border-radius: 50%;
  object-fit: cover;
  background: var(--control-bg);
}
.menu-avatar.small {
  width: 36px;
  height: 36px;
}
.menu-avatar.placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--fs-lg);
  font-weight: var(--fw-bold);
  text-transform: uppercase;
  color: var(--text);
}
.menu-account-text {
  flex: 1;
  min-width: 0;
}
.menu-name {
  font-size: var(--fs-md);
  font-weight: var(--fw-bold);
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.menu-name.small {
  font-size: var(--fs-sm);
}
.menu-detail {
  margin-top: 0.15em;
  font-size: var(--fs-xs);
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.menu-discord-badge {
  display: flex;
  flex-shrink: 0;
  color: #5865f2;
}
.menu-row {
  align-self: stretch;
  padding: 0.5em 0.6em;
  margin: 0 -0.6em;
  border: none;
  border-radius: var(--radius-sm);
  background: none;
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  text-align: left;
  cursor: pointer;
}
.menu-row:hover {
  background: var(--highlight-strong);
}
.menu-divider {
  height: 1px;
  margin: 0.1em 0;
  background: var(--border-strong);
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

.sidebar-area.collapsed,
.lyrics-fullscreen .sidebar-area,
.lyrics-fullscreen .panel-area,
.lyrics-fullscreen .topbar {
  display: none;
}
.app.lyrics-fullscreen {
  grid-template-rows: 1fr auto;
}
.lyrics-fullscreen .content-area {
  margin: 0.5em;
}
.sidebar-area.collapsed + .content-area {
  margin-left: 0.5em;
}
.exit-fullscreen {
  position: absolute;
  top: 1em;
  right: 1em;
  z-index: 5;
}
.sidebar-area {
  width: 280px;
  flex-shrink: 0;
  padding: 1em 0.5em;
  overflow-y: auto;
  background: var(--sidebar-bg);
  border-radius: var(--radius-md);
  margin: 0.5em 0.5em 0.5em 0.5em;
  transition: width 240ms cubic-bezier(0.2, 0, 0, 1);
}
/* The covers (40px) with the same room either side as when it's open.
   Shutting waits for the library's text to fade out first. */
.sidebar-area.rail {
  width: calc(40px + 2em);
  transition-delay: 70ms;
}
@media (prefers-reduced-motion: reduce) {
  .sidebar-area {
    transition: none;
  }
}

.content-area {
  position: relative;
  flex: 1;
  min-width: 0;
  --content-pad-x: 2em;
  --content-pad-y: 1.5em;
  padding: var(--content-pad-y) var(--content-pad-x);
  overflow: hidden;
  display: flex;
  background: var(--main-bg);
  border-radius: var(--radius-md);
  margin: 0.5em 0.5em 0.5em 0;
}

.panel-area {
  width: 340px;
  flex-shrink: 0;
  overflow: hidden;
  background: var(--main-bg);
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
  background: var(--player-bg);
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
  background-color: var(--text);
  border-color: var(--text);
  color: var(--bg);
}
.pill-button.primary:hover {
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
