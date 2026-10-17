import * as api from "./api.js";

/**
 * Themes work like Spicetify's: a color scheme (color.ini) sets the colors,
 * and an optional user.css restyles more. A scheme's colors become
 * `--spice-<key>` and `--spice-rgb-<key>` variables, as in Spicetify, and
 * SpotiLarp's own colors are drawn from them. user.css written for Spotify's
 * window aims at Spotify's own page, so only some of it (fonts, scrollbars,
 * general rules) carries over.
 */

const PREFS_KEY = "spotilarp.theme";
/** The styles last applied, so a restart shows them before anything loads. */
const APPLIED_KEY = "spotilarp.themeStyles";
const COLORS_STYLE_ID = "spotilarp-theme-colors";
const CSS_STYLE_ID = "spotilarp-theme-css";

export const BUILT_IN_ID = "builtin:SpotiLarp";

/**
 * @typedef {{ id: string, name: string, source: "builtin"|"own"|"spicetify", schemes: string[], has_css: boolean }} ThemeInfo
 * @typedef {{ name: string, colors: Record<string, string> }} ColorScheme
 * @typedef {{ id: string, scheme: string, css: boolean }} ThemePrefs
 */

/**
 * Built-in schemes, in color.ini's keys. "Dark" is SpotiLarp's own look.
 * @type {ColorScheme[]}
 */
const BUILT_IN_SCHEMES = [
  { name: "Dark", colors: {} },
  {
    name: "Light",
    colors: {
      text: "121212",
      subtext: "5c5c5c",
      main: "ffffff",
      sidebar: "eeeeee",
      player: "eeeeee",
      card: "f4f4f4",
      highlight: "e8e8e8",
      "highlight-elevated": "dcdcdc",
      button: "1aa34a",
      "button-active": "1db954",
      "button-disabled": "b8b8b8",
      "selected-row": "000000",
      notification: "2e77d0",
      "notification-error": "d31225",
    },
  },
  {
    name: "Nord",
    colors: {
      text: "eceff4",
      subtext: "a9b1c1",
      main: "2e3440",
      sidebar: "272c36",
      player: "272c36",
      card: "353c4a",
      highlight: "3b4252",
      "highlight-elevated": "434c5e",
      button: "88c0d0",
      "button-active": "8fbcbb",
      "button-disabled": "4c566a",
      notification: "5e81ac",
      "notification-error": "bf616a",
    },
  },
  {
    name: "Dracula",
    colors: {
      text: "f8f8f2",
      subtext: "bcc0d4",
      main: "282a36",
      sidebar: "21222c",
      player: "21222c",
      card: "2f3241",
      highlight: "363948",
      "highlight-elevated": "44475a",
      button: "bd93f9",
      "button-active": "ff79c6",
      "button-disabled": "44475a",
      notification: "6272a4",
      "notification-error": "ff5555",
    },
  },
  {
    name: "Catppuccin Mocha",
    colors: {
      text: "cdd6f4",
      subtext: "a6adc8",
      main: "1e1e2e",
      sidebar: "181825",
      player: "11111b",
      card: "262637",
      highlight: "313244",
      "highlight-elevated": "45475a",
      button: "cba6f7",
      "button-active": "f5c2e7",
      "button-disabled": "45475a",
      notification: "89b4fa",
      "notification-error": "f38ba8",
    },
  },
  {
    name: "Gruvbox",
    colors: {
      text: "ebdbb2",
      subtext: "a89984",
      main: "282828",
      sidebar: "1d2021",
      player: "1d2021",
      card: "32302f",
      highlight: "3c3836",
      "highlight-elevated": "504945",
      button: "fe8019",
      "button-active": "fabd2f",
      "button-disabled": "504945",
      notification: "458588",
      "notification-error": "fb4934",
    },
  },
];

/** @type {ThemeInfo} */
const BUILT_IN = {
  id: BUILT_IN_ID,
  name: "SpotiLarp",
  source: "builtin",
  schemes: BUILT_IN_SCHEMES.map((s) => s.name),
  has_css: false,
};

/** @type {{ list: ThemeInfo[], prefs: ThemePrefs, loading: boolean, error: string }} */
export const themes = $state({ list: [BUILT_IN], prefs: readPrefs(), loading: false, error: "" });

let applyVersion = 0;

/** @returns {ThemePrefs} */
function readPrefs() {
  try {
    const saved = JSON.parse(localStorage.getItem(PREFS_KEY) ?? "null");
    if (saved && typeof saved.id === "string") {
      return { id: saved.id, scheme: String(saved.scheme ?? ""), css: !!saved.css };
    }
  } catch {}
  return { id: BUILT_IN_ID, scheme: "Dark", css: false };
}

