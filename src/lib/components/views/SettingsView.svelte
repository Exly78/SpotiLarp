<script>
  import { onMount, tick } from "svelte";
  import * as api from "../../api.js";
  import { restartAfterPlayerRebuild } from "../../player.svelte.js";
  import { queue, toggleAutoplay } from "../../queue.svelte.js";
  import { sleep, setSleepTimer } from "../../sleep.svelte.js";
  import { notify } from "../../toast.svelte.js";
  import { addLocalFolder, removeLocalFolder } from "../../localFiles.svelte.js";
  import Icon from "../Icon.svelte";
  import Toggle from "../Toggle.svelte";
  import { SHORTCUTS } from "../../keys.js";
  import { history, clearHistory } from "../../history.svelte.js";
  import { episodes, clearEpisodeProgress } from "../../podcasts.svelte.js";
  import { zoom, setZoom, ZOOM_LEVELS } from "../../zoom.svelte.js";
  import { themes, loadThemes, selectTheme, applyTheme } from "../../themes.svelte.js";

  const CACHE_SIZES = [
    { mb: 0, label: "Off" },
    { mb: 512, label: "500 MB" },
    { mb: 1024, label: "1 GB" },
    { mb: 2048, label: "2 GB" },
    { mb: 5120, label: "5 GB" },
    { mb: 10240, label: "10 GB" },
  ];
  const SLEEP_OPTIONS = [15, 30, 45, 60, 90];
  const EQ_BANDS = ["32", "64", "125", "250", "500", "1K", "2K", "4K", "8K", "16K"];
  const EQ_MAX_DB = 12;
  /** @type {Record<string, { label: string, gains: number[] }>} */
  const EQ_PRESETS = {
    flat: { label: "Flat", gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
    bass_boost: { label: "Bass booster", gains: [6, 5, 4, 2, 0, 0, 0, 0, 0, 0] },
    bass_reducer: { label: "Bass reducer", gains: [-6, -5, -4, -2, 0, 0, 0, 0, 0, 0] },
    treble_boost: { label: "Treble booster", gains: [0, 0, 0, 0, 0, 1, 2, 4, 5, 6] },
    vocal: { label: "Vocal", gains: [-2, -2, -1, 0, 2, 4, 4, 2, 0, -1] },
    pop: { label: "Pop", gains: [-1, 0, 2, 3, 4, 3, 1, 0, -1, -1] },
    rock: { label: "Rock", gains: [4, 3, 2, 0, -1, -1, 1, 2, 3, 4] },
    electronic: { label: "Electronic", gains: [5, 4, 1, 0, -2, 1, 0, 1, 4, 5] },
    hip_hop: { label: "Hip-hop", gains: [5, 4, 2, 3, -1, -1, 1, -1, 2, 3] },
    acoustic: { label: "Acoustic", gains: [3, 3, 2, 1, 1, 1, 2, 2, 2, 1] },
    classical: { label: "Classical", gains: [4, 3, 2, 1, -1, -1, 0, 2, 3, 4] },
  };

  /** @type {import("../../types.js").Settings|null} */
  let settings = $state(null);
  let autostart = $state(false);
  /** @type {number|null} */
  let cacheBytes = $state(null);
  let saving = $state(false);
  let clearing = $state(false);
  let choosingFolder = $state(false);
  /** @type {string[]} */
  let devices = $state([]);
  /** @type {number[]} */
  let gains = $state([]);
  let query = $state("");
  let nothingMatches = $state(false);
  /** @type {HTMLDivElement|undefined} */
  let sectionsEl = $state();

  // Narrows the page to the rows mentioning what's typed; a section whose
  // title matches keeps all its rows.
  $effect(() => {
    const words = query.trim().toLowerCase();
    const root = sectionsEl;
    if (!root) return;
    tick().then(() => {
      let anyShown = false;
      for (const section of root.querySelectorAll("section")) {
        const title = section.querySelector("h2")?.textContent?.toLowerCase() ?? "";
        const titleMatches = !words || title.includes(words);
        let rowsShown = 0;
        for (const item of section.querySelectorAll(":scope > :not(h2)")) {
          const el = /** @type {HTMLElement} */ (item);
          const shown = titleMatches || (el.textContent ?? "").toLowerCase().includes(words);
          el.style.display = shown ? "" : "none";
          if (shown) rowsShown++;
        }
        const sectionShown = titleMatches || rowsShown > 0;
        /** @type {HTMLElement} */ (section).style.display = sectionShown ? "" : "none";
        anyShown ||= sectionShown;
      }
      nothingMatches = !anyShown;
    });
  });

  const remainingLabel = $derived.by(() => {
    if (sleep.endOfTrack) return "Stops when this song ends";
    if (sleep.endsAt === null) return "Off";
    const minutes = Math.ceil(sleep.remainingMs / 60_000);
    return `Pausing in ${minutes} min`;
  });

  /** @type {{ onChangeClientId?: () => void }} */
  let { onChangeClientId } = $props();

  /** @type {string|null} */
  let clientId = $state(null);

  let importingThemes = $state(false);
  const currentTheme = $derived(themes.list.find((t) => t.id === themes.prefs.id));
  const ownThemes = $derived(themes.list.filter((t) => t.source === "own"));
  const spicetifyThemes = $derived(themes.list.filter((t) => t.source === "spicetify"));

  async function addThemes() {
    const folder = await api.pickFolder().catch(() => null);
    if (!folder) return;
    importingThemes = true;
    try {
      const names = await api.importThemes(folder);
      await loadThemes();
      notify(names.length === 1 ? `Added "${names[0]}"` : `Added ${names.length} themes`);
      if (names.length === 1) await selectTheme(`own:${names[0]}`);
    } catch (e) {
      notify(`Couldn't add themes: ${e}`);
    } finally {
      importingThemes = false;
    }
  }

  async function reloadThemes() {
    await loadThemes();
    await applyTheme();
    notify("Themes reloaded");
  }

  onMount(async () => {
    loadThemes();
    api.getClientId().then((id) => (clientId = id)).catch(() => {});
    settings = await api.getSettings();
    gains = [...settings.eq_gains];
    refreshDevices();
    api.getAutostart().then((enabled) => (autostart = enabled)).catch(() => {});
    refreshCacheSize();
  });

  function refreshDevices() {
    api.listOutputDevices().then((names) => (devices = names)).catch(() => {});
  }

  /** @param {string} preset */
  function choosePreset(preset) {
    const chosen = EQ_PRESETS[preset];
    if (!chosen) return;
    gains = [...chosen.gains];
    update({ eq_preset: preset, eq_gains: gains });
  }

  /**
   * @param {number} band
   * @param {number} value
   */
  function previewBand(band, value) {
    gains[band] = value;
    api.previewEqualizer(true, $state.snapshot(gains)).catch(() => {});
  }

  function saveBands() {
    update({ eq_preset: "custom", eq_gains: $state.snapshot(gains) });
  }

  /** @param {number} gain */
  function formatGain(gain) {
    return gain > 0 ? `+${gain}` : `${gain}`;
  }

  function refreshCacheSize() {
    api.getCacheSize().then((bytes) => (cacheBytes = bytes)).catch(() => {});
  }

  /** @param {Partial<import("../../types.js").Settings>} change */
  async function update(change) {
    if (!settings || saving) return;
    const next = { ...settings, ...change };
    saving = true;
    try {
      const rebuilt = await api.setSettings(next);
      settings = next;
      if (rebuilt) restartAfterPlayerRebuild();
    } catch (e) {
      notify(`Couldn't save settings: ${e}`);
    } finally {
      saving = false;
    }
  }

  async function addFolder() {
    choosingFolder = true;
    try {
      const next = await addLocalFolder();
      if (next) settings = next;
    } catch (e) {
      notify(`Couldn't add the folder: ${e}`);
    } finally {
      choosingFolder = false;
    }
  }

  /** @param {string} folder */
  async function removeFolder(folder) {
    try {
      settings = await removeLocalFolder(folder);
    } catch (e) {
      notify(`Couldn't remove the folder: ${e}`);
    }
  }

  async function toggleAutostart() {
    try {
      await api.setAutostart(!autostart);
      autostart = !autostart;
    } catch (e) {
      notify(`Couldn't change launch on startup: ${e}`);
    }
  }

  async function clearCache() {
    clearing = true;
    try {
      await api.clearCache();
      notify("Song cache cleared");
    } catch (e) {
      notify(`${e}`);
    } finally {
      clearing = false;
      refreshCacheSize();
    }
  }

  /** @param {number} bytes */
  function formatBytes(bytes) {
    if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
    return `${Math.round(bytes / 1024 ** 2)} MB`;
  }
</script>

<h1 class="title">Settings</h1>

{#if !settings}
  <p class="hint">Loading...</p>
{:else}
  <input
    type="search"
    class="settings-search"
    placeholder="Search settings"
    bind:value={query}
    spellcheck="false"
    aria-label="Search settings"
  />
  {#if nothingMatches}
    <p class="hint">No settings match "{query.trim()}".</p>
  {/if}
  <div bind:this={sectionsEl}>
  <section>
    <h2>Appearance</h2>
    <div class="row">
      <div>
        <div class="label">Theme</div>
        <div class="hint">
          Themes work like Spicetify's: a folder with a color.ini of color schemes, and optionally a user.css.
          {#if spicetifyThemes.length > 0}The themes you installed for Spicetify are listed too.{/if}
          Find more at github.com/spicetify/spicetify-themes.
        </div>
      </div>
      <select value={themes.prefs.id} onchange={(e) => selectTheme(e.currentTarget.value)} aria-label="Theme">
        <optgroup label="Built in">
          {#each themes.list.filter((t) => t.source === "builtin") as theme (theme.id)}
            <option value={theme.id}>{theme.name}</option>
          {/each}
        </optgroup>
        {#if ownThemes.length > 0}
          <optgroup label="Your themes">
            {#each ownThemes as theme (theme.id)}
              <option value={theme.id}>{theme.name}</option>
            {/each}
          </optgroup>
        {/if}
        {#if spicetifyThemes.length > 0}
          <optgroup label="From Spicetify">
            {#each spicetifyThemes as theme (theme.id)}
              <option value={theme.id}>{theme.name}</option>
            {/each}
          </optgroup>
        {/if}
        {#if !currentTheme}
          <option value={themes.prefs.id}>{themes.prefs.id.split(":").slice(1).join(":")}</option>
        {/if}
      </select>
    </div>
    {#if currentTheme && currentTheme.schemes.length > 1}
      <div class="row">
        <div>
          <div class="label">Color scheme</div>
          <div class="hint">{currentTheme.schemes.length} in this theme.</div>
        </div>
        <select
          value={themes.prefs.scheme}
          onchange={(e) => selectTheme(themes.prefs.id, { scheme: e.currentTarget.value, css: themes.prefs.css })}
          aria-label="Color scheme"
        >
          {#each currentTheme.schemes as scheme}
            <option value={scheme}>{scheme}</option>
          {/each}
        </select>
      </div>
    {/if}
    {#if currentTheme?.has_css}
      <div class="row">
        <div>
          <div class="label">Use the theme's CSS</div>
          <div class="hint">
            {#if currentTheme.source === "spicetify"}
              Its user.css was written for Spotify's own window: fonts, scrollbars and a few details carry over, the
              rest doesn't apply here. Turn it off if anything looks broken.
            {:else}
              The theme's user.css can restyle more than colors.
            {/if}
            If a theme ever leaves SpotiLarp unusable, run <span class="mono">spotilarp reset-theme</span>.
          </div>
        </div>
        <Toggle
          on={themes.prefs.css}
          onclick={() => selectTheme(themes.prefs.id, { scheme: themes.prefs.scheme, css: !themes.prefs.css })}
          label="Use the theme's CSS"
        />
      </div>
    {/if}
    <div class="row">
      <div>
        <div class="label">Your themes</div>
        <div class="hint">
          Add a theme folder, or a folder of them (a download of spicetify-themes, say), or put them in the themes folder
          yourself. Reload after editing one.
        </div>
      </div>
      <div class="buttons">
        <button type="button" class="secondary" onclick={addThemes} disabled={importingThemes}>
          {importingThemes ? "Adding..." : "Add themes..."}
        </button>
        <button type="button" class="secondary" onclick={() => api.openThemesFolder().catch((e) => notify(`Couldn't open the folder: ${e}`))}>
          Open folder
        </button>
        <button type="button" class="secondary" onclick={reloadThemes} disabled={themes.loading}>Reload</button>
      </div>
    </div>
    {#if themes.error}
      <p class="hint error-text">{themes.error}</p>
    {/if}
  </section>

  <section>
    <h2>Spotify app</h2>
    <div class="row">
      <div>
        <div class="label">Client ID</div>
        <div class="hint">
          The app you made on Spotify's developer dashboard, which SpotiLarp uses to read your library and search.
          {#if clientId}Now using <span class="mono">{clientId.slice(0, 4)}…{clientId.slice(-4)}</span>.{/if}
          Spotify limits how often each app can ask it things, so if it holds this one back for hours, make another
          app and switch to its Client ID here. You'll log in again with the new app; playback stays connected.
        </div>
      </div>
      {#if onChangeClientId}
        <button type="button" class="secondary" onclick={onChangeClientId}>Change...</button>
      {/if}
    </div>
  </section>

  <section>
    <h2>Playback</h2>
    <div class="row">
      <div>
        <div class="label">Audio quality</div>
        <div class="hint">Higher quality uses more data.</div>
      </div>
      <select
        value={settings.audio_quality}
        onchange={(e) => update({ audio_quality: /** @type {any} */ (e.currentTarget.value) })}
        disabled={saving}
      >
        <option value="low">Low (96 kbps)</option>
        <option value="normal">Normal (160 kbps)</option>
        <option value="high">Very high (320 kbps)</option>
      </select>
    </div>
    <div class="row">
      <div>
        <div class="label">Normalize volume</div>
        <div class="hint">Plays every song at about the same loudness.</div>
      </div>
      <Toggle
        on={settings.normalize_volume}
        onclick={() => update({ normalize_volume: !settings?.normalize_volume })}
        label="Normalize volume"
        disabled={saving}
      />
    </div>
    <div class="row">
      <div>
        <div class="label">Autoplay</div>
        <div class="hint">When your music ends (and repeat is off), keep playing similar songs.</div>
      </div>
      <Toggle on={queue.autoplay} onclick={toggleAutoplay} label="Autoplay" />
    </div>
  </section>

  <section>
    <h2>Local files</h2>
    <div class="row">
      <div>
        <div class="label">Music folders</div>
        <div class="hint">
          Songs in these folders show up in Local Files, with the title, artist, album and cover art read from each file.
          MP3, FLAC, M4A, OGG and WAV.
        </div>
      </div>
      <button type="button" class="secondary" onclick={addFolder} disabled={saving || choosingFolder}>
        Add folder
      </button>
    </div>
    <div class="row">
      <div>
        <div class="label">Show local covers on Discord</div>
        <div class="hint">
          Discord can only show images from the web, so a local song's cover is uploaded to Litterbox
          (catbox.moe's temporary host, which deletes it after 3 days) while it's in your status.
        </div>
      </div>
      <Toggle
        on={settings.discord_local_covers}
        onclick={() => update({ discord_local_covers: !settings?.discord_local_covers })}
        label="Show local covers on Discord"
        disabled={saving}
      />
    </div>
    {#each settings.local_folders as folder (folder)}
      <div class="folder">
        <Icon name="folder" size={16} />
        <span class="folder-path" title={folder}>{folder}</span>
        <button
          type="button"
          class="remove-folder"
          onclick={() => removeFolder(folder)}
          aria-label={`Remove ${folder}`}
          title="Remove"
        >
          <Icon name="close" size={14} />
        </button>
      </div>
    {/each}
  </section>

  <section>
    <h2>Sound</h2>
    <div class="row">
      <div>
        <div class="label">Output device</div>
        <div class="hint">Where SpotiLarp plays, without changing the Windows default.</div>
      </div>
      <div class="controls">
        <select
          value={settings.output_device ?? ""}
          onchange={(e) => update({ output_device: e.currentTarget.value || null })}
          onfocus={refreshDevices}
          disabled={saving}
        >
          <option value="">System default</option>
          {#each devices as device}
            <option value={device}>{device}</option>
          {/each}
          {#if settings.output_device && !devices.includes(settings.output_device)}
            <option value={settings.output_device}>{settings.output_device} (not connected)</option>
          {/if}
        </select>
      </div>
    </div>
    <div class="row">
      <div>
        <div class="label">Equalizer</div>
        <div class="hint">Shape the sound with a preset, or drag the bands.</div>
      </div>
      <div class="controls">
        {#if settings.eq_enabled}
          <select value={settings.eq_preset} onchange={(e) => choosePreset(e.currentTarget.value)} disabled={saving}>
            {#each Object.entries(EQ_PRESETS) as [key, preset]}
              <option value={key}>{preset.label}</option>
            {/each}
            {#if settings.eq_preset === "custom"}
              <option value="custom">Custom</option>
            {/if}
          </select>
        {/if}
        <Toggle
          on={settings.eq_enabled}
          onclick={() => update({ eq_enabled: !settings?.eq_enabled })}
          label="Equalizer"
          disabled={saving}
        />
      </div>
    </div>
    {#if settings.eq_enabled}
      <div class="eq">
        {#each EQ_BANDS as band, i}
          <div class="band">
            <span class="gain">{formatGain(gains[i] ?? 0)}</span>
            <input
              type="range"
              min={-EQ_MAX_DB}
              max={EQ_MAX_DB}
              step="1"
              value={gains[i] ?? 0}
              oninput={(e) => previewBand(i, Number(e.currentTarget.value))}
              onchange={saveBands}
              aria-label={`${band} Hz`}
            />
            <span class="freq">{band}</span>
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <section>
    <h2>Storage</h2>
    <div class="row">
      <div>
        <div class="label">Song cache</div>
        <div class="hint">
          Keeps played songs on disk so replays don't download again. Changes apply after restarting the app.
          {#if cacheBytes !== null}Using {formatBytes(cacheBytes)}.{/if}
        </div>
      </div>
      <div class="controls">
        <select
          value={settings.cache_limit_mb}
          onchange={(e) => update({ cache_limit_mb: Number(e.currentTarget.value) })}
          disabled={saving}
        >
          {#each CACHE_SIZES as size}
            <option value={size.mb}>{size.label}</option>
          {/each}
        </select>
        <button type="button" class="secondary" onclick={clearCache} disabled={clearing || !cacheBytes}>
          {clearing ? "Clearing..." : "Clear"}
        </button>
      </div>
    </div>
    <div class="row">
      <div>
        <div class="label">Listening history</div>
        <div class="hint">
          Songs played in SpotiLarp, shown under Queue → Recent next to Spotify's own history (which doesn't include
          them). Kept on this computer only. {history.entries.length}
          {history.entries.length === 1 ? "play" : "plays"} saved.
        </div>
      </div>
      <button type="button" class="secondary" onclick={clearHistory} disabled={history.entries.length === 0}>Clear</button>
    </div>
    <div class="row">
      <div>
        <div class="label">Podcast progress</div>
        <div class="hint">
          Where you stopped in each episode and which ones you finished. Spotify doesn't hear about listening in SpotiLarp,
          so this is kept on this computer only, and progress from your other devices doesn't show here.
        </div>
      </div>
      <button
        type="button"
        class="secondary"
        onclick={clearEpisodeProgress}
        disabled={Object.keys(episodes.byUri).length === 0}
      >
        Clear
      </button>
    </div>
  </section>

  <section>
    <h2>Desktop</h2>
    <div class="row">
      <div>
        <div class="label">Zoom</div>
        <div class="hint">How big everything is drawn. Ctrl+= and Ctrl+- (or Ctrl+scroll) change it anywhere, Ctrl+0 resets it.</div>
      </div>
      <select value={zoom.level} onchange={(e) => setZoom(Number(e.currentTarget.value))} aria-label="Zoom">
        {#each ZOOM_LEVELS as level}
          <option value={level}>{Math.round(level * 100)}%</option>
        {/each}
      </select>
    </div>

    <div class="row">
      <div>
        <div class="label">Close button minimizes to the tray</div>
        <div class="hint">Music keeps playing; click the tray icon to bring the window back.</div>
      </div>
      <Toggle
        on={settings.close_to_tray}
        onclick={() => update({ close_to_tray: !settings?.close_to_tray })}
        label="Close to tray"
        disabled={saving}
      />
    </div>
    <div class="row">
      <div>
        <div class="label">Launch on startup</div>
        <div class="hint">Opens minimized when you sign in to your computer.</div>
      </div>
      <Toggle on={autostart} onclick={toggleAutostart} label="Launch on startup" />
    </div>
    <div class="row">
      <div>
        <div class="label">Song change notifications</div>
        <div class="hint">Only while the SpotiLarp window isn't in front.</div>
      </div>
      <Toggle
        on={settings.notifications}
        onclick={() => update({ notifications: !settings?.notifications })}
        label="Song change notifications"
        disabled={saving}
      />
    </div>
  </section>

  <section>
    <h2>Sleep timer</h2>
    <div class="hint status-line">{remainingLabel}</div>
    <div class="chips">
      {#each SLEEP_OPTIONS as minutes}
        <button type="button" class="chip" onclick={() => setSleepTimer(minutes)}>{minutes} min</button>
      {/each}
      <button type="button" class="chip" class:active={sleep.endOfTrack} onclick={() => setSleepTimer("track")}>
        End of song
      </button>
      {#if sleep.endsAt !== null || sleep.endOfTrack}
        <button type="button" class="chip" onclick={() => setSleepTimer(null)}>Turn off</button>
      {/if}
    </div>
  </section>

  <section>
    <h2>Keyboard shortcuts</h2>
    {#each SHORTCUTS as [keys, action]}
      <div class="shortcut"><kbd>{keys}</kbd><span>{action}</span></div>
    {/each}
  </section>

  <section>
    <h2>Command line</h2>
    <div class="hint status-line">
      Run SpotiLarp's program (spotilarp.exe in its install folder on Windows) with a command to control the copy
      that's already open, from a terminal, a launcher or a hotkey tool:
    </div>
    <div class="shortcut"><kbd>spotilarp play-pause</kbd><span>Also play, pause, next, previous, mute, like</span></div>
    <div class="shortcut"><kbd>spotilarp volume 40</kbd><span>Also volume-up / volume-down [step]</span></div>
    <div class="shortcut"><kbd>spotilarp seek 15</kbd><span>Skip 15 seconds (seek -- -15 goes back); seek-to 90 jumps</span></div>
    <div class="shortcut"><kbd>spotilarp shuffle smart</kbd><span>on, smart, off, or cycle; repeat off / all / one</span></div>
    <div class="shortcut"><kbd>spotilarp &lt;Spotify link&gt;</kbd><span>Plays a song, album, playlist or artist link</span></div>
    <div class="shortcut"><kbd>spotilarp search words</kbd><span>Opens a search</span></div>
  </section>
  </div>
{/if}

<style>
.settings-search {
  display: block;
  width: 100%;
  max-width: 360px;
  margin: -0.4em 0 1.6em;
  padding: 0.55em 1em;
  border: 1px solid transparent;
  border-radius: var(--radius-pill);
  background: var(--control-bg);
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  outline: none;
}
.settings-search:focus {
  border-color: var(--text);
}
.title {
  margin: 0 0 1em;
  font-size: var(--fs-xl);
  font-weight: var(--fw-black);
  letter-spacing: -0.02em;
  color: var(--text);
}
section {
  max-width: 760px;
  margin-bottom: 2em;
}
h2 {
  margin: 0 0 0.6em;
  font-size: var(--fs-md);
  font-weight: var(--fw-bold);
  color: var(--text);
}
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 2em;
  padding: 0.7em 0;
}
.label {
  font-size: var(--fs-sm);
  color: var(--text);
}
.hint {
  margin-top: 0.2em;
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
.status-line {
  margin-bottom: 0.8em;
}
.controls {
  display: flex;
  align-items: center;
  gap: 0.6em;
  flex-shrink: 0;
}
select {
  flex-shrink: 0;
  padding: 0.5em 0.9em;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: var(--control-bg);
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  cursor: pointer;
}
.secondary,
.chip {
  padding: 0.45em 1em;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-pill);
  background: none;
  color: var(--text);
  font-family: inherit;
  font-size: var(--fs-sm);
  cursor: pointer;
  transition: border-color var(--transition);
}
.secondary:hover:not(:disabled),
.chip:hover {
  border-color: var(--text);
}
.secondary:disabled {
  opacity: 0.5;
  cursor: default;
}
.eq {
  display: flex;
  justify-content: space-between;
  gap: 0.5em;
  max-width: 560px;
  margin: 0.5em 0 0.5em auto;
  padding: 1em 1.25em;
  border-radius: var(--radius-md);
  background: var(--surface-raised);
}
.band {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.4em;
}
.band input {
  writing-mode: vertical-lr;
  direction: rtl;
  width: 18px;
  height: 120px;
  accent-color: var(--accent);
  cursor: pointer;
}
.gain,
.freq {
  font-size: var(--fs-xs);
  font-variant-numeric: tabular-nums;
  color: var(--text-muted);
}
.gain {
  min-width: 2.2em;
  text-align: center;
  color: var(--text);
}
.folder {
  display: flex;
  align-items: center;
  gap: 0.6em;
  margin-bottom: 0.4em;
  padding: 0.55em 0.6em 0.55em 0.9em;
  border-radius: var(--radius-sm);
  background: var(--surface-raised);
  font-size: var(--fs-sm);
  color: var(--text-dim);
}
.folder-path {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text);
}
.remove-folder {
  display: flex;
  padding: 0.35em;
  border: none;
  border-radius: 50%;
  background: none;
  color: var(--text-muted);
  cursor: pointer;
  transition: color var(--transition);
}
.remove-folder:hover {
  color: var(--text);
}
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5em;
}
.buttons {
  display: flex;
  flex-shrink: 0;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.5em;
}
.mono {
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
  color: var(--text);
}
.error-text {
  color: var(--danger);
}
.chip.active {
  border-color: var(--accent);
  color: var(--accent);
}
.shortcut {
  display: flex;
  align-items: center;
  gap: 1em;
  padding: 0.35em 0;
  font-size: var(--fs-sm);
  color: var(--text-dim);
}
kbd {
  min-width: 11em;
  font-family: inherit;
  font-size: var(--fs-xs);
  font-weight: var(--fw-bold);
  color: var(--text);
}
</style>
