<script>
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { appState, goBack, notify } from '../stores/app.svelte.js';
  import { 
    ArrowLeft, Settings, ZoomIn, ZoomOut, ChevronLeft, ChevronRight,
    Sun, Moon, Eye, Sliders, Maximize2, Minimize2, RotateCcw, ShieldAlert
  } from 'lucide-svelte';

  let manga = appState.activeManga;
  let chapter = appState.activeChapter;
  let pages = [];
  let loading = true;
  let loadError = null;
  let unlistenCf = null;
  let currentPageIndex = 0;
  let showHud = true;
  let showSettings = false; // Toggled by TAB key!
  let hudTimeout = null;

  // Reader image settings
  let readerMode = 'webtoon'; // 'webtoon' | 'single'
  let zoomPercent = 100;
  let contrast = 100; // 50 to 150
  let brightness = 100; // 50 to 150
  let invertColors = false;

  let scrollContainer;

  async function loadPages() {
    if (!manga || !chapter) return;
    loading = true;
    loadError = null;
    try {
      pages = await invoke('get_pages', {
        sourceId: manga.source_id,
        chapter,
      });

      if (!pages || pages.length === 0) {
        loadError = 'No images returned for this chapter.';
      } else {
        // Try to restore saved progress
        try {
          const prog = await invoke('get_reading_progress', { mangaId: manga.id });
          if (prog && prog.chapter_id === chapter.id && prog.page_index < pages.length) {
            currentPageIndex = prog.page_index;
          }
        } catch (_) {}
      }
    } catch (e) {
      console.error('Failed to load chapter pages:', e);
      loadError = String(e);
      notify('Failed to load pages: ' + e);
    } finally {
      loading = false;
    }
  }

  async function handleSolveCloudflare() {
    if (!manga) return;
    try {
      const targetUrl = chapter?.url || manga.url || 'https://google.com';
      await invoke('open_cloudflare_solver', {
        url: targetUrl,
        title: `${manga.title} — Cloudflare Verification`,
        sourceId: manga.source_id,
      });
      notify('Opening Cloudflare solver in light browser...');
    } catch (e) {
      notify('Solver error: ' + e);
    }
  }

  function handleKeydown(e) {
    // TAB KEY BINDING FOR READER SETTINGS
    if (e.key === 'Tab') {
      e.preventDefault();
      showSettings = !showSettings;
      if (showSettings) showHud = true;
      return;
    }

    if (e.key === 'Escape') {
      if (showSettings) {
        showSettings = false;
      } else {
        goBack();
      }
      return;
    }

    if (e.key === 'ArrowRight' || e.key === 'PageDown' || e.key === ' ') {
      if (readerMode === 'single' && currentPageIndex < pages.length - 1) {
        setPage(currentPageIndex + 1);
      }
    } else if (e.key === 'ArrowLeft' || e.key === 'PageUp') {
      if (readerMode === 'single' && currentPageIndex > 0) {
        setPage(currentPageIndex - 1);
      }
    }
  }

  function setPage(index) {
    currentPageIndex = Math.max(0, Math.min(pages.length - 1, index));
    saveCurrentProgress();

    if (readerMode === 'webtoon' && scrollContainer) {
      const targetImg = scrollContainer.querySelector(`[data-page-index="${currentPageIndex}"]`);
      if (targetImg) {
        targetImg.scrollIntoView({ behavior: 'smooth' });
      }
    }
  }

  function handleScroll(e) {
    if (readerMode !== 'webtoon') return;
    const container = e.target;
    const pageElements = container.querySelectorAll('.reader-page-img');
    const containerMid = container.scrollTop + container.clientHeight / 3;

    for (let i = 0; i < pageElements.length; i++) {
      const el = pageElements[i];
      if (el.offsetTop <= containerMid && (el.offsetTop + el.clientHeight) > containerMid) {
        if (currentPageIndex !== i) {
          currentPageIndex = i;
          saveCurrentProgress();
        }
        break;
      }
    }
  }

  function saveCurrentProgress() {
    if (!manga || !chapter || pages.length === 0) return;
    invoke('save_reading_progress', {
      progress: {
        manga_id: manga.id,
        chapter_id: chapter.id,
        page_index: currentPageIndex,
        scroll_offset_y: 0.0,
        chapter_number: chapter.number ?? 0.0,
        updated_at: Date.now(),
      },
    }).catch(() => {});
  }

  function toggleHud() {
    showHud = !showHud;
    if (!showHud) showSettings = false;
  }

  function handleMouseMove() {
    showHud = true;
    clearTimeout(hudTimeout);
    hudTimeout = setTimeout(() => {
      if (!showSettings) {
        showHud = false;
      }
    }, 4500);
  }

  onMount(async () => {
    window.addEventListener('keydown', handleKeydown);
    loadPages();
    try {
      unlistenCf = await listen('cloudflare-solved', (event) => {
        if (event.payload?.source_id === manga?.source_id) {
          notify('Cloudflare verification passed! Reloading pages...');
          loadPages();
        }
      });
    } catch (_) {}
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
    clearTimeout(hudTimeout);
    saveCurrentProgress();
    if (unlistenCf) unlistenCf();
  });
