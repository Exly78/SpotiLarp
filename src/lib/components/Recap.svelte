<script>
  import { onMount, untrack } from "svelte";
  import { fade } from "svelte/transition";
  import * as api from "../api.js";
  import { tracker, allPlays, asTrack, fillInTracks } from "../tracker.svelte.js";
  import { yearsPlayed } from "../trackerStats.js";
  import { recapOf, RECAP_MIN_PLAYS } from "../recap.js";
  import { cardArtists, cardColor, cardSongs, darkTint, drawPlaylistCover, savedName } from "../shareCard.js";
  import { sharing, openShareCard } from "../sharing.svelte.js";
  import { createPlaylist, adjustTrackCount, updatePlaylistDetails } from "../library.svelte.js";
  import { playFromList } from "../queue.svelte.js";
  import { notify } from "../toast.svelte.js";
  import { cached, MINUTE } from "../requestCache.js";
  import { compactDuration, coverAtLeast, formatCount } from "../utils.js";
  import Icon from "./Icon.svelte";

  /**
   * A year of listening as a run of full-screen slides, one stat each, that
   * move on by themselves; the last one shares it. From the tracker's plays.
   * @type {{ year: number, onClose: () => void }}
   */
  let { year: startYear, onClose } = $props();

  /**
   * @typedef {"intro"|"minutes"|"first"|"artist"|"artists"|"song"|"songs"|"album"|"genres"|"repeat"|"clock"|"day"|"new"|"streak"|"summary"} SlideKind
   */

  /** How long each slide stays up; the last waits. @type {Partial<Record<SlideKind, number>>} */
  const DURATIONS = { intro: 6000, artists: 9000, songs: 9000, genres: 8000, summary: Infinity };
  const DEFAULT_DURATION = 7000;
  /** How long the big numbers take to count up. */
  const COUNT_MS = 1600;
  const LIST_SIZE = 5;
  const ARTIST_LOOKUPS = 10;
  const DETAILS_FRESH_MS = 7 * 24 * 60 * MINUTE;
  const GENRES_SHOWN = 5;
  /** Days drawn in the streak's chain, at most. */
  const STREAK_DOTS = 56;
  /** Fields and the ink on them, for the slides without a picture to take a color from. @type {Partial<Record<SlideKind, [string, string]>>} */
  const FIELDS = {
    intro: ["#1ed760", "#04210f"],
    minutes: ["#2340d8", "#f9e65b"],
    artists: ["#c7b8ff", "#2a1460"],
    songs: ["#ff8fb1", "#3d0b1c"],
    genres: ["#0f4d3a", "#b6f5c8"],
    clock: ["#14123a", "#ff9e5e"],
    day: ["#f0542a", "#2b0a00"],
    new: ["#ffe14d", "#2e2300"],
    streak: ["#7a1fa2", "#ffd6f5"],
  };
  const PICTURE_FIELD = "#2a2a2a";
  /** @type {Record<import("../recap.js").Daypart, { title: string, span: string }>} */
  const DAYPARTS = {
    morning: { title: "You're an early riser", span: "between 5 AM and noon" },
    afternoon: { title: "You're an afternoon listener", span: "between noon and 5 PM" },
    evening: { title: "You're an evening listener", span: "between 5 and 10 PM" },
    night: { title: "You're a night owl", span: "between 10 PM and 5 AM" },
  };

  const reducedMotion = typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
  const dayFormat = new Intl.DateTimeFormat(undefined, { month: "long", day: "numeric" });
  const weekdayDayFormat = new Intl.DateTimeFormat(undefined, { weekday: "long", month: "long", day: "numeric" });
  const firstFormat = new Intl.DateTimeFormat(undefined, { month: "long", day: "numeric", hour: "numeric", minute: "2-digit" });
  const monthYearFormat = new Intl.DateTimeFormat(undefined, { month: "long", year: "numeric" });
  const monthFormat = new Intl.DateTimeFormat(undefined, { month: "long" });
  const monthLetterFormat = new Intl.DateTimeFormat(undefined, { month: "narrow" });
  const weekdayFormat = new Intl.DateTimeFormat(undefined, { weekday: "long" });
  const hourFormat = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" });

  /** @type {HTMLDivElement|undefined} */
  let root = $state();
  let year = $state(untrack(() => startYear));
  let index = $state(0);
  let paused = $state(false);
  let elapsed = $state(0);
  let saving = $state(false);
  /** The playlist "Save as playlist" made, once it has. @type {import("../types.js").Playlist|null} */
  let saved = $state(null);

  const plays = $derived(allPlays());
  const recap = $derived(recapOf(plays, year));
  const years = $derived(yearsPlayed(plays));
  const enough = $derived(recap.plays >= RECAP_MIN_PLAYS);
  const historyBefore = $derived(plays.length > 0 && plays[0].t < recap.from);

  // ---- Artist pictures and genres ----

  /** By artist id; null while it's being looked up or when it couldn't be. @type {Record<string, { image: string|null, genres: string[] }|null>} */
  let details = $state({});

  $effect(() => {
    const ids = recap.artists.slice(0, ARTIST_LOOKUPS).map((artist) => artist.id).filter(Boolean);
    untrack(() => {
      for (const id of ids) {
        if (id in details) continue;
        details[id] = null;
        cached(`artist-details:${id}`, DETAILS_FRESH_MS, () =>
          api.getArtist(id).then((found) => ({ image: coverAtLeast(found.images, 640), genres: found.genres ?? [] })),
        )
          .then((found) => {
            details[id] = found;
          })
          .catch(() => {});
      }
    });
  });

  // Imported history comes without pictures or artist links: looked up for what's shown.
  $effect(() => {
    const wanted = [
      ...recap.songs.slice(0, LIST_SIZE).map((song) => song.track),
      ...recap.artists.slice(0, LIST_SIZE).map((artist) => artist.track),
      recap.album?.track,
      recap.firstPlay?.track,
      recap.onRepeat?.track,
    ].filter((track) => track !== undefined);
    untrack(() => fillInTracks(wanted));
  });

  /** @type {Record<string, string|null>} */
  const pictures = $derived(Object.fromEntries(Object.entries(details).map(([id, found]) => [id, found?.image ?? null])));

  /** Genres weighed by how much each artist was played. */
  const genres = $derived.by(() => {
    /** @type {Map<string, number>} */
    const scores = new Map();
    for (const artist of recap.artists) {
      for (const genre of details[artist.id]?.genres ?? []) scores.set(genre, (scores.get(genre) ?? 0) + artist.ms);
    }
    return [...scores.entries()].sort((a, b) => b[1] - a[1]).slice(0, GENRES_SHOWN).map(([name]) => name);
  });

  // ---- Colors from the pictures ----

  /** By picture. @type {Record<string, string>} */
  let tints = $state({});

  /** @param {string|null|undefined} url */
  function tint(url) {
    return (url && tints[url]) || "";
  }

  const topArtist = $derived(recap.artists[0] ?? null);
  const topSong = $derived(recap.songs[0] ?? null);
  const artistPicture = $derived(topArtist ? (pictures[topArtist.id] ?? topArtist.track.album.image ?? null) : null);

  $effect(() => {
    const urls = [
      recap.firstPlay?.track.album.image,
      artistPicture,
      topSong?.track.album.image,
      recap.album?.image,
      recap.onRepeat?.track.album.image,
    ];
    untrack(() => {
      for (const url of urls) {
        if (!url || url in tints) continue;
        tints[url] = "";
        darkTint(url).then((found) => {
          tints[url] = found;
        });
      }
    });
  });

  // ---- The slides ----

  const slides = $derived.by(() => {
    /** @type {SlideKind[]} */
    const list = ["intro"];
    if (!enough) return list;
    list.push("minutes");
    if (recap.firstPlay) list.push("first");
    if (topArtist) list.push("artist");
    if (recap.artists.length >= 3) list.push("artists");
    if (topSong) list.push("song");
    if (recap.songs.length >= 3) list.push("songs");
    if (recap.album) list.push("album");
    if (genres.length >= 2) list.push("genres");
    if (recap.onRepeat) list.push("repeat");
    list.push("clock");
    if (recap.biggestDay) list.push("day");
    if (historyBefore && recap.newSongs > 0) list.push("new");
    if (recap.streak.days >= 3) list.push("streak");
    list.push("summary");
    return list;
  });
  /** Kept as they were when the show started, so pictures and genres arriving late don't move the slides. @type {SlideKind[]|null} */
  let started = $state(null);
  const shownSlides = $derived(started ?? slides);
  const kind = $derived(shownSlides[index] ?? "intro");
  const duration = $derived(DURATIONS[kind] ?? DEFAULT_DURATION);
  const halted = $derived(paused || sharing.card !== null);

  /** @param {number} to */
  function goTo(to) {
    if (to < 0 || to >= shownSlides.length) return;
    if (to > 0 && started === null) started = [...slides];
    if (to === 0) started = null;
    index = to;
    elapsed = 0;
  }

  function next() {
    if (index < shownSlides.length - 1) goTo(index + 1);
  }

  function back() {
    if (index > 0) goTo(index - 1);
    else elapsed = 0;
  }

  /** @param {number} picked */
  function pickYear(picked) {
    year = picked;
    started = null;
    index = 0;
    elapsed = 0;
  }

  // Moves on once the slide's time is up.
  $effect(() => {
    if (halted || duration === Infinity) return;
    const length = duration;
    // Each slide starts its own clock.
    index;
    let last = performance.now();
    let frame = requestAnimationFrame(function tick(now) {
      elapsed += now - last;
      last = now;
      if (elapsed >= length) untrack(next);
      else frame = requestAnimationFrame(tick);
    });
    return () => cancelAnimationFrame(frame);
  });

  /** A number counting up as its slide opens. @param {number} target */
  function counted(target) {
    if (reducedMotion) return target;
    const t = Math.min(1, elapsed / COUNT_MS);
    return Math.round(target * (1 - (1 - t) ** 3));
  }

  /**
   * A size (in the slide's width) at which a number fits across it.
   * @param {string} text
   * @param {number} max
   */
  function fit(text, max) {
    // Gotham Black's figures run about 0.64em wide, set this tight.
    return `${Math.min(max, 84 / (0.64 * Math.max(1, text.length))).toFixed(1)}cqw`;
  }

  /** @param {number} i */
  function progressOf(i) {
    if (i < index) return 1;
    if (i > index) return 0;
    return duration === Infinity ? 1 : Math.min(1, elapsed / duration);
  }

  /** The picture a slide is about, blurred behind it; null for the slides about numbers. */
  const backdrop = $derived.by(() => {
    switch (kind) {
      case "first":
        return recap.firstPlay?.track.album.image ?? null;
      case "artist":
      case "summary":
        return artistPicture ?? topSong?.track.album.image ?? null;
      case "song":
        return topSong?.track.album.image ?? null;
      case "album":
        return recap.album?.image ?? null;
      case "repeat":
        return recap.onRepeat?.track.album.image ?? null;
      default:
        return null;
    }
  });
  const pictureSlide = $derived(["first", "artist", "song", "album", "repeat", "summary"].includes(kind));
  /** @type {[string, string]} */
  const field = $derived(
    pictureSlide ? [tint(backdrop) || (kind === "summary" ? "#3b1e6e" : PICTURE_FIELD), "#ffffff"] : (FIELDS[kind] ?? [PICTURE_FIELD, "#ffffff"]),
  );

  // ---- Words ----

  const name = savedName();
  const minutes = $derived(Math.round(recap.ms / 60_000));
  const yearStillGoing = $derived(new Date().getFullYear() === recap.year);

  const minutesLine = $derived.by(() => {
    const days = recap.ms / 86_400_000;
    const hours = Math.round(recap.ms / 3_600_000);
    const amount = days >= 2 ? `That's ${Math.round(days)} days of music` : hours >= 1 ? `That's ${hours} ${hours === 1 ? "hour" : "hours"} of music` : "";
    if (!recap.previousMs) return amount ? `${amount}.` : "";
    const percent = Math.round(((recap.ms - recap.previousMs) / recap.previousMs) * 100);
    const then = yearStillGoing ? "this time last year" : String(recap.year - 1);
    const compared = percent === 0 ? `about the same as ${then}` : `${Math.abs(percent)}% ${percent > 0 ? "more" : "less"} than ${then}`;
    return amount ? `${amount}, ${compared}.` : `${compared.charAt(0).toUpperCase()}${compared.slice(1)}.`;
  });

  const busiestMonth = $derived(recap.months.indexOf(Math.max(...recap.months)));
  const monthPeak = $derived(Math.max(1, ...recap.months));
  const hourPeak = $derived(Math.max(1, ...recap.hours));

  /** @param {number} month 0 to 11 */
  function monthAt(month) {
    return new Date(recap.year, month, 1);
  }

  /** @param {number} hour */
  function hourName(hour) {
    const at = new Date();
    at.setHours(hour, 0, 0, 0);
    return hourFormat.format(at);
  }

  /** @param {number} weekday Sunday first */
  function weekdayName(weekday) {
    // 2023-01-01 was a Sunday.
    return weekdayFormat.format(new Date(2023, 0, 1 + weekday));
  }

  const artistSince = $derived.by(() => {
    if (!topArtist) return "";
    if (topArtist.firstHeard >= recap.from) return `You only found them this year, in ${monthFormat.format(topArtist.firstHeard)}.`;
    return `You've been listening since ${monthYearFormat.format(topArtist.firstHeard)}.`;
  });

  /** @param {number} count @param {string} one @param {string} many */
  function plural(count, one, many) {
    return `${formatCount(count)} ${count === 1 ? one : many}`;
  }

  // ---- Doing things with it ----

  const topTracks = $derived(recap.songs.map((song) => asTrack(song.track)));
  const playlistName = $derived(`Your top songs ${recap.year}`);
  const recapTitle = $derived(name ? `${name}'s ${recap.year} Recap` : `My ${recap.year} Recap`);

  /** @returns {import("../shareCard.js").ShareCard} */
  function recapCard() {
    return {
      kind: "recap",
      title: String(recap.year),
      name,
      ms: recap.ms,
      plays: recap.plays,
      genre: genres[0] ?? "",
      artists: cardArtists(recap.artists, pictures),
      songs: cardSongs(recap.songs),
    };
  }

  /** What the playlist says about the year, within Spotify's 300 characters. */
  function playlistDescription() {
    const parts = [`Top songs of ${recap.year}, from ${formatCount(minutes)} minutes of music.`];
    if (topArtist) parts.push(`Top artist: ${topArtist.name}.`);
    if (genres[0]) parts.push(`Top genre: ${genres[0]}.`);
    parts.push("Made with SpotiLarp.");
    return parts.join(" ").slice(0, 300);
  }

  /** @param {Event} evt */
  function playTop(evt) {
    evt.stopPropagation();
    if (topTracks.length > 0) playFromList(topTracks, 0, playlistName);
  }

  /**
   * Saves the year's top songs as a playlist with the Recap on its cover, so
   * a link to it shows the year wherever it's posted (Discord included).
   * @param {Event} evt
   */
  async function saveAsPlaylist(evt) {
    evt.stopPropagation();
    const uris = topTracks.map((track) => track.uri).filter((uri) => uri.startsWith("spotify:track:"));
    if (saving || uris.length === 0) return;
    saving = true;
    try {
      const playlist = await createPlaylist(recapTitle);
      await api.addToPlaylist(playlist.id, uris);
      adjustTrackCount(playlist.id, uris.length);
      await updatePlaylistDetails(playlist, { name: recapTitle, description: playlistDescription(), public: false }).catch(() => {});
      let covered = false;
      try {
        const card = recapCard();
        covered = await api.setPlaylistCover(playlist.id, await drawPlaylistCover({ ...card, color: await cardColor(card) }));
      } catch (e) {
        console.warn("Couldn't put the Recap on the playlist's cover:", e);
      }
      saved = playlist;
      notify(
        covered
          ? `Saved as "${recapTitle}". Copy its link to show it off on Discord.`
          : `Saved as "${recapTitle}". Log out and back in to put your Recap on its cover.`,
      );
    } catch (e) {
      notify(`Couldn't save the playlist: ${e}`);
    } finally {
      saving = false;
    }
  }

  /**
   * The saved playlist's link. It's made public first, since a private
   * playlist's link opens for no one else, and Discord can't show it.
   * @param {Event} evt
   */
  async function copyPlaylistLink(evt) {
    evt.stopPropagation();
    if (!saved) return;
    try {
      if (!saved.public) {
        await updatePlaylistDetails(saved, { name: saved.name, description: "", public: true });
        saved.public = true;
      }
      await navigator.clipboard.writeText(`https://open.spotify.com/playlist/${saved.id}`);
      notify("Link copied. The playlist is public now, so anyone with the link can open it.");
    } catch (e) {
      notify(`Couldn't copy the link: ${e}`);
    }
  }

  /** @param {Event} evt */
  function share(evt) {
    evt.stopPropagation();
    openShareCard(recapCard());
  }

  /** @param {KeyboardEvent} evt */
  function onKeydown(evt) {
    // The share dialog over this has the keys while it's open.
    if (sharing.card) return;
    // Ahead of the app's shortcuts, which wait until this closes.
    evt.stopPropagation();
    const onButton = evt.target instanceof HTMLElement && evt.target.closest("button") !== null;
    if (evt.key === "Escape") onClose();
    else if (evt.key === "ArrowRight") next();
    else if (evt.key === "ArrowLeft") back();
    else if (evt.code === "Space" && !onButton) paused = !paused;
    else return;
    evt.preventDefault();
  }

  onMount(() => root?.focus());
