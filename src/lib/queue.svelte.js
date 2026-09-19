import { listen } from "@tauri-apps/api/event";
import * as api from "./api.js";

/** @type {{ tracks: import("./types.js").Track[], currentIndex: number, repeatMode: string }} */
export const queue = $state({ tracks: [], currentIndex: -1, repeatMode: "off" });

const REPEAT_MODES = ["off", "all", "one"];

export function cycleRepeatMode() {
  const i = REPEAT_MODES.indexOf(queue.repeatMode);
  queue.repeatMode = REPEAT_MODES[(i + 1) % REPEAT_MODES.length];
}

export function hasNext() {
  if (queue.currentIndex < 0 || queue.tracks.length === 0) return false;
  if (queue.currentIndex < queue.tracks.length - 1) return true;
  
  return queue.repeatMode === "all" && queue.tracks.length > 1;
}

export function hasPrevious() {
  if (queue.currentIndex <= 0) {
    return queue.currentIndex === 0 && queue.repeatMode === "all" && queue.tracks.length > 1;
  }
  return true;
}

/**
 * @param {import("./types.js").Track[]} list
 * @param {number} index
 */
export async function playFromList(list, index) {
  queue.tracks = list;
  queue.currentIndex = index;
  const track = queue.tracks[queue.currentIndex];
  if (track) {
    await api.playTrack(track.uri);
  }
}

export async function next() {
  if (!hasNext()) return;
  queue.currentIndex = queue.currentIndex < queue.tracks.length - 1 ? queue.currentIndex + 1 : 0;
  await api.playTrack(queue.tracks[queue.currentIndex].uri);
}

export async function previous() {
  if (!hasPrevious()) return;
  queue.currentIndex = queue.currentIndex > 0 ? queue.currentIndex - 1 : queue.tracks.length - 1;
  await api.playTrack(queue.tracks[queue.currentIndex].uri);
}

listen("player-event", (event) => {
  if (event.payload.type !== "EndOfTrack") return;
  if (queue.repeatMode === "one") {
    
    const track = queue.tracks[queue.currentIndex];
    if (track) api.playTrack(track.uri);
  } else {
    next();
  }
});
