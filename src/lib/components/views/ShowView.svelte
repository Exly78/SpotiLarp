<script>
  import { onMount, untrack } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import * as api from "../../api.js";
  import { queue, playFromList, addManyToQueue } from "../../queue.svelte.js";
  import { player, togglePlay } from "../../player.svelte.js";
  import { isShowSaved, saveShow, removeShow } from "../../library.svelte.js";
  import { episodeTrack, isPlayed, setPlayed } from "../../podcasts.svelte.js";
  import { notify } from "../../toast.svelte.js";
  import { coverAtLeast } from "../../utils.js";
  import PageHeader from "../PageHeader.svelte";
  import PlayButton from "../PlayButton.svelte";
  import MoreMenu from "../MoreMenu.svelte";
  import EpisodeList from "../EpisodeList.svelte";

  /** @type {{ id: string, name?: string }} */
  let { id, name = "" } = $props();

  const PAGE_SIZE = 50;
  /** Shows you listen to from the first episode, by id. */
  const OLDEST_FIRST_KEY = "spotilarp.showsOldestFirst";

  /** @type {import("../../types.js").Show|null} */
  let show = $state(null);
  /** @type {import("../../types.js").Track[]} in the order shown */
  let episodes = $state([]);
  // Spotify's list is read in slots; it leaves out episodes that can't play
  // here, so slots read and episodes shown can differ.
  let slotsRead = $state(0);
  let total = $state(0);
  let loading = $state(true);
  let loadingMore = $state(false);
  let error = $state("");
  let oldestFirst = $state(untrack(() => readOldestFirst().includes(id)));
  let hidePlayed = $state(false);
  let aboutOpen = $state(false);
  let saveBusy = $state(false);

  const saved = $derived(isShowSaved(id));
  const title = $derived.by(() => show?.name ?? name);
  const view = $derived(/** @type {import("../../nav.svelte.js").View} */ ({ type: "show", id, name: title }));
  const shown = $derived(hidePlayed ? episodes.filter((e) => !isPlayed(e)) : episodes);
  const hasMore = $derived(slotsRead < total);
  const playingHere = $derived(
    queue.contextView?.type === "show" && queue.contextView.id === id && queue.current?.uri === player.track?.uri,
  );
  const link = $derived(`https://open.spotify.com/show/${id}`);

  /** @returns {string[]} */
  function readOldestFirst() {
    try {
      const saved = JSON.parse(localStorage.getItem(OLDEST_FIRST_KEY) ?? "[]");
      return Array.isArray(saved) ? saved : [];
    } catch {
      return [];
    }
  }

  onMount(async () => {
    try {
      show = await api.getShow(id);
      total = show.total_episodes ?? 0;
      await loadMore();
    } catch (e) {
      error = `Couldn't load this podcast: ${e}`;
    } finally {
      loading = false;
    }
  });

  /** @param {import("../../types.js").Episode} episode */
  function toTrack(episode) {
    return episodeTrack(episode, show);
  }

  async function loadMore() {
    if (!show) return;
    if (oldestFirst) {
      if (total === 0) total = (await api.getShowEpisodes(id, 0, 1)).total;
      // Spotify lists newest first, so the oldest are read from the end.
      const remaining = total - slotsRead;
      if (remaining <= 0) return;
      const offset = Math.max(0, remaining - PAGE_SIZE);
      const page = await api.getShowEpisodes(id, offset, remaining - offset);
      slotsRead += remaining - offset;
      episodes = [...episodes, ...page.items.map(toTrack).reverse()];
    } else {
      const page = await api.getShowEpisodes(id, slotsRead, PAGE_SIZE);
      total = page.total;
      slotsRead = Math.min(slotsRead + PAGE_SIZE, total);
      episodes = [...episodes, ...page.items.map(toTrack)];
    }
  }

  async function showMore() {
    if (loadingMore) return;
    loadingMore = true;
    try {
      await loadMore();
    } catch (e) {
      notify(`Couldn't load more episodes: ${e}`);
    } finally {
      loadingMore = false;
    }
  }

  /** @param {boolean} oldest */
  async function setOrder(oldest) {
    if (oldest === oldestFirst || loadingMore) return;
    oldestFirst = oldest;
    const others = readOldestFirst().filter((saved) => saved !== id);
    try {
      localStorage.setItem(OLDEST_FIRST_KEY, JSON.stringify(oldest ? [...others, id] : others));
    } catch {}
    episodes = [];
    slotsRead = 0;
    await showMore();
  }

  // Play carries on where you are: the first episode (in the order shown)
  // you haven't finished.
  function play() {
    if (playingHere) {
      togglePlay();
      return;
    }
    if (shown.length === 0) return;
    const next = shown.findIndex((e) => !isPlayed(e) && e.playable !== false);
    playFromList(shown, Math.max(next, 0), title, view);
  }

  async function toggleSaved() {
    if (!show || saveBusy) return;
    saveBusy = true;
    try {
      if (saved) {
        await removeShow(id);
        notify("Removed from Your Library");
      } else {
        await saveShow(show);
        notify("Added to Your Library");
      }
    } catch (e) {
      notify(`Couldn't update Your Library: ${e}`);
    } finally {
      saveBusy = false;
    }
  }

  function markAllPlayed() {
    for (const episode of episodes) if (!isPlayed(episode)) setPlayed(episode, true);
    notify(hasMore ? "Marked the loaded episodes as played" : "Marked every episode as played");
  }

  async function copyLink() {
    try {
      await navigator.clipboard.writeText(link);
      notify("Link copied");
    } catch (e) {
      notify(`Couldn't copy the link: ${e}`);
    }
  }
