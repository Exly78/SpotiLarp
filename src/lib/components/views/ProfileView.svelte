<script module>
  import { BoundedCache } from "../../boundedCache.js";

  // Profiles already seen, for going back to one.
  /** @type {BoundedCache<string, import("../../types.js").UserProfile>} */
  const cache = new BoundedCache(10);
</script>

<script>
  import { onMount, untrack } from "svelte";
  import * as api from "../../api.js";
  import { navigate, openArtist, openPlaylistCard, openProfile, playlistFromCard } from "../../nav.svelte.js";
  import { playPlaylist } from "../../collections.js";
  import { coverAtLeast, formatCount } from "../../utils.js";
  import { cached, MINUTE } from "../../requestCache.js";
  import PageHeader from "../PageHeader.svelte";
  import MediaCard from "../MediaCard.svelte";
  import Section from "../Section.svelte";
  import Shelf from "../Shelf.svelte";
  import TrackList from "../TrackList.svelte";

  /** @typedef {import("../../types.js").ProfileEntry} ProfileEntry */
  /** @typedef {import("../../types.js").ProfilePlaylist} ProfilePlaylist */

  /** @type {{ id: string, name?: string }} */
  let { id, name = "" } = $props();

  const TOP_ARTISTS = 20;
  const TOP_TRACKS = 4;
  /** Home asks for the same top tracks, so this shares its saved answer. */
  const TOP_TRACKS_ASKED = 20;

  let profile = $state(untrack(() => cache.get(id)) ?? null);
  let error = $state("");
  /** @type {import("../../types.js").ArtistDetails[]} */
  let topArtists = $state([]);
  /** @type {import("../../types.js").Track[]} */
  let topTracks = $state([]);

  const view = $derived.by(() => /** @type {import("../../nav.svelte.js").View} */ ({ type: "profile", id, name: profile?.name ?? name }));

  onMount(async () => {
    try {
      const found = await api.getUserProfile(id);
      cache.set(id, found);
      profile = found;
      // Spotify only shows you your own top lists.
      if (found.is_current_user) loadTop();
    } catch (e) {
      if (!profile) error = `Failed to load the profile: ${e}`;
    }
  });

  async function loadTop() {
    const [artists, tracks] = await Promise.all([
      cached(`top-artists:short_term:${TOP_ARTISTS}`, 60 * MINUTE, () => api.getTopArtists("short_term", TOP_ARTISTS)).catch(() => []),
      cached(`top-tracks:short_term:${TOP_TRACKS_ASKED}`, 60 * MINUTE, () => api.getTopTracks("short_term", TOP_TRACKS_ASKED)).catch(() => []),
    ]);
    topArtists = artists;
    topTracks = tracks.slice(0, TOP_TRACKS);
  }

  /** @param {ProfileEntry} entry */
  function openEntry(entry) {
    if (entry.kind === "artist") openArtist({ id: entry.id, name: entry.name });
    else openProfile({ id: entry.id, name: entry.name });
  }

  /** @param {ProfileEntry} entry */
  function entrySubtitle(entry) {
    if (entry.kind === "artist") return "Artist";
    return entry.followers != null ? `${formatCount(entry.followers)} ${entry.followers === 1 ? "follower" : "followers"}` : "Profile";
  }

  /** @param {ProfilePlaylist} playlist */
  function playlistCard(playlist) {
    return {
      id: playlist.id,
      name: playlist.name,
      description: "",
      images: playlist.images.slice(0, 1).map((url) => ({ url, width: null, height: null })),
      owner: playlist.owner,
    };
  }

  /** @param {number} n @param {string} one @param {string} many */
  function counted(n, one, many) {
    return `${formatCount(n)} ${n === 1 ? one : many}`;
  }
</script>

{#if error}
  <p class="error">{error}</p>
{:else if !profile}
  <PageHeader label="Profile" title={name || "Loading..."} round />
{:else}
  <PageHeader image={profile.image} round label="Profile" title={profile.name}>
    {#snippet meta()}
      {#if profile}
        {[
          counted(profile.playlist_count, "Public Playlist", "Public Playlists"),
          counted(profile.followers, "Follower", "Followers"),
          `${formatCount(profile.following)} Following`,
        ].join(" • ")}
      {/if}
    {/snippet}
  </PageHeader>

  {#if topArtists.length > 0}
    <Shelf title="Top artists this month" items={topArtists} onShowAll={() => navigate({ type: "stats" })}>
      {#snippet card(artist)}
        <MediaCard
          image={coverAtLeast(artist.images, 200)}
          title={artist.name}
          subtitle="Artist"
          round
          onclick={() => openArtist(artist)}
        />
      {/snippet}
    </Shelf>
    <p class="private-note">Only visible to you</p>
  {/if}

  {#if topTracks.length > 0}
    <Section title="Top tracks this month">
      {#snippet action()}
        <button type="button" class="show-all" onclick={() => navigate({ type: "stats" })}>Show all</button>
      {/snippet}
      <TrackList tracks={topTracks} contextName="Top tracks this month" contextView={view} showHeader={false} />
    </Section>
    <p class="private-note">Only visible to you</p>
  {/if}

  {#if profile.playlists.length > 0}
    <Shelf title="Public Playlists" items={profile.playlists}>
      {#snippet card(playlist)}
        <MediaCard
          image={playlist.images[0] ?? null}
          title={playlist.name}
          subtitle={playlist.owner ? `By ${playlist.owner}` : "Playlist"}
          onclick={() => openPlaylistCard(playlistCard(playlist))}
          onplay={() => playPlaylist(playlistFromCard(playlistCard(playlist)))}
        />
      {/snippet}
    </Shelf>
  {/if}

  {#if profile.recently_played_artists.length > 0}
    <Shelf title="Recently played artists" items={profile.recently_played_artists}>
      {#snippet card(entry)}
        <MediaCard image={entry.image} title={entry.name} subtitle={entrySubtitle(entry)} round onclick={() => openEntry(entry)} />
      {/snippet}
    </Shelf>
  {/if}

  {#if profile.following_list.length > 0}
    <Shelf title="Following" items={profile.following_list}>
      {#snippet card(entry)}
        <MediaCard image={entry.image} title={entry.name} subtitle={entrySubtitle(entry)} round onclick={() => openEntry(entry)} />
      {/snippet}
    </Shelf>
  {/if}

  {#if profile.followers_list.length > 0}
    <Shelf title="Followers" items={profile.followers_list}>
      {#snippet card(entry)}
        <MediaCard image={entry.image} title={entry.name} subtitle={entrySubtitle(entry)} round onclick={() => openEntry(entry)} />
      {/snippet}
    </Shelf>
  {/if}
{/if}

<style>
.error {
  color: var(--danger);
}
.private-note {
  margin: -1.2em 0 2em;
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
.show-all {
  padding: 0;
  border: none;
  background: none;
  color: var(--text-muted);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.show-all:hover {
  color: var(--text);
  text-decoration: underline;
}
</style>
