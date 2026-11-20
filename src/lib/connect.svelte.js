import { listen } from "@tauri-apps/api/event";
import * as api from "./api.js";
import { notify } from "./toast.svelte.js";

/**
 * Spotify Connect (playback/connect.rs): where SpotiLarp's music comes out.
 * "here" is SpotiLarp's own players. "receiving" is another Spotify app
 * playing on SpotiLarp, with its own queue (next and previous are its). "remote"
 * is SpotiLarp's queue playing on another device, which the backend drives
 * and reports on with the usual player events.
 * @type {{ output: "here"|"receiving"|"remote", device: string|null, deviceId: string|null }}
 */
export const connect = $state({ output: "here", device: null, deviceId: null });

/** @type {((from: string, to: string) => void)[]} */
const changeHandlers = [];

/**
 * Called when the music moves (player.svelte.js puts SpotiLarp's own song back).
 * @param {(from: string, to: string) => void} handler
 */
export function onOutputChange(handler) {
  changeHandlers.push(handler);
}

/** @param {import("./api.js").ConnectStatus} status */
export function applyStatus(status) {
  const from = connect.output;
  connect.output = status.output;
  connect.device = status.device;
  connect.deviceId = status.device_id;
  if (status.reason) notify(status.reason);
  if (from !== status.output) for (const handler of changeHandlers) handler(from, status.output);
}

listen("connect-mode", (event) => applyStatus(/** @type {import("./api.js").ConnectStatus} */ (event.payload)));

// The page reloading doesn't move the music: pick up where it is.
api.connectStatus().then(applyStatus, () => {});