</script>

{#if error}
  <p class="status error">{error}</p>
{:else if !show}
  <PageHeader icon="podcast" label="Podcast" title={name || "Loading..."} />
{:else}
  <PageHeader image={coverAtLeast(show.images, 300)} icon={show.images.length ? null : "podcast"} label="Podcast" {title}>
    {#snippet meta()}
      {total} {total === 1 ? "episode" : "episodes"}
    {/snippet}
    {#snippet actions()}
      <PlayButton
        onclick={play}
        playing={playingHere && player.isPlaying}
        disabled={loading || (shown.length === 0 && !playingHere)}
        label={`Play ${title}`}
      />
      <button type="button" class="follow-button" disabled={saveBusy} onclick={toggleSaved}>
        {saved ? "Following" : "Follow"}
      </button>
      <MoreMenu
        label={`More options for ${title}`}
        items={[
          { label: oldestFirst ? "Newest first" : "Oldest first", onclick: () => setOrder(!oldestFirst), disabled: loadingMore },
          { label: hidePlayed ? "Show played episodes" : "Hide played episodes", onclick: () => (hidePlayed = !hidePlayed) },
          { label: "Add loaded episodes to queue", onclick: () => addManyToQueue(shown), disabled: shown.length === 0 },
          { label: "Mark all as played", onclick: markAllPlayed, disabled: episodes.length === 0 },
          null,
          { label: saved ? "Unfollow" : "Follow", onclick: toggleSaved },
          { label: "Copy link to podcast", onclick: copyLink },
          { label: "Open in Spotify", onclick: () => openUrl(link).catch((e) => notify(`Couldn't open Spotify: ${e}`)) },
        ]}
      />
    {/snippet}
  </PageHeader>

  {#if show.description}
    <section class="about">
      <h2>About</h2>
      <p class="description" class:open={aboutOpen}>{show.description}</p>
      {#if show.description.length > 240}
        <button type="button" class="text-button" onclick={() => (aboutOpen = !aboutOpen)}>
          {aboutOpen ? "Show less" : "Show more"}
        </button>
      {/if}
    </section>
  {/if}

  <div class="list-heading">
    <h2>{oldestFirst ? "Episodes, oldest first" : "Episodes"}</h2>
    {#if hidePlayed}<span class="hint">Played episodes hidden</span>{/if}
  </div>
  {#if loading}
    <p class="status">Loading episodes...</p>
  {:else if episodes.length === 0 && !hasMore}
    <p class="status">This podcast has no episodes you can play here.</p>
  {:else}
    {#if shown.length === 0}
      <p class="status">You've played every episode loaded so far.</p>
    {/if}
    <EpisodeList episodes={shown} contextName={title} contextView={view} />
    {#if hasMore}
      <button type="button" class="more" onclick={showMore} disabled={loadingMore}>
        {loadingMore ? "Loading..." : "Load more episodes"}
      </button>
    {/if}
  {/if}
{/if}

<style>
.status {
  font-size: var(--fs-sm);
  color: var(--text-muted);
}
.status.error {
  color: var(--danger);
}
.follow-button {
  padding: 0.45em 1.1em;
  border: 1px solid color-mix(in srgb, var(--text) 50%, transparent);
  border-radius: var(--radius-pill);
  background: none;
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
  transition: border-color var(--transition), transform var(--transition);
}
.follow-button:hover {
  border-color: var(--text);
  transform: scale(1.04);
}
.follow-button:disabled {
  opacity: 0.6;
  cursor: default;
}
h2 {
  margin: 0 0 0.6em;
  font-size: var(--fs-md);
  font-weight: var(--fw-bold);
  color: var(--text);
}
.about {
  max-width: 80ch;
  margin-bottom: 1.75em;
}
.description {
  margin: 0;
  font-size: var(--fs-sm);
  line-height: 1.55;
  color: var(--text-dim);
  white-space: pre-line;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  line-clamp: 3;
  -webkit-box-orient: vertical;
}
.description.open {
  display: block;
}
.text-button {
  margin-top: 0.4em;
  padding: 0;
  border: none;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.text-button:hover {
  text-decoration: underline;
}
.list-heading {
  display: flex;
  align-items: baseline;
  gap: 0.8em;
}
.hint {
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
.more {
  margin: 0.75em 0 0 0.6em;
  padding: 0.45em 1.2em;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-pill);
  background: none;
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.more:hover:not(:disabled) {
  border-color: var(--text);
}
.more:disabled {
  opacity: 0.6;
  cursor: default;
}
</style>
