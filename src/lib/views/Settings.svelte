<script>
  import { onMount } from 'svelte';
  import { appState, updateSettings, notify } from '../stores/app.svelte.js';
  import { 
    Settings, Palette, ShieldAlert, Cpu, HardDrive, 
    Download, Check, RefreshCw 
  } from 'lucide-svelte';

  let currentSettings = $state(null);

  const themeOptions = [
    { id: 'dark', label: 'Dark Default', isAmoled: false },
    { id: 'amoled', label: 'AMOLED Pitch Black', isAmoled: true },
    { id: 'light', label: 'Light', isAmoled: false },
  ];

  const colorThemes = [
    { id: 'Blue', color: '#4285f4' },
    { id: 'Purple', color: '#a855f7' },
    { id: 'Teal', color: '#14b8a6' },
    { id: 'Green', color: '#22c55e' },
    { id: 'Red', color: '#ef4444' },
    { id: 'Orange', color: '#f97316' },
    { id: 'Pink', color: '#ec4899' },
    { id: 'Yellow', color: '#eab308' },
  ];

  onMount(() => {
    if (appState.settings) {
      currentSettings = JSON.parse(JSON.stringify(appState.settings));
    }
  });

  async function handleThemeChange(opt) {
    if (!currentSettings) return;
    currentSettings.is_amoled = opt.isAmoled;
    currentSettings.app_theme = opt.id === 'light' ? 'Light' : 'Dark';
    await save();
  }

  async function handleColorChange(colorId) {
    if (!currentSettings) return;
    currentSettings.color_theme = colorId;
    await save();
  }

  async function handleNsfwToggle() {
    if (!currentSettings) return;
    currentSettings.nsfw_enabled = !currentSettings.nsfw_enabled;
    await save();
  }

  async function save() {
    await updateSettings(currentSettings);
    notify('Settings saved successfully');
  }
</script>

