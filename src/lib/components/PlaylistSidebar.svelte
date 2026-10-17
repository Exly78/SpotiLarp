<script>
  import { tick } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { fly, fade } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import * as api from "../api.js";
  import { smallestCover, releaseType, coverAtLeast } from "../utils.js";
  import {
    library,
    loadLibrary,
    createPlaylist,
    renamePlaylist,
    deletePlaylist,
    removeAlbum,
    removeShow,
    isOwned,
    adjustTrackCount,
  } from "../library.svelte.js";
  import { likeTracks } from "../likes.svelte.js";
  import { drag } from "../drag.svelte.js";
  import { nav, navigate, openAlbum, openRadio, openShow } from "../nav.svelte.js";
  import { notify } from "../toast.svelte.js";
  import { isPinned, togglePin, pins } from "../pins.svelte.js";
  import { playedAt } from "../played.svelte.js";
  import { playPlaylist, queuePlaylist, playAlbum, queueAlbum } from "../collections.js";
  import Icon from "./Icon.svelte";

  import { LIKED_SONGS_ID, LOCAL_URI_PREFIX } from "../constants.js";

  /**
   * @typedef {import("../types.js").Playlist} Playlist
   * @typedef {import("../types.js").Album} Album
   * @typedef {import("../types.js").Show} Show
   */

  /** @type {{ loggedIn: boolean }} */
  let { loggedIn } = $props();

  const LIKED_SONGS = {
    id: LIKED_SONGS_ID,
    name: "Liked Songs",
    isLikedSongs: true,
    images: [],
    track_count: { total: 0 },
  };

  const selectedId = $derived(nav.view.type === "playlist" ? nav.view.playlist.id : null);
  const selectedAlbumId = $derived(nav.view.type === "album" ? nav.view.id : null);
  const selectedShowId = $derived(nav.view.type === "show" ? nav.view.id : null);

  const FILTER_KEY = "spotilarp.libraryFilter";
  /** Before the filter was a dropdown, "By you" was a switch of its own. */
  const BY_YOU_KEY = "spotilarp.libraryByYou";
  /** @type {Record<string, string>} */
  const FILTERS = { all: "Everything", playlists: "Playlists", mine: "Playlists by you", albums: "Albums", podcasts: "Podcasts" };
  const SORT_KEY = "spotilarp.librarySort";
  /** @type {Record<string, string>} */
  const SORTS = { default: "Spotify order", recent: "Recently played", name: "Alphabetical" };

  let filter = $state(readFilter());
  let sort = $state(readSetting(SORT_KEY, "default"));
  let search = $state("");

  // "Playlists by you" is only playlists the signed-in account owns.
  const byYou = $derived(filter === "mine");
  const showPlaylists = $derived(filter === "all" || filter === "playlists" || filter === "mine");
  const showAlbums = $derived(filter === "all" || filter === "albums");
  const showShows = $derived(filter === "all" || filter === "podcasts");
  // Albums and Podcasts are offered once there are some (or while chosen).
  const filterOptions = $derived(
    Object.entries(FILTERS).filter(
      ([value]) =>
        value === filter ||
        ((value !== "albums" || library.albums.length > 0) && (value !== "podcasts" || library.shows.length > 0)),
    ),
  );

  function readFilter() {
    const saved = readSetting(FILTER_KEY, "");
    if (saved in FILTERS) return saved;
    return readSetting(BY_YOU_KEY, "false") === "true" ? "mine" : "all";
  }

  /** @param {string} value */
  function chooseFilter(value) {
    filter = value;
    saveSetting(FILTER_KEY, value);
  }

  /**
   * @param {string} key
   * @param {string} fallback
   */
  function readSetting(key, fallback) {
    try {
      return localStorage.getItem(key) ?? fallback;
    } catch {
      return fallback;
    }
  }

  /**
   * @param {string} key
   * @param {string} value
   */
  function saveSetting(key, value) {
    try {
      localStorage.setItem(key, value);
    } catch {}
  }

  /** @param {string} value */
  function chooseSort(value) {
    sort = value;
    saveSetting(SORT_KEY, value);
  }

  /**
   * Pinned items first (in pin order), then the chosen order.
   * @template {{ name: string }} T
   * @param {T[]} items
   * @param {(item: T) => string} keyOf
   */
  function arrange(items, keyOf) {
    const query = search.trim().toLowerCase();
    const matching = query ? items.filter((item) => item.name.toLowerCase().includes(query)) : items;
    const sorted = [...matching];
    if (sort === "name") sorted.sort((a, b) => a.name.localeCompare(b.name));
    else if (sort === "recent") sorted.sort((a, b) => playedAt(keyOf(b)) - playedAt(keyOf(a)));
    const pinOrder = (/** @type {T} */ item) => {
      const index = pins.keys.indexOf(keyOf(item));
      return index < 0 ? Infinity : index;
    };
    return sorted.sort((a, b) => pinOrder(a) - pinOrder(b));
  }

  const shownPlaylists = $derived(
    showPlaylists
      ? arrange(
          byYou ? library.playlists.filter((p) => !!library.userId && p.owner?.id === library.userId) : library.playlists,
          (p) => `playlist:${p.id}`,
        )
      : [],
  );
  const OPEN_FOLDERS_KEY = "spotilarp.openFolders";
  /** Folders start closed, like Spotify's. */
  const openFolders = new SvelteSet(readOpenFolders());

  /** @returns {string[]} */
  function readOpenFolders() {
    try {
      const saved = JSON.parse(localStorage.getItem(OPEN_FOLDERS_KEY) ?? "[]");
      return Array.isArray(saved) ? saved.filter((id) => typeof id === "string") : [];
    } catch {
      return [];
    }
  }

  /** @param {string} id */
  function toggleFolder(id) {
    if (openFolders.has(id)) openFolders.delete(id);
    else openFolders.add(id);
    saveSetting(OPEN_FOLDERS_KEY, JSON.stringify([...openFolders]));
  }

  /**
   * @typedef {{ kind: "playlist", key: string, playlist: Playlist, depth: number }
   *   | { kind: "folder", key: string, id: string, name: string, count: number, depth: number, open: boolean }} Row
   */

  // In Spotify's order, playlists sit in their folders (read from the
  // playback session); searching, "By you" or another order lists them flat.
  const useFolders = $derived(sort === "default" && !search.trim() && !byYou && !!library.folders);

  /** @type {Row[]} */
  const playlistRows = $derived.by(() => {
    if (!useFolders || !library.folders) {
      return shownPlaylists.map((playlist) => ({ kind: "playlist", key: playlist.id, playlist, depth: 0 }));
    }
    const entries = library.folders;
    const byId = new Map(library.playlists.map((p) => [p.id, p]));
    const pinned = shownPlaylists.filter((p) => isPinned(`playlist:${p.id}`));
    const pinnedIds = new Set(pinned.map((p) => p.id));
    const inFolders = new Set(entries.flatMap((e) => (e.type === "playlist" ? [e.id] : [])));

    /** @type {Map<string, number>} */
    const counts = new Map();
    /** @type {string[]} */
    const path = [];
    for (const entry of entries) {
      if (entry.type === "folder_start") path.push(entry.id);
      else if (entry.type === "folder_end") path.pop();
      else if (byId.has(entry.id)) for (const id of path) counts.set(id, (counts.get(id) ?? 0) + 1);
    }

    /** @type {Row[]} */
    const rows = [...pinned, ...library.playlists.filter((p) => !inFolders.has(p.id) && !pinnedIds.has(p.id))].map(
      (playlist) => ({ kind: "playlist", key: playlist.id, playlist, depth: 0 }),
    );
    /** @type {boolean[]} whether each enclosing folder is open */
    const open = [];
    const shown = () => open.every(Boolean);
    for (const entry of entries) {
      if (entry.type === "folder_start") {
        if (shown()) {
          rows.push({
            kind: "folder",
            key: `folder:${entry.id}`,
            id: entry.id,
            name: entry.name || "Folder",
            count: counts.get(entry.id) ?? 0,
            depth: open.length,
            open: openFolders.has(entry.id),
          });
        }
        open.push(openFolders.has(entry.id));
      } else if (entry.type === "folder_end") {
        open.pop();
      } else {
        const playlist = byId.get(entry.id);
        if (playlist && !pinnedIds.has(playlist.id) && shown()) {
          rows.push({ kind: "playlist", key: playlist.id, playlist, depth: open.length });
        }
      }
    }
    return rows;
  });

  const shownAlbums = $derived(showAlbums ? arrange(library.albums, (a) => `album:${a.id}`) : []);
  const shownShows = $derived(showShows ? arrange(library.shows, (s) => `show:${s.id}`) : []);

  let creating = $state(false);
  let newName = $state("");
  let saving = $state(false);
  /** @type {HTMLInputElement|undefined} */
  let nameInput = $state();

  /** @type {string|null} */
  let dropTargetId = $state(null);

  /** @type {{ playlist: Playlist|null, album: Album|null, show: Show|null, x: number, y: number, confirmDelete: boolean }|null} */
  let menu = $state(null);
  /** @type {HTMLDivElement|undefined} */
  let menuEl = $state();
  let menuHeight = $state(0);
  const MENU_EDGE_MARGIN = 8;
  /** @type {string|null} */
  let renamingId = $state(null);
  let renameValue = $state("");

  /** @param {Playlist} playlist */
  function open(playlist) {
    navigate({ type: "playlist", playlist });
  }

  async function startCreating() {
    creating = true;
    newName = "";
    await tick();
    nameInput?.focus();
  }

  async function submitNew() {
    const name = newName.trim();
    if (!name || saving) return;
    saving = true;
    try {
      const playlist = await createPlaylist(name);
      creating = false;
      open(playlist);
    } catch (e) {
      notify(`Couldn't create the playlist: ${e}`);
    } finally {
      saving = false;
    }
  }

  /** @param {KeyboardEvent} evt */
  function onNameKeydown(evt) {
    if (evt.key === "Enter") submitNew();
    if (evt.key === "Escape") creating = false;
  }

  /**
   * @param {DragEvent} evt
   * @param {Playlist} playlist
   */
  function onDragOver(evt, playlist) {
    if (drag.tracks.length === 0 || !isOwned(playlist)) return;
    evt.preventDefault();
    if (evt.dataTransfer) evt.dataTransfer.dropEffect = "copy";
    dropTargetId = playlist.id;
  }

  /** @param {Playlist} playlist */
  function onDragLeave(playlist) {
    if (dropTargetId === playlist.id) dropTargetId = null;
  }

  /**
   * @param {DragEvent} evt
   * @param {Playlist} playlist
   */
  async function onDrop(evt, playlist) {
    evt.preventDefault();
    dropTargetId = null;
    if (playlist.isLikedSongs) {
      const songs = drag.tracks.filter((t) => t.id);
      if (songs.length > 0) await likeTracks(songs.map((t) => t.id));
      return;
    }
    // Local files go in playlists too (Spotify can't like them).
    const tracks = drag.tracks.filter((t) => t.id || t.uri.startsWith(LOCAL_URI_PREFIX));
    if (tracks.length === 0) return;
    try {
      await api.addToPlaylist(playlist.id, tracks.map((t) => t.uri));
      adjustTrackCount(playlist.id, tracks.length);
      notify(`Added to ${playlist.name}`);
    } catch (e) {
      notify(`Couldn't add it to ${playlist.name}: ${e}`);
    }
  }

  /**
   * @param {MouseEvent} evt
   * @param {Playlist} playlist
   */
  function openMenu(evt, playlist) {
    evt.preventDefault();
    menu = { playlist, album: null, show: null, x: evt.clientX, y: evt.clientY, confirmDelete: false };
  }

  /**
   * @param {MouseEvent} evt
   * @param {Album} album
   */
  function openAlbumMenu(evt, album) {
    evt.preventDefault();
    menu = { playlist: null, album, show: null, x: evt.clientX, y: evt.clientY, confirmDelete: false };
  }

  /**
   * @param {MouseEvent} evt
   * @param {Show} show
   */
  function openShowMenu(evt, show) {
    evt.preventDefault();
    menu = { playlist: null, album: null, show, x: evt.clientX, y: evt.clientY, confirmDelete: false };
  }

  /** @param {Show} show */
  async function unfollowShow(show) {
    menu = null;
    try {
      await removeShow(show.id);
      notify(`Removed ${show.name} from Your Library`);
    } catch (e) {
      notify(`Couldn't remove the podcast: ${e}`);
    }
  }

  /** @param {() => unknown} action */
  function fromMenu(action) {
    menu = null;
    action();
  }

  /** @param {Album} album */
  async function removeSavedAlbum(album) {
    menu = null;
    if (!album.id) return;
    try {
      await removeAlbum(album.id);
      notify(`Removed ${album.name} from Your Library`);
    } catch (e) {
      notify(`Couldn't remove the album: ${e}`);
    }
  }

  /** @param {Album} album */
  function albumSubtitle(album) {
    const artists = (album.artists ?? []).map((a) => a.name).join(", ");
    return artists ? `${releaseType(album.album_type)} · ${artists}` : releaseType(album.album_type);
  }

  /** @param {PointerEvent} evt */
  function onWindowPointerDown(evt) {
    if (menu && menuEl && !menuEl.contains(/** @type {Node} */ (evt.target))) menu = null;
  }

  /** @param {Playlist} playlist */
  async function startRename(playlist) {
    menu = null;
    renamingId = playlist.id;
    renameValue = playlist.name;
    await tick();
    const input = /** @type {HTMLInputElement|null} */ (document.querySelector(".rename-input"));
    input?.focus();
    input?.select();
  }

  /** @param {Playlist} playlist */
  async function submitRename(playlist) {
    const name = renameValue.trim();
    renamingId = null;
    if (!name || name === playlist.name) return;
    try {
      await renamePlaylist(playlist.id, name);
    } catch (e) {
      notify(`Couldn't rename the playlist: ${e}`);
    }
  }

  /**
   * @param {KeyboardEvent} evt
   * @param {Playlist} playlist
   */
  function onRenameKeydown(evt, playlist) {
    if (evt.key === "Enter") submitRename(playlist);
    if (evt.key === "Escape") renamingId = null;
  }

  /** @param {Playlist} playlist */
  async function confirmDelete(playlist) {
    if (!menu) return;
    if (!menu.confirmDelete) {
      menu.confirmDelete = true;
      return;
    }
    menu = null;
    try {
      await deletePlaylist(playlist.id);
      if (selectedId === playlist.id) navigate({ type: "home" });
      notify(isOwned(playlist) ? `Deleted ${playlist.name}` : `Removed ${playlist.name} from Your Library`);
    } catch (e) {
      notify(`Couldn't remove the playlist: ${e}`);
    }
  }