</script>

<div
  class="reader-container"
  class:invert={invertColors}
  on:mousemove={handleMouseMove}
  on:click={toggleHud}
>
  <!-- TOP HUD BAR -->
  <header class="hud-top" class:visible={showHud} on:click|stopPropagation>
    <div class="hud-left">
      <button class="hud-btn" on:click={goBack} title="Back to Details">
        <ArrowLeft size={20} strokeWidth={2} />
      </button>
      <div class="hud-titles">
        <span class="manga-name">{manga?.title}</span>
        <span class="chapter-name">{chapter?.title}</span>
      </div>
    </div>

    <div class="hud-right">
      <span class="page-counter">{currentPageIndex + 1} / {pages.length}</span>
      <button
        class="hud-btn"
        on:click={handleSolveCloudflare}
        title="Pass Cloudflare Challenge (Light Browser)"
      >
        <ShieldAlert size={18} />
      </button>
      <button
        class="hud-btn"
        class:active={showSettings}
        on:click={() => showSettings = !showSettings}
        title="Settings (Tab key)"
      >
        <Settings size={20} />
      </button>
    </div>
  </header>

  <!-- READER CANVAS -->
  <main
    class="reader-canvas"
    bind:this={scrollContainer}
    on:scroll={handleScroll}
  >
    {#if loading}
      <div class="loading-state">
        <div class="spinner"></div>
        <p>Loading chapter pages...</p>
      </div>
    {:else if loadError || pages.length === 0}
      <div class="reader-error-state" on:click|stopPropagation>
        <ShieldAlert size={44} class="cf-alert-icon" />
        <h2>Unable to load chapter images</h2>
        <p class="error-msg">{loadError || 'Empty page list returned.'}</p>
        <p class="error-sub">This source may require passing a Cloudflare challenge or captcha check.</p>
        <div class="error-btn-row">
          <button class="cf-solve-btn" on:click={handleSolveCloudflare}>
            <ShieldAlert size={18} />
            Pass Cloudflare (Light Browser)
          </button>
          <button class="cf-retry-btn" on:click={loadPages}>
            <RotateCcw size={16} />
            Retry
          </button>
          <button class="cf-back-btn" on:click={goBack}>
            <ArrowLeft size={16} />
            Back
          </button>
        </div>
      </div>
    {:else if readerMode === 'webtoon'}
      <!-- Continuous Webtoon Strip -->
      <div
        class="webtoon-strip"
        style:max-width="{zoomPercent}%"
        style:filter="contrast({contrast}%) brightness({brightness}%)"
      >
        {#each pages as page, idx (page.image_url)}
          <img
            src={page.image_url}
            alt="Page {idx + 1}"
            loading="lazy"
            decoding="async"
            data-page-index={idx}
            class="reader-page-img"
          />
        {/each}
      </div>
    {:else}
      <!-- Single Page Mode -->
      <div
        class="single-page-wrapper"
        style:transform="scale({zoomPercent / 100})"
        style:filter="contrast({contrast}%) brightness({brightness}%)"
      >
        {#if pages[currentPageIndex]}
          <img
            src={pages[currentPageIndex].image_url}
            alt="Page {currentPageIndex + 1}"
            class="reader-single-img"
          />
        {/if}
      </div>
    {/if}
  </main>

  <!-- SETTINGS MODAL (TAB KEY) -->
  {#if showSettings}
    <div class="settings-modal" on:click|stopPropagation>
      <div class="modal-header">
        <span class="modal-title">Reader Settings <small>(Tab)</small></span>
        <button class="close-btn" on:click={() => showSettings = false}>✕</button>
      </div>

      <div class="setting-group">
        <label>View Mode</label>
        <div class="toggle-group">
          <button
            class="toggle-opt"
            class:active={readerMode === 'webtoon'}
            on:click={() => readerMode = 'webtoon'}
          >
            Webtoon (Vertical)
          </button>
          <button
            class="toggle-opt"
            class:active={readerMode === 'single'}
            on:click={() => readerMode = 'single'}
          >
            Single Page
          </button>
        </div>
      </div>

      <div class="setting-group">
        <label>Zoom: {zoomPercent}%</label>
        <input type="range" min="60" max="150" step="5" bind:value={zoomPercent} />
      </div>

      <div class="setting-group">
        <label>Brightness: {brightness}%</label>
        <input type="range" min="50" max="150" step="5" bind:value={brightness} />
      </div>

      <div class="setting-group">
        <label>Contrast: {contrast}%</label>
        <input type="range" min="50" max="150" step="5" bind:value={contrast} />
      </div>

      <div class="setting-group">
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={invertColors} />
          <span>Invert Colors (Night Reading)</span>
        </label>
      </div>
    </div>
  {/if}

  <!-- BOTTOM HUD BAR -->
  <footer class="hud-bottom" class:visible={showHud} on:click|stopPropagation>
    <div class="hud-bottom-content">
      <button
        class="slider-step-btn"
        disabled={currentPageIndex <= 0}
        on:click={() => setPage(currentPageIndex - 1)}
      >
        <ChevronLeft size={20} />
      </button>

      <input
        type="range"
        min="0"
        max={Math.max(0, pages.length - 1)}
        value={currentPageIndex}
        on:input={(e) => setPage(parseInt(e.target.value))}
        class="page-slider"
      />

      <button
        class="slider-step-btn"
        disabled={currentPageIndex >= pages.length - 1}
        on:click={() => setPage(currentPageIndex + 1)}
      >
        <ChevronRight size={20} />
      </button>
    </div>
  </footer>
</div>

<style>
  .reader-container {
    position: fixed;
    inset: 0;
    background: var(--z-reader-bg);
    overflow: hidden;
    z-index: 100;
    user-select: none;
  }

  .reader-container.invert {
    filter: invert(1) hue-rotate(180deg);
  }

  .hud-top, .hud-bottom {
    position: fixed;
    left: 0;
    right: 0;
    background: rgba(12, 14, 20, 0.88);
    backdrop-filter: blur(14px);
    padding: var(--z-space-3) var(--z-space-6);
    color: var(--z-text);
    z-index: 110;
    transition: transform var(--z-duration-normal) var(--z-ease-out),
                opacity var(--z-duration-normal) var(--z-ease-out);
    pointer-events: none;
  }

  .hud-top {
    top: 0;
    transform: translateY(-100%);
    opacity: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--z-border);
  }

  .hud-top.visible {
    transform: translateY(0);
    opacity: 1;
    pointer-events: auto;
  }

  .hud-bottom {
    bottom: 0;
    transform: translateY(100%);
    opacity: 0;
    border-top: 1px solid var(--z-border);
  }

  .hud-bottom.visible {
    transform: translateY(0);
    opacity: 1;
    pointer-events: auto;
  }

  .hud-left {
    display: flex;
    align-items: center;
    gap: var(--z-space-4);
  }

  .hud-btn {
    width: 36px;
    height: 36px;
    border-radius: var(--z-radius-full);
    color: var(--z-text);
    transition: all var(--z-duration-fast) var(--z-ease-out);
  }

  .hud-btn:hover {
    background: var(--z-surface-raised);
  }

  .hud-btn.active {
    background: var(--z-accent);
    color: #ffffff;
  }

  .hud-titles {
    display: flex;
    flex-direction: column;
    max-width: 50vw;
  }

  .manga-name {
    font-size: var(--z-text-xs);
    color: var(--z-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chapter-name {
    font-size: var(--z-text-sm);
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .hud-right {
    display: flex;
    align-items: center;
    gap: var(--z-space-4);
  }

  .page-counter {
    font-size: var(--z-text-sm);
    font-weight: 600;
    color: var(--z-text-muted);
  }

  .hud-bottom-content {
    max-width: 700px;
    margin: 0 auto;
    display: flex;
    align-items: center;
    gap: var(--z-space-4);
  }

  .page-slider {
    flex: 1;
    accent-color: var(--z-accent);
    cursor: pointer;
  }

  .slider-step-btn {
    width: 32px;
    height: 32px;
    border-radius: var(--z-radius-full);
    color: var(--z-text);
    transition: all var(--z-duration-fast) var(--z-ease-out);
  }

  .slider-step-btn:hover:not(:disabled) {
    background: var(--z-surface-raised);
  }

  .slider-step-btn:disabled {
    opacity: 0.3;
  }

  .reader-canvas {
    width: 100vw;
    height: 100vh;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex;
    justify-content: center;
    background: var(--z-reader-bg);
  }

  .webtoon-strip {
    display: flex;
    flex-direction: column;
    width: 100%;
    margin: 0 auto;
  }

  .reader-page-img {
    width: 100%;
    height: auto;
    display: block;
    margin: 0 auto;
    user-select: none;
  }

  .single-page-wrapper {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    width: 100%;
  }

  .reader-single-img {
    max-height: 100vh;
    max-width: 100vw;
    object-fit: contain;
  }

  .settings-modal {
    position: fixed;
    top: 70px;
    right: 24px;
    width: 300px;
    background: rgba(20, 25, 35, 0.95);
    backdrop-filter: blur(16px);
    border: 1px solid var(--z-border);
    border-radius: var(--z-radius-lg);
    padding: var(--z-space-4);
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.6);
    z-index: 120;
    display: flex;
    flex-direction: column;
    gap: var(--z-space-3);
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--z-border);
    padding-bottom: var(--z-space-2);
  }

  .modal-title {
    font-size: var(--z-text-sm);
    font-weight: 700;
  }

  .modal-title small {
    color: var(--z-accent);
    font-size: 11px;
  }

  .close-btn {
    font-size: 14px;
    color: var(--z-text-muted);
  }

  .setting-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: var(--z-text-xs);
    color: var(--z-text-muted);
  }

  .setting-group input[type="range"] {
    accent-color: var(--z-accent);
  }

  .toggle-group {
    display: flex;
    background: var(--z-surface-raised);
    padding: 2px;
    border-radius: var(--z-radius-md);
  }

  .toggle-opt {
    flex: 1;
    padding: 6px 8px;
    border-radius: var(--z-radius-sm);
    font-size: var(--z-text-xs);
    color: var(--z-text-muted);
  }

  .toggle-opt.active {
    background: var(--z-accent);
    color: #ffffff;
    font-weight: 600;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: var(--z-space-2);
    cursor: pointer;
    font-size: var(--z-text-xs);
    color: var(--z-text);
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100vh;
    gap: var(--z-space-3);
    color: var(--z-text-muted);
  }

  .spinner {
    width: 36px;
    height: 36px;
    border: 3px solid var(--z-border);
    border-top-color: var(--z-accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .reader-error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 80vh;
    padding: var(--z-space-8);
    text-align: center;
    max-width: 500px;
    margin: 0 auto;
  }

  :global(.cf-alert-icon) {
    color: #a78bfa;
    margin-bottom: var(--z-space-4);
  }

  .reader-error-state h2 {
    font-size: var(--z-text-xl);
    font-weight: 700;
    color: var(--z-text);
    margin-bottom: var(--z-space-2);
  }

  .error-msg {
    font-size: var(--z-text-sm);
    color: #ef4444;
    margin-bottom: var(--z-space-2);
  }

  .error-sub {
    font-size: var(--z-text-xs);
    color: var(--z-text-muted);
    line-height: 1.5;
    margin-bottom: var(--z-space-6);
  }

  .error-btn-row {
    display: flex;
    align-items: center;
    gap: var(--z-space-3);
    flex-wrap: wrap;
    justify-content: center;
  }

  .cf-solve-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    background: #7c3aed;
    color: #ffffff;
    font-size: var(--z-text-xs);
    font-weight: 600;
    padding: 10px 18px;
    border-radius: var(--z-radius-md);
    border: none;
    cursor: pointer;
    transition: background 0.2s;
  }

  .cf-solve-btn:hover {
    background: #6d28d9;
  }

  .cf-retry-btn, .cf-back-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--z-surface-raised);
    color: var(--z-text);
    border: 1px solid var(--z-border);
    font-size: var(--z-text-xs);
    font-weight: 500;
    padding: 10px 16px;
    border-radius: var(--z-radius-md);
    cursor: pointer;
    transition: all 0.2s;
  }

  .cf-retry-btn:hover, .cf-back-btn:hover {
    background: var(--z-border-hover);
  }
</style>
