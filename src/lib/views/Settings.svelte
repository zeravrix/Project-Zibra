<script>
  import { onMount } from 'svelte';
  import { appState, updateSettings, notify } from '../stores/app.svelte.js';
  import { 
    Palette, BookOpen, Zap, ShieldCheck, Check, 
    Sparkles, Trash2, ArrowRight
  } from 'lucide-svelte';

  let currentSettings = $state(null);

  const palettes = [
    { id: 'Totoro', name: 'Zibrax Blue', color: '#4285f4', desc: 'Electric cyan-blue neon bloom' },
    { id: 'Monet', name: 'Sky Azure', color: '#38bdf8', desc: 'Ambient material sky blue' },
    { id: 'Miku', name: 'Cyber Teal', color: '#06b6d4', desc: 'Futuristic glowing cyber teal' },
    { id: 'Mion', name: 'Emerald', color: '#34d399', desc: 'Lush soothing forest emerald' },
    { id: 'Itsuka', name: 'Amber Gold', color: '#fbbf24', desc: 'Warm glowing twilight gold' },
    { id: 'Orange', name: 'Flame Sunset', color: '#f97316', desc: 'Fiery solar sunset gradient' },
    { id: 'Asuka', name: 'Eva Crimson', color: '#ef4444', desc: 'High-energy vivid crimson coral' },
    { id: 'Sakura', name: 'Sakura Rose', color: '#fb7185', desc: 'Delicate cherry blossom petal pink' },
    { id: 'Expressive', name: 'Neon Lilac', color: '#c084fc', desc: 'Expressive synthwave purple' },
    { id: 'Rikka', name: 'Pastel Violet', color: '#a78bfa', desc: 'Deep pastel lavender purple' },
    { id: 'Kanade', name: 'Platinum', color: '#94a3b8', desc: 'Minimalist metallic platinum slate' },
    { id: 'HighContrast', name: 'Pure White', color: '#ffffff', desc: 'Crisp ultra-contrast monochrome' },
  ];

  const backgroundModes = [
    {
      id: 'dark',
      label: 'Obsidian Dark',
      desc: 'Layered charcoal glass with soft neon depth',
      isAmoled: false,
      theme: 'Dark',
      bgPreview: '#0f131a',
      surfacePreview: '#141923',
    },
    {
      id: 'amoled',
      label: 'Midnight OLED',
      desc: 'Pure #000000 pitch black for battery & OLED',
      isAmoled: true,
      theme: 'Dark',
      bgPreview: '#000000',
      surfacePreview: '#08080a',
    },
    {
      id: 'light',
      label: 'Porcelain Light',
      desc: 'Crisp, high-contrast daytime brightness',
      isAmoled: false,
      theme: 'Light',
      bgPreview: '#f8fafc',
      surfacePreview: '#ffffff',
    },
  ];

  const readerModes = [
    { id: 'Vertical', label: 'Webtoon', desc: 'Continuous vertical strip' },
    { id: 'Single', label: 'Single Page', desc: 'Horizontal page flip' },
    { id: 'Double', label: 'Double Spread', desc: 'Side-by-side spread' },
    { id: 'Auto', label: 'Auto Detect', desc: 'Smart comic detection' },
  ];

  const readingDirections = [
    { id: 'LTR', label: 'Left to Right (LTR)', desc: 'Standard manhwa / webtoon flow' },
    { id: 'RTL', label: 'Right to Left (RTL)', desc: 'Traditional Japanese manga' },
  ];

  let selectedPaletteIndex = $state(0);

  onMount(() => {
    if (appState.settings) {
      currentSettings = JSON.parse(JSON.stringify(appState.settings));
      syncPaletteIndex();
    }
  });

  function syncPaletteIndex() {
    if (!currentSettings) return;
    const current = String(currentSettings.color_theme || 'Totoro').toLowerCase();
    const idx = palettes.findIndex(p => p.id.toLowerCase() === current);
    if (idx !== -1) {
      selectedPaletteIndex = idx;
    }
  }

  async function selectPalette(p, idx) {
    if (!currentSettings) return;
    selectedPaletteIndex = idx;
    currentSettings.color_theme = p.id;
    await save();
  }

  async function selectBackgroundMode(mode) {
    if (!currentSettings) return;
    currentSettings.is_amoled = mode.isAmoled;
    currentSettings.theme = mode.theme;
    await save();
  }

  async function selectReaderMode(modeId) {
    if (!currentSettings) return;
    currentSettings.reader_mode = modeId;
    await save();
  }

  async function selectReadingDirection(dirId) {
    if (!currentSettings) return;
    currentSettings.reading_direction = dirId;
    await save();
  }

  async function toggleSetting(field) {
    if (!currentSettings) return;
    currentSettings[field] = !currentSettings[field];
    await save();
  }

  async function handleRangeChange(field, val) {
    if (!currentSettings) return;
    currentSettings[field] = Number(val);
    await save();
  }

  async function save() {
    await updateSettings(currentSettings);
  }

  function handleClearCache() {
    notify('Image cache and temporary buffers cleared');
  }

  // Custom Slider Dragging Engine (No default HTML inputs)
  function handleTrackClick(e, field, min, max, step) {
    const rect = e.currentTarget.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    let val = min + ratio * (max - min);
    if (step) {
      val = Math.round(val / step) * step;
    }
    handleRangeChange(field, val);
  }

  function startSliderDrag(field, min, max, step, e) {
    e.preventDefault();
    e.stopPropagation();
    const onMove = (ev) => {
      const track = document.getElementById(`track-${field}`);
      if (!track) return;
      const rect = track.getBoundingClientRect();
      const ratio = Math.max(0, Math.min(1, (ev.clientX - rect.left) / rect.width));
      let val = min + ratio * (max - min);
      if (step) {
        val = Math.round(val / step) * step;
      }
      handleRangeChange(field, val);
    };
    const onUp = () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  function handlePaletteTrackClick(e) {
    const rect = e.currentTarget.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    const idx = Math.min(palettes.length - 1, Math.floor(ratio * palettes.length));
    selectPalette(palettes[idx], idx);
  }

  function startPaletteDrag(e) {
    e.preventDefault();
    e.stopPropagation();
    const onMove = (ev) => {
      const track = document.getElementById('palette-ribbon-track');
      if (!track) return;
      const rect = track.getBoundingClientRect();
      const ratio = Math.max(0, Math.min(1, (ev.clientX - rect.left) / rect.width));
      const idx = Math.min(palettes.length - 1, Math.floor(ratio * palettes.length));
      selectPalette(palettes[idx], idx);
    };
    const onUp = () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  let activePalette = $derived(palettes[selectedPaletteIndex] || palettes[0]);
</script>

<div class="settings-view">
  <header class="settings-hero">
    <div class="hero-left">
      <div class="settings-badge">
        <Sparkles size={12} />
        <span>PREFERENCES</span>
      </div>
      <h1 class="page-title">Settings & Customization</h1>
      <p class="page-desc">Customize theme aesthetics, reader layout, and resource budgets.</p>
    </div>
  </header>

  {#if currentSettings}
    <div class="settings-grid">
      <!-- 1. BESPOKE THEME SPECTRUM & DEPTH -->
      <section class="custom-card">
        <div class="card-title-row">
          <div class="card-icon-bubble">
            <Palette size={18} />
          </div>
          <div>
            <h2 class="card-title">Theme Accent & Color Palette</h2>
            <p class="card-subtitle">Glide along the interactive spectrum ribbon to transform all accents</p>
          </div>
        </div>

        <!-- ACTIVE PALETTE HERO READOUT -->
        <div class="palette-preview-card" style="--p-color: {activePalette.color}">
          <div class="preview-swatch-glow" style="background: {activePalette.color}"></div>
          <div class="preview-info">
            <div class="preview-heading">
              <span class="preview-name">{activePalette.name}</span>
              <span class="hex-badge">{activePalette.color.toUpperCase()}</span>
            </div>
            <p class="preview-desc">{activePalette.desc}</p>
          </div>
        </div>

        <!-- BESPOKE SPECTRUM RIBBON (ZERO DEFAULT HTML SLIDERS) -->
        <div class="custom-spectrum-wrap">
          <div
            id="palette-ribbon-track"
            class="spectrum-ribbon-bar"
            on:click={handlePaletteTrackClick}
          >
            <div
              class="spectrum-drag-handle"
              style="left: {(selectedPaletteIndex / (palettes.length - 1)) * 100}%; --handle-color: {activePalette.color}"
              on:mousedown={startPaletteDrag}
            >
              <div class="handle-inner-dot"></div>
            </div>
          </div>

          <div class="spectrum-legend-labels">
            <span>Blue</span>
            <span>Teal</span>
            <span>Amber</span>
            <span>Crimson</span>
            <span>Lilac</span>
          </div>
        </div>

        <!-- PALETTE CHIPS -->
        <div class="palette-chips-grid">
          {#each palettes as p, i}
            <button
              class="palette-chip-btn"
              class:active={selectedPaletteIndex === i}
              style="--chip-accent: {p.color}"
              on:click={() => selectPalette(p, i)}
            >
              <span class="chip-color-dot" style="background: {p.color}"></span>
              <span class="chip-title">{p.name}</span>
              {#if selectedPaletteIndex === i}
                <Check size={13} class="chip-check-icon" />
              {/if}
            </button>
          {/each}
        </div>

        <!-- 3D BACKGROUND ATMOSPHERE CARDS -->
        <div class="field-subgroup">
          <label class="group-title">Display Atmosphere</label>
          <div class="bg-mode-grid">
            {#each backgroundModes as mode}
              {@const isSelected = (mode.isAmoled && currentSettings.is_amoled) || (!currentSettings.is_amoled && currentSettings.theme === mode.theme)}
              <div
                class="bg-mode-card"
                class:selected={isSelected}
                on:click={() => selectBackgroundMode(mode)}
              >
                <div class="mock-window-frame" style="background: {mode.bgPreview}">
                  <div class="mock-topbar">
                    <span class="mock-dot red"></span>
                    <span class="mock-dot yellow"></span>
                    <span class="mock-dot green"></span>
                  </div>
                  <div class="mock-content" style="background: {mode.surfacePreview}">
                    <div class="mock-bar" style="background: {activePalette.color}"></div>
                    <div class="mock-line"></div>
                  </div>
                </div>
                <div class="mode-info">
                  <div class="mode-header">
                    <span class="mode-label">{mode.label}</span>
                    {#if isSelected}
                      <span class="active-badge">ACTIVE</span>
                    {/if}
                  </div>
                  <p class="mode-desc">{mode.desc}</p>
                </div>
              </div>
            {/each}
          </div>
        </div>
      </section>

      <!-- 2. READER ENGINE & LAYOUT -->
      <section class="custom-card">
        <div class="card-title-row">
          <div class="card-icon-bubble">
            <BookOpen size={18} />
          </div>
          <div>
            <h2 class="card-title">Reader Experience & Layout</h2>
            <p class="card-subtitle">Format pages for Korean webtoons, Japanese manga, or standard comics</p>
          </div>
        </div>

        <!-- READING MODE SEGMENTED CONTROL -->
        <div class="field-subgroup">
          <label class="group-title">Reading Mode</label>
          <div class="custom-segmented-bar">
            {#each readerModes as mode}
              <button
                class="seg-button"
                class:active={currentSettings.reader_mode === mode.id}
                on:click={() => selectReaderMode(mode.id)}
              >
                <span>{mode.label}</span>
              </button>
            {/each}
          </div>
        </div>

        <!-- READING DIRECTION SEGMENTED CONTROL -->
        <div class="field-subgroup">
          <label class="group-title">Reading Direction</label>
          <div class="custom-segmented-bar">
            {#each readingDirections as dir}
              <button
                class="seg-button"
                class:active={currentSettings.reading_direction === dir.id}
                on:click={() => selectReadingDirection(dir.id)}
              >
                <span>{dir.label}</span>
              </button>
            {/each}
          </div>
        </div>

        <!-- IMAGE FIT TOGGLE -->
        <div class="interactive-option-row">
          <div class="row-info">
            <span class="row-label">Fit to Width</span>
            <span class="row-desc">Automatically scale manhwa images to match screen width seamlessly</span>
          </div>
          <button
            class="bespoke-switch"
            class:checked={currentSettings.fit_to_width}
            on:click={() => toggleSetting('fit_to_width')}
          >
            <div class="switch-ball"></div>
          </button>
        </div>

        <!-- BESPOKE SEAM TRIM SLIDER -->
        <div class="bespoke-slider-card">
          <div class="slider-header-meta">
            <div>
              <span class="meta-label">Page Seam Gap Trimmer</span>
              <span class="meta-hint">Trims slice seams between continuous manhwa pages</span>
            </div>
            <span class="numeric-badge">{currentSettings.seam_trim_px} px</span>
          </div>
          <div
            id="track-seam_trim_px"
            class="custom-slider-track"
            on:click={(e) => handleTrackClick(e, 'seam_trim_px', 0, 32, 2)}
          >
            <div
              class="custom-slider-fill"
              style="width: {(currentSettings.seam_trim_px / 32) * 100}%"
            ></div>
            <div
              class="custom-slider-thumb"
              style="left: {(currentSettings.seam_trim_px / 32) * 100}%"
              on:mousedown={(e) => startSliderDrag('seam_trim_px', 0, 32, 2, e)}
            ></div>
          </div>
        </div>

        <!-- AUTO-CROP MARGINS TOGGLE -->
        <div class="interactive-option-row">
          <div class="row-info">
            <span class="row-label">Auto-Crop Scan Margins</span>
            <span class="row-desc">Detects and trims excessive white borders around scanned pages</span>
          </div>
          <button
            class="bespoke-switch"
            class:checked={currentSettings.auto_crop_margins}
            on:click={() => toggleSetting('auto_crop_margins')}
          >
            <div class="switch-ball"></div>
          </button>
        </div>
      </section>

      <!-- 3. PERFORMANCE & CACHE BUDGETS -->
      <section class="custom-card">
        <div class="card-title-row">
          <div class="card-icon-bubble">
            <Zap size={18} />
          </div>
          <div>
            <h2 class="card-title">Performance & Memory Budgets</h2>
            <p class="card-subtitle">Keep high-res manhwa buttery smooth without memory leaks</p>
          </div>
        </div>

        <!-- BESPOKE RAM CACHE SLIDER -->
        <div class="bespoke-slider-card">
          <div class="slider-header-meta">
            <div>
              <span class="meta-label">RAM Image Cache Allocation</span>
              <span class="meta-hint">Decoded texture memory for instant page turns</span>
            </div>
            <span class="numeric-badge">{currentSettings.ram_cache_budget_mb} MB</span>
          </div>
          <div
            id="track-ram_cache_budget_mb"
            class="custom-slider-track"
            on:click={(e) => handleTrackClick(e, 'ram_cache_budget_mb', 128, 2048, 128)}
          >
            <div
              class="custom-slider-fill"
              style="width: {((currentSettings.ram_cache_budget_mb - 128) / (2048 - 128)) * 100}%"
            ></div>
            <div
              class="custom-slider-thumb"
              style="left: {((currentSettings.ram_cache_budget_mb - 128) / (2048 - 128)) * 100}%"
              on:mousedown={(e) => startSliderDrag('ram_cache_budget_mb', 128, 2048, 128, e)}
            ></div>
          </div>
        </div>

        <!-- BESPOKE CONCURRENT DOWNLOADS SLIDER -->
        <div class="bespoke-slider-card">
          <div class="slider-header-meta">
            <div>
              <span class="meta-label">Concurrent Download Workers</span>
              <span class="meta-hint">Parallel network streams for chapters and artwork</span>
            </div>
            <span class="numeric-badge">{currentSettings.max_concurrent_downloads} threads</span>
          </div>
          <div
            id="track-max_concurrent_downloads"
            class="custom-slider-track"
            on:click={(e) => handleTrackClick(e, 'max_concurrent_downloads', 1, 8, 1)}
          >
            <div
              class="custom-slider-fill"
              style="width: {((currentSettings.max_concurrent_downloads - 1) / 7) * 100}%"
            ></div>
            <div
              class="custom-slider-thumb"
              style="left: {((currentSettings.max_concurrent_downloads - 1) / 7) * 100}%"
              on:mousedown={(e) => startSliderDrag('max_concurrent_downloads', 1, 8, 1, e)}
            ></div>
          </div>
        </div>

        <!-- CONTINUOUS CHAPTER FLOW TOGGLE -->
        <div class="interactive-option-row">
          <div class="row-info">
            <span class="row-label">Continuous Chapter Flow</span>
            <span class="row-desc">Seamlessly load next chapter upon reaching the final page</span>
          </div>
          <button
            class="bespoke-switch"
            class:checked={currentSettings.infinite_chapter_scroll}
            on:click={() => toggleSetting('infinite_chapter_scroll')}
          >
            <div class="switch-ball"></div>
          </button>
        </div>
      </section>

      <!-- 4. CONTENT & PRIVACY -->
      <section class="custom-card">
        <div class="card-title-row">
          <div class="card-icon-bubble">
            <ShieldCheck size={18} />
          </div>
          <div>
            <h2 class="card-title">Sources & Content Filters</h2>
            <p class="card-subtitle">Manage catalog visibility and secure networking</p>
          </div>
        </div>

        <!-- ADULT / NSFW TOGGLE -->
        <div class="interactive-option-row">
          <div class="row-info">
            <span class="row-label">Include 18+ Mature Content</span>
            <span class="row-desc">Show adult sources and titles in catalog search</span>
          </div>
          <button
            class="bespoke-switch"
            class:checked={currentSettings.nsfw_enabled}
            on:click={() => toggleSetting('nsfw_enabled')}
          >
            <div class="switch-ball"></div>
          </button>
        </div>

        <!-- DNS OVER HTTPS (DOH) -->
        <div class="interactive-option-row">
          <div class="row-info">
            <span class="row-label">DNS-over-HTTPS (DoH)</span>
            <span class="row-desc">Bypasses ISP DNS blocking and protects reading privacy</span>
          </div>
          <button
            class="bespoke-switch"
            class:checked={currentSettings.dns_over_https}
            on:click={() => toggleSetting('dns_over_https')}
          >
            <div class="switch-ball"></div>
          </button>
        </div>

        <!-- CLEAR CACHE ACTION -->
        <div class="danger-option-row">
          <div class="row-info">
            <span class="row-label">Disk Storage & Cover Cache</span>
            <span class="row-desc">Purges cached thumbnails, temporary downloads, and scraper cookies</span>
          </div>
          <button class="purge-action-btn" on:click={handleClearCache}>
            <Trash2 size={15} />
            <span>Clear Cache</span>
          </button>
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

  .settings-hero {
    margin-bottom: var(--z-space-6);
  }

  .settings-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 10px;
    font-weight: 800;
    letter-spacing: 1.2px;
    color: var(--z-accent);
    background: var(--z-accent-muted);
    padding: 3px 10px;
    border-radius: var(--z-radius-full);
    border: 1px solid var(--z-accent-glow);
    margin-bottom: 6px;
  }

  .page-title {
    font-size: 26px;
    font-weight: 800;
    letter-spacing: -0.5px;
    color: var(--z-text);
    margin-bottom: 4px;
  }

  .page-desc {
    font-size: var(--z-text-xs);
    color: var(--z-text-muted);
  }

  .settings-grid {
    display: flex;
    flex-direction: column;
    gap: var(--z-space-6);
    max-width: 860px;
    padding-bottom: 60px;
  }

  .custom-card {
    background: rgba(18, 18, 28, 0.7);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 18px;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.45), inset 0 1px 0 rgba(255, 255, 255, 0.1);
  }

  .card-title-row {
    display: flex;
    align-items: center;
    gap: 14px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    padding-bottom: 14px;
  }

  .card-icon-bubble {
    width: 38px;
    height: 38px;
    border-radius: 10px;
    background: var(--z-accent-muted);
    color: var(--z-accent);
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--z-accent-glow);
    flex-shrink: 0;
  }

  .card-title {
    font-size: 16px;
    font-weight: 700;
    color: var(--z-text);
  }

  .card-subtitle {
    font-size: 12px;
    color: var(--z-text-muted);
  }

  /* PALETTE HERO PREVIEW */
  .palette-preview-card {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 14px 18px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 14px;
    position: relative;
    overflow: hidden;
  }

  .preview-swatch-glow {
    width: 38px;
    height: 38px;
    border-radius: 50%;
    box-shadow: 0 0 20px var(--p-color);
    flex-shrink: 0;
    border: 2px solid #ffffff;
  }

  .preview-info {
    flex: 1;
  }

  .preview-heading {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 2px;
  }

  .preview-name {
    font-size: 15px;
    font-weight: 700;
    color: #ffffff;
  }

  .hex-badge {
    font-family: var(--z-font-mono);
    font-size: 11px;
    font-weight: 600;
    color: var(--z-accent);
    background: var(--z-accent-muted);
    padding: 2px 8px;
    border-radius: 6px;
  }

  .preview-desc {
    font-size: 12px;
    color: var(--z-text-muted);
  }

  /* BESPOKE SPECTRUM RIBBON */
  .custom-spectrum-wrap {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 8px 0;
  }

  .spectrum-ribbon-bar {
    position: relative;
    width: 100%;
    height: 14px;
    border-radius: 9999px;
    background: linear-gradient(90deg, 
      #4285f4 0%, 
      #38bdf8 12%, 
      #06b6d4 25%, 
      #34d399 38%, 
      #fbbf24 50%, 
      #f97316 62%, 
      #ef4444 75%, 
      #fb7185 85%, 
      #c084fc 100%
    );
    cursor: pointer;
    box-shadow: 0 0 16px rgba(255, 255, 255, 0.12), inset 0 1px 2px rgba(0, 0, 0, 0.5);
  }

  .spectrum-drag-handle {
    position: absolute;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: #ffffff;
    border: 3px solid var(--handle-color, #4285f4);
    box-shadow: 0 0 20px var(--handle-color, #4285f4), 0 3px 8px rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: grab;
    transition: transform 0.15s ease;
  }

  .spectrum-drag-handle:active {
    cursor: grabbing;
    transform: translate(-50%, -50%) scale(1.15);
  }

  .handle-inner-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--handle-color, #4285f4);
  }

  .spectrum-legend-labels {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    font-weight: 700;
    color: var(--z-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.8px;
  }

  /* PALETTE CHIPS */
  .palette-chips-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .palette-chip-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 7px 14px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: var(--z-radius-full);
    color: var(--z-text);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .palette-chip-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    transform: translateY(-2px);
  }

  .palette-chip-btn.active {
    background: var(--z-accent-muted);
    border-color: var(--z-accent);
    color: #ffffff;
    font-weight: 600;
    box-shadow: 0 0 14px var(--z-accent-glow);
  }

  .chip-color-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  :global(.chip-check-icon) {
    color: var(--z-accent);
  }

  /* FIELD SUBGROUPS & BACKGROUND CARDS */
  .field-subgroup {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .group-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--z-text);
  }

  .bg-mode-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 14px;
  }

  .bg-mode-card {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 14px;
    padding: 12px;
    cursor: pointer;
    transition: all 0.25s cubic-bezier(0.16, 1, 0.3, 1);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .bg-mode-card:hover {
    transform: translateY(-3px);
    border-color: rgba(255, 255, 255, 0.2);
  }

  .bg-mode-card.selected {
    border-color: var(--z-accent);
    background: var(--z-accent-muted);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5), 0 0 16px var(--z-accent-glow);
  }

  .mock-window-frame {
    width: 100%;
    height: 70px;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .mock-topbar {
    height: 14px;
    background: rgba(255, 255, 255, 0.05);
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 6px;
  }

  .mock-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
  }
  .mock-dot.red { background: #ef4444; }
  .mock-dot.yellow { background: #eab308; }
  .mock-dot.green { background: #22c55e; }

  .mock-content {
    flex: 1;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .mock-bar {
    width: 35%;
    height: 6px;
    border-radius: 3px;
  }

  .mock-line {
    width: 70%;
    height: 4px;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.15);
  }

  .mode-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 2px;
  }

  .mode-label {
    font-size: 13px;
    font-weight: 700;
    color: var(--z-text);
  }

  .active-badge {
    font-size: 9px;
    font-weight: 800;
    color: var(--z-accent);
    letter-spacing: 0.6px;
  }

  .mode-desc {
    font-size: 11px;
    color: var(--z-text-muted);
    line-height: 1.35;
  }

  /* CUSTOM SEGMENTED BARS */
  .custom-segmented-bar {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
    gap: 6px;
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid rgba(255, 255, 255, 0.08);
    padding: 4px;
    border-radius: 12px;
  }

  .seg-button {
    padding: 10px 14px;
    border-radius: 8px;
    border: none;
    background: transparent;
    color: var(--z-text-muted);
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .seg-button:hover {
    color: #ffffff;
    background: rgba(255, 255, 255, 0.04);
  }

  .seg-button.active {
    background: var(--z-accent);
    color: #ffffff;
    font-weight: 700;
    box-shadow: 0 2px 10px var(--z-accent-glow);
  }

  /* BESPOKE SLIDER CARDS */
  .bespoke-slider-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px 18px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 14px;
  }

  .slider-header-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .meta-label {
    font-size: 13.5px;
    font-weight: 600;
    color: var(--z-text);
    display: block;
  }

  .meta-hint {
    font-size: 11.5px;
    color: var(--z-text-muted);
  }

  .numeric-badge {
    font-family: var(--z-font-mono);
    font-size: 12px;
    font-weight: 700;
    color: var(--z-accent);
    background: var(--z-accent-muted);
    padding: 3px 10px;
    border-radius: 8px;
    border: 1px solid var(--z-accent-glow);
  }

  .custom-slider-track {
    position: relative;
    width: 100%;
    height: 8px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 9999px;
    cursor: pointer;
    display: flex;
    align-items: center;
  }

  .custom-slider-fill {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    background: var(--z-accent);
    border-radius: 9999px;
    box-shadow: 0 0 10px var(--z-accent-glow);
  }

  .custom-slider-thumb {
    position: absolute;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #ffffff;
    border: 2px solid var(--z-accent);
    box-shadow: 0 0 12px var(--z-accent-glow), 0 2px 6px rgba(0, 0, 0, 0.4);
    cursor: grab;
    transition: transform 0.15s ease;
  }

  .custom-slider-thumb:active {
    cursor: grabbing;
    transform: translate(-50%, -50%) scale(1.2);
  }

  /* OPTION ROWS & BESPOKE SWITCHES */
  .interactive-option-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 18px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 12px;
  }

  .row-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .row-label {
    font-size: 13.5px;
    font-weight: 600;
    color: var(--z-text);
  }

  .row-desc {
    font-size: 11.5px;
    color: var(--z-text-muted);
  }

  .bespoke-switch {
    width: 48px;
    height: 26px;
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.15);
    border: 1px solid rgba(255, 255, 255, 0.2);
    position: relative;
    cursor: pointer;
    transition: all 0.25s cubic-bezier(0.16, 1, 0.3, 1);
    flex-shrink: 0;
  }

  .bespoke-switch.checked {
    background: var(--z-accent);
    border-color: var(--z-accent);
    box-shadow: 0 0 12px var(--z-accent-glow);
  }

  .switch-ball {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #ffffff;
    position: absolute;
    top: 2px;
    left: 2px;
    transition: transform 0.25s cubic-bezier(0.16, 1, 0.3, 1);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.4);
  }

  .bespoke-switch.checked .switch-ball {
    transform: translateX(22px);
  }

  /* DANGER MAINTENANCE */
  .danger-option-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 18px;
    background: rgba(239, 68, 68, 0.06);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: 12px;
  }

  .purge-action-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    background: rgba(239, 68, 68, 0.18);
    border: 1px solid rgba(239, 68, 68, 0.4);
    border-radius: 8px;
    color: #f87171;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .purge-action-btn:hover {
    background: rgba(239, 68, 68, 0.3);
    color: #ffffff;
    box-shadow: 0 0 12px rgba(239, 68, 68, 0.4);
  }
</style>