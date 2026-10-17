/**
 * Whether keys pressed in `target` are typing, which shortcuts leave alone.
 * @param {EventTarget|null} target
 */
export function isTextField(target) {
  if (target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) return true;
  if (target instanceof HTMLElement && target.isContentEditable) return true;
  return target instanceof HTMLInputElement && !["range", "checkbox", "radio", "button"].includes(target.type);
}

/** Every keyboard shortcut, for Settings and the shortcuts overlay (`?`). */
export const SHORTCUTS = [
  ["Space", "Play / pause"],
  ["Ctrl+Right / Ctrl+Left", "Next / previous song"],
  ["Shift+Right / Shift+Left", "Skip forward / back 5 seconds"],
  ["Ctrl+Up / Ctrl+Down", "Volume up / down"],
  ["M", "Mute / unmute"],
  ["B", "Like / unlike the playing song"],
  ["S or Ctrl+S", "Shuffle: off, on, smart (spreads artists out, plays recent songs last)"],
  ["R or Ctrl+R", "Repeat"],
  ["Q", "Show / hide the queue"],
  ["L", "Show / hide lyrics"],
  ["F", "Full-screen lyrics (Esc leaves)"],
  ["Ctrl+F or /", "Search (paste a Spotify link and press Enter to open it)"],
  ["Alt+Left / Alt+Right", "Back / forward"],
  ["Ctrl+H / Ctrl+L", "Home / Liked Songs"],
  ["Ctrl+Shift+A / Ctrl+Shift+B", "Go to the playing artist / album (or podcast)"],
  ["Ctrl+B", "Show / hide the library"],
  ["Ctrl+= / Ctrl+- / Ctrl+0", "Zoom in / out / back to 100% (or Ctrl+scroll)"],
  ["Ctrl+,", "Settings"],
  ["Ctrl+click / Shift+click", "Select songs in a list (then right-click or drag them)"],
  ["Ctrl+A", "Select every song in the list"],
  ["Ctrl+C / Ctrl+X", "Copy the selected songs' links / cut them from your playlist"],
  ["Ctrl+V", "Add copied song links to your playlist or Liked Songs"],
  ["Delete", "Remove the selected songs from your playlist"],
  ["? or Ctrl+/", "Show these shortcuts"],
  ["Ctrl+Q", "Quit"],
  ["Media keys", "Play, pause, next, previous (also from the Windows media overlay)"],
];
