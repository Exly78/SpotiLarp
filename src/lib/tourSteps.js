/**
 * What the tours show (FeatureTour.svelte). Each step points at a control
 * (`target`, a CSS selector); one whose control isn't on screen (a narrow
 * window hides some) shows in the middle. A step can carry lists instead of
 * (or under) its text: a release's changes.
 * @typedef {{ heading: string, items: string[] }} TourList
 * @typedef {{ target: string|null, title: string, text: string, list?: TourList[] }} TourStep
 * @typedef {{ new?: string[], improved?: string[], fixed?: string[] }} Changes
 * @typedef {{ version: string, steps: TourStep[], changes?: Changes }} Release
 */

/** The first-run tour: things that are easy to miss. @type {TourStep[]} */
export const TOUR_STEPS = [
  {
    target: 'button[aria-label="Show now playing panel"], button[aria-label="Hide now playing panel"]',
    title: "The Now Playing panel",
    text: "The artist, live lyrics, credits, upcoming concerts and the song's looping video, for whatever's playing.",
  },
  {
    target: 'button[title^="Lyrics ("]',
    title: "Lyrics",
    text: "Press L to follow along and F for full screen. Lyrics in another script can be shown in Latin letters.",
  },
  {
    target: 'button[title$="(click to change)"]',
    title: "Smart shuffle",
    text: "Click shuffle twice. It keeps one artist's songs apart and saves what you played lately for the end.",
  },
  {
    target: 'button[aria-label="Show Jam"], button[aria-label="Hide Jam"]',
    title: "Listen together",
    text: "Start a Jam and send friends the code, and everyone hears the same song at the same moment. Each person needs SpotiLarp and Spotify Premium.",
  },
  {
    target: 'button[aria-label="Open mini player"]',
    title: "Mini player",
    text: "Shrink SpotiLarp to a small player while you do something else.",
  },
  {
    target: null,
    title: "Keyboard shortcuts",
    text: "Press ? any time to see them all. Right-click a song or playlist for more you can do with it.",
  },
];

/**
 * What's new in each update, newest first. Updating from an older version
 * shows the steps of every release since, once (tour.svelte.js), so a new
 * release only needs its entry here, under the version in package.json.
 * `steps` point out the big things; `changes` lists everything in the
 * release, small things too, on a card of its own after them.
 * @type {Release[]}
 */
export const RELEASES = [
  {
    version: "1.3.0",
    steps: [
      {
        target: 'button[aria-label="Connect to a device"]',
        title: "Play on your other devices",
        text: "Pick your phone, a speaker or the Spotify app here and the music plays there, with SpotiLarp still in charge of the queue. SpotiLarp shows up in your other Spotify apps' device lists too, so they can play on it.",
      },
      {
        target: 'button[aria-label="Settings"]',
        title: "Listen in the Spotify app",
        text: "Settings → Spotify desktop app → Play through the Spotify app sends SpotiLarp's music, a Jam or listening along to Spotify's own app. With Spicetify, that works without Premium too.",
      },
      {
        target: "button.playlist-button:has(.stats-thumb)",
        title: "Your stats",
        text: "How much you listen and when, your top songs, artists and albums, and Spotify's own ranking of you next to them.",
      },
      {
        target: 'button[aria-label="Settings"]',
        title: "Last.fm",
        text: "Connect Last.fm in Settings and the songs you play in SpotiLarp go to your Last.fm profile.",
      },
    ],
    changes: {
      new: [
        "SpotiLarp keeps your listening history, plays in Spotify's other apps included",
        "Import Spotify's data download to bring in years of past listening",
        "Stats for 7 days to all time: time listened, skips, streaks and songs new to you",
        "Charts of your listening over time, and the days and hours you listen most",
        "Last.fm shows what you're playing, and scrobbles made offline are sent later",
        "A new Home: what's playing, your last 7 days, On repeat, Your mixes and more",
        "Play your own music files without setting up Spotify",
        "The library sidebar can shrink to a strip of covers",
        "Play buttons on cards, so you can play without opening them",
        "A page's title and play button stay at the top as you scroll",
        "A clear button in the search box",
      ],
      improved: [
        "Bigger page headers, in the cover's color from edge to edge",
        "Updates show as a small \"Update available\" button in the top bar",
        "A friendlier setup screen that checks your Client ID as you type",
        "Now Playing and the queue say what you're playing from, with a link back",
        "Smart shuffle also knows what you played lately in Spotify's other apps",
        "Clearing your listening history asks first",
        "A fresh look: bigger Home button and profile picture, restyled menus, the real Discord logo",
      ],
      fixed: [
        "Canvas videos no longer crash the app on Linux, and Linux packages include what they need",
        "Logins stay saved on Linux without a keyring, and logging out works there",
        "The Now Playing artist bio is cut to three lines again on Linux and macOS",
        "Lyrics no longer show a sideways scrollbar",
      ],
    },
  },
];

/**
 * "1.10.2" as numbers, for comparing versions. Anything after a `-` (a
 * pre-release) is left out.
 * @param {string} version
 */
function parts(version) {
  return version
    .split("-")[0]
    .split(".")
    .map((part) => Number(part) || 0);
}

/**
 * Below zero when `a` is older than `b`, above when newer.
 * @param {string} a
 * @param {string} b
 */
export function compareVersions(a, b) {
  const [x, y] = [parts(a), parts(b)];
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    const difference = (x[i] ?? 0) - (y[i] ?? 0);
    if (difference !== 0) return difference;
  }
  return 0;
}

const CHANGE_GROUPS = /** @type {const} */ ([
  ["new", "New"],
  ["improved", "Improved"],
  ["fixed", "Fixed"],
]);

/**
 * A release's steps, then a card with everything else it changed.
 * @param {Release} release
 * @returns {TourStep[]}
 */
export function releaseSteps(release) {
  const list = CHANGE_GROUPS.map(([key, heading]) => ({ heading, items: release.changes?.[key] ?? [] })).filter(
    (group) => group.items.length > 0,
  );
  if (list.length === 0) return release.steps;
  const title = release.steps.length > 0 ? `Everything else in ${release.version}` : `What's new in ${release.version}`;
  return [...release.steps, { target: null, title, text: "", list }];
}

/**
 * The steps of every release after `seen`, up to and including `current`, oldest first.
 * @param {string} seen
 * @param {string} current
 * @returns {TourStep[]}
 */
export function stepsSince(seen, current) {
  return RELEASES.filter(({ version }) => compareVersions(version, seen) > 0 && compareVersions(version, current) <= 0)
    .sort((a, b) => compareVersions(a.version, b.version))
    .flatMap(releaseSteps);
}

/**
 * The newest release up to `current` that has steps, for showing again from Settings.
 * @param {string} current
 */
export function latestRelease(current) {
  return RELEASES.filter(({ version }) => compareVersions(version, current) <= 0).sort((a, b) =>
    compareVersions(b.version, a.version),
  )[0];
}
