import * as api from "./api.js";
import { dominantColor } from "./colors.js";
import { topArtists, topSongs, totals } from "./trackerStats.js";
import { formatCount } from "./utils.js";

/**
 * Stats to share: a picture drawn here (1080×1920, the shape of a phone's
 * story) and a link to a page on GitHub Pages (docs/stats/) showing the same
 * card. The link carries the stats themselves after the #, which browsers
 * never send to the server, so nothing is uploaded anywhere.
 *
 * @typedef {{ name: string, id: string, image: string|null }} CardArtist
 * @typedef {{ name: string, artist: string, id: string, image: string|null }} CardSong
 * @typedef {Object} ShareCard
 * @property {"recap"|"stats"} kind a year's Recap, or a stretch of time on the Stats page
 * @property {string} title "2026", "The last 4 weeks"
 * @property {string} name whose stats, or ""
 * @property {number} ms time listened
 * @property {number} plays
 * @property {string} genre the top genre, or ""
 * @property {CardArtist[]} artists the top ones, most played first
 * @property {CardSong[]} songs the top ones, most played first
 * @property {string} [color] the background, "#rrggbb"
 */

const SHARE_PAGE = "https://exly78.github.io/SpotiLarp/stats/";
/** Where the app keeps your Spotify name (+page.svelte). */
const SAVED_NAME_KEY = "spotilarp.cache.display-name";
/** Artists and songs on a card. */
export const CARD_TOP = 5;

const WIDTH = 1080;
const HEIGHT = 1920;
const PAD = 88;
const FONT = `"Gotham", "Circular Std", Inter, "Helvetica Neue", Helvetica, Arial, sans-serif`;
const WHITE = "#ffffff";
const DIM = "rgba(255, 255, 255, 0.74)";
/** Behind the card when its pictures give no color. */
const FALLBACK_COLOR = "#3b1e6e";
/** Darkest a background's relative luminance gets brought down to, so white reads on it. */
const MAX_LUMINANCE = 0.14;
const IMAGE_TIMEOUT_MS = 8000;
const COVER = /^https:\/\/i\.scdn\.co\/image\/([0-9a-f]{40})$/;

/** Your Spotify name as last seen, or "". */
export function savedName() {
  try {
    return localStorage.getItem(SAVED_NAME_KEY) ?? "";
  } catch {
    return "";
  }
}

/** @param {string} uri */
function trackId(uri) {
  return uri.startsWith("spotify:track:") ? uri.slice("spotify:track:".length) : "";
}

/**
 * @param {{ id: string, name: string }[]} artists
 * @param {Record<string, string|null>} pictures by artist id
 * @returns {CardArtist[]}
 */
export function cardArtists(artists, pictures) {
  return artists.slice(0, CARD_TOP).map((artist) => ({ name: artist.name, id: artist.id, image: pictures[artist.id] ?? null }));
}

/**
 * @param {{ track: import("./types.js").TrackerTrack }[]} songs
 * @returns {CardSong[]}
 */
export function cardSongs(songs) {
  return songs.slice(0, CARD_TOP).map(({ track }) => ({
    name: track.name,
    artist: track.artists[0]?.name ?? "",
    id: trackId(track.uri),
    image: track.album.image ?? null,
  }));
}

/**
 * A card for some of the tracker's plays (a stretch of time on the Stats page).
 * @param {import("./tracker.svelte.js").Play[]} plays
 * @param {{ title: string, pictures: Record<string, string|null>, genre?: string }} details
 * @returns {ShareCard}
 */
export function statsCard(plays, { title, pictures, genre = "" }) {
  const sum = totals(plays);
  const playable = plays.filter((play) => !play.track.uri.startsWith("spotilarp:unmatched:"));
  return {
    kind: "stats",
    title,
    name: savedName(),
    ms: sum.ms,
    plays: sum.plays,
    genre,
    artists: cardArtists(topArtists(plays, CARD_TOP), pictures),
    songs: cardSongs(topSongs(playable, CARD_TOP)),
  };
}

