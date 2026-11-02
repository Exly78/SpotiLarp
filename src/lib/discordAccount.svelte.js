import * as api from "./api.js";
import { notify } from "./toast.svelte.js";

/**
 * The connected Discord account (discord_account.rs), for features that need
 * to know who you are on Discord.
 * @typedef {{ id: string, username: string, name: string, avatar_url: string }} DiscordProfile
 * @type {{ profile: DiscordProfile|null, connecting: boolean }}
 */
export const discordAccount = $state({ profile: null, connecting: false });

/** Shows the account as last seen, then asks Discord whether its name or picture changed. */
async function load() {
  try {
    discordAccount.profile = await api.discordAccount();
    if (discordAccount.profile) discordAccount.profile = await api.refreshDiscordAccount();
  } catch {
    // Offline, say: the last-seen account stays.
  }
}

load();

export async function connectDiscord() {
  if (discordAccount.connecting) return;
  discordAccount.connecting = true;
  try {
    const profile = await api.connectDiscord();
    discordAccount.profile = profile;
    notify(`Connected Discord as ${profile.name}`);
  } catch (e) {
    if (e !== api.LOGIN_CANCELLED) notify(`Couldn't connect Discord: ${e}`);
  } finally {
    discordAccount.connecting = false;
  }
}

export function cancelConnectDiscord() {
  api.cancelLogin().catch(() => {});
}

export async function disconnectDiscord() {
  try {
    await api.disconnectDiscord();
    discordAccount.profile = null;
    notify("Disconnected Discord");
  } catch (e) {
    notify(`Couldn't disconnect Discord: ${e}`);
  }
}
