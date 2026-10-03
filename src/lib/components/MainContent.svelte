<script>
  import { nav, viewKey } from "../nav.svelte.js";
  import Lyrics from "./Lyrics.svelte";
  import HomeView from "./views/HomeView.svelte";
  import PlaylistView from "./views/PlaylistView.svelte";
  import ArtistView from "./views/ArtistView.svelte";
  import AlbumView from "./views/AlbumView.svelte";
  import SearchView from "./views/SearchView.svelte";
  import SettingsView from "./views/SettingsView.svelte";
  import StatsView from "./views/StatsView.svelte";

  /** @type {{ loggedIn: boolean, loggingIn?: boolean, onRelogin: () => void }} */
  let { loggedIn, loggingIn = false, onRelogin } = $props();

  /** @type {HTMLDivElement|undefined} */
  let scroller = $state();
  const key = $derived(viewKey(nav.view));

  $effect(() => {
    key;
    if (scroller) scroller.scrollTop = 0;
  });
</script>

<div class="main-content">
  <div class="results-area" bind:this={scroller}>
    <div class="lyrics-wrap" class:hidden={!nav.lyrics}>
      <Lyrics visible={nav.lyrics} />
    </div>
    {#if !nav.lyrics}
      {#key key}
        {#if nav.view.type === "playlist"}
          <PlaylistView playlist={nav.view.playlist} />
        {:else if nav.view.type === "artist"}
          <ArtistView id={nav.view.id} name={nav.view.name} />
        {:else if nav.view.type === "album"}
          <AlbumView id={nav.view.id} name={nav.view.name} />
        {:else if nav.view.type === "search"}
          <SearchView query={nav.view.query} />
        {:else if nav.view.type === "settings"}
          <SettingsView />
        {:else if nav.view.type === "stats"}
          <StatsView />
        {:else}
          <HomeView {loggedIn} {loggingIn} {onRelogin} />
        {/if}
      {/key}
    {/if}
  </div>
</div>

<style>
.main-content {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  min-width: 0;
}
.lyrics-wrap {
  width: 100%;
  height: 100%;
}
.lyrics-wrap.hidden {
  display: none;
}
.results-area {
  flex: 1;
  overflow-x: hidden;
  overflow-y: auto;
  padding-right: 0.25em;
}
</style>