</script>

<svelte:window
  onpointerdown={onWindowPointerDown}
  onkeydown={(e) => e.key === "Escape" && (menu = null)}
  onwheel={() => (menu = null)}
/>

<nav class="sidebar">
  <div class="header-row">
    <h2>Your Library</h2>
    <div class="header-buttons">
      <button
        type="button"
        class="refresh"
        onclick={startCreating}
        disabled={!loggedIn}
        aria-label="Create playlist"
        title="Create playlist"
      >
        <Icon name="plus" size={16} />
      </button>
      <button
        type="button"
        class="refresh"
        onclick={loadLibrary}
        disabled={library.loading || !loggedIn}
        aria-label="Refresh library"
        title="Refresh library"
      >
        <Icon name="refresh" size={16} class={library.loading ? "spin" : ""} />
      </button>
    </div>
  </div>
  {#if creating}
    <div class="new-playlist">
      <input
        bind:this={nameInput}
        bind:value={newName}
        onkeydown={onNameKeydown}
        onblur={() => !newName.trim() && (creating = false)}
        placeholder="Playlist name"
        disabled={saving}
        spellcheck="false"
        aria-label="New playlist name"
      />
    </div>
  {/if}
  {#if !loggedIn}
    <p class="status">Log in to see your playlists.</p>
  {:else}
    <ul class="playlist-list pinned">
      <li>
        <button
          type="button"
          class="playlist-button"
          class:active={selectedId === LIKED_SONGS_ID}
          class:drop-target={dropTargetId === LIKED_SONGS_ID}
          onclick={() => open(LIKED_SONGS)}
          ondblclick={() => playPlaylist(LIKED_SONGS)}
          title="Double-click to play"
          ondragover={(e) => onDragOver(e, LIKED_SONGS)}
          ondragleave={() => onDragLeave(LIKED_SONGS)}
          ondrop={(e) => onDrop(e, LIKED_SONGS)}
        >
          <div class="thumb liked-thumb">
            <Icon name="heart-filled" size={20} />
          </div>
          <div class="meta">
            <div class="name">Liked Songs</div>
          </div>
        </button>
      </li>
      <li>
        <button
          type="button"
          class="playlist-button"
          class:active={nav.view.type === "stats"}
          onclick={() => navigate({ type: "stats" })}
        >
          <div class="thumb stats-thumb">
            <Icon name="chart" size={20} />
          </div>
          <div class="meta">
            <div class="name">Your stats</div>
          </div>
        </button>
      </li>
      <li>
        <button
          type="button"
          class="playlist-button"
          class:active={nav.view.type === "local"}
          onclick={() => navigate({ type: "local" })}
        >
          <div class="thumb local-thumb">
            <Icon name="folder" size={20} />
          </div>
          <div class="meta">
            <div class="name">Local Files</div>
          </div>
        </button>
      </li>
    </ul>
    {#if library.loading && library.playlists.length === 0}
      <ul class="playlist-list" transition:fade={{ duration: 150 }}>
        {#each { length: 6 } as _}
          <li class="skeleton-row">
            <div class="skeleton thumb"></div>
            <div class="skeleton-lines">
              <div class="skeleton line-name"></div>
              <div class="skeleton line-count"></div>
            </div>
          </li>
        {/each}
      </ul>
    {:else if library.error}
      <p class="status error" transition:fade={{ duration: 150 }}>{library.error}</p>
    {:else}
      <div class="library-tools">
        <select class="library-select" value={filter} onchange={(e) => chooseFilter(e.currentTarget.value)} aria-label="Show">
          {#each filterOptions as [value, label]}
            <option {value}>{label}</option>
          {/each}
        </select>
        <select class="library-select" value={sort} onchange={(e) => chooseSort(e.currentTarget.value)} aria-label="Sort by">
          {#each Object.entries(SORTS) as [value, label]}
            <option {value}>{label}</option>
          {/each}
        </select>
      </div>
      <input
        type="search"
        class="library-search"
        placeholder="Search in Your Library"
        bind:value={search}
        spellcheck="false"
        aria-label="Search in Your Library"
      />
      {#if showPlaylists && library.playlists.length === 0}
        <p class="status" transition:fade={{ duration: 150 }}>No playlists yet.</p>
      {:else if search.trim() && shownPlaylists.length === 0 && shownAlbums.length === 0 && shownShows.length === 0}
        <p class="status" transition:fade={{ duration: 150 }}>Nothing in Your Library matches "{search.trim()}".</p>
      {/if}
      <ul class="playlist-list">
        {#each playlistRows as row, i (row.key)}
          <li
            in:fly={{ y: 8, duration: 220, delay: Math.min(i * 25, 300), easing: cubicOut }}
            style={row.depth > 0 ? `padding-left: ${row.depth * 0.9}em` : ""}
          >
            {#if row.kind === "folder"}
              <button
                type="button"
                class="playlist-button"
                onclick={() => toggleFolder(row.id)}
                aria-expanded={row.open}
                title={row.open ? "Close folder" : "Open folder"}
              >
                <div class="thumb folder-thumb">
                  <Icon name="folder" size={20} />
                </div>
                <div class="meta">
                  <div class="name">{row.name}</div>
                  <div class="count">{row.count} {row.count === 1 ? "playlist" : "playlists"}</div>
                </div>
                <Icon name="chevron-right" size={16} class={row.open ? "folder-chevron open" : "folder-chevron"} />
              </button>
            {:else if renamingId === row.playlist.id}
              {@const playlist = row.playlist}
              <div class="rename-row">
                <input
                  class="rename-input"
                  bind:value={renameValue}
                  onkeydown={(e) => onRenameKeydown(e, playlist)}
                  onblur={() => submitRename(playlist)}
                  spellcheck="false"
                  aria-label="Playlist name"
                />
              </div>
            {:else}
              {@const playlist = row.playlist}
              <button
                type="button"
                class="playlist-button"
                class:active={selectedId === playlist.id}
                class:drop-target={dropTargetId === playlist.id}
                onclick={() => open(playlist)}
                ondblclick={() => playPlaylist(playlist)}
                oncontextmenu={(e) => openMenu(e, playlist)}
                ondragover={(e) => onDragOver(e, playlist)}
                ondragleave={() => onDragLeave(playlist)}
                ondrop={(e) => onDrop(e, playlist)}
              >
                {#if smallestCover(playlist.images)}
                  <img src={smallestCover(playlist.images)} alt="" class="thumb" />
                {:else}
                  <div class="thumb placeholder"></div>
                {/if}
                <div class="meta">
                  <div class="name">
                    {#if isPinned(`playlist:${playlist.id}`)}<span class="pin" title="Pinned">●</span>{/if}{playlist.name}
                  </div>
                  <div class="count">
                    {playlist.track_count.total} {playlist.track_count.total === 1 ? "track" : "tracks"}
                    {#if playlist.owner?.display_name && playlist.owner.id !== library.userId}
                      · {playlist.owner.display_name}
                    {/if}
                  </div>
                </div>
              </button>
            {/if}
          </li>
        {/each}
        {#each shownAlbums as album, i (album.id)}
          <li in:fly={{ y: 8, duration: 220, delay: Math.min(i * 25, 300), easing: cubicOut }}>
            <button
              type="button"
              class="playlist-button"
              class:active={selectedAlbumId === album.id}
              onclick={() => openAlbum(album)}
              ondblclick={() => playAlbum(album)}
              oncontextmenu={(e) => openAlbumMenu(e, album)}
            >
              {#if smallestCover(album.images)}
                <img src={smallestCover(album.images)} alt="" class="thumb" />
              {:else}
                <div class="thumb placeholder"></div>
              {/if}
              <div class="meta">
                <div class="name">
                  {#if isPinned(`album:${album.id}`)}<span class="pin" title="Pinned">●</span>{/if}{album.name}
                </div>
                <div class="count">{albumSubtitle(album)}</div>
              </div>
            </button>
          </li>
        {/each}
        {#each shownShows as show, i (show.id)}
          <li in:fly={{ y: 8, duration: 220, delay: Math.min(i * 25, 300), easing: cubicOut }}>
            <button
              type="button"
              class="playlist-button"
              class:active={selectedShowId === show.id}
              onclick={() => openShow(show)}
              oncontextmenu={(e) => openShowMenu(e, show)}
            >
              {#if smallestCover(show.images)}
                <img src={smallestCover(show.images)} alt="" class="thumb" />
              {:else}
                <div class="thumb folder-thumb"><Icon name="podcast" size={20} /></div>
              {/if}
              <div class="meta">
                <div class="name">
                  {#if isPinned(`show:${show.id}`)}<span class="pin" title="Pinned">●</span>{/if}{show.name}
                </div>
                <div class="count">Podcast</div>
              </div>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</nav>

{#if menu}
  <div
    class="menu"
    role="menu"
    bind:this={menuEl}
    bind:offsetHeight={menuHeight}
    style="left: {menu.x}px; top: {Math.max(MENU_EDGE_MARGIN, Math.min(menu.y, window.innerHeight - menuHeight - MENU_EDGE_MARGIN))}px"
  >
    {#if menu.show}
      {@const show = menu.show}
      {@const pinKey = `show:${show.id}`}
      <button type="button" role="menuitem" onclick={() => fromMenu(() => openShow(show))}>Go to podcast</button>
      <div class="divider"></div>
      <button type="button" role="menuitem" onclick={() => fromMenu(() => togglePin(pinKey))}>
        {isPinned(pinKey) ? "Unpin" : "Pin to top"}
      </button>
      <button type="button" role="menuitem" onclick={() => unfollowShow(show)}>Unfollow</button>
    {:else if menu.album}
      {@const album = menu.album}
      {@const pinKey = `album:${album.id}`}
      <button type="button" role="menuitem" onclick={() => fromMenu(() => playAlbum(album))}>Play</button>
      <button type="button" role="menuitem" onclick={() => fromMenu(() => queueAlbum(album))}>Add to queue</button>
      <button
        type="button"
        role="menuitem"
        onclick={() => fromMenu(() => openRadio(`spotify:album:${album.id}`, album.name, coverAtLeast(album.images, 300)))}
      >
        Go to album radio
      </button>
      <div class="divider"></div>
      <button type="button" role="menuitem" onclick={() => fromMenu(() => togglePin(pinKey))}>
        {isPinned(pinKey) ? "Unpin" : "Pin to top"}
      </button>
      <button type="button" role="menuitem" onclick={() => removeSavedAlbum(album)}>Remove from Your Library</button>
    {:else if menu.playlist}
      {@const playlist = menu.playlist}
      {@const owned = isOwned(playlist)}
      {@const pinKey = `playlist:${playlist.id}`}
      <button type="button" role="menuitem" onclick={() => fromMenu(() => playPlaylist(playlist))}>Play</button>
      <button type="button" role="menuitem" onclick={() => fromMenu(() => queuePlaylist(playlist))}>Add to queue</button>
      <button
        type="button"
        role="menuitem"
        onclick={() =>
          fromMenu(() => openRadio(`spotify:playlist:${playlist.id}`, playlist.name, coverAtLeast(playlist.images, 300)))}
      >
        Go to playlist radio
      </button>
      <div class="divider"></div>
      <button type="button" role="menuitem" onclick={() => fromMenu(() => togglePin(pinKey))}>
        {isPinned(pinKey) ? "Unpin" : "Pin to top"}
      </button>
      {#if owned}
        <button type="button" role="menuitem" onclick={() => startRename(playlist)}>Rename</button>
      {/if}
      <button type="button" role="menuitem" class:danger={menu.confirmDelete} onclick={() => confirmDelete(playlist)}>
        {#if menu.confirmDelete}
          Click again to confirm
        {:else}
          {owned ? "Delete playlist" : "Remove from Your Library"}
        {/if}
      </button>
    {/if}
  </div>
{/if}

<style>
.sidebar {
  display: flex;
  flex-direction: column;
  gap: 0.75em;
  width: 100%;
  height: 100%;
  overflow-y: auto;
}
.header-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 0.4em;
}
.library-tools {
  display: flex;
  gap: 0.4em;
  padding: 0 0.4em;
}
.library-search,
.library-select {
  min-width: 0;
  padding: 0.45em 0.8em;
  border: 1px solid transparent;
  border-radius: var(--radius-pill);
  background: var(--control-bg);
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-xs);
  outline: none;
}
.library-select {
  flex: 1 1 0;
  text-overflow: ellipsis;
  cursor: pointer;
}
.library-select:hover {
  background: var(--control-hover);
}
.library-search {
  margin: 0 0.4em;
}
.library-search:focus,
.library-select:focus {
  border-color: var(--text);
}
.folder-thumb {
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  background: var(--surface-hover);
}
:global(.folder-chevron) {
  flex-shrink: 0;
  margin-left: auto;
  color: var(--text-muted);
  transition: transform var(--transition);
}
:global(.folder-chevron.open) {
  transform: rotate(90deg);
}
.pin {
  margin-right: 0.35em;
  font-size: 0.7em;
  color: var(--accent);
  vertical-align: 0.15em;
}
h2 {
  font-size: var(--fs-xs);
  text-transform: uppercase;
  letter-spacing: 0.08em;
  font-weight: var(--fw-bold);
  color: var(--text-muted);
  margin: 0;
}
.refresh {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  background: none;
  border: none;
  border-radius: 50%;
  color: var(--text-muted);
  cursor: pointer;
  transition: background-color var(--transition), color var(--transition);
}
.refresh:hover {
  background-color: var(--surface-hover);
  color: var(--text);
}
.refresh:disabled {
  cursor: default;
  color: var(--text-muted);
}
:global(.spin) {
  animation: spin 1s linear infinite;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}
.status {
  font-size: var(--fs-sm);
  color: var(--text-muted);
  margin: 0;
  padding: 0.4em;
}
.status.error {
  color: var(--danger);
}
.playlist-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.skeleton-row {
  display: flex;
  align-items: center;
  gap: 0.7em;
  padding: 0.45em 0.5em;
}
.skeleton-lines {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 0.4em;
}
.line-name {
  width: 70%;
  height: 0.8em;
}
.line-count {
  width: 40%;
  height: 0.65em;
}

.playlist-button {
  display: flex;
  align-items: center;
  gap: 0.7em;
  width: 100%;
  padding: 0.45em 0.5em;
  border-radius: var(--radius-md);
  border: none;
  background: none;
  color: inherit;
  font: inherit;
  cursor: pointer;
  text-align: left;
  position: relative;
  transition: background-color var(--transition), transform var(--transition), box-shadow var(--transition);
}
.playlist-button:hover {
  background-color: var(--surface-hover);
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25);
}
.playlist-button.active {
  background-color: var(--surface-hover);
}
.playlist-button.active .name {
  color: var(--accent);
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
.liked-thumb {
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #4b1fa8, #8f8ff0);
  color: #fff;
}
.playlist-list.pinned {
  margin-bottom: 0.5em;
  padding-bottom: 0.5em;
  border-bottom: 1px solid var(--border);
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
  transition: color var(--transition);
}
.count {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  margin-top: 0.15em;
}
.header-buttons {
  display: flex;
  align-items: center;
  gap: 0.2em;
}
.new-playlist input {
  width: 100%;
  padding: 0.5em 0.8em;
  border: 1px solid var(--text);
  border-radius: var(--radius-sm);
  background: var(--control-bg);
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  outline: none;
}
.count {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.playlist-button.drop-target {
  background-color: var(--accent-bg-strong);
  box-shadow: inset 0 0 0 1px var(--accent);
}
.stats-thumb {
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #1e3264, #1ed760);
  color: #fff;
}
.local-thumb {
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #0b4d33, #1ed760);
  color: #fff;
}
.rename-row {
  padding: 0.35em 0.5em;
}
.rename-input {
  width: 100%;
  padding: 0.5em 0.8em;
  border: 1px solid var(--text);
  border-radius: var(--radius-sm);
  background: var(--control-bg);
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  outline: none;
}
.menu {
  position: fixed;
  z-index: 60;
  min-width: 200px;
  padding: 4px;
  border-radius: var(--radius-sm);
  background: var(--surface-hover);
  box-shadow: 0 16px 24px rgba(0, 0, 0, 0.3), 0 6px 8px rgba(0, 0, 0, 0.2);
}
.menu button {
  display: block;
  width: 100%;
  padding: 0.7em 0.8em;
  border: none;
  border-radius: 2px;
  background: none;
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  text-align: left;
  cursor: pointer;
}
.menu button:hover {
  background: var(--highlight-strong);
}
.menu button.danger {
  color: var(--danger);
}
.divider {
  height: 1px;
  margin: 4px 0;
  background: var(--border-strong);
}
</style>
