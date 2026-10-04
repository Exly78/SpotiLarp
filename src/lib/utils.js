/** @param {import("./types.js").Image[] | null | undefined} images */
export function smallestCover(images) {
  if (!images || images.length === 0) return null;
  return images.reduce((a, b) => ((a.width ?? 0) < (b.width ?? 0) ? a : b)).url;
}

/** @param {number} ms */
export function formatTime(ms) {
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

/** @param {number} ms */
export function formatDuration(ms) {
  const totalMinutes = Math.round(ms / 60000);
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  return hours > 0 ? `${hours} hr ${minutes} min` : `${minutes} min`;
}

/**
 * @param {import("./types.js").Image[] | null | undefined} images
 * @param {number} minWidth
 */
export function coverAtLeast(images, minWidth) {
  if (!images || images.length === 0) return null;
  const sorted = [...images].sort((a, b) => (a.width ?? 0) - (b.width ?? 0));
  return (sorted.find((image) => (image.width ?? 0) >= minWidth) ?? sorted[sorted.length - 1]).url;
}

/** @param {string|null|undefined} date */
export function releaseYear(date) {
  return date ? date.slice(0, 4) : "";
}

const countFormatter = new Intl.NumberFormat();

/** @param {number} count */
export function formatCount(count) {
  return countFormatter.format(count);
}

/** @type {Record<string, string>} */
const RELEASE_TYPES = { album: "Album", single: "Single", ep: "EP", compilation: "Compilation" };

/** @param {string|null|undefined} type */
export function releaseType(type) {
  return RELEASE_TYPES[type ?? ""] ?? "Album";
}

/**
 * Spotify sends bios and playlist descriptions as HTML with links to artists,
 * albums and playlists. Parsed into an inert document (nothing in it loads or
 * runs) and flattened to runs of text, keeping the links.
 * @param {string|null|undefined} html
 * @returns {{ text: string, href?: string }[]}
 */
export function htmlRuns(html) {
  if (!html) return [];
  const body = new DOMParser().parseFromString(html, "text/html").body;
  /** @type {{ text: string, href?: string }[]} */
  const runs = [];
  /** @param {Node} node */
  const walk = (node) => {
    for (const child of node.childNodes) {
      if (child.nodeType === Node.TEXT_NODE) runs.push({ text: child.textContent ?? "" });
      else if (child.nodeName === "BR") runs.push({ text: "\n" });
      else if (child.nodeName === "A") {
        runs.push({ text: child.textContent ?? "", href: /** @type {Element} */ (child).getAttribute("href") ?? undefined });
      } else walk(child);
    }
  };
  walk(body);
  return runs;
}

/** @param {string|null|undefined} html */
export function htmlToText(html) {
  return htmlRuns(html)
    .map((run) => run.text)
    .join("");
}