/** The line under the card's title. @param {ShareCard} card */
export function cardSubtitle(card) {
  const whose = card.name ? `${card.name}'s` : "My";
  return card.kind === "recap" ? `${whose} year in music` : `${whose} listening`;
}

/** A name that's safe in a file name and a request header. @param {ShareCard} card */
export function cardFileName(card) {
  const title = card.title.replace(/[^\x20-\x7e]/g, "").replace(/[\\/:*?"<>|]/g, "").trim();
  return card.kind === "recap" ? `SpotiLarp Recap ${title}.png` : `SpotiLarp stats - ${title || "listening"}.png`;
}

// ---- The background ----

/**
 * The card's background, from its main picture, darkened until white reads on it.
 * @param {ShareCard} card
 */
export async function cardColor(card) {
  return (await darkTint(mainPicture(card))) || FALLBACK_COLOR;
}

/**
 * A picture's main color as "#rrggbb", darkened until white reads on it, or
 * "" when it has none to speak of.
 * @param {string|null|undefined} url
 */
export async function darkTint(url) {
  const found = url ? await dominantColor(url) : "";
  const match = /rgb\((\d+),\s*(\d+),\s*(\d+)\)/.exec(found);
  if (!match) return "";
  let [r, g, b] = match.slice(1).map(Number);
  while (luminance(r, g, b) > MAX_LUMINANCE) {
    [r, g, b] = [r * 0.9, g * 0.9, b * 0.9];
  }
  return `#${[r, g, b].map((v) => Math.round(v).toString(16).padStart(2, "0")).join("")}`;
}

/** @param {number} r @param {number} g @param {number} b */
function luminance(r, g, b) {
  const linear = (/** @type {number} */ v) => {
    const c = v / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
}

/** The top artist's picture, else the top song's cover. @param {ShareCard} card */
function mainPicture(card) {
  return card.artists[0]?.image ?? card.songs[0]?.image ?? null;
}

// ---- The picture ----

/**
 * Draws the card as a PNG.
 * @param {ShareCard} card
 * @returns {Promise<Blob>}
 */
export async function drawCard(card) {
  const picture = mainPicture(card);
  const [image] = await Promise.all([picture ? loadImage(picture) : Promise.resolve(null), loadFonts()]);
  const canvas = document.createElement("canvas");
  canvas.width = WIDTH;
  canvas.height = HEIGHT;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("Couldn't draw the picture");
  const inner = WIDTH - PAD * 2;

  ctx.fillStyle = card.color || FALLBACK_COLOR;
  ctx.fillRect(0, 0, WIDTH, HEIGHT);
  if (image) drawBlurred(ctx, image, WIDTH, HEIGHT);

  // Whose it is, and where it came from.
  ctx.fillStyle = "#1ed760";
  ctx.beginPath();
  ctx.arc(PAD + 13, 118, 13, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = WHITE;
  ctx.font = `700 38px ${FONT}`;
  ctx.fillText("SpotiLarp", PAD + 42, 131);

  // The title as big as it fits, on up to two lines.
  const title = fitLines(ctx, card.title, 800, inner, 330, 2);
  ctx.font = `800 ${title.size}px ${FONT}`;
  let y = 200 + title.size * 0.74;
  title.lines.forEach((line, i) => {
    if (i > 0) y += title.size * 0.94;
    ctx.fillText(line, PAD - title.size * 0.04, y);
  });
  y += 84;
  ctx.font = `400 46px ${FONT}`;
  ctx.fillStyle = DIM;
  ctx.fillText(ellipsize(ctx, cardSubtitle(card), inner), PAD, y);

  // The top artist's picture as a sleeve with its record sliding out, in what's left above the lists.
  const boxTop = y + 56;
  const boxHeight = 1206 - boxTop;
  if (image) {
    const side = Math.min(boxHeight, 620);
    const top = boxTop + (boxHeight - side) / 2;
    drawRecord(ctx, image, PAD + side * 0.98, top + side / 2, side * 0.47);
    ctx.save();
    ctx.shadowColor = "rgba(0, 0, 0, 0.45)";
    ctx.shadowBlur = 60;
    ctx.shadowOffsetY = 24;
    ctx.beginPath();
    ctx.roundRect(PAD, top, side, side, 14);
    ctx.fillStyle = "#000000";
    ctx.fill();
    ctx.restore();
    ctx.save();
    ctx.beginPath();
    ctx.roundRect(PAD, top, side, side, 14);
    ctx.clip();
    drawCover(ctx, image, PAD, top, side, side);
    ctx.restore();
  } else {
    ctx.save();
    ctx.beginPath();
    ctx.roundRect(PAD, boxTop, inner, boxHeight, 28);
    ctx.clip();
    ctx.fillStyle = "rgba(0, 0, 0, 0.2)";
    ctx.fillRect(PAD, boxTop, inner, boxHeight);
    const top = card.artists[0]?.name ?? card.songs[0]?.name ?? "";
    const fitted = fitLines(ctx, top, 800, inner - 112, 150, 3);
    ctx.font = `800 ${fitted.size}px ${FONT}`;
    ctx.fillStyle = WHITE;
    let lineY = boxTop + boxHeight - 64 - (fitted.lines.length - 1) * fitted.size * 0.98;
    for (const line of fitted.lines) {
      ctx.fillText(line, PAD + 56, lineY);
      lineY += fitted.size * 0.98;
    }
    ctx.restore();
  }

  // Top artists and songs, side by side.
  const column = inner / 2 - 20;
  const columns = [
    { x: PAD, heading: "Top artists", names: card.artists.map((a) => a.name) },
    { x: PAD + inner / 2 + 20, heading: "Top songs", names: card.songs.map((s) => s.name) },
  ];
  for (const { x, heading, names } of columns) {
    ctx.font = `700 34px ${FONT}`;
    ctx.fillStyle = DIM;
    ctx.fillText(heading, x, 1290);
    names.forEach((name, i) => {
      const rowY = 1364 + i * 64;
      ctx.font = `700 36px ${FONT}`;
      ctx.fillStyle = DIM;
      ctx.fillText(String(i + 1), x, rowY);
      ctx.font = `500 36px ${FONT}`;
      ctx.fillStyle = WHITE;
      ctx.fillText(ellipsize(ctx, name, column - 46), x + 46, rowY);
    });
  }

  // Time listened, and the top genre (or plays, without one).
  const figures = [
    { label: "Minutes listened", value: formatCount(Math.round(card.ms / 60_000)) },
    card.genre ? { label: "Top genre", value: capitalize(card.genre) } : { label: "Plays", value: formatCount(card.plays) },
  ];
  figures.forEach(({ label, value }, i) => {
    const x = columns[i].x;
    ctx.font = `700 34px ${FONT}`;
    ctx.fillStyle = DIM;
    ctx.fillText(label, x, 1724);
    const fitted = fitLines(ctx, value, 800, column, 84, 1);
    ctx.font = `800 ${fitted.size}px ${FONT}`;
    ctx.fillStyle = WHITE;
    ctx.fillText(ellipsize(ctx, fitted.lines[0] ?? "", column), x - 2, 1814);
  });

  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => (blob ? resolve(blob) : reject(new Error("Couldn't draw the picture"))), "image/png");
  });
}

/**
 * Draws a picture cropped to fill a box.
 * @param {CanvasRenderingContext2D} ctx
 * @param {CanvasImageSource & { width: number, height: number }} image
 * @param {number} x
 * @param {number} y
 * @param {number} w
 * @param {number} h
 */
function drawCover(ctx, image, x, y, w, h) {
  const scale = Math.max(w / image.width, h / image.height);
  const drawnW = image.width * scale;
  const drawnH = image.height * scale;
  ctx.drawImage(image, x + (w - drawnW) / 2, y + (h - drawnH) / 2, drawnW, drawnH);
}

/**
 * The picture blown up across the whole card, so blurred only its colors
 * are left (shrunk to a few pixels and stretched back, which every canvas
 * can do), shaded darker toward the lists.
 * @param {CanvasRenderingContext2D} ctx
 * @param {HTMLImageElement} image
 * @param {number} width
 * @param {number} height
 */
function drawBlurred(ctx, image, width, height) {
  const tiny = document.createElement("canvas");
  tiny.width = 9;
  tiny.height = 16;
  const small = document.createElement("canvas");
  small.width = 36;
  small.height = 64;
  const tinyCtx = tiny.getContext("2d");
  const smallCtx = small.getContext("2d");
  if (!tinyCtx || !smallCtx) return;
  drawCover(tinyCtx, image, 0, 0, tiny.width, tiny.height);
  smallCtx.imageSmoothingQuality = "high";
  smallCtx.drawImage(tiny, 0, 0, small.width, small.height);
  ctx.save();
  ctx.imageSmoothingQuality = "high";
  ctx.globalAlpha = 0.8;
  ctx.drawImage(small, -width * 0.1, -height * 0.1, width * 1.2, height * 1.2);
  ctx.restore();
  const shade = ctx.createLinearGradient(0, 0, 0, height);
  shade.addColorStop(0, "rgba(0, 0, 0, 0.2)");
  shade.addColorStop(0.5, "rgba(0, 0, 0, 0.32)");
  shade.addColorStop(1, "rgba(0, 0, 0, 0.66)");
  ctx.fillStyle = shade;
  ctx.fillRect(0, 0, width, height);
}

/**
 * A record with the picture on its label, like the Recap's.
 * @param {CanvasRenderingContext2D} ctx
 * @param {HTMLImageElement} image
 * @param {number} cx
 * @param {number} cy
 * @param {number} r
 */
function drawRecord(ctx, image, cx, cy, r) {
  ctx.save();
  ctx.shadowColor = "rgba(0, 0, 0, 0.5)";
  ctx.shadowBlur = 40;
  ctx.shadowOffsetY = 16;
  ctx.beginPath();
  ctx.arc(cx, cy, r, 0, Math.PI * 2);
  ctx.fillStyle = "#161616";
  ctx.fill();
  ctx.restore();

  ctx.save();
  ctx.strokeStyle = "rgba(255, 255, 255, 0.06)";
  ctx.lineWidth = 1.5;
  for (let groove = r * 0.42; groove < r * 0.97; groove += 6) {
    ctx.beginPath();
    ctx.arc(cx, cy, groove, 0, Math.PI * 2);
    ctx.stroke();
  }
  if (typeof ctx.createConicGradient === "function") {
    const sheen = ctx.createConicGradient(0.35, cx, cy);
    sheen.addColorStop(0, "rgba(255, 255, 255, 0.16)");
    sheen.addColorStop(0.1, "rgba(255, 255, 255, 0)");
    sheen.addColorStop(0.48, "rgba(255, 255, 255, 0)");
    sheen.addColorStop(0.58, "rgba(255, 255, 255, 0.12)");
    sheen.addColorStop(0.7, "rgba(255, 255, 255, 0)");
    sheen.addColorStop(1, "rgba(255, 255, 255, 0.16)");
    ctx.beginPath();
    ctx.arc(cx, cy, r, 0, Math.PI * 2);
    ctx.fillStyle = sheen;
    ctx.fill();
  }
  const label = r * 0.36;
  ctx.beginPath();
  ctx.arc(cx, cy, label, 0, Math.PI * 2);
  ctx.clip();
  drawCover(ctx, image, cx - label, cy - label, label * 2, label * 2);
  ctx.restore();

  ctx.beginPath();
  ctx.arc(cx, cy, r * 0.03, 0, Math.PI * 2);
  ctx.fillStyle = "#0d0d0d";
  ctx.fill();
}

/** @param {string} text */
function capitalize(text) {
  return text.replace(/(^|\s)\p{Ll}/gu, (letter) => letter.toUpperCase());
}

async function loadFonts() {
  try {
    await Promise.all(["800 100px", "700 36px", "500 36px", "400 36px"].map((font) => document.fonts.load(`${font} "Gotham"`)));
  } catch {
    // The fallbacks in FONT draw it instead.
  }
}

/**
 * A picture that can be drawn without tainting the canvas (Spotify's allow
 * it), or null if it doesn't load.
 * @param {string} url
 * @returns {Promise<HTMLImageElement|null>}
 */
function loadImage(url) {
  return new Promise((resolve) => {
    const image = new Image();
    const timer = setTimeout(() => resolve(null), IMAGE_TIMEOUT_MS);
    image.crossOrigin = "anonymous";
    image.onload = () => {
      clearTimeout(timer);
      resolve(image);
    };
    image.onerror = () => {
      clearTimeout(timer);
      resolve(null);
    };
    image.src = url;
  });
}

/**
 * The biggest size (down to a floor) at which `text` wraps into `maxLines`
 * lines no wider than `width`. Below the floor, the last line runs long.
 * @param {CanvasRenderingContext2D} ctx
 * @param {string} text
 * @param {number} weight
 * @param {number} width
 * @param {number} maxSize
 * @param {number} maxLines
 */
function fitLines(ctx, text, weight, width, maxSize, maxLines) {
  const words = text.split(/\s+/).filter(Boolean);
  const minSize = Math.min(48, maxSize);
  for (let size = maxSize; size >= minSize; size -= 4) {
    ctx.font = `${weight} ${size}px ${FONT}`;
    const lines = wrap(ctx, words, width);
    if (lines.length <= maxLines && lines.every((line) => ctx.measureText(line).width <= width)) return { size, lines };
  }
  ctx.font = `${weight} ${minSize}px ${FONT}`;
  const lines = wrap(ctx, words, width);
  return { size: minSize, lines: [...lines.slice(0, maxLines - 1), ellipsize(ctx, lines.slice(maxLines - 1).join(" "), width)] };
}

/**
 * @param {CanvasRenderingContext2D} ctx
 * @param {string[]} words
 * @param {number} width
 */
function wrap(ctx, words, width) {
  /** @type {string[]} */
  const lines = [];
  for (const word of words) {
    const last = lines[lines.length - 1];
    if (last !== undefined && ctx.measureText(`${last} ${word}`).width <= width) lines[lines.length - 1] = `${last} ${word}`;
    else lines.push(word);
  }
  return lines;
}

/**
 * @param {CanvasRenderingContext2D} ctx
 * @param {string} text
 * @param {number} width
 */
function ellipsize(ctx, text, width) {
  if (ctx.measureText(text).width <= width) return text;
  const characters = [...text];
  let low = 0;
  let high = characters.length;
  while (low < high) {
    const mid = (low + high + 1) >> 1;
    if (ctx.measureText(`${characters.slice(0, mid).join("").trimEnd()}…`).width <= width) low = mid;
    else high = mid - 1;
  }
  return `${characters.slice(0, low).join("").trimEnd()}…`;
}

// ---- The Recap playlist's cover ----

/** Spotify's limit on a cover, as base64 text. */
const COVER_MAX = 256_000;
const COVER_SIZE = 640;

/**
 * A square cover for the Recap's playlist, as base64 JPEG small enough for
 * Spotify. It's what a link to the playlist shows in Discord and elsewhere.
 * @param {ShareCard} card
 */
export async function drawPlaylistCover(card) {
  const picture = mainPicture(card);
  const [image] = await Promise.all([picture ? loadImage(picture) : Promise.resolve(null), loadFonts()]);
  const canvas = document.createElement("canvas");
  canvas.width = COVER_SIZE;
  canvas.height = COVER_SIZE;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("Couldn't draw the cover");
  const pad = 44;

  ctx.fillStyle = card.color || FALLBACK_COLOR;
  ctx.fillRect(0, 0, COVER_SIZE, COVER_SIZE);
  if (image) {
    drawBlurred(ctx, image, COVER_SIZE, COVER_SIZE);
    // The record sliding off the bottom corner, the picture on its label.
    drawRecord(ctx, image, COVER_SIZE - 130, COVER_SIZE - 120, 230);
  }

  ctx.fillStyle = "#1ed760";
  ctx.beginPath();
  ctx.arc(pad + 10, 62, 10, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = WHITE;
  ctx.font = `700 28px ${FONT}`;
  ctx.fillText("SpotiLarp", pad + 30, 72);

  const year = fitLines(ctx, card.title, 800, COVER_SIZE - pad * 2, 250, 1);
  ctx.font = `800 ${year.size}px ${FONT}`;
  const yearBaseline = 110 + year.size * 0.74;
  ctx.fillText(year.lines[0] ?? "", pad - year.size * 0.04, yearBaseline);
  ctx.font = `800 96px ${FONT}`;
  ctx.fillText("Recap", pad - 3, yearBaseline + 100);

  if (card.name) {
    ctx.font = `700 34px ${FONT}`;
    ctx.fillStyle = DIM;
    ctx.fillText(ellipsize(ctx, card.name, COVER_SIZE / 2 - pad), pad, COVER_SIZE - pad);
  }

  for (let quality = 0.9; quality >= 0.4; quality -= 0.1) {
    const blob = await new Promise((resolve) => canvas.toBlob(resolve, "image/jpeg", quality));
    if (!blob) break;
    const base64 = base64Of(new Uint8Array(await blob.arrayBuffer()));
    if (base64.length <= COVER_MAX) return base64;
  }
  throw new Error("Couldn't make the cover small enough for Spotify");
}

// ---- Saving, copying and the link ----

/** @param {ShareCard} card @param {Blob} png */
export async function saveCardImage(card, png) {
  return api.saveImage(new Uint8Array(await png.arrayBuffer()), cardFileName(card));
}

/** @param {Blob} png */
export async function copyCardImage(png) {
  await navigator.clipboard.write([new ClipboardItem({ "image/png": png })]);
}

/** The picture's id on Spotify's image server, or "" for any other picture. @param {string|null} url */
function coverId(url) {
  return COVER.exec(url ?? "")?.[1] ?? "";
}

/**
 * The link to the card's page. docs/stats/index.html reads it back, so the
 * two change together.
 * @param {ShareCard} card
 */
export async function shareLink(card) {
  const data = {
    v: 1,
    k: card.kind === "recap" ? "r" : "s",
    t: card.title,
    n: card.name || undefined,
    m: Math.round(card.ms / 60_000),
    p: card.plays,
    g: card.genre || undefined,
    c: card.color || undefined,
    a: card.artists.map((artist) => [artist.name, artist.id, coverId(artist.image)]),
    s: card.songs.map((song) => [song.name, song.artist, song.id, coverId(song.image)]),
  };
  return `${SHARE_PAGE}#${await pack(JSON.stringify(data))}`;
}

/**
 * Deflated where the browser can ("z"), else as it is ("j"), then base64url.
 * @param {string} text
 */
async function pack(text) {
  const bytes = new TextEncoder().encode(text);
  if (typeof CompressionStream !== "function") return `j${base64url(bytes)}`;
  const stream = new Blob([bytes]).stream().pipeThrough(new CompressionStream("deflate-raw"));
  return `z${base64url(new Uint8Array(await new Response(stream).arrayBuffer()))}`;
}

/** @param {Uint8Array} bytes */
function base64Of(bytes) {
  let binary = "";
  for (let i = 0; i < bytes.length; i += 0x8000) binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(binary);
}

/** @param {Uint8Array} bytes */
function base64url(bytes) {
  return base64Of(bytes).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}
