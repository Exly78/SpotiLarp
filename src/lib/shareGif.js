import { GIFEncoder, applyPalette, quantize } from "gifenc";
import {
  DIM,
  FALLBACK_COLOR,
  FONT,
  WHITE,
  capitalize,
  cardSubtitle,
  drawBlurred,
  drawCover,
  drawRecord,
  ellipsize,
  fitLines,
  loadFonts,
  loadImage,
  mainPicture,
} from "./shareCard.js";
import { formatCount } from "./utils.js";

/**
 * A stats card as a small looping GIF, a few scenes in the Recap's style,
 * for posting as a link: Discord plays a GIF link right in the chat.
 *
 * @typedef {import("./shareCard.js").ShareCard} ShareCard
 * @typedef {{ main: HTMLImageElement|null, songs: (HTMLImageElement|null)[] }} Pictures
 * @typedef {{ frames: number, hold: number, draw: (ctx: CanvasRenderingContext2D, t: number) => void }} Scene
 *   `t` runs from 0 to 1 as the scene comes in; the last frame is held for `hold` ms
 */

const SIZE = 420;
const PAD = 30;
const FRAME_MS = 50;
const INK_GREEN = "#04210f";

/** @param {number} t */
const ease = (t) => 1 - (1 - Math.min(1, Math.max(0, t))) ** 3;

/**
 * @param {ShareCard} card
 * @returns {Promise<Uint8Array>}
 */
export async function drawShareGif(card) {
  const [main, ...songs] = await Promise.all([
    (() => {
      const picture = mainPicture(card);
      return picture ? loadImage(picture) : Promise.resolve(null);
    })(),
    ...card.songs.map((song) => (song.image ? loadImage(song.image) : Promise.resolve(null))),
    loadFonts().then(() => null),
  ]);
  /** @type {Pictures} */
  const pictures = { main, songs: songs.slice(0, card.songs.length) };

  const canvas = document.createElement("canvas");
  canvas.width = SIZE;
  canvas.height = SIZE;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) throw new Error("Couldn't draw the GIF");

  const gif = GIFEncoder();
  for (const scene of scenes(card, pictures)) {
    for (let frame = 0; frame < scene.frames; frame++) {
      const t = scene.frames === 1 ? 1 : frame / (scene.frames - 1);
      ctx.save();
      scene.draw(ctx, t);
      ctx.restore();
      const { data } = ctx.getImageData(0, 0, SIZE, SIZE);
      const palette = quantize(data, 256);
      const last = frame === scene.frames - 1;
      gif.writeFrame(applyPalette(data, palette), SIZE, SIZE, { palette, delay: last ? scene.hold : FRAME_MS });
    }
  }
  gif.finish();
  return gif.bytes();
}

/**
 * @param {ShareCard} card
 * @param {Pictures} pictures
 * @returns {Scene[]}
 */
