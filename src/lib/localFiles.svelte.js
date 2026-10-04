import * as api from "./api.js";
import { notify } from "./toast.svelte.js";

/** @type {{ tracks: import("./types.js").Track[], folders: string[], skipped: number, loading: boolean, loaded: boolean, error: string }} */
export const localFiles = $state({ tracks: [], folders: [], skipped: 0, loading: false, loaded: false, error: "" });

let loadToken = 0;

/** @param {() => Promise<import("./types.js").LocalLibrary>} request */
async function load(request) {
  const token = ++loadToken;
  localFiles.loading = true;
  localFiles.error = "";
  try {
    const library = await request();
    if (token !== loadToken) return;
    localFiles.tracks = library.tracks;
    localFiles.folders = library.folders;
    localFiles.skipped = library.skipped;
    localFiles.loaded = true;
  } catch (e) {
    if (token === loadToken) localFiles.error = `Couldn't load your local files: ${e}`;
  } finally {
    if (token === loadToken) localFiles.loading = false;
  }
}

export const loadLocalFiles = () => load(api.getLocalFiles);
export const rescanLocalFiles = () => load(api.rescanLocalFiles);

/**
 * @param {import("./types.js").Settings} settings
 * @param {string[]} folders
 */
async function saveFolders(settings, folders) {
  const next = { ...settings, local_folders: folders };
  await api.setSettings(next);
  loadLocalFiles();
  return next;
}

/** @returns {Promise<import("./types.js").Settings|null>} the saved settings, or null if nothing was added */
export async function addLocalFolder() {
  const folder = await api.pickFolder();
  if (!folder) return null;
  const settings = await api.getSettings();
  if (settings.local_folders.includes(folder)) {
    notify("That folder is already in Local Files");
    return null;
  }
  return saveFolders(settings, [...settings.local_folders, folder]);
}

/** @param {string} folder */
export async function removeLocalFolder(folder) {
  const settings = await api.getSettings();
  return saveFolders(
    settings,
    settings.local_folders.filter((f) => f !== folder),
  );
}
