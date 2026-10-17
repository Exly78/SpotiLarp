<script>
  import { onMount } from "svelte";
  import * as api from "../../api.js";
  import { playFromList, addManyToQueue } from "../../queue.svelte.js";
  import { isAlbumSaved, saveAlbum, removeAlbum } from "../../library.svelte.js";
  import { openArtist, openRadio } from "../../nav.svelte.js";
  import { notify } from "../../toast.svelte.js";
  import { formatDuration, coverAtLeast, releaseYear } from "../../utils.js";
  import TrackList from "../TrackList.svelte";
  import PageHeader from "../PageHeader.svelte";
  import PlayButton from "../PlayButton.svelte";
  import Icon from "../Icon.svelte";
  import ShuffleButton from "../ShuffleButton.svelte";
  import MoreMenu from "../MoreMenu.svelte";

  /** @type {{ id: string, name?: string }} */
  let { id, name = "" } = $props();

  /** @type {import("../../types.js").AlbumDetails|null} */
  let album = $state(null);
  let error = $state("");
  let saveBusy = $state(false);

  const saved = $derived(isAlbumSaved(id));
  const view = $derived.by(() => /** @type {import("../../nav.svelte.js").View} */ ({ type: "album", id, name: album?.name ?? name }));

  const totalMs = $derived.by(() => (album ? album.tracks.reduce((sum, t) => sum + t.duration_ms, 0) : 0));
  const label = $derived.by(() =>
    album?.album_type === "single" ? "Single" : album?.album_type === "compilation" ? "Compilation" : "Album",
  );

  onMount(async () => {
    try {
      album = await api.getAlbum(id);
    } catch (e) {
      error = `Failed to load album: ${e}`;
    }
  });

  function playAll() {
    if (!album || album.tracks.length === 0) return;
    playFromList(album.tracks, -1, album.name, view);
  }

  async function copyLink() {
    try {
      await navigator.clipboard.writeText(`https://open.spotify.com/album/${id}`);
      notify("Link copied");
    } catch (e) {
      notify(`Couldn't copy the link: ${e}`);
    }
  }

  async function toggleSaved() {
    if (!album || saveBusy) return;
    saveBusy = true;
    try {
      if (saved) {
        await removeAlbum(id);
        notify("Removed from Your Library");
      } else {
        await saveAlbum(album);
        notify("Added to Your Library");
      }
    } catch (e) {
      notify(`Couldn't update Your Library: ${e}`);
    } finally {
      saveBusy = false;
    }
  }
</script>

{#if error}
  <p class="error">{error}</p>
{:else if !album}
  <PageHeader label="Album" title={name || "Loading..."} />
{:else}
  <PageHeader image={coverAtLeast(album.images, 300)} {label} title={album.name}>
    {#snippet meta()}
      {#each album?.artists ?? [] as artist, i}
        {#if i > 0}, {/if}<button type="button" class="link" onclick={() => openArtist(artist)}>{artist.name}</button>
      {/each}
      {#if album?.release_date} · {releaseYear(album.release_date)}{/if}
      · {album?.tracks.length} {album?.tracks.length === 1 ? "song" : "songs"}, {formatDuration(totalMs)}
    {/snippet}
    {#snippet actions()}
      <PlayButton onclick={playAll} label={`Play ${album?.name}`} />
      <button
        type="button"
        class="save-button"
        class:saved
        disabled={saveBusy}
        onclick={toggleSaved}
        aria-label={saved ? "Remove from Your Library" : "Save to Your Library"}
        title={saved ? "Remove from Your Library" : "Save to Your Library"}
      >
        <Icon name={saved ? "added" : "add-circle"} size={32} />
      </button>
      <ShuffleButton />
      <MoreMenu
        label={`More options for ${album?.name}`}
        items={[
          { label: "Add to queue", onclick: () => album && addManyToQueue(album.tracks) },
          { label: "Go to album radio", onclick: () => album && openRadio(`spotify:album:${id}`, album.name, coverAtLeast(album.images, 300)) },
          null,
          { label: saved ? "Remove from Your Library" : "Save to Your Library", onclick: toggleSaved },
          { label: "Copy link to album", onclick: copyLink },
        ]}
      />
    {/snippet}
  </PageHeader>
  <TrackList tracks={album.tracks} contextName={album.name} contextView={view} primary showAlbum={false} showCovers={false} />
{/if}

<style>
.error {
  font-size: var(--fs-sm);
  color: var(--danger);
}
.link {
  padding: 0;
  border: none;
  background: none;
  color: var(--text);
  font: inherit;
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.link:hover {
  text-decoration: underline;
}
.save-button {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: none;
  background: none;
  color: var(--text-muted);
  cursor: pointer;
  transition: color var(--transition), transform var(--transition);
}
.save-button:hover:not(:disabled) {
  color: var(--text);
  transform: scale(1.06);
}
.save-button.saved,
.save-button.saved:hover:not(:disabled) {
  color: var(--accent);
}
.save-button:disabled {
  opacity: 0.6;
  cursor: default;
}
</style>