<div class="settings-view">
  <header class="header">
    <h1 class="page-title">
      <Settings size={26} class="title-icon" />
      Settings
    </h1>
  </header>

  {#if currentSettings}
    <div class="settings-sections">
      <!-- APPEARANCE & THEMES -->
      <section class="section-card">
        <div class="section-header">
          <Palette size={20} class="section-icon" />
          <h2 class="section-title">Appearance & Color Themes</h2>
        </div>

        <div class="setting-item">
          <div class="item-info">
            <span class="item-label">Background Theme</span>
            <span class="item-desc">Select AMOLED for deep OLED pure black or modern dark</span>
          </div>
          <div class="theme-picker">
            {#each themeOptions as opt}
              <button
                class="theme-btn"
                class:active={(opt.isAmoled && currentSettings.is_amoled) || (!currentSettings.is_amoled && currentSettings.app_theme?.toLowerCase() === opt.id)}
                on:click={() => handleThemeChange(opt)}
              >
                {opt.label}
              </button>
            {/each}
          </div>
        </div>

        <div class="setting-item">
          <div class="item-info">
            <span class="item-label">Accent Palette</span>
            <span class="item-desc">Highlights, cards bloom, active pills, and progress bars</span>
          </div>
          <div class="colors-row">
            {#each colorThemes as c}
              <button
                class="color-dot"
                class:active={currentSettings.color_theme === c.id}
                style:background={c.color}
                on:click={() => handleColorChange(c.id)}
                title={c.id}
              >
                {#if currentSettings.color_theme === c.id}
                  <Check size={14} color="#ffffff" strokeWidth={3} />
                {/if}
              </button>
            {/each}
          </div>
        </div>
      </section>

      <!-- CONTENT & SOURCES -->
      <section class="section-card">
        <div class="section-header">
          <ShieldAlert size={20} class="section-icon" />
          <h2 class="section-title">Sources & Content Filters</h2>
        </div>

        <div class="setting-item">
          <div class="item-info">
            <span class="item-label">Enable 18+ / Adult Content</span>
            <span class="item-desc">Show adult sources and titles in catalog search</span>
          </div>
          <button
            class="toggle-switch"
            class:on={currentSettings.nsfw_enabled}
            on:click={handleNsfwToggle}
          >
            <div class="switch-handle" />
          </button>
        </div>

        <div class="setting-item">
          <div class="item-info">
            <span class="item-label">Preferred Catalog Language</span>
            <span class="item-desc">Default language filter when browsing sources</span>
          </div>
          <select
            class="lang-select"
            bind:value={currentSettings.preferred_source_language}
            on:change={save}
          >
            <option value="ALL">ALL (All Languages)</option>
            <option value="en">English (en)</option>
            <option value="ar">Arabic (ar)</option>
            <option value="es">Spanish (es)</option>
            <option value="fr">French (fr)</option>
            <option value="id">Indonesian (id)</option>
            <option value="ru">Russian (ru)</option>
          </select>
        </div>
      </section>

      <!-- PERFORMANCE & CACHE -->
      <section class="section-card">
        <div class="section-header">
          <Cpu size={20} class="section-icon" />
          <h2 class="section-title">Performance & Storage</h2>
        </div>

        <div class="setting-item">
          <div class="item-info">
            <span class="item-label">RAM Cache Budget</span>
            <span class="item-desc">{currentSettings.ram_cache_budget_mb} MB reserved in memory</span>
          </div>
          <input
            type="range"
            min="64"
            max="1024"
            step="64"
            bind:value={currentSettings.ram_cache_budget_mb}
            on:change={save}
            class="slider"
          />
        </div>

        <div class="setting-item">
          <div class="item-info">
            <span class="item-label">Max Concurrent Downloads</span>
            <span class="item-desc">{currentSettings.max_concurrent_downloads} parallel downloads</span>
          </div>
          <input
            type="range"
            min="1"
            max="8"
            step="1"
            bind:value={currentSettings.max_concurrent_downloads}
            on:change={save}
            class="slider"
          />
        </div>
      </section>
    </div>
  {/if}
</div>

<style>
  .settings-view {
    flex: 1;
    height: 100vh;
    overflow-y: auto;
    padding: var(--z-space-6) var(--z-space-8);
    background: var(--z-bg);
  }

  .header {
    margin-bottom: var(--z-space-6);
  }

  .page-title {
    font-size: var(--z-text-2xl);
    font-weight: 700;
    color: var(--z-text);
    display: flex;
    align-items: center;
    gap: var(--z-space-3);
  }

  :global(.title-icon) {
    color: var(--z-accent);
  }

  .settings-sections {
    display: flex;
    flex-direction: column;
    gap: var(--z-space-5);
    max-width: 800px;
  }

  .section-card {
    background: var(--z-surface);
    border: 1px solid var(--z-border);
    border-radius: var(--z-radius-lg);
    padding: var(--z-space-5);
    display: flex;
    flex-direction: column;
    gap: var(--z-space-4);
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: var(--z-space-3);
    border-bottom: 1px solid var(--z-border);
    padding-bottom: var(--z-space-3);
  }

  :global(.section-icon) {
    color: var(--z-accent);
  }

  .section-title {
    font-size: var(--z-text-base);
    font-weight: 600;
  }

  .setting-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--z-space-4);
    flex-wrap: wrap;
  }

  .item-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .item-label {
    font-size: var(--z-text-sm);
    font-weight: 500;
    color: var(--z-text);
  }

  .item-desc {
    font-size: var(--z-text-xs);
    color: var(--z-text-muted);
  }

  .theme-picker {
    display: flex;
    background: var(--z-surface-raised);
    padding: 3px;
    border-radius: var(--z-radius-md);
  }

  .theme-btn {
    padding: 6px 14px;
    border-radius: var(--z-radius-sm);
    font-size: var(--z-text-xs);
    font-weight: 500;
    color: var(--z-text-muted);
    transition: all var(--z-duration-fast) var(--z-ease-out);
  }

  .theme-btn.active {
    background: var(--z-accent);
    color: #ffffff;
    font-weight: 600;
  }

  .colors-row {
    display: flex;
    align-items: center;
    gap: var(--z-space-2);
  }

  .color-dot {
    width: 28px;
    height: 28px;
    border-radius: var(--z-radius-full);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform var(--z-duration-fast) var(--z-ease-out);
  }

  .color-dot:hover {
    transform: scale(1.15);
  }

  .color-dot.active {
    box-shadow: 0 0 0 2px var(--z-surface), 0 0 0 4px var(--z-accent);
  }

  .toggle-switch {
    width: 48px;
    height: 26px;
    background: var(--z-surface-raised);
    border: 1px solid var(--z-border);
    border-radius: var(--z-radius-full);
    position: relative;
    cursor: pointer;
    transition: background var(--z-duration-fast) var(--z-ease-out);
  }

  .toggle-switch.on {
    background: var(--z-accent);
    border-color: var(--z-accent);
  }

  .switch-handle {
    width: 20px;
    height: 20px;
    background: #ffffff;
    border-radius: var(--z-radius-full);
    position: absolute;
    top: 2px;
    left: 2px;
    transition: transform var(--z-duration-fast) var(--z-ease-out);
  }

  .toggle-switch.on .switch-handle {
    transform: translateX(22px);
  }

  .lang-select {
    background: var(--z-surface-raised);
    border: 1px solid var(--z-border);
    color: var(--z-text);
    padding: 6px 12px;
    border-radius: var(--z-radius-md);
    font-size: var(--z-text-sm);
  }

  .slider {
    accent-color: var(--z-accent);
  }
</style>
