import { listen } from "@tauri-apps/api/event";
import * as api from "./api.js";
import { player, togglePlay, previousOrRestart, seekTo, setVolume, toggleMute } from "./player.svelte.js";
import { next, toggleShuffle, setShuffleMode, cycleRepeatMode, setRepeatMode } from "./queue.svelte.js";
import { toggleLike } from "./likes.svelte.js";
import { navigate } from "./nav.svelte.js";
import { openSpotifyLink } from "./openLink.js";
import { resetTheme } from "./themes.svelte.js";

/**
 * Acts on `spotilarp <verb>` from the command line (see remote.rs).
 * @param {import("./types.js").RemoteCommand} command
 */
export function runRemoteCommand(command) {
  switch (command.action) {
    case "toggle":
      togglePlay();
      break;
    case "play":
      if (!player.isPlaying) togglePlay();
      break;
    case "pause":
      if (player.isPlaying) togglePlay();
      break;
    case "next":
      next();
      break;
    case "previous":
      previousOrRestart();
      break;
    case "volume":
      setVolume(clamp(command.percent, 0, 100));
      break;
    case "volume_by":
      setVolume(clamp(player.volume + command.delta, 0, 100));
      break;
    case "mute":
      toggleMute();
      break;
    case "shuffle":
      if (command.mode) setShuffleMode(command.mode);
      else toggleShuffle();
      break;
    case "repeat":
      if (command.mode) setRepeatMode(command.mode);
      else cycleRepeatMode();
      break;
    case "like":
      toggleLike(player.track?.trackId);
      break;
    case "seek_by":
      if (player.track) seekTo(clamp(player.positionMs + command.seconds * 1000, 0, player.track.durationMs));
      break;
    case "seek_to":
      if (player.track) seekTo(clamp(command.seconds * 1000, 0, player.track.durationMs));
      break;
    case "open":
      openSpotifyLink(command.link, { play: true });
      break;
    case "search":
      navigate({ type: "search", query: command.query });
      break;
    case "reset_theme":
      resetTheme();
      break;
  }
}

/**
 * @param {number} value
 * @param {number} min
 * @param {number} max
 */
function clamp(value, min, max) {
  return Math.min(Math.max(value, min), max);
}

listen("remote-command", (event) => runRemoteCommand(/** @type {import("./types.js").RemoteCommand} */ (event.payload)));

/** Runs what this launch was started with (a link, say), once the user is logged in. */
export async function runLaunchCommands() {
  try {
    for (const command of await api.takeLaunchCommands()) runRemoteCommand(command);
  } catch {}
}