/** @param {ThemePrefs} prefs */
function savePrefs(prefs) {
  try {
    localStorage.setItem(PREFS_KEY, JSON.stringify(prefs));
  } catch {}
}

/**
 * @param {string} id
 * @param {string} text
 */
function setStyle(id, text) {
  let el = document.getElementById(id);
  if (!text) {
    el?.remove();
    return;
  }
  if (!el) {
    el = document.createElement("style");
    el.id = id;
  }
  el.textContent = text;
  // Last in <head>, after the app's own styles, so the theme wins.
  document.head.appendChild(el);
}

/**
 * @param {string} colors
 * @param {string} css
 */
function applyStyles(colors, css) {
  setStyle(COLORS_STYLE_ID, colors);
  // user.css after the colors, so it can change them.
  setStyle(CSS_STYLE_ID, css);
  try {
    if (colors || css) localStorage.setItem(APPLIED_KEY, JSON.stringify({ colors, css }));
    else localStorage.removeItem(APPLIED_KEY);
  } catch {}
}

// What was showing last time, at once (the theme's files are read again below).
try {
  const applied = JSON.parse(localStorage.getItem(APPLIED_KEY) ?? "null");
  if (applied) {
    setStyle(COLORS_STYLE_ID, String(applied.colors ?? ""));
    setStyle(CSS_STYLE_ID, String(applied.css ?? ""));
  }
} catch {}

/** Finds the themes in SpotiLarp's Themes folder and Spicetify's. */
export async function loadThemes() {
  themes.loading = true;
  themes.error = "";
  try {
    /** @type {ThemeInfo[]} */
    const found = await api.listThemes();
    themes.list = [BUILT_IN, ...found];
  } catch (e) {
    themes.error = `Couldn't look for themes: ${e}`;
  } finally {
    themes.loading = false;
  }
}

/**
 * Switches theme. Without a scheme, the theme's first; without `css`, the
 * theme's CSS is used for SpotiLarp's own themes and left off for
 * Spicetify's (made for Spotify's window).
 * @param {string} id
 * @param {{ scheme?: string, css?: boolean }} [options]
 */
export async function selectTheme(id, { scheme, css } = {}) {
  const info = themes.list.find((t) => t.id === id);
  const prefs = {
    id,
    scheme: scheme ?? info?.schemes[0] ?? "",
    css: css ?? (info ? info.source === "own" : false),
  };
  themes.prefs = prefs;
  savePrefs(prefs);
  await applyTheme(prefs);
}

/** Back to the built-in dark look (also `spotilarp reset-theme`). */
export function resetTheme() {
  return selectTheme(BUILT_IN_ID, { scheme: "Dark", css: false });
}

/**
 * Reads the chosen theme's files again (after editing them, say) and applies it.
 * @param {ThemePrefs} [prefs]
 */
export async function applyTheme(prefs = themes.prefs) {
  const version = ++applyVersion;
  if (prefs.id === BUILT_IN_ID) {
    const scheme = BUILT_IN_SCHEMES.find((s) => s.name === prefs.scheme) ?? BUILT_IN_SCHEMES[0];
    applyStyles(Object.keys(scheme.colors).length ? colorsCss(scheme.colors) : "", "");
    return;
  }
  try {
    /** @type {{ schemes: ColorScheme[], css: string|null }} */
    const data = await api.getTheme(prefs.id);
    if (version !== applyVersion) return;
    const scheme = data.schemes.find((s) => s.name === prefs.scheme) ?? data.schemes[0];
    applyStyles(scheme ? colorsCss(scheme.colors) : "", prefs.css && data.css ? data.css : "");
  } catch (e) {
    if (version !== applyVersion) return;
    themes.error = `${e}`;
    // The theme was removed: the built-in look rather than half a theme.
    applyStyles("", "");
  }
}

/** Spotify's own colors, which Spicetify fills in for keys a scheme leaves out. */
const SPOTIFY_DEFAULTS = {
  text: "ffffff",
  subtext: "b3b3b3",
  main: "121212",
  notification: "2e77d0",
  "notification-error": "e22134",
  shadow: "000000",
};

/**
 * The CSS for a color scheme: Spicetify's variables, and SpotiLarp's colors
 * made from them. Schemes name things differently (the "text" theme has
 * `accent` where most have `button`), so each color falls back on the
 * likeliest stand-ins, and then on a mix of text and background.
 * @param {Record<string, string>} colors color.ini keys to hex, without `#`
 */
