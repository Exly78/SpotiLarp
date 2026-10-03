<script>
  import { onMount } from "svelte";
  import * as api from "../../api.js";
  import { playFromList } from "../../queue.svelte.js";
  import { refreshLiked } from "../../likes.svelte.js";
  import { openAlbum } from "../../nav.svelte.js";
  import { notify } from "../../toast.svelte.js";
  import { coverAtLeast, releaseYear } from "../../utils.js";
  import TrackList from "../TrackList.svelte";
  import PageHeader from "../PageHeader.svelte";
  import PlayButton from "../PlayButton.svelte";
  import MediaCard from "../MediaCard.svelte";
  import Section from "../Section.svelte";

  /** @type {{ id: string, name: string }} */
  let { id, name } = $props();

  const POPULAR_PREVIEW = 5;

  /** @type {import("../../types.js").ArtistDetails|null} */
  let artist = $state(null);
  /** @type {import("../../types.js").Track[]} */
  let popular = $state([]);
  /** @type {import("../../types.js").Album[]} */
  let releases = $state([]);
  let following = $state(false);
  let followBusy = $state(false);
  let showAllPopular = $state(false);
  let error = $state("");

  const albums = $derived(releases.filter((r) => r.album_type !== "single"));
  const singles = $derived(releases.filter((r) => r.album_type === "single"));

  onMount(() => {
    api.getArtist(id).then((a) => (artist = a)).catch((e) => (error = `Failed to load artist: ${e}`));
    api
      .getArtistPopularTracks(id, name)
      .then((tracks) => {
        popular = tracks;
        refreshLiked(tracks.map((t) => t.id));
      })
      .catch(() => {});
    api.getArtistAlbums(id).then((r) => (releases = r)).catch(() => {});
    api.isFollowingArtist(id).then((f) => (following = f)).catch(() => {});
  });

  async function toggleFollow() {
    if (followBusy) return;
    followBusy = true;
    try {
      if (following) await api.unfollowArtist(id);
      else await api.followArtist(id);
      following = !following;
    } catch (e) {
      notify(`Couldn't update follow: ${e}`);
    } finally {
      followBusy = false;
    }
  }
</script>

{#if error && !artist}
  <p class="error">{error}</p>
{:else}
  <PageHeader image={coverAtLeast(artist?.images, 300)} round label="Artist" title={artist?.name ?? name}>
    {#snippet meta()}
      {#if artist?.genres.length}{artist.genres.slice(0, 3).join(" · ")}{/if}
    {/snippet}
    {#snippet actions()}
      <PlayButton
        onclick={() => playFromList(popular, 0, artist?.name ?? name)}
        disabled={popular.length === 0}
        label={`Play ${artist?.name ?? name}`}
      />
      <button type="button" class="follow-button" class:following disabled={followBusy} onclick={toggleFollow}>
        {following ? "Following" : "Follow"}
      </button>
    {/snippet}
  </PageHeader>

  {#if popular.length > 0}
    <Section title="Popular">
      {#snippet action()}
        {#if popular.length > POPULAR_PREVIEW}
          <button type="button" class="more" onclick={() => (showAllPopular = !showAllPopular)}>
            {showAllPopular ? "Show less" : "See more"}
          </button>
        {/if}
      {/snippet}
      <TrackList
        tracks={showAllPopular ? popular : popular.slice(0, POPULAR_PREVIEW)}
        contextName={artist?.name ?? name}
      />
    </Section>
  {/if}

  {#if albums.length > 0}
    <Section title="Albums" grid>
      {#each albums as album (album.id)}
        <MediaCard
          image={coverAtLeast(album.images, 200)}
          title={album.name}
          subtitle={releaseYear(album.release_date)}
          onclick={() => openAlbum(album)}
        />
      {/each}
    </Section>
  {/if}

  {#if singles.length > 0}
    <Section title="Singles and EPs" grid>
      {#each singles as single (single.id)}
        <MediaCard
          image={coverAtLeast(single.images, 200)}
          title={single.name}
          subtitle={releaseYear(single.release_date)}
          onclick={() => openAlbum(single)}
        />
      {/each}
    </Section>
  {/if}
{/if}

<style>
.error {
  font-size: var(--fs-sm);
  color: var(--danger);
}
.follow-button {
  padding: 0.45em 1.1em;
  border: 1px solid var(--text-muted);
  border-radius: var(--radius-pill);
  background: none;
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
  transition: border-color var(--transition), color var(--transition);
}
.follow-button:hover {
  border-color: var(--text);
}
.follow-button.following {
  border-color: var(--accent);
  color: var(--accent);
}
.follow-button:disabled {
  opacity: 0.6;
  cursor: default;
}
.more {
  padding: 0;
  border: none;
  background: none;
  color: var(--text-muted);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.more:hover {
  color: var(--text);
  text-decoration: underline;
}
</style>
