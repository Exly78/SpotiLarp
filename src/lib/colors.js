/** @type {Map<string, Promise<string>>} */
const cache = new Map();

/**
 * @param {string|null|undefined} url
 * @returns {Promise<string>}
 */
export function dominantColor(url) {
  if (!url) return Promise.resolve("");
  let result = cache.get(url);
  if (!result) {
    result = extract(url);
    cache.set(url, result);
  }
  return result;
}

/** @param {string} url */
function extract(url) {
  return new Promise((resolve) => {
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.onload = () => {
      try {
        const size = 32;
        const canvas = document.createElement("canvas");
        canvas.width = size;
        canvas.height = size;
        const ctx = canvas.getContext("2d");
        if (!ctx) return resolve("");
        ctx.drawImage(img, 0, 0, size, size);
        const { data } = ctx.getImageData(0, 0, size, size);

        const bucketCount = 24;
        const buckets = Array.from({ length: bucketCount }, () => ({ count: 0, hSum: 0, sSum: 0, lSum: 0 }));
        let sampled = 0;
        for (let i = 0; i < data.length; i += 4) {
          const [h, s, l] = rgbToHsl(data[i], data[i + 1], data[i + 2]);
          sampled++;
          if (l < 8 || l > 92 || s < 10) continue;
          const bucket = buckets[Math.floor(h / (360 / bucketCount)) % bucketCount];
          bucket.count++;
          bucket.hSum += h;
          bucket.sSum += s;
          bucket.lSum += l;
        }

        let best = null;
        let bestScore = 0;
        for (const bucket of buckets) {
          if (bucket.count === 0) continue;
          const avgS = bucket.sSum / bucket.count;
          const score = bucket.count * avgS;
          if (score > bestScore) {
            bestScore = score;
            best = { h: bucket.hSum / bucket.count, s: avgS, l: bucket.lSum / bucket.count, count: bucket.count };
          }
        }
        resolve(best && best.count / sampled >= 0.03 ? vividize(best.h, best.s, best.l) : "");
      } catch {
        resolve("");
      }
    };
    img.onerror = () => resolve("");
    img.src = url;
  });
}

/**
 * A color Spotify picked for an image ("#rrggbb"), toned like the ones taken
 * from cover art so white text stays readable on it. Spotify's are often
 * near-white.
 * @param {string} hex
 */
export function tintFromHex(hex) {
  const match = /^#?([0-9a-f]{6})$/i.exec(hex);
  if (!match) return "";
  const n = parseInt(match[1], 16);
  const [h, s, l] = rgbToHsl((n >> 16) & 255, (n >> 8) & 255, n & 255);
  return vividize(h, s, l);
}

/**
 * @param {number} h
 * @param {number} s
 * @param {number} l
 */
function vividize(h, s, l) {
  if (s >= 35) {
    const boundedS = Math.min(62, s);
    const boundedL = Math.min(46, Math.max(l, 26));
    const [r, g, b] = hslToRgb(h, boundedS, boundedL);
    return `rgb(${r}, ${g}, ${b})`;
  }
  const boundedL = Math.min(30, Math.max(l * 0.55, 12));
  const [r, g, b] = hslToRgb(h, s, boundedL);
  return `rgb(${r}, ${g}, ${b})`;
}

/**
 * @param {number} r
 * @param {number} g
 * @param {number} b
 */
function rgbToHsl(r, g, b) {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  let h = 0;
  let s = 0;
  const l = (max + min) / 2;
  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r:
        h = (g - b) / d + (g < b ? 6 : 0);
        break;
      case g:
        h = (b - r) / d + 2;
        break;
      default:
        h = (r - g) / d + 4;
    }
    h /= 6;
  }
  return [h * 360, s * 100, l * 100];
}

/**
 * @param {number} h
 * @param {number} s
 * @param {number} l
 */
function hslToRgb(h, s, l) {
  h /= 360;
  s /= 100;
  l /= 100;
  if (s === 0) {
    const v = Math.round(l * 255);
    return [v, v, v];
  }
  /**
   * @param {number} p
   * @param {number} q
   * @param {number} t
   */
  const hue2rgb = (p, q, t) => {
    if (t < 0) t += 1;
    if (t > 1) t -= 1;
    if (t < 1 / 6) return p + (q - p) * 6 * t;
    if (t < 1 / 2) return q;
    if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
    return p;
  };
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  return [
    Math.round(hue2rgb(p, q, h + 1 / 3) * 255),
    Math.round(hue2rgb(p, q, h) * 255),
    Math.round(hue2rgb(p, q, h - 1 / 3) * 255),
  ];
}
