<script module>
  /** Where the page was scrolled to when it was last taken down (the interface sleeps in the background). */
  let savedScroll = { key: "", top: 0 };
</script>

<script>
  import { onMount } from "svelte";
  import { nav, viewKey } from "../nav.svelte.js";
  import Lyrics from "./Lyrics.svelte";
  import HomeView from "./views/HomeView.svelte";
  import PlaylistView from "./views/PlaylistView.svelte";
  import ArtistView from "./views/ArtistView.svelte";
  import DiscographyView from "./views/DiscographyView.svelte";
  import AlbumView from "./views/AlbumView.svelte";
  import SearchView from "./views/SearchView.svelte";
  import SettingsView from "./views/SettingsView.svelte";
  import StatsView from "./views/StatsView.svelte";
  import LocalFilesView from "./views/LocalFilesView.svelte";
  import RadioView from "./views/RadioView.svelte";
  import ShowView from "./views/ShowView.svelte";

  /** @type {{ loggedIn: boolean, loggingIn?: boolean, onRelogin: () => void, onChangeClientId: () => void }} */
  let { loggedIn, loggingIn = false, onRelogin, onChangeClientId } = $props();

  /** @type {HTMLDivElement|undefined} */
  let scroller = $state();
  const key = $derived(viewKey(nav.view));
  // The artist page's banner runs to the panel's edges, like Spotify's.
  const fullBleed = $derived(!nav.lyrics && nav.view.type === "artist");

  $effect(() => {
    key;
    if (scroller) scroller.scrollTop = 0;
  });

  onMount(() => {
    if (savedScroll.key === key && savedScroll.top > 0) {
      const top = savedScroll.top;
      requestAnimationFrame(() => {
        if (scroller) scroller.scrollTop = top;
      });
    }
    return () => {
      savedScroll = { key, top: scroller?.scrollTop ?? 0 };
    };
  });
</script>

<div class="main-content">
  <div class="results-area" class:full-bleed={fullBleed} bind:this={scroller}>
    <div class="lyrics-wrap" class:hidden={!nav.lyrics}>
      <Lyrics visible={nav.lyrics} />
    </div>
    {#if !nav.lyrics}
      {#key key}
        {#if nav.view.type === "playlist"}
          <PlaylistView playlist={nav.view.playlist} />
        {:else if nav.view.type === "artist"}
          <ArtistView id={nav.view.id} name={nav.view.name} />
        {:else if nav.view.type === "discography"}
          <DiscographyView id={nav.view.id} name={nav.view.name} group={nav.view.group} />
        {:else if nav.view.type === "album"}
          <AlbumView id={nav.view.id} name={nav.view.name} />
        {:else if nav.view.type === "search"}
          <SearchView query={nav.view.query} />
        {:else if nav.view.type === "settings"}
          <SettingsView {onChangeClientId} />
        {:else if nav.view.type === "stats"}
          <StatsView />
        {:else if nav.view.type === "local"}
          <LocalFilesView />
        {:else if nav.view.type === "radio"}
          <RadioView seed={nav.view.seed} name={nav.view.name} image={nav.view.image} />
        {:else if nav.view.type === "show"}
          <ShowView id={nav.view.id} name={nav.view.name} />
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
/* Cancels .content-area's padding (+page.svelte); the view pads itself. */
.results-area.full-bleed {
  margin: -1.5em -2em;
  padding-right: 0;
}
</style>
