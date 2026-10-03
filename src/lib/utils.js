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
