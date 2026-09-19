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