function scenes(card, pictures) {
  const minutes = Math.round(card.ms / 60_000);
  const top = card.artists[0];
  /** @type {Scene[]} */
  const list = [
    // Whose year (or stretch of it) this is.
    {
      frames: 8,
      hold: 1500,
      draw(ctx, t) {
        const e = ease(t);
        ctx.fillStyle = "#1ed760";
        ctx.fillRect(0, 0, SIZE, SIZE);
        band(ctx, card.kind === "recap" ? "Larped" : "Stats", 70, "rgba(4, 33, 15, 0.22)", t);
        band(ctx, card.kind === "recap" ? "Larped" : "Stats", 430, "rgba(4, 33, 15, 0.22)", 1 - t);
        brand(ctx, INK_GREEN);
        ctx.globalAlpha = e;
        ctx.fillStyle = INK_GREEN;
        const title = fitLines(ctx, card.title, 800, SIZE - PAD * 2, card.kind === "recap" ? 150 : 84, 2);
        ctx.font = `800 ${title.size}px ${FONT}`;
        let y = 290 - (title.lines.length - 1) * title.size * 0.92 + (1 - e) * 40;
        for (const line of title.lines) {
          ctx.fillText(line, PAD - 4, y);
          y += title.size * 0.92;
        }
        ctx.font = `700 26px ${FONT}`;
        ctx.fillText(ellipsize(ctx, cardSubtitle(card), SIZE - PAD * 2), PAD, 350 + (1 - e) * 30);
      },
    },
    // Time listened, counting up.
    {
      frames: 12,
      hold: 1500,
      draw(ctx, t) {
        const e = ease(t);
        ctx.fillStyle = "#2340d8";
        ctx.fillRect(0, 0, SIZE, SIZE);
        ctx.fillStyle = "#f9e65b";
        ctx.font = `700 28px ${FONT}`;
        ctx.fillText(card.kind === "recap" ? `In ${card.title} I listened for` : "I listened for", PAD, 150);
        const figure = formatCount(Math.round(minutes * e));
        const fitted = fitLines(ctx, formatCount(minutes), 800, SIZE - PAD * 2, 120, 1);
        ctx.font = `800 ${fitted.size}px ${FONT}`;
        ctx.fillText(figure, PAD - 4, 160 + fitted.size * 0.85);
        ctx.font = `700 34px ${FONT}`;
        ctx.fillText("minutes", PAD, 205 + fitted.size * 0.85);
        band(ctx, "minutes", 455, "rgba(249, 230, 91, 0.3)", t);
      },
    },
  ];

  if (top) {
    // The top artist, over their own colors.
    list.push({
      frames: 8,
      hold: 1600,
      draw(ctx, t) {
        const e = ease(t);
        ctx.fillStyle = card.color || FALLBACK_COLOR;
        ctx.fillRect(0, 0, SIZE, SIZE);
        if (pictures.main) {
          drawBlurred(ctx, pictures.main, SIZE, SIZE);
          const r = 92 * (0.85 + 0.15 * e);
          ctx.save();
          ctx.globalAlpha = e;
          ctx.shadowColor = "rgba(0, 0, 0, 0.45)";
          ctx.shadowBlur = 30;
          ctx.shadowOffsetY = 10;
          ctx.beginPath();
          ctx.arc(SIZE / 2, 150, r, 0, Math.PI * 2);
          ctx.fillStyle = "#000000";
          ctx.fill();
          ctx.restore();
          ctx.save();
          ctx.globalAlpha = e;
          ctx.beginPath();
          ctx.arc(SIZE / 2, 150, r, 0, Math.PI * 2);
          ctx.clip();
          drawCover(ctx, pictures.main, SIZE / 2 - r, 150 - r, r * 2, r * 2);
          ctx.restore();
        }
        ctx.fillStyle = DIM;
        ctx.font = `700 22px ${FONT}`;
        ctx.textAlign = "center";
        ctx.fillText(card.kind === "recap" ? "My top artist" : "Most played artist", SIZE / 2, 296);
        ctx.fillStyle = WHITE;
        ctx.globalAlpha = e;
        const name = fitLines(ctx, top.name, 800, SIZE - PAD * 2, 50, 2);
        ctx.font = `800 ${name.size}px ${FONT}`;
        let y = 296 + name.size + (1 - e) * 24;
        for (const line of name.lines) {
          ctx.fillText(line, SIZE / 2, y);
          y += name.size * 0.98;
        }
      },
    });
  }

  if (card.songs.length > 0) {
    // The top songs, sliding in one after another.
    list.push({
      frames: 12,
      hold: 2200,
      draw(ctx, t) {
        ctx.fillStyle = "#ff8fb1";
        ctx.fillRect(0, 0, SIZE, SIZE);
        ctx.fillStyle = "#3d0b1c";
        ctx.font = `800 34px ${FONT}`;
        ctx.fillText("My top songs", PAD, 72);
        card.songs.forEach((song, i) => {
          const e = ease(t * 1.7 - i * 0.16);
          const y = 104 + i * 60;
          const x = PAD - (1 - e) * 60;
          ctx.globalAlpha = e;
          ctx.font = `800 30px ${FONT}`;
          ctx.fillStyle = "#3d0b1c";
          ctx.fillText(String(i + 1), x, y + 34);
          const cover = pictures.songs[i];
          ctx.save();
          ctx.beginPath();
          ctx.roundRect(x + 32, y, 46, 46, 5);
          ctx.clip();
          if (cover) drawCover(ctx, cover, x + 32, y, 46, 46);
          else {
            ctx.fillStyle = "rgba(61, 11, 28, 0.16)";
            ctx.fillRect(x + 32, y, 46, 46);
          }
          ctx.restore();
          ctx.fillStyle = "#3d0b1c";
          ctx.font = `700 21px ${FONT}`;
          ctx.fillText(ellipsize(ctx, song.name, SIZE - PAD - x - 92), x + 92, y + 22);
          ctx.font = `400 16px ${FONT}`;
          ctx.fillText(ellipsize(ctx, song.artist, SIZE - PAD - x - 92), x + 92, y + 42);
        });
        ctx.globalAlpha = 1;
      },
    });
  }

  // The top genre (or plays), and where it came from.
  list.push({
    frames: 8,
    hold: 2000,
    draw(ctx, t) {
      const e = ease(t);
      ctx.fillStyle = "#14123a";
      ctx.fillRect(0, 0, SIZE, SIZE);
      if (pictures.main) {
        ctx.save();
        ctx.translate(SIZE - 70, SIZE - 60);
        ctx.rotate(t * Math.PI * 0.6);
        drawRecord(ctx, pictures.main, 0, 0, 150);
        ctx.restore();
      }
      ctx.globalAlpha = e;
      ctx.fillStyle = "#ff9e5e";
      ctx.font = `700 24px ${FONT}`;
      ctx.fillText(card.genre ? "My top genre" : "Plays", PAD, 120);
      const value = card.genre ? capitalize(card.genre) : formatCount(card.plays);
      const fitted = fitLines(ctx, value, 800, SIZE - PAD * 2, 64, 2);
      ctx.font = `800 ${fitted.size}px ${FONT}`;
      let y = 120 + fitted.size + (1 - e) * 24;
      for (const line of fitted.lines) {
        ctx.fillText(line, PAD - 2, y);
        y += fitted.size * 0.98;
      }
      ctx.globalAlpha = 1;
      brand(ctx, "#ff9e5e", SIZE - 64);
      ctx.fillStyle = "rgba(255, 158, 94, 0.7)";
      ctx.font = `400 15px ${FONT}`;
      ctx.fillText("github.com/Exly78/SpotiLarp", PAD, SIZE - 36);
    },
  });
  return list;
}

/**
 * "SpotiLarp" with its dot.
 * @param {CanvasRenderingContext2D} ctx
 * @param {string} color
 * @param {number} [y]
 */
function brand(ctx, color, y = 44) {
  ctx.fillStyle = color;
  ctx.beginPath();
  ctx.arc(PAD + 8, y - 7, 8, 0, Math.PI * 2);
  ctx.fill();
  ctx.font = `700 22px ${FONT}`;
  ctx.fillText("SpotiLarp", PAD + 24, y);
}

/**
 * Outlined words across the scene, drifting as `shift` goes from 0 to 1.
 * @param {CanvasRenderingContext2D} ctx
 * @param {string} word
 * @param {number} y
 * @param {string} color
 * @param {number} shift
 */
function band(ctx, word, y, color, shift) {
  ctx.save();
  ctx.translate(-40 - shift * 60, y);
  ctx.rotate(-0.12);
  ctx.font = `800 96px ${FONT}`;
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.5;
  ctx.strokeText(`${word} ${word} ${word}`, 0, 0);
  ctx.restore();
}
