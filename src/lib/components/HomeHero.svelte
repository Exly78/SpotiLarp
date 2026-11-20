<script>
  import { player, togglePlay } from "../player.svelte.js";
  import { queue, contextLabel, autoplaying } from "../queue.svelte.js";
  import { jam } from "../jam.svelte.js";
  import { lastPlayAt } from "../tracker.svelte.js";
  import { navigate, viewKey } from "../nav.svelte.js";
  import { recentContexts, viewImage, isLikedSongs } from "../played.svelte.js";
  import { dominantColor } from "../colors.js";
  import { formatTime, timeAgo } from "../utils.js";
  import Icon from "./Icon.svelte";
  import PlayButton from "./PlayButton.svelte";

  /**
   * Home's big card: what's playing, or what was, and where it's from. With
   * nothing loaded, `fallback` (the last page played from) starts again.
   * @type {{
   *   fallback?: { title: string, kind: string, image: string|null, liked?: boolean, at: number, open: () => void, play?: () => void }|null,
   * }}
   */
  let { fallback = null } = $props();

  /** Liked Songs' purple, like its picture in the library. */
  const LIKED_TINT = "rgb(84, 52, 176)";

  const track = $derived(player.track);
  const guest = $derived(jam.role === "guest");
  const contextView = $derived(guest || autoplaying() ? null : queue.contextView);
  const contextName = $derived(guest ? `${jam.hostName || "Someone"}'s Jam` : contextLabel());
  const contextImage = $derived.by(() => {
    if (!contextView) return null;
    const key = viewKey(contextView);
    return recentContexts.items.find((item) => item.key === key)?.image ?? viewImage(contextView);
  });
  // Liked Songs has no picture of its own: it gets the heart, not a song's cover.
  const liked = $derived(track ? isLikedSongs(contextView) : !!fallback?.liked);
  const image = $derived(liked ? null : track ? contextImage || track.coverUrl : (fallback?.image ?? null));
  const lastPlayedAt = $derived(lastPlayAt());
  const progress = $derived(track && track.durationMs > 0 ? Math.min(1, player.positionMs / track.durationMs) : 0);

  let extractedTint = $state("");
  const tint = $derived(liked ? LIKED_TINT : extractedTint);
  $effect(() => {
    const url = image;
    dominantColor(url).then((color) => {
      if (image === url) extractedTint = color;
    });
  });

  function openContext() {
    if (contextView) navigate(contextView);
  }
</script>