export function colorsCss(colors) {
  /** @param {...string} keys */
  const pick = (...keys) => keys.map((key) => colors[key]).find((value) => value && isHex(value));
  const text = pick("text") ?? SPOTIFY_DEFAULTS.text;
  const main = pick("main") ?? SPOTIFY_DEFAULTS.main;
  const button = pick("button", "accent", "play-button", "nav-active", "banner", "playback-bar") ?? "1ed760";
  const sidebar = pick("sidebar", "main-secondary") ?? main;
  /** @type {Record<string, string>} */
  const resolved = {
    ...colors,
    text,
    subtext: pick("subtext") ?? mix(text, main, 0.7),
    main,
    sidebar,
    player: pick("player") ?? sidebar,
    card: pick("card", "main-elevated") ?? mix(text, main, 0.04),
    highlight: pick("highlight") ?? mix(text, main, 0.1),
    "highlight-elevated": pick("highlight-elevated") ?? mix(text, main, 0.18),
    button,
    "button-active": pick("button-active", "accent-active") ?? button,
    "button-disabled": pick("button-disabled", "border-inactive") ?? mix(text, main, 0.3),
    "selected-row": pick("selected-row") ?? text,
    notification: pick("notification") ?? SPOTIFY_DEFAULTS.notification,
    "notification-error": pick("notification-error") ?? SPOTIFY_DEFAULTS["notification-error"],
    shadow: pick("shadow") ?? SPOTIFY_DEFAULTS.shadow,
  };
  const lines = [];
  for (const [key, hex] of Object.entries(resolved)) {
    if (!isHex(hex) || !/^[a-z0-9-]+$/.test(key)) continue;
    lines.push(`--spice-${key}: #${hex};`, `--spice-rgb-${key}: ${rgb(hex).join(",")};`);
  }
  const light = luminance(main) > 0.5;
  lines.push(
    "--text: var(--spice-text);",
    "--text-dim: var(--spice-subtext);",
    "--text-muted: var(--spice-subtext);",
    "--bg: var(--spice-sidebar);",
    "--surface: var(--spice-main);",
    "--main-bg: var(--spice-main);",
    "--sidebar-bg: var(--spice-sidebar);",
    "--player-bg: var(--spice-player);",
    "--surface-raised: var(--spice-card);",
    "--surface-hover: var(--spice-highlight);",
    "--control-bg: var(--spice-highlight);",
    "--control-hover: var(--spice-highlight-elevated);",
    "--highlight-strong: var(--spice-highlight-elevated);",
    "--selected-row-bg: rgba(var(--spice-rgb-selected-row), 0.16);",
    "--track-bg: rgba(var(--spice-rgb-text), 0.3);",
    "--toggle-off: var(--spice-button-disabled);",
    "--border: rgba(var(--spice-rgb-text), 0.08);",
    "--border-strong: rgba(var(--spice-rgb-text), 0.16);",
    "--accent: var(--spice-button);",
    "--accent-hover: var(--spice-button-active);",
    `--accent-text: ${readableOn(button)};`,
    "--accent-bg: rgba(var(--spice-rgb-button), 0.14);",
    "--accent-bg-strong: rgba(var(--spice-rgb-button), 0.24);",
    "--toast-bg: var(--spice-notification);",
    `--toast-text: ${readableOn(resolved.notification)};`,
    "--danger: var(--spice-notification-error);",
    `color-scheme: ${light ? "light" : "dark"};`,
  );
  // html:root outranks the app's own :root colors.
  return `html:root {\n  ${lines.join("\n  ")}\n}\n`;
}

/** @param {string} value */
function isHex(value) {
  return /^[0-9a-f]{6}$/i.test(value);
}

/** @param {string} hex */
function rgb(hex) {
  return [0, 2, 4].map((i) => parseInt(hex.slice(i, i + 2), 16));
}

/**
 * `a` laid over `b` at `amount` (0 to 1), as hex.
 * @param {string} a
 * @param {string} b
 * @param {number} amount
 */
function mix(a, b, amount) {
  const [x, y] = [rgb(a), rgb(b)];
  return x.map((v, i) => Math.round(v * amount + y[i] * (1 - amount)).toString(16).padStart(2, "0")).join("");
}

/**
 * Black or white, whichever reads better on `hex`.
 * @param {string} hex
 */
function readableOn(hex) {
  const l = luminance(hex);
  return (l + 0.05) / 0.05 > 1.05 / (l + 0.05) ? "#000000" : "#ffffff";
}

/** Relative luminance, 0 (black) to 1 (white). @param {string} hex */
function luminance(hex) {
  const [r, g, b] = rgb(hex).map((v) => {
    const c = v / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

// Picks up edits to the chosen theme's files since last time.
if (themes.prefs.id !== BUILT_IN_ID) applyTheme().catch(() => {});
