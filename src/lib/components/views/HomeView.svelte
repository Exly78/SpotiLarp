<script>
  import { untrack } from "svelte";
  import * as api from "../../api.js";
  import { playFromList } from "../../queue.svelte.js";
  import { refreshLiked } from "../../likes.svelte.js";
  import { library } from "../../library.svelte.js";
  import { navigate, openArtist } from "../../nav.svelte.js";
  import { coverAtLeast, smallestCover } from "../../utils.js";
  import { LIKED_SONGS_ID } from "../../constants.js";
  import TrackList from "../TrackList.svelte";
  import MediaCard from "../MediaCard.svelte";
  import Section from "../Section.svelte";

  /** @type {{ loggedIn: boolean, loggingIn?: boolean, onRelogin: () => void }} */
  let { loggedIn, loggingIn = false, onRelogin } = $props();

  const RECENT_TILES = 8;
  const PLAYLIST_CARDS = 12;

  /** @type {import("../../types.js").Track[]} */
  let recent = $state([]);
  /** @type {import("../../types.js").Track[]} */
  let topTracks = $state([]);
  /** @type {import("../../types.js").ArtistDetails[]} */
  let topArtists = $state([]);

  const greeting = $derived.by(() => {
    const hour = new Date().getHours();
    return hour < 12 ? "Good morning" : hour < 18 ? "Good afternoon" : "Good evening";
  });

  const likedSongs = {
    id: LIKED_SONGS_ID,
    name: "Liked Songs",
    isLikedSongs: true,
    images: [],
    track_count: { total: 0 },
  };

  let loadedForLogin = false;
  $effect(() => {
    if (!loggedIn) {
      loadedForLogin = false;
      recent = [];
      topTracks = [];
      topArtists = [];
    } else if (!loadedForLogin) {
      loadedForLogin = true;
      untrack(load);
    }
  });

  function load() {
    api
      .getRecentlyPlayed()
      .then((tracks) => (recent = tracks))
      .catch(() => {});
    api
      .getTopTracks("short_term", 20)
      .then((tracks) => {
        topTracks = tracks;
        refreshLiked(tracks.map((t) => t.id));
      })
      .catch(() => {});
    api
      .getTopArtists("medium_term", 12)
      .then((artists) => (topArtists = artists))
      .catch(() => {});
  }
</script>

<h1 class="greeting">{greeting}</h1>

{#if !loggedIn && loggingIn}
  <p class="status">
    Finish logging in in your browser. If Spotify shows an error like <span class="mono">INVALID_CLIENT</span>,
    check your Client ID and that your app's Redirect URI is exactly
    <span class="mono">http://127.0.0.1:8898/callback</span>, then cancel and try again.
  </p>
{:else if !loggedIn}
  <p class="status">Log in with Spotify to see your music here.</p>
{:else}
  {#if library.missingScopes.length > 0}
    <div class="banner">
      <span>Log in again to see your listening history and edit your playlists.</span>
      <button type="button" class="banner-button" onclick={onRelogin}>Log in again</button>
    </div>
  {/if}

  {#if recent.length > 0}
    <div class="tiles">
      {#each recent.slice(0, RECENT_TILES) as track, i}
        <button type="button" class="tile" onclick={() => playFromList(recent, i, "Recently played")}>
          {#if smallestCover(track.album.images)}
            <img src={smallestCover(track.album.images)} alt="" />
          {:else}
            <div class="tile-placeholder"></div>
          {/if}
          <span class="tile-name">{track.name}</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if topTracks.length > 0}
    <Section title="Your top tracks this month">
      <TrackList tracks={topTracks.slice(0, 10)} contextName="Your top tracks" />
    </Section>
  {/if}

  {#if topArtists.length > 0}
    <Section title="Your top artists" grid>
      {#each topArtists as artist (artist.id)}
        <MediaCard
          image={coverAtLeast(artist.images, 200)}
          title={artist.name}
          subtitle="Artist"
          round
          onclick={() => openArtist(artist)}
        />
      {/each}
    </Section>
  {/if}

  <Section title="Your playlists" grid>
    <MediaCard
      image={null}
      title="Liked Songs"
      subtitle="Playlist"
      onclick={() => navigate({ type: "playlist", playlist: likedSongs })}
    />
    {#each library.playlists.slice(0, PLAYLIST_CARDS) as playlist (playlist.id)}
      <MediaCard
        image={coverAtLeast(playlist.images, 200)}
        title={playlist.name}
        subtitle={playlist.owner?.display_name ? `By ${playlist.owner.display_name}` : "Playlist"}
        onclick={() => navigate({ type: "playlist", playlist })}
      />
    {/each}
  </Section>
{/if}

<style>
.greeting {
  margin: 0 0 0.8em;
  font-size: var(--fs-xl);
  font-weight: var(--fw-black);
  letter-spacing: -0.02em;
  color: var(--text);
}
.status {
  max-width: 60ch;
  font-size: var(--fs-sm);
  line-height: 1.5;
  color: var(--text-muted);
}
.mono {
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
  color: var(--text);
}
.banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1em;
  margin-bottom: 1.5em;
  padding: 0.8em 1em;
  border-radius: var(--radius-md);
  background: var(--accent-bg);
  font-size: var(--fs-sm);
  color: var(--text);
}
.banner-button {
  flex-shrink: 0;
  padding: 0.45em 1.1em;
  border: none;
  border-radius: var(--radius-pill);
  background: var(--accent);
  color: var(--accent-text);
  font-family: inherit;
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.tiles {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 0.6em;
  margin-bottom: 2em;
}
.tile {
  display: flex;
  align-items: center;
  gap: 0.8em;
  min-width: 0;
  height: 56px;
  padding: 0 0.8em 0 0;
  overflow: hidden;
  border: none;
  border-radius: var(--radius-sm);
  background: rgba(255, 255, 255, 0.07);
  color: var(--text);
  font: inherit;
  text-align: left;
  cursor: pointer;
  transition: background-color var(--transition);
}
.tile:hover {
  background: rgba(255, 255, 255, 0.14);
}
.tile img,
.tile-placeholder {
  width: 56px;
  height: 56px;
  flex-shrink: 0;
  object-fit: cover;
  background: var(--surface-hover);
}
.tile-name {
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
