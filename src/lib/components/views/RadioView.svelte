<script module>
  import { BoundedCache } from "../../boundedCache.js";

  /**
   * Spotify mixes a radio afresh every time it's asked, so a page keeps the
   * songs it showed (going back to it plays the same mix) until Refresh.
   * @type {BoundedCache<string, import("../../types.js").Track[]>}
   */
  const mixes = new BoundedCache(10);

  export function clearRadioCache() {
    mixes.clear();
  }
</script>

<script>
  import { onMount, untrack } from "svelte";
  import * as api from "../../api.js";
  import { playFromList, addManyToQueue } from "../../queue.svelte.js";
  import { createPlaylist, adjustTrackCount } from "../../library.svelte.js";
  import { navigate } from "../../nav.svelte.js";
  import { notify } from "../../toast.svelte.js";
  import { formatDuration } from "../../utils.js";
  import TrackList from "../TrackList.svelte";
  import PageHeader from "../PageHeader.svelte";
  import PlayButton from "../PlayButton.svelte";
  import ShuffleButton from "../ShuffleButton.svelte";
  import MoreMenu from "../MoreMenu.svelte";

  /** @type {{ seed: string, name: string, image?: string|null }} */
  let { seed, name, image = null } = $props();

  /** @type {Record<string, string>} */
  const LABELS = { track: "Song radio", playlist: "Playlist radio", album: "Album radio", artist: "Artist radio" };

  const cached = untrack(() => mixes.get(seed));
  /** @type {import("../../types.js").Track[]} */
  let tracks = $state(cached ?? []);
  let loading = $state(!cached);
  let refreshing = $state(false);
  let saving = $state(false);
  let error = $state("");

  const title = $derived(`${name} Radio`);
  const label = $derived(LABELS[seed.split(":")[1] ?? ""] ?? "Radio");
  const totalMs = $derived(tracks.reduce((sum, t) => sum + t.duration_ms, 0));
  const view = $derived(/** @type {import("../../nav.svelte.js").View} */ ({ type: "radio", seed, name, image }));

  onMount(() => {
    if (!cached) load();
  });

  async function load() {
    try {
      const result = await api.getRadioTracks(seed);
      tracks = result;
      mixes.set(seed, result);
      error = "";
    } catch (e) {
      // A failed refresh keeps the mix on screen.
      if (tracks.length === 0) error = `Couldn't load this radio: ${e}`;
      else notify(`Couldn't get a new mix: ${e}`);
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  function refresh() {
    if (refreshing) return;
    refreshing = true;
    load();
  }

  function playAll() {
    if (tracks.length === 0) return;
    playFromList(tracks, -1, title, view);
  }

  async function saveAsPlaylist() {
    if (saving || tracks.length === 0) return;
    saving = true;
    try {
      const playlist = await createPlaylist(title);
      await api.addToPlaylist(playlist.id, tracks.map((t) => t.uri));
      adjustTrackCount(playlist.id, tracks.length);
      notify(`Saved as "${playlist.name}"`);
      navigate({ type: "playlist", playlist });
    } catch (e) {
      notify(`Couldn't save the radio: ${e}`);
    } finally {
      saving = false;
    }
  }
</script>

<PageHeader {image} icon={image ? null : "radio"} {label} {title}>
  {#snippet meta()}
    Songs Spotify picked to go with {name}
    {#if !loading && tracks.length > 0} · {tracks.length} songs, {formatDuration(totalMs)}{/if}
  {/snippet}
  {#snippet actions()}
    <PlayButton onclick={playAll} disabled={loading || tracks.length === 0} label={`Play ${title}`} />
    <ShuffleButton />
    <MoreMenu
      label={`More options for ${title}`}
      items={[
        { label: refreshing ? "Refreshing..." : "Refresh (new mix)", onclick: refresh, disabled: refreshing || loading },
        { label: "Add to queue", onclick: () => addManyToQueue(tracks), disabled: tracks.length === 0 },
        null,
        { label: saving ? "Saving..." : "Save as playlist", onclick: saveAsPlaylist, disabled: saving || tracks.length === 0 },
      ]}
    />
  {/snippet}
</PageHeader>

{#if loading}
  <p class="status">Mixing your radio...</p>
{:else if error}
  <p class="status error">{error}</p>
{:else}
  <TrackList {tracks} contextName={title} contextView={view} primary />
{/if}

<style>
.status {
  font-size: var(--fs-sm);
  color: var(--text-muted);
}
.status.error {
  color: var(--danger);
}
</style>