{#snippet cover()}
  {#if liked}
    <div class="cover liked"><Icon name="heart-filled" size={72} /></div>
  {:else if image}
    <img src={image} alt="" class="cover" />
  {:else}
    <div class="cover placeholder"></div>
  {/if}
{/snippet}

<article class="hero" style:--tint={tint || null}>
  {#if track}
    <button type="button" class="cover-button" onclick={openContext} disabled={!contextView} aria-label={contextName ? `Open ${contextName}` : track.name}>
      {@render cover()}
    </button>
    <div class="body">
      <div class="eyebrow">{player.isPlaying ? "Now playing" : "Pick up where you left off"}</div>
      <!-- A page played from leads; a list with no page ("Recently played") just says where the song came from. -->
      {#if contextName && contextView}
        <h2 class="title"><button type="button" class="title-link" onclick={openContext}><span class="title-text">{contextName}</span></button></h2>
        <div class="song"><span class="song-name">{track.name}</span> · {track.artists}</div>
      {:else}
        <h2 class="title">{track.name}</h2>
        <div class="song">{contextName ? `${track.artists} · from ${contextName}` : track.artists}</div>
      {/if}
      <div class="progress" aria-hidden="true"><span class="fill" style:width="{progress * 100}%"></span></div>
      <div class="controls">
        <PlayButton size={48} label={`Play ${track.name}`} playing={player.isPlaying} onclick={togglePlay} />
        <span class="meta">
          {#if player.isPlaying || !lastPlayedAt}
            {formatTime(player.positionMs)} / {formatTime(track.durationMs)}
          {:else}
            Last played {timeAgo(lastPlayedAt)}
          {/if}
        </span>
      </div>
    </div>
  {:else if fallback}
    <button type="button" class="cover-button" onclick={fallback.open} aria-label={`Open ${fallback.title}`}>
      {@render cover()}
    </button>
    <div class="body">
      <div class="eyebrow">Jump back in</div>
      <h2 class="title"><button type="button" class="title-link" onclick={fallback.open}><span class="title-text">{fallback.title}</span></button></h2>
      <div class="song">{fallback.kind} · played {timeAgo(fallback.at)}</div>
      <div class="controls">
        {#if fallback.play}
          <PlayButton size={48} label={`Play ${fallback.title}`} onclick={fallback.play} />
        {/if}
      </div>
    </div>
  {:else}
    <div class="cover placeholder"></div>
    <div class="body">
      <div class="eyebrow">Welcome</div>
      <h2 class="title">Nothing played yet</h2>
      <div class="song">Play a playlist from your library and it'll show up here.</div>
    </div>
  {/if}
</article>

<style>
.hero {
  --tint: var(--surface-hover);
  display: flex;
  align-items: center;
  gap: 1.5em;
  min-width: 0;
  padding: 1.25em;
  border-radius: var(--radius-lg);
  background: linear-gradient(120deg, color-mix(in srgb, var(--tint) 75%, var(--surface-raised)), var(--surface-raised) 80%);
  transition: background 600ms ease;
}
.cover-button {
  flex-shrink: 0;
  padding: 0;
  border: none;
  background: none;
  cursor: pointer;
}
.cover-button:disabled {
  cursor: default;
}
.cover {
  display: block;
  width: 200px;
  height: 200px;
  flex-shrink: 0;
  border-radius: var(--radius-md);
  object-fit: cover;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
}
.cover.placeholder {
  background: linear-gradient(135deg, var(--surface-raised), var(--surface-hover));
}
.cover.liked {
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #4b1fa8, #8f8ff0);
  color: #fff;
}
.body {
  display: flex;
  flex-direction: column;
  min-width: 0;
  flex: 1;
}
.eyebrow {
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-dim);
}
.title {
  margin: 0.2em 0 0.25em;
  font-size: 2em;
  font-weight: var(--fw-black);
  letter-spacing: -0.02em;
  line-height: 1.1;
  color: var(--text);
  overflow-wrap: anywhere;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.title-link {
  display: block;
  width: 100%;
  padding: 0;
  border: none;
  background: none;
  color: inherit;
  font: inherit;
  letter-spacing: inherit;
  text-align: left;
  cursor: pointer;
}
/* Clamped inside the button too: a button lays out its own text, so the title's clamp doesn't reach it. */
.title-text {
  display: -webkit-box;
  overflow: hidden;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
}
.title-link:hover {
  text-decoration: underline;
}
.song {
  font-size: var(--fs-sm);
  color: var(--text-dim);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.song-name {
  color: var(--text);
  font-weight: var(--fw-medium);
}
.progress {
  height: 4px;
  margin: 1em 0 0.9em;
  border-radius: var(--radius-pill);
  background: color-mix(in srgb, var(--text) 18%, transparent);
  overflow: hidden;
}
.fill {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--text);
}
.controls {
  display: flex;
  align-items: center;
  gap: 0.9em;
  min-height: 48px;
}
.meta {
  font-size: var(--fs-sm);
  font-variant-numeric: tabular-nums;
  color: var(--text-dim);
}
@container home (max-width: 560px) {
  .hero {
    flex-direction: column;
    align-items: stretch;
    gap: 1em;
  }
  .cover {
    width: 120px;
    height: 120px;
  }
  .title {
    font-size: 1.5em;
  }
}
</style>
