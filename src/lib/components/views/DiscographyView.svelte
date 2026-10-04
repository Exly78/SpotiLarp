<script module>
  /** @type {Map<string, import("../../types.js").Album[]>} */
  const cache = new Map();
</script>

<script>
  import { untrack } from "svelte";
  import * as api from "../../api.js";
  import { openAlbum, openArtist } from "../../nav.svelte.js";
  import { coverAtLeast, releaseType, releaseYear } from "../../utils.js";
  import MediaCard from "../MediaCard.svelte";
  import Section from "../Section.svelte";

  /** @typedef {import("../../types.js").ReleaseGroup} ReleaseGroup */

  /** @type {{ id: string, name: string, group: ReleaseGroup }} */
  let { id, name, group: initialGroup } = $props();

  /** @type {{ key: ReleaseGroup, label: string }[]} */
  const GROUPS = [
    { key: "all", label: "All" },
    { key: "album", label: "Albums" },
    { key: "single", label: "Singles and EPs" },
    { key: "compilation", label: "Compilations" },
  ];

  let group = $state(untrack(() => initialGroup));
  /** @type {import("../../types.js").Album[]} */
  let releases = $state([]);
  let loading = $state(true);
  let error = $state("");

  $effect(() => {
    const key = `${id}:${group}`;
    const cached = cache.get(key);
    releases = cached ?? [];
    loading = !cached;
    error = "";
    api
      .getArtistDiscography(id, group)
      .then((result) => {
        cache.set(key, result);
        if (`${id}:${group}` === key) releases = result;
      })
      .catch((e) => {
        if (`${id}:${group}` === key && !cached) error = `Couldn't load the releases: ${e}`;
      })
      .finally(() => {
        if (`${id}:${group}` === key) loading = false;
      });
  });
</script>

<header class="header">
  <button type="button" class="artist-link" onclick={() => openArtist({ id, name })}>{name}</button>
  <h1>Discography</h1>
  <div class="chips" role="tablist" aria-label="Release type">
    {#each GROUPS as option (option.key)}
      <button
        type="button"
        role="tab"
        class="chip"
        class:selected={group === option.key}
        aria-selected={group === option.key}
        onclick={() => (group = option.key)}
      >
        {option.label}
      </button>
    {/each}
  </div>
</header>

{#if error}
  <p class="status error">{error}</p>
{:else if loading}
  <p class="status">Loading releases...</p>
{:else if releases.length === 0}
  <p class="status">Nothing here yet.</p>
{:else}
  <Section title={`${releases.length} ${releases.length === 1 ? "release" : "releases"}`} grid>
    {#each releases as release}
      <MediaCard
        image={coverAtLeast(release.images, 200)}
        title={release.name}
        subtitle={[releaseYear(release.release_date), releaseType(release.album_type)].filter(Boolean).join(" • ")}
        onclick={() => openAlbum(release)}
      />
    {/each}
  </Section>
{/if}

<style>
.header {
  margin-bottom: 1.5em;
}
.artist-link {
  padding: 0;
  border: none;
  background: none;
  color: var(--text-muted);
  font-family: inherit;
  font-size: var(--fs-sm);
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.artist-link:hover {
  color: var(--text);
  text-decoration: underline;
}
h1 {
  margin: 0.15em 0 0.6em;
  font-size: var(--fs-xl);
  font-weight: var(--fw-black);
  letter-spacing: -0.02em;
  color: var(--text);
}
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5em;
}
.chip {
  padding: 0.45em 0.95em;
  border: none;
  border-radius: var(--radius-pill);
  background: var(--control-bg);
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  cursor: pointer;
  transition: background-color var(--transition);
}
.chip:hover {
  background: #333;
}
.chip.selected {
  background: var(--text);
  color: #000;
}
.status {
  font-size: var(--fs-sm);
  color: var(--text-muted);
}
.status.error {
  color: var(--danger);
}
</style>
