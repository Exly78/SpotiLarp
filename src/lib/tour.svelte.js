import { getVersion } from "@tauri-apps/api/app";
import { TOUR_STEPS, stepsSince, latestRelease, releaseSteps, compareVersions } from "./tourSteps.js";

/**
 * The tours (FeatureTour.svelte): the feature tour, shown once when the app
 * first opens for real and again from Settings, and what's new, shown once
 * after an update (its steps are in tourSteps.js).
 */

const SEEN_KEY = "spotilarp.tourSeen";
/** The version whose news was last seen. */
const NEWS_SEEN_KEY = "spotilarp.whatsNewSeen";

/**
 * `label` names a what's-new tour ("New in 1.3.0"); `version` is the app's.
 * @type {{ open: boolean, steps: import("./tourSteps.js").TourStep[], label: string, version: string }}
 */
export const tour = $state({ open: false, steps: TOUR_STEPS, label: "", version: "" });

getVersion()
  .then((version) => (tour.version = version))
  .catch(() => {});

export function startTour() {
  tour.steps = TOUR_STEPS;
  tour.label = "";
  tour.open = true;
}

/** @param {import("./tourSteps.js").TourStep[]} steps */
export function startWhatsNew(steps, version = tour.version) {
  tour.steps = steps;
  tour.label = `New in ${version}`;
  tour.open = true;
}

/** The newest release's news again (Settings). */
export function showLatestNews() {
  const release = latestRelease(tour.version);
  if (release) startWhatsNew(releaseSteps(release), release.version);
}

/** Whether there's a release's news to show again. */
export function hasNews() {
  return !!tour.version && !!latestRelease(tour.version);
}

function markNewsSeen() {
  try {
    if (tour.version) localStorage.setItem(NEWS_SEEN_KEY, tour.version);
  } catch {}
}

export function finishTour() {
  tour.open = false;
  try {
    localStorage.setItem(SEEN_KEY, "true");
  } catch {}
  // A first tour covers this version too: nothing in it is news to someone new.
  markNewsSeen();
}

/** Without storage it counts as seen, so it can't come back every launch. */
export function tourSeen() {
  try {
    return localStorage.getItem(SEEN_KEY) === "true";
  } catch {
    return true;
  }
}

/**
 * What's new since the version whose news was last seen, oldest first.
 * Nothing new just notes the version, so the next update's news is counted
 * from here.
 * @returns {import("./tourSteps.js").TourStep[]}
 */
export function pendingWhatsNew() {
  if (!tour.version) return [];
  let seen;
  try {
    seen = localStorage.getItem(NEWS_SEEN_KEY);
  } catch {
    return [];
  }
  // Updated from before there was a what's new: this version's news is theirs.
  const latest = latestRelease(tour.version);
  const steps = seen ? stepsSince(seen, tour.version) : latest?.version === tour.version ? releaseSteps(latest) : [];
  if (steps.length === 0 && (!seen || compareVersions(seen, tour.version) < 0)) markNewsSeen();
  return steps;
}
