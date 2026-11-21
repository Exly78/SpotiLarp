/**
 * What's open over the app for sharing stats: a year's Recap
 * (components/Recap.svelte) and the share dialog (components/ShareStatsDialog.svelte),
 * which can open over the Recap. Both are drawn by +page.svelte, above everything else.
 *
 * @typedef {import("./shareCard.js").ShareCard} ShareCard
 */

/** @type {{ recapYear: number|null, card: ShareCard|null }} */
export const sharing = $state({ recapYear: null, card: null });

/** @param {number} year */
export function openRecap(year) {
  sharing.recapYear = year;
}

export function closeRecap() {
  sharing.recapYear = null;
}

/** @param {ShareCard} card */
export function openShareCard(card) {
  sharing.card = card;
}

export function closeShareCard() {
  sharing.card = null;
}