</script>

<svelte:window onkeydowncapture={onKeydown} />

<!-- Outlined words drifting across a slide, behind what it says. -->
{#snippet band(/** @type {string} */ words, /** @type {string} */ top, reverse = false, size = 26)}
  <div class="band" class:reverse style:top style:font-size="{size}cqw" aria-hidden="true">
    <div class="band-track">
      {#each [0, 1] as copy (copy)}<span>{words}&ensp;{words}&ensp;{words}&ensp;</span>{/each}
    </div>
  </div>
{/snippet}

<!-- A cover with its record sliding out and spinning. -->
{#snippet sleeve(/** @type {string|null|undefined} */ image, small = false)}
  <div class="sleeve rise" class:small>
    <div class="record-slot">
      <div class="record">
        {#if image}<img class="label" src={image} alt="" />{/if}
      </div>
    </div>
    <div class="cover">
      {#if image}<img src={image} alt="" />{/if}
    </div>
  </div>
{/snippet}

<div
  class="recap"
  role="dialog"
  aria-modal="true"
  aria-label={`Your ${recap.year} Recap`}
  tabindex="-1"
  bind:this={root}
  transition:fade={{ duration: 160 }}
>
  <button type="button" class="side-arrow left" onclick={back} disabled={index === 0} aria-label="Previous">
    <Icon name="chevron-left" size={22} />
  </button>

  <div class="stage" style:--field={field[0]} style:--ink={field[1]}>
    {#key backdrop}
      {#if backdrop}
        <div class="backdrop" transition:fade={{ duration: 400 }}>
          <img src={backdrop} alt="" />
        </div>
      {/if}
    {/key}

    <ol class="progress" aria-hidden="true">
      {#each shownSlides as slide, i (slide)}
        <li><span class="fill" style:transform="scaleX({progressOf(i)})"></span></li>
      {/each}
    </ol>
    <div class="controls">
      {#if duration !== Infinity}
        <button type="button" class="control" onclick={() => (paused = !paused)} aria-label={paused ? "Play" : "Pause"}>
          <Icon name={paused ? "play" : "pause"} size={16} />
        </button>
      {/if}
      <button type="button" class="control" onclick={onClose} aria-label="Close">
        <Icon name="close" size={16} />
      </button>
    </div>

    <!-- Tapping the left of a slide goes back, anywhere else moves on. -->
    <button type="button" class="zone back" onclick={back} tabindex="-1" aria-hidden="true"></button>
    <button type="button" class="zone forward" onclick={next} tabindex="-1" aria-hidden="true"></button>

    {#key `${recap.year} ${index}`}
      <section class="slide {kind}" aria-roledescription="slide" aria-label={`${index + 1} of ${shownSlides.length}`}>
        {#if kind === "intro"}
          {@render band(`${recap.year} Recap`, "4%")}
          {@render band(`${recap.year} Recap`, "19%", true)}
          {@render band(`${recap.year} Recap`, "34%")}
          <div class="bottom">
            <p class="lead rise">Your</p>
            <h2 class="giant rise" style:--d="1">{recap.year}</h2>
            <p class="lead rise" style:--d="2">Recap</p>
            {#if !tracker.loaded}
              <p class="body rise" style:--d="3">Loading your listening...</p>
            {:else if enough}
              <p class="body rise" style:--d="3">
                The songs, artists and hours that made your year, from {plural(recap.plays, "play", "plays")}.
              </p>
            {:else}
              <p class="body rise" style:--d="3">
                Not much to look back on in {recap.year} yet. Play some music, or import your Spotify history from the
                Stats page.
              </p>
            {/if}
            {#if years.length > 1}
              <div class="years rise" style:--d="4">
                {#each years.slice(0, 6) as option (option)}
                  <button type="button" class="year" class:active={option === recap.year} onclick={() => pickYear(option)}>{option}</button>
                {/each}
              </div>
            {/if}
          </div>
        {:else if kind === "minutes"}
          {@render band("minutes", "7%", false, 22)}
          <div class="middle">
            <p class="lead rise">You listened for</p>
            <p class="huge figure rise" style:--d="1" style:font-size={fit(formatCount(minutes), 21)}>{formatCount(counted(minutes))}</p>
            <p class="lead rise" style:--d="1">minutes</p>
            {#if minutesLine}<p class="body rise" style:--d="2">{minutesLine}</p>{/if}
          </div>
          <figure class="months">
            <ol>
              {#each recap.months as ms, month (month)}
                <li class:top={month === busiestMonth} style:--m={month}>
                  <span class="month-bar" style:height="{Math.max(3, (ms / monthPeak) * 100)}%"></span>
                  <span class="month-letter">{monthLetterFormat.format(monthAt(month))}</span>
                </li>
              {/each}
            </ol>
            <figcaption class="rise" style:--d="5">Most of it in {monthFormat.format(monthAt(busiestMonth))}</figcaption>
          </figure>
        {:else if kind === "first" && recap.firstPlay}
          {@const track = recap.firstPlay.track}
          {@render sleeve(track.album.image)}
          <div class="bottom">
            <p class="lead rise" style:--d="1">Your year started with</p>
            <h2 class="name rise" style:--d="2">{track.name}</h2>
            <p class="body strong rise" style:--d="2">{track.artists.map((a) => a.name).join(", ")}</p>
            <p class="body rise" style:--d="3">On {firstFormat.format(recap.firstPlay.t)}.</p>
          </div>
        {:else if kind === "artist" && topArtist}
          <div class="portrait rise">
            <svg class="ring" viewBox="0 0 100 100" aria-hidden="true">
              <path id="recap-ring" d="M50,50 m-45,0 a45,45 0 1,1 90,0 a45,45 0 1,1 -90,0" fill="none" />
              <text><textPath href="#recap-ring" textLength="282">Your top artist of {recap.year} ✦ Your top artist of {recap.year} ✦</textPath></text>
            </svg>
            <div class="photo">
              {#if artistPicture}<img src={artistPicture} alt="" />{/if}
            </div>
          </div>
          <div class="bottom">
            <p class="lead rise" style:--d="1">Your top artist</p>
            <h2 class="name rise" style:--d="2">{topArtist.name}</h2>
            <p class="body rise" style:--d="3">
              You played them {plural(topArtist.plays, "time", "times")}, for {compactDuration(topArtist.ms)}. {artistSince}
            </p>
          </div>
        {:else if kind === "artists"}
          {@render band("top artists", "84%", true, 14)}
          <div class="middle">
            <h2 class="title rise">Your top artists</h2>
            <ol class="ranking">
              {#each recap.artists.slice(0, LIST_SIZE) as artist, i (artist.name)}
                <li class="enter" class:first={i === 0} style:--d={i + 1}>
                  <span class="rank">{i + 1}</span>
                  <span class="thumb round">
                    {#if pictures[artist.id] ?? artist.track.album.image}<img src={pictures[artist.id] ?? artist.track.album.image} alt="" />{/if}
                  </span>
                  <span class="row-text">
                    <span class="row-name">{artist.name}</span>
                    <span class="row-sub">{plural(artist.plays, "play", "plays")}</span>
                  </span>
                </li>
              {/each}
            </ol>
          </div>
        {:else if kind === "song" && topSong}
          {@render sleeve(topSong.track.album.image)}
          <div class="bottom">
            <p class="lead rise" style:--d="1">Your top song</p>
            <h2 class="name rise" style:--d="2">{topSong.track.name}</h2>
            <p class="body strong rise" style:--d="2">{topSong.track.artists.map((a) => a.name).join(", ")}</p>
            <p class="body rise" style:--d="3">You played it {plural(topSong.plays, "time", "times")}.</p>
            <button type="button" class="action rise" style:--d="4" onclick={playTop}>
              <Icon name="play" size={14} /> Play your top songs
            </button>
          </div>
        {:else if kind === "songs"}
          {@render band("on repeat", "84%", false, 14)}
          <div class="middle">
            <h2 class="title rise">Your top songs</h2>
            <ol class="ranking">
              {#each recap.songs.slice(0, LIST_SIZE) as song, i (song.track.uri)}
                <li class="enter" class:first={i === 0} style:--d={i + 1}>
                  <span class="rank">{i + 1}</span>
                  <span class="thumb">
                    {#if song.track.album.image}<img src={song.track.album.image} alt="" />{/if}
                  </span>
                  <span class="row-text">
                    <span class="row-name">{song.track.name}</span>
                    <span class="row-sub">{song.track.artists[0]?.name ?? ""}</span>
                  </span>
                </li>
              {/each}
            </ol>
          </div>
        {:else if kind === "album" && recap.album}
          {@render sleeve(recap.album.image)}
          <div class="bottom">
            <p class="lead rise" style:--d="1">Your top album</p>
            <h2 class="name rise" style:--d="2">{recap.album.name}</h2>
            <p class="body strong rise" style:--d="2">{recap.album.artist}</p>
            <p class="body rise" style:--d="3">{plural(recap.album.plays, "play", "plays")} from it, {compactDuration(recap.album.ms)} in all.</p>
          </div>
        {:else if kind === "genres"}
          {@render band(genres[0], "6%", false, 20)}
          {@render band(genres[1], "80%", true, 20)}
          <div class="middle">
            <p class="lead rise">You were into</p>
            <h2 class="name genre rise" style:--d="1">{genres[0]}</h2>
            <ol class="genre-list" start="2">
              {#each genres.slice(1) as genre, i (genre)}
                <li class="enter" style:--d={i + 2}>{genre}</li>
              {/each}
            </ol>
          </div>
        {:else if kind === "repeat" && recap.onRepeat}
          {@const repeat = recap.onRepeat}
          {@render sleeve(repeat.track.album.image, true)}
          <div class="bottom">
            <p class="lead rise" style:--d="1">On {weekdayDayFormat.format(repeat.day)} you played</p>
            <h2 class="name rise" style:--d="2">{repeat.track.name}</h2>
            <p class="huge figure rise" style:--d="3">{counted(repeat.plays)}</p>
            <p class="lead rise" style:--d="3">times in one day</p>
          </div>
        {:else if kind === "clock"}
          {@const part = DAYPARTS[recap.daypart]}
          <div class="top-text">
            <h2 class="name rise">{part.title}</h2>
            <p class="body rise" style:--d="1">
              {Math.round(recap.daypartShare * 100)}% of your listening was {part.span}, and
              {weekdayName(recap.peakWeekday)}s were your biggest days.
            </p>
          </div>
          <figure class="dial rise" style:--d="2">
            <svg viewBox="-100 -100 200 200" role="img" aria-label={`Most of your listening at ${hourName(recap.peakHour)}`}>
              <circle class="face" r="31" />
              {#each [0, 6, 12, 18] as hour (hour)}
                {@const angle = (hour / 24) * Math.PI * 2 - Math.PI / 2}
                <circle class="tick" cx={Math.cos(angle) * 25} cy={Math.sin(angle) * 25} r="1.6" />
              {/each}
              {#each recap.hours as ms, hour (hour)}
                {@const angle = (hour / 24) * Math.PI * 2 - Math.PI / 2}
                {@const reach = 41 + (ms / hourPeak) * 54}
                <line
                  class="spoke"
                  class:top={hour === recap.peakHour}
                  x1={Math.cos(angle) * 37}
                  y1={Math.sin(angle) * 37}
                  x2={Math.cos(angle) * reach}
                  y2={Math.sin(angle) * reach}
                  pathLength="1"
                  style:--h={hour}
                />
              {/each}
              <text class="dial-time" y="4">{hourName(recap.peakHour)}</text>
            </svg>
          </figure>
        {:else if kind === "day" && recap.biggestDay}
          {@const day = recap.biggestDay}
          <div class="calendar rise">
            <div class="page">
              <div class="calendar-month">{monthFormat.format(day.day)}</div>
              <div class="calendar-date">{new Date(day.day).getDate()}</div>
              <div class="calendar-weekday">{weekdayFormat.format(day.day)}</div>
            </div>
          </div>
          <div class="bottom">
            <p class="lead rise" style:--d="1">Your biggest day</p>
            <p class="body rise" style:--d="2">
              {compactDuration(day.ms)} of music, {plural(day.plays, "play", "plays")}{#if day.artist}, mostly {day.artist}{/if}.
            </p>
          </div>
        {:else if kind === "new"}
          <div class="bubbles" aria-hidden="true">
            {#each Array.from({ length: 9 }, (_, i) => i) as i (i)}<span style:--b={i}></span>{/each}
          </div>
          <div class="middle">
            <p class="lead rise">New to you this year</p>
            <p class="huge figure rise" style:--d="1" style:font-size={fit(formatCount(recap.newSongs), 21)}>{formatCount(counted(recap.newSongs))}</p>
            <p class="body rise" style:--d="1">{recap.newSongs === 1 ? "song" : "songs"} you'd never played before</p>
            {#if recap.newArtists > 0}
              <p class="big figure rise" style:--d="2">{formatCount(counted(recap.newArtists))}</p>
              <p class="body rise" style:--d="2">{recap.newArtists === 1 ? "artist" : "artists"} you'd never played before</p>
            {/if}
          </div>
        {:else if kind === "streak"}
          <div class="middle">
            <p class="lead rise">Your longest run</p>
            <p class="huge figure rise" style:--d="1">{counted(recap.streak.days)}</p>
            <p class="lead rise" style:--d="1">days of music in a row</p>
            <p class="body rise" style:--d="2">From {dayFormat.format(recap.streak.from)} to {dayFormat.format(recap.streak.to)}.</p>
            <ol class="chain" aria-hidden="true">
              {#each Array.from({ length: Math.min(recap.streak.days, STREAK_DOTS) }, (_, i) => i) as i (i)}
                <li style:--i={i}></li>
              {/each}
              {#if recap.streak.days > STREAK_DOTS}<li class="more">+{recap.streak.days - STREAK_DOTS}</li>{/if}
            </ol>
          </div>
        {:else if kind === "summary"}
          <div class="summary-top">
            <h2 class="giant rise">{recap.year}</h2>
            <p class="body rise" style:--d="1">{name ? `${name}'s` : "Your"} year in music</p>
          </div>
          <div class="columns rise" style:--d="2">
            <div>
              <h3>Top artists</h3>
              <ol>
                {#each recap.artists.slice(0, LIST_SIZE) as artist (artist.name)}<li>{artist.name}</li>{/each}
              </ol>
            </div>
            <div>
              <h3>Top songs</h3>
              <ol>
                {#each recap.songs.slice(0, LIST_SIZE) as song (song.track.uri)}<li>{song.track.name}</li>{/each}
              </ol>
            </div>
            <div>
              <h3>Minutes listened</h3>
              <p class="summary-figure">{formatCount(minutes)}</p>
            </div>
            <div>
              {#if genres[0]}
                <h3>Top genre</h3>
                <p class="summary-figure genre-figure">{genres[0]}</p>
              {:else}
                <h3>Plays</h3>
                <p class="summary-figure">{formatCount(recap.plays)}</p>
              {/if}
            </div>
          </div>
          <div class="summary-actions rise" style:--d="3">
            <button type="button" class="action solid" onclick={share}>Share</button>
            {#if saved}
              <button type="button" class="action" onclick={copyPlaylistLink}>Copy playlist link</button>
            {:else}
              <button type="button" class="action" onclick={saveAsPlaylist} disabled={saving}>
                {saving ? "Saving..." : "Save as playlist"}
              </button>
            {/if}
            <button type="button" class="action" onclick={playTop}><Icon name="play" size={14} /> Play</button>
          </div>
        {/if}
      </section>
    {/key}
  </div>

  <button type="button" class="side-arrow right" onclick={next} disabled={index >= shownSlides.length - 1} aria-label="Next">
    <Icon name="chevron-right" size={22} />
  </button>
</div>

<style>
.recap {
  position: fixed;
  inset: 0;
  z-index: 80;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 1.5em;
  background: rgba(0, 0, 0, 0.88);
  outline: none;
}
.stage {
  --height: min(calc(100vh - 4em), 880px);
  position: relative;
  isolation: isolate;
  container-type: size;
  width: calc(var(--height) * 9 / 16);
  height: var(--height);
  border-radius: 18px;
  overflow: hidden;
  background: var(--field);
  color: var(--ink);
  box-shadow: 0 30px 80px rgba(0, 0, 0, 0.5);
  transition: background-color 500ms ease, color 500ms ease;
  user-select: none;
}
/* Film grain over everything, so the flat colors read like print. */
.stage::after {
  content: "";
  position: absolute;
  inset: 0;
  z-index: 4;
  pointer-events: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='180' height='180'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='.85' numOctaves='3' stitchTiles='stitch'/%3E%3CfeColorMatrix values='0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 .9 0'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
  mix-blend-mode: overlay;
  opacity: 0.32;
}
.side-arrow {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  border: none;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.12);
  color: #fff;
  cursor: pointer;
  transition: background-color var(--transition), opacity var(--transition);
}
.side-arrow:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.24);
}
.side-arrow:disabled {
  opacity: 0;
  cursor: default;
}

/* ---- The picture a slide is about, blurred into its background ---- */
.backdrop {
  position: absolute;
  inset: 0;
  z-index: 0;
  overflow: hidden;
}
.backdrop img {
  position: absolute;
  top: -25%;
  left: -25%;
  width: 150%;
  height: 150%;
  object-fit: cover;
  filter: blur(48px) saturate(1.6) brightness(0.72);
  animation: drift 26s ease-in-out infinite alternate;
}
.backdrop::after {
  content: "";
  position: absolute;
  inset: 0;
  background: linear-gradient(180deg, rgba(0, 0, 0, 0.05) 0%, rgba(0, 0, 0, 0.12) 45%, rgba(0, 0, 0, 0.62) 100%);
}
@keyframes drift {
  to {
    transform: scale(1.14) rotate(5deg);
  }
}

/* ---- Progress and controls ---- */
.progress {
  position: absolute;
  top: 12px;
  left: 12px;
  right: 12px;
  z-index: 3;
  display: flex;
  gap: 4px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.progress li {
  flex: 1;
  height: 3px;
  border-radius: 2px;
  background: color-mix(in srgb, var(--ink) 28%, transparent);
  overflow: hidden;
}
.fill {
  display: block;
  height: 100%;
  background: var(--ink);
  transform-origin: left;
}
.controls {
  position: absolute;
  top: 24px;
  right: 10px;
  z-index: 3;
  display: flex;
  gap: 2px;
}
.control {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 34px;
  border: none;
  border-radius: 50%;
  background: none;
  color: var(--ink);
  cursor: pointer;
}
.control:hover {
  background: color-mix(in srgb, var(--ink) 16%, transparent);
}
.zone {
  position: absolute;
  top: 0;
  bottom: 0;
  z-index: 1;
  padding: 0;
  border: none;
  background: none;
  cursor: pointer;
}
.zone.back {
  left: 0;
  width: 30%;
}
.zone.forward {
  right: 0;
  width: 70%;
}

/* ---- Slides ---- */
.slide {
  position: absolute;
  inset: 0;
  z-index: 2;
  display: flex;
  flex-direction: column;
  padding: 18cqw 8cqw 9cqw;
  pointer-events: none;
}
.slide button {
  pointer-events: auto;
}
.bottom {
  margin-top: auto;
}
.middle {
  margin: auto 0;
}
/* No specificity, so each kind of line sets its own margins. */
:where(.slide) :is(p, h2, h3) {
  margin: 0;
}
.lead {
  font-size: 6.4cqw;
  font-weight: var(--fw-bold);
  letter-spacing: -0.01em;
  line-height: 1.15;
}
.body {
  margin-top: 3cqw;
  max-width: 34ch;
  font-size: 4.6cqw;
  line-height: 1.4;
  opacity: 0.9;
}
.body.strong {
  margin-top: 1.5cqw;
  font-weight: var(--fw-bold);
  opacity: 1;
}
.giant {
  margin: -1cqw 0 0 -1.2cqw;
  font-size: 32cqw;
  font-weight: var(--fw-black);
  letter-spacing: -0.055em;
  line-height: 0.9;
}
.huge {
  margin: 1cqw 0;
  font-size: 21cqw;
  font-weight: var(--fw-black);
  letter-spacing: -0.045em;
  line-height: 0.95;
  overflow-wrap: anywhere;
}
.big {
  margin-top: 7cqw;
  font-size: 13cqw;
  font-weight: var(--fw-black);
  letter-spacing: -0.04em;
  line-height: 0.95;
}
.figure {
  font-variant-numeric: tabular-nums;
}
.name {
  margin-top: 2cqw;
  font-size: 11cqw;
  font-weight: var(--fw-black);
  letter-spacing: -0.035em;
  line-height: 1;
  overflow-wrap: anywhere;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.title {
  margin-bottom: 7cqw;
  font-size: 9cqw;
  font-weight: var(--fw-black);
  letter-spacing: -0.03em;
  line-height: 1;
}
.thumb img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.action {
  display: inline-flex;
  align-items: center;
  gap: 0.5em;
  margin-top: 5cqw;
  padding: 2.6cqw 5cqw;
  border: 1.5px solid currentColor;
  border-radius: var(--radius-pill);
  background: none;
  color: inherit;
  font-family: inherit;
  font-size: 3.8cqw;
  font-weight: var(--fw-bold);
  cursor: pointer;
  transition: background-color var(--transition), transform var(--transition);
}
.action:hover:not(:disabled) {
  background: color-mix(in srgb, var(--ink) 14%, transparent);
}
.action:active:not(:disabled) {
  transform: scale(0.97);
}
.action:disabled {
  opacity: 0.6;
  cursor: default;
}
.action.solid {
  border-color: var(--ink);
  background: var(--ink);
  color: #111;
}
.action.solid:hover:not(:disabled) {
  background: color-mix(in srgb, var(--ink) 86%, transparent);
}

/* ---- Outlined words drifting behind ---- */
.band {
  position: absolute;
  left: -15%;
  right: -15%;
  z-index: -1;
  overflow: hidden;
  white-space: nowrap;
  font-weight: var(--fw-black);
  letter-spacing: -0.04em;
  line-height: 1.05;
  color: transparent;
  -webkit-text-stroke: 1.5px color-mix(in srgb, var(--ink) 40%, transparent);
  transform: rotate(-7deg);
}
.band-track {
  display: flex;
  width: max-content;
  animation: marquee 24s linear infinite;
}
.band.reverse .band-track {
  animation-direction: reverse;
}
@keyframes marquee {
  to {
    transform: translateX(-50%);
  }
}

/* The intro's years. */
.years {
  display: flex;
  flex-wrap: wrap;
  gap: 2cqw;
  margin-top: 6cqw;
}
.year {
  padding: 1.6cqw 3.6cqw;
  border: 1.5px solid color-mix(in srgb, var(--ink) 40%, transparent);
  border-radius: var(--radius-pill);
  background: none;
  color: inherit;
  font-family: inherit;
  font-size: 3.6cqw;
  font-weight: var(--fw-bold);
  cursor: pointer;
}
.year:hover,
.year.active {
  border-color: var(--ink);
}
.year.active {
  background: var(--ink);
  color: var(--field);
}

/* ---- A record sliding out of its sleeve ---- */
.sleeve {
  position: relative;
  flex-shrink: 0;
  width: 62cqw;
  aspect-ratio: 1;
}
.sleeve.small {
  width: 48cqw;
}
.cover {
  position: absolute;
  inset: 0;
  z-index: 1;
  border-radius: 1.4cqw;
  overflow: hidden;
  background: color-mix(in srgb, var(--ink) 14%, transparent);
  box-shadow: 0 3cqw 8cqw rgba(0, 0, 0, 0.45), 0 0 0 1px rgba(255, 255, 255, 0.08);
}
.cover img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.record-slot {
  position: absolute;
  top: 3%;
  left: 3%;
  width: 94%;
  height: 94%;
  animation: slide-out 1s 350ms cubic-bezier(0.2, 0.8, 0.2, 1) both;
}
/* The light on the grooves stays put while the record turns. */
.record-slot::after {
  content: "";
  position: absolute;
  inset: 0;
  border-radius: 50%;
  background: conic-gradient(
    from 20deg,
    rgba(255, 255, 255, 0.16),
    transparent 10%,
    transparent 48%,
    rgba(255, 255, 255, 0.12) 58%,
    transparent 70%
  );
  pointer-events: none;
}
@keyframes slide-out {
  from {
    transform: translateX(0);
  }
  to {
    transform: translateX(40%);
  }
}
.record {
  position: relative;
  width: 100%;
  height: 100%;
  border-radius: 50%;
  background: repeating-radial-gradient(circle, #141414 0 0.35cqw, #202020 0.35cqw 0.7cqw);
  box-shadow: 0 2cqw 6cqw rgba(0, 0, 0, 0.55);
  animation: spin 2.4s linear infinite;
}
.record::before {
  content: "";
  position: absolute;
  top: 48.5%;
  left: 48.5%;
  z-index: 1;
  width: 3%;
  height: 3%;
  border-radius: 50%;
  background: #0d0d0d;
}
.label {
  position: absolute;
  top: 32%;
  left: 32%;
  width: 36%;
  height: 36%;
  border-radius: 50%;
  object-fit: cover;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

/* ---- The top artist, ringed with words that turn ---- */
.portrait {
  position: relative;
  flex-shrink: 0;
  width: 76cqw;
  aspect-ratio: 1;
  margin-left: -1cqw;
}
.ring {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  overflow: visible;
  animation: spin 28s linear infinite;
}
.ring text {
  fill: var(--ink);
  font-size: 6.4px;
  font-weight: var(--fw-bold);
}
.photo {
  position: absolute;
  inset: 13%;
  border-radius: 50%;
  overflow: hidden;
  background: color-mix(in srgb, var(--ink) 14%, transparent);
  box-shadow: 0 4cqw 10cqw rgba(0, 0, 0, 0.45);
}
.photo img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
}

/* ---- Months: bars that grow in turn ---- */
.months {
  margin: 0;
}
.months ol {
  display: flex;
  align-items: flex-end;
  gap: 1.6cqw;
  height: 26cqw;
  margin: 0;
  padding: 0;
  list-style: none;
}
.months li {
  position: relative;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  flex: 1;
  height: 100%;
}
.month-bar {
  display: block;
  border-radius: 1cqw 1cqw 0 0;
  background: color-mix(in srgb, var(--ink) 38%, transparent);
  transform-origin: bottom;
  animation: grow 700ms cubic-bezier(0.2, 0.8, 0.2, 1) both;
  animation-delay: calc(450ms + var(--m) * 45ms);
}
.top .month-bar {
  background: var(--ink);
}
@keyframes grow {
  from {
    transform: scaleY(0);
  }
}
.month-letter {
  position: absolute;
  bottom: -5.5cqw;
  left: 0;
  right: 0;
  font-size: 3cqw;
  text-align: center;
  opacity: 0.7;
}
.top .month-letter {
  font-weight: var(--fw-bold);
  opacity: 1;
}
.months figcaption {
  margin-top: 8cqw;
  font-size: 4.2cqw;
  font-weight: var(--fw-bold);
}

/* ---- Top five lists ---- */
.ranking {
  display: flex;
  flex-direction: column;
  gap: 3.2cqw;
  margin: 0;
  padding: 0;
  list-style: none;
}
.ranking li {
  display: flex;
  align-items: center;
  gap: 3.6cqw;
  min-width: 0;
}
.enter {
  animation: enter 600ms cubic-bezier(0.2, 0.8, 0.2, 1) both;
  animation-delay: calc(var(--d, 0) * 90ms);
}
@keyframes enter {
  from {
    opacity: 0;
    transform: translateX(-12cqw);
  }
}
.rank {
  width: 8cqw;
  flex-shrink: 0;
  font-size: 9cqw;
  font-weight: var(--fw-black);
  letter-spacing: -0.04em;
  line-height: 1;
  font-variant-numeric: tabular-nums;
  color: transparent;
  -webkit-text-stroke: 1.5px var(--ink);
}
.first .rank {
  color: var(--ink);
}
.thumb {
  flex-shrink: 0;
  width: 14cqw;
  height: 14cqw;
  border-radius: 1.6cqw;
  overflow: hidden;
  background: color-mix(in srgb, var(--ink) 14%, transparent);
  box-shadow: 0 1.4cqw 3cqw rgba(0, 0, 0, 0.18);
}
.thumb.round {
  border-radius: 50%;
}
.first .thumb {
  width: 22cqw;
  height: 22cqw;
}
.row-text {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.row-name,
.row-sub {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.row-name {
  font-size: 5.2cqw;
  font-weight: var(--fw-bold);
  letter-spacing: -0.01em;
}
.first .row-name {
  font-size: 7cqw;
  font-weight: var(--fw-black);
  letter-spacing: -0.03em;
}
.row-sub {
  font-size: 3.7cqw;
  opacity: 0.75;
}

/* ---- Genres: the top one big, the rest after it ---- */
.genre,
.genre-figure {
  text-transform: capitalize;
}
.genre-list {
  margin: 6cqw 0 0;
  padding: 0;
  list-style: none;
  counter-reset: genre 1;
}
.genre-list li {
  counter-increment: genre;
  padding: 2.4cqw 0;
  border-top: 1px solid color-mix(in srgb, var(--ink) 30%, transparent);
  font-size: 5.6cqw;
  font-weight: var(--fw-bold);
  text-transform: capitalize;
}
.genre-list li::before {
  content: counter(genre);
  display: inline-block;
  width: 8cqw;
  opacity: 0.6;
}

/* ---- The day as a 24-hour dial ---- */
.top-text {
  margin-top: 2cqw;
}
.dial {
  margin: auto 0;
}
.dial svg {
  display: block;
  width: 84cqw;
  overflow: visible;
}
.face {
  fill: color-mix(in srgb, var(--ink) 12%, transparent);
}
.tick {
  fill: var(--ink);
  opacity: 0.55;
}
.spoke {
  stroke: color-mix(in srgb, var(--ink) 38%, transparent);
  stroke-width: 5.5;
  stroke-linecap: round;
  stroke-dasharray: 1 2;
  stroke-dashoffset: 1;
  animation: draw 650ms cubic-bezier(0.2, 0.8, 0.2, 1) forwards;
  animation-delay: calc(450ms + var(--h) * 28ms);
}
.spoke.top {
  stroke: var(--ink);
}
@keyframes draw {
  to {
    stroke-dashoffset: 0;
  }
}
.dial-time {
  fill: var(--ink);
  font-size: 10.5px;
  font-weight: var(--fw-black);
  letter-spacing: -0.03em;
  text-anchor: middle;
}

/* ---- The biggest day, torn off a calendar ---- */
.calendar {
  margin: 4cqw 0 0 6cqw;
}
.page {
  width: 60cqw;
  border-radius: 3cqw;
  overflow: hidden;
  background: #fff6ee;
  text-align: center;
  box-shadow: 0 5cqw 12cqw rgba(0, 0, 0, 0.3);
  transform: rotate(-6deg);
}
.calendar-month {
  padding: 3cqw 0;
  background: var(--ink);
  color: var(--field);
  font-size: 5.4cqw;
  font-weight: var(--fw-black);
}
.calendar-date {
  padding-top: 3cqw;
  font-size: 34cqw;
  font-weight: var(--fw-black);
  letter-spacing: -0.06em;
  line-height: 1;
}
.calendar-weekday {
  padding: 1cqw 0 5cqw;
  font-size: 4.6cqw;
  font-weight: var(--fw-bold);
}

/* ---- New finds: bubbles rising ---- */
.bubbles {
  position: absolute;
  inset: 0;
  z-index: -1;
  overflow: hidden;
}
.bubbles span {
  position: absolute;
  bottom: -30cqw;
  left: calc(var(--b) * 11% - 4%);
  width: 12cqw;
  aspect-ratio: 1;
  border: 0.6cqw solid color-mix(in srgb, var(--ink) 35%, transparent);
  border-radius: 50%;
  animation: float calc(10s + var(--b) * 1.3s) linear infinite;
  animation-delay: calc(var(--b) * -2.3s);
}
.bubbles span:nth-child(3n) {
  width: 22cqw;
}
.bubbles span:nth-child(3n + 1) {
  width: 7cqw;
}
.bubbles span:nth-child(4n) {
  border: none;
  background: color-mix(in srgb, var(--ink) 16%, transparent);
}
@keyframes float {
  to {
    transform: translateY(-240cqw);
  }
}

/* ---- The streak as a chain of days ---- */
.chain {
  display: flex;
  flex-wrap: wrap;
  gap: 1.6cqw;
  margin: 7cqw 0 0;
  padding: 0;
  list-style: none;
}
.chain li {
  width: 4.6cqw;
  height: 4.6cqw;
  border-radius: 50%;
  background: var(--ink);
  animation: pop 320ms cubic-bezier(0.3, 1.6, 0.5, 1) both;
  animation-delay: calc(500ms + var(--i) * 26ms);
}
.chain .more {
  width: auto;
  height: auto;
  background: none;
  font-size: 4cqw;
  font-weight: var(--fw-black);
  line-height: 4.6cqw;
  animation-delay: 1.9s;
}
@keyframes pop {
  from {
    transform: scale(0);
  }
}

/* ---- The last slide: the year on one page, and what to do with it ---- */
.summary-top .body {
  margin-top: 1cqw;
}
.columns {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 6cqw 5cqw;
  margin-top: 8cqw;
}
.columns > div {
  min-width: 0;
}
.columns h3 {
  margin-bottom: 1.6cqw;
  font-size: 3.6cqw;
  font-weight: var(--fw-bold);
  opacity: 0.75;
}
.columns ol {
  margin: 0;
  padding: 0;
  list-style: none;
  counter-reset: row;
}
.columns li {
  counter-increment: row;
  font-size: 4.2cqw;
  font-weight: var(--fw-medium);
  line-height: 1.55;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.columns li::before {
  content: counter(row);
  display: inline-block;
  width: 4.6cqw;
  font-weight: var(--fw-bold);
  opacity: 0.7;
}
.summary-figure {
  font-size: 8.4cqw;
  font-weight: var(--fw-black);
  letter-spacing: -0.03em;
  line-height: 1.05;
  font-variant-numeric: tabular-nums;
  overflow-wrap: anywhere;
}
.summary-actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 2.4cqw;
  margin-top: auto;
}
.summary-actions .action {
  justify-content: center;
  margin-top: 0;
}
.summary-actions .solid {
  grid-column: 1 / -1;
}

/* Each slide's entrance: its lines rise in turn. */
.rise {
  animation: rise 560ms cubic-bezier(0.2, 0.7, 0.2, 1) both;
  animation-delay: calc(var(--d, 0) * 110ms);
}
@keyframes rise {
  from {
    opacity: 0;
    transform: translateY(6cqw);
  }
}
@media (prefers-reduced-motion: reduce) {
  .rise,
  .enter,
  .month-bar,
  .chain li,
  .band-track,
  .record,
  .ring,
  .backdrop img {
    animation: none;
  }
  .record-slot {
    animation: none;
    transform: translateX(40%);
  }
  .spoke {
    animation: none;
    stroke-dashoffset: 0;
  }
  .bubbles {
    display: none;
  }
  .stage {
    transition: none;
  }
}
</style>
