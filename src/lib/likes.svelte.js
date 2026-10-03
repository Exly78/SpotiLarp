import { SvelteSet } from "svelte/reactivity";
import * as api from "./api.js";
import { notify } from "./toast.svelte.js";

export const likedIds = new SvelteSet();
export const likeBusy = new SvelteSet();

/** @param {string[]} ids */
export async function refreshLiked(ids) {
  const unique = [...new Set(ids.filter(Boolean))];
  if (unique.length === 0) return;
  try {
    const results = await api.areTracksLiked(unique);
    unique.forEach((id, i) => {
      if (likeBusy.has(id)) return;
      if (results[i]) likedIds.add(id);
      else likedIds.delete(id);
    });
  } catch (e) {
    console.error("Failed to check liked tracks:", e);
  }
}

/** @param {string[]} ids */
export function markLiked(ids) {
  for (const id of ids) if (id) likedIds.add(id);
}

/** @param {string|null|undefined} id */
export async function toggleLike(id) {
  if (!id || likeBusy.has(id)) return;
  likeBusy.add(id);
  try {
    if (likedIds.has(id)) {
      await api.unlikeTrack(id);
      likedIds.delete(id);
    } else {
      await api.likeTrack(id);
      likedIds.add(id);
    }
  } catch (e) {
    notify(`Couldn't update Liked Songs: ${e}`);
  } finally {
    likeBusy.delete(id);
  }
}

/** @param {string[]} ids */
export async function likeTracks(ids) {
  const toLike = [...new Set(ids.filter((id) => id && !likedIds.has(id)))];
  try {
    for (const id of toLike) {
      await api.likeTrack(id);
      likedIds.add(id);
    }
    notify(ids.length === 1 ? "Added to Liked Songs" : `Added ${toLike.length} songs to Liked Songs`);
  } catch (e) {
    notify(`Couldn't update Liked Songs: ${e}`);
  }
}
