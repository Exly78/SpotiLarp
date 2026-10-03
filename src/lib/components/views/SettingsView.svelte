<script>
  import { onMount } from "svelte";
  import * as api from "../../api.js";
  import { restartAfterPlayerRebuild } from "../../player.svelte.js";
  import { queue, toggleAutoplay } from "../../queue.svelte.js";
  import { sleep, setSleepTimer } from "../../sleep.svelte.js";
  import { notify } from "../../toast.svelte.js";
  import Toggle from "../Toggle.svelte";

  const CACHE_SIZES = [
    { mb: 0, label: "Off" },
    { mb: 512, label: "500 MB" },
    { mb: 1024, label: "1 GB" },
    { mb: 2048, label: "2 GB" },
    { mb: 5120, label: "5 GB" },
    { mb: 10240, label: "10 GB" },
  ];
  const SLEEP_OPTIONS = [15, 30, 45, 60, 90];
  const SHORTCUTS = [
    ["Space", "Play / pause"],
    ["Ctrl+Right / Ctrl+Left", "Next / previous song"],
    ["Shift+Right / Shift+Left", "Skip forward / back 5 seconds"],
    ["Ctrl+Up / Ctrl+Down", "Volume up / down"],
    ["M", "Mute / unmute"],
    ["Ctrl+S", "Shuffle"],
    ["Ctrl+R", "Repeat"],
    ["Ctrl+F or /", "Search"],
    ["Alt+Left / Alt+Right", "Back / forward"],
    ["Media keys", "Play, pause, next, previous (also from the Windows media overlay)"],
  ];
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
  /** @type {string[]} */
  let devices = $state([]);
  /** @type {number[]} */
  let gains = $state([]);

  const remainingLabel = $derived.by(() => {
    if (sleep.endOfTrack) return "Stops when this song ends";
    if (sleep.endsAt === null) return "Off";
    const minutes = Math.ceil(sleep.remainingMs / 60_000);
    return `Pausing in ${minutes} min`;
  });

  onMount(async () => {
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
  </section>

  <section>
    <h2>Desktop</h2>
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
{/if}

<style>
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
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5em;
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
