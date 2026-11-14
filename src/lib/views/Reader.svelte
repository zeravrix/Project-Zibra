<script>
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { appState, goBack, notify, updateSettings } from '../stores/app.svelte.js';
  import { 
    ArrowLeft, Settings, ZoomIn, ZoomOut, ChevronLeft, ChevronRight,
    Sun, Moon, Eye, Sliders, Maximize2, Minimize2, RotateCcw, ShieldAlert,
    BookOpen, Columns, AlignJustify, ArrowRightLeft, Check
  } from 'lucide-svelte';

  let manga = appState.activeManga;
  let chapter = appState.activeChapter;
  let pages = [];
  let loading = true;
  let loadError = null;
  let unlistenCf = null;
  let currentPageIndex = 0;
  let showHud = true;
  let showSettings = false;
  let hudTimeout = null;
  let isFullscreen = false;

  // Reader configuration (loaded from settings or defaults)
  let readerMode = 'webtoon'; // 'webtoon' | 'single' | 'double'
  let readingDirection = 'LTR'; // 'LTR' | 'RTL'
  let pageGap = 0; // 0, 4, 8, 16 px for webtoon
  let zoomPercent = 100;
  let contrast = 100; // 50 to 150
  let brightness = 100; // 50 to 150
  let invertColors = false;

  let scrollContainer;

  // Active slider drag state
  let draggingSlider = null;

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
        // Preload adjacent images for smooth endless scrolling
        preloadUpcomingPages(0);

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

  function preloadUpcomingPages(startIndex) {
    if (!pages || pages.length === 0) return;
    const end = Math.min(pages.length, startIndex + 4);
    for (let i = startIndex; i < end; i++) {
      if (pages[i]?.image_url) {
        const img = new Image();
        img.src = pages[i].image_url;
      }
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
    if (e.key === 'Tab') {
      e.preventDefault();
      showSettings = !showSettings;
      if (showSettings) showHud = true;
      return;
    }

    if (e.key === 'Escape') {
      if (showSettings) {
        showSettings = false;
      } else if (isFullscreen) {
        toggleFullscreen();
      } else {
        goBack();
      }
      return;
    }

    if (e.key.toLowerCase() === 'f') {
      toggleFullscreen();
      return;
    }

    // Navigation hotkeys
    const isRtl = readingDirection === 'RTL';
    if (e.key === 'ArrowRight' || e.key === 'PageDown' || e.key === ' ') {
      e.preventDefault();
      if (isRtl) {
        prevStep();
      } else {
        nextStep();
      }
    } else if (e.key === 'ArrowLeft' || e.key === 'PageUp') {
      e.preventDefault();
      if (isRtl) {
        nextStep();
      } else {
        prevStep();
      }
    }
  }

  function nextStep() {
    if (readerMode === 'double') {
      if (currentPageIndex === 0) {
        setPage(1);
      } else {
        setPage(Math.min(pages.length - 1, currentPageIndex + 2));
      }
    } else if (readerMode === 'single') {
      if (currentPageIndex < pages.length - 1) {
        setPage(currentPageIndex + 1);
      }
    } else if (readerMode === 'webtoon' && scrollContainer) {
      scrollContainer.scrollBy({ top: window.innerHeight * 0.75, behavior: 'smooth' });
    }
  }

  function prevStep() {
    if (readerMode === 'double') {
      if (currentPageIndex <= 1) {
        setPage(0);
      } else {
        setPage(Math.max(1, currentPageIndex - 2));
      }
    } else if (readerMode === 'single') {
      if (currentPageIndex > 0) {
        setPage(currentPageIndex - 1);
      }
    } else if (readerMode === 'webtoon' && scrollContainer) {
      scrollContainer.scrollBy({ top: -window.innerHeight * 0.75, behavior: 'smooth' });
    }
  }

  function setPage(index) {
    currentPageIndex = Math.max(0, Math.min(pages.length - 1, index));
    saveCurrentProgress();
    preloadUpcomingPages(currentPageIndex);

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
          preloadUpcomingPages(currentPageIndex);
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

  function toggleFullscreen() {
    if (!document.fullscreenElement) {
      document.documentElement.requestFullscreen().then(() => isFullscreen = true).catch(() => {});
    } else {
      document.exitFullscreen().then(() => isFullscreen = false).catch(() => {});
    }
  }

  function changeReaderMode(mode) {
    readerMode = mode;
    if (appState.settings) {
      appState.settings.reader_mode = mode === 'webtoon' ? 'Vertical' : mode === 'double' ? 'Double' : 'Single';
      updateSettings(appState.settings);
    }
  }

  function changeDirection(dir) {
    readingDirection = dir;
    if (appState.settings) {
      appState.settings.reading_direction = dir;
      updateSettings(appState.settings);
    }
  }

  // Double spread computed pages
  $: isCoverSpread = readerMode === 'double' && currentPageIndex === 0;
  $: spreadPages = (() => {
    if (readerMode !== 'double') return [];
    if (currentPageIndex === 0) return [pages[0]];
    const baseIdx = currentPageIndex % 2 === 1 ? currentPageIndex : currentPageIndex - 1;
    const p1 = pages[baseIdx];
    const p2 = pages[baseIdx + 1];

    if (readingDirection === 'RTL') {
      // Right to left (Manga): right is p1, left is p2
      return [p2, p1].filter(Boolean);
    } else {
      // Left to right: left is p1, right is p2
      return [p1, p2].filter(Boolean);
    }
  })();

  // Custom slider mouse handler
  function startDrag(field, min, max, step, e) {
    e.preventDefault();
    draggingSlider = field;
    const track = e.currentTarget.closest('.custom-track');
    if (!track) return;

    const updateVal = (clientX) => {
      const rect = track.getBoundingClientRect();
      const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
      let val = min + ratio * (max - min);
      if (step) val = Math.round(val / step) * step;

      if (field === 'zoom') zoomPercent = val;
      else if (field === 'brightness') brightness = val;
      else if (field === 'contrast') contrast = val;
    };

    updateVal(e.clientX);

    const onMove = (ev) => updateVal(ev.clientX);
    const onUp = () => {
      draggingSlider = null;
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };

    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  onMount(async () => {
    window.addEventListener('keydown', handleKeydown);
    if (appState.settings?.reader_mode) {
      const rm = appState.settings.reader_mode.toLowerCase();
      if (rm.includes('double')) readerMode = 'double';
      else if (rm.includes('single')) readerMode = 'single';
      else readerMode = 'webtoon';
    }
    if (appState.settings?.reading_direction) {
      readingDirection = appState.settings.reading_direction;
    }
    if (typeof appState.settings?.seam_trim_px === 'number') {
      pageGap = appState.settings.seam_trim_px;
    }

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
  role="region"
  aria-label="Manga Reader"
>
  <!-- TOP HUD BAR -->
  <header class="hud-top" class:visible={showHud} on:click|stopPropagation>
    <div class="hud-left">
      <button class="hud-btn" on:click={goBack} title="Back to Details">
        <ArrowLeft size={18} strokeWidth={2} />
      </button>
      <div class="hud-titles">
        <span class="manga-name">{manga?.title || 'Manga'}</span>
        <span class="chapter-name">{chapter?.title || 'Chapter'}</span>
      </div>
    </div>

    <div class="hud-right">
      <span class="page-counter">
        {#if readerMode === 'double' && currentPageIndex > 0 && currentPageIndex < pages.length - 1}
          {currentPageIndex}–{currentPageIndex + 1} / {pages.length}
        {:else}
          {currentPageIndex + 1} / {pages.length}
        {/if}
      </span>
      <button
        class="hud-btn"
        on:click={toggleFullscreen}
        title={isFullscreen ? "Exit Fullscreen (F)" : "Fullscreen (F)"}
      >
        {#if isFullscreen}
          <Minimize2 size={17} />
        {:else}
          <Maximize2 size={17} />
        {/if}
      </button>
      <button
        class="hud-btn"
        on:click={handleSolveCloudflare}
        title="Pass Cloudflare Challenge"
      >
        <ShieldAlert size={17} />
      </button>
      <button
        class="hud-btn"
        class:active={showSettings}
        on:click={() => showSettings = !showSettings}
        title="Reader Settings (Tab)"
      >
        <Settings size={18} />
      </button>
    </div>
  </header>

  <!-- READER CANVAS -->
  <main
    class="reader-canvas"
    class:spread-canvas={readerMode === 'double'}
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
        <ShieldAlert size={40} class="cf-alert-icon" />
        <h2>Unable to load chapter images</h2>
        <p class="error-msg">{loadError || 'Empty page list returned.'}</p>
        <p class="error-sub">This source may require passing a Cloudflare challenge or captcha check.</p>
        <div class="error-btn-row">
          <button class="cf-solve-btn" on:click={handleSolveCloudflare}>
            <ShieldAlert size={16} />
            Pass Cloudflare
          </button>
          <button class="cf-retry-btn" on:click={loadPages}>
            <RotateCcw size={15} />
            Retry
          </button>
          <button class="cf-back-btn" on:click={goBack}>
            <ArrowLeft size={15} />
            Back
          </button>
        </div>
      </div>

    {:else if readerMode === 'webtoon'}
      <!-- Endless Vertical Webtoon Strip -->
      <div
        class="webtoon-strip"
        style="max-width: {zoomPercent}%; row-gap: {pageGap}px; filter: contrast({contrast}%) brightness({brightness}%);"
      >
        {#each pages as page, idx (page.image_url)}
          <img
            src={page.image_url}
            alt="Page {idx + 1}"
            loading={idx < 3 ? 'eager' : 'lazy'}
            decoding="async"
            data-page-index={idx}
            class="reader-page-img"
          />
        {/each}

        <!-- End of Chapter Card -->
        <div class="chapter-end-card">
          <span class="end-badge">END OF CHAPTER</span>
          <p class="end-title">{chapter?.title || 'Chapter Completed'}</p>
          <div class="end-actions">
            <button class="end-btn" on:click={goBack}>Back to Details</button>
          </div>
        </div>
      </div>

    {:else if readerMode === 'double'}
      <!-- 2 Pages Double Spread Mode -->
      <div
        class="double-page-container"
        style="transform: scale({zoomPercent / 100}); filter: contrast({contrast}%) brightness({brightness}%);"
      >
        <div class="double-page-row" class:single-cover={isCoverSpread}>
          {#each spreadPages as sp, sIdx (sp.image_url)}
            <img
              src={sp.image_url}
              alt="Page {currentPageIndex + sIdx}"
              class="reader-spread-img"
            />
          {/each}
        </div>
      </div>

    {:else}
      <!-- Single Page Mode -->
      <div
        class="single-page-wrapper"
        style="transform: scale({zoomPercent / 100}); filter: contrast({contrast}%) brightness({brightness}%);"
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

  <!-- PROFESSIONAL READER SETTINGS (TAB KEY) -->
  {#if showSettings}
    <div class="settings-drawer" on:click|stopPropagation role="dialog" aria-label="Reader Settings">
      <div class="drawer-header">
        <div class="drawer-title-row">
          <Sliders size={16} class="drawer-icon" />
          <span class="drawer-title">Reader Settings</span>
          <span class="tab-pill">TAB</span>
        </div>
        <button class="drawer-close-btn" on:click={() => showSettings = false}>✕</button>
      </div>

      <!-- VIEW MODE SELECTOR -->
      <div class="drawer-group">
        <span class="drawer-group-label">Reading Mode</span>
        <div class="segmented-control">
          <button
            class="seg-item"
            class:active={readerMode === 'webtoon'}
            on:click={() => changeReaderMode('webtoon')}
          >
            <AlignJustify size={14} />
            <span>Webtoon</span>
          </button>
          <button
            class="seg-item"
            class:active={readerMode === 'single'}
            on:click={() => changeReaderMode('single')}
          >
            <BookOpen size={14} />
            <span>Single</span>
          </button>
          <button
            class="seg-item"
            class:active={readerMode === 'double'}
            on:click={() => changeReaderMode('double')}
          >
            <Columns size={14} />
            <span>2 Pages</span>
          </button>
        </div>
      </div>

      <!-- READING DIRECTION (FOR DOUBLE OR SINGLE) -->
      {#if readerMode !== 'webtoon'}
        <div class="drawer-group">
          <span class="drawer-group-label">Reading Direction</span>
          <div class="segmented-control">
            <button
              class="seg-item"
              class:active={readingDirection === 'RTL'}
              on:click={() => changeDirection('RTL')}
            >
              <span>RTL (Manga)</span>
            </button>
            <button
              class="seg-item"
              class:active={readingDirection === 'LTR'}
              on:click={() => changeDirection('LTR')}
            >
              <span>LTR (Western)</span>
            </button>
          </div>
        </div>
      {/if}

      <!-- WEBTOON SEAM GAP -->
      {#if readerMode === 'webtoon'}
        <div class="drawer-group">
          <span class="drawer-group-label">Page Seam Gap</span>
          <div class="segmented-control">
            <button class="seg-item" class:active={pageGap === 0} on:click={() => pageGap = 0}>
              <span>0px (Seamless)</span>
            </button>
            <button class="seg-item" class:active={pageGap === 4} on:click={() => pageGap = 4}>
              <span>4px</span>
            </button>
            <button class="seg-item" class:active={pageGap === 8} on:click={() => pageGap = 8}>
              <span>8px</span>
            </button>
          </div>
        </div>
      {/if}

      <!-- ZOOM SLIDER -->
      <div class="drawer-group">
        <div class="slider-label-row">
          <span class="drawer-group-label">Zoom Scale</span>
          <span class="slider-val-badge">{zoomPercent}%</span>
        </div>
        <div
          class="custom-track"
          on:mousedown={(e) => startDrag('zoom', 60, 160, 5, e)}
        >
          <div class="track-fill" style="width: {((zoomPercent - 60) / 100) * 100}%"></div>
          <div class="track-thumb" style="left: {((zoomPercent - 60) / 100) * 100}%"></div>
        </div>
      </div>

      <!-- BRIGHTNESS SLIDER -->
      <div class="drawer-group">
        <div class="slider-label-row">
          <span class="drawer-group-label">Brightness</span>
          <span class="slider-val-badge">{brightness}%</span>
        </div>
        <div
          class="custom-track"
          on:mousedown={(e) => startDrag('brightness', 50, 150, 5, e)}
        >
          <div class="track-fill" style="width: {((brightness - 50) / 100) * 100}%"></div>
          <div class="track-thumb" style="left: {((brightness - 50) / 100) * 100}%"></div>
        </div>
      </div>

      <!-- CONTRAST SLIDER -->
      <div class="drawer-group">
        <div class="slider-label-row">
          <span class="drawer-group-label">Contrast</span>
          <span class="slider-val-badge">{contrast}%</span>
        </div>
        <div
          class="custom-track"
          on:mousedown={(e) => startDrag('contrast', 50, 150, 5, e)}
        >
          <div class="track-fill" style="width: {((contrast - 50) / 100) * 100}%"></div>
          <div class="track-thumb" style="left: {((contrast - 50) / 100) * 100}%"></div>
        </div>
      </div>

      <!-- INVERT TOGGLE -->
      <div class="toggle-row">
        <span class="drawer-group-label">Invert Colors (Night Reading)</span>
        <button
          class="switch-btn"
          class:checked={invertColors}
          on:click={() => invertColors = !invertColors}
        >
          <div class="switch-ball"></div>
        </button>
      </div>

      <!-- SHORTCUTS CHEAT SHEET -->
      <div class="shortcut-legend">
        <div class="sc-item"><kbd>←</kbd> <kbd>→</kbd> Flip / Scroll</div>
        <div class="sc-item"><kbd>Tab</kbd> Settings</div>
        <div class="sc-item"><kbd>F</kbd> Fullscreen</div>
        <div class="sc-item"><kbd>Esc</kbd> Exit</div>
      </div>
    </div>
  {/if}

  <!-- BOTTOM HUD BAR -->
  <footer class="hud-bottom" class:visible={showHud} on:click|stopPropagation>
    <div class="hud-bottom-content">
      <button
        class="slider-step-btn"
        disabled={currentPageIndex <= 0}
        on:click={prevStep}
        title="Previous Page (←)"
      >
        <ChevronLeft size={18} />
      </button>

      <div class="bottom-page-indicator">
        <span class="bottom-cur">{currentPageIndex + 1}</span>
        <span class="bottom-sep">/</span>
        <span class="bottom-total">{pages.length}</span>
      </div>

      <button
        class="slider-step-btn"
        disabled={currentPageIndex >= pages.length - 1}
        on:click={nextStep}
        title="Next Page (→)"
      >
        <ChevronRight size={18} />
      </button>
    </div>
  </footer>
</div>

<style>
  .reader-container {
    position: fixed;
    inset: 0;
    background: var(--z-reader-bg, #1c1a24);
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
    background: rgba(35, 33, 44, 0.90);
    backdrop-filter: blur(16px);
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
    gap: var(--z-space-3);
  }

  .hud-btn {
    width: 34px;
    height: 34px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--z-radius-full);
    color: var(--z-text);
    background: transparent;
    border: 1px solid transparent;
    transition: all var(--z-duration-fast) var(--z-ease-out);
    cursor: pointer;
  }

  .hud-btn:hover {
    background: var(--z-surface-raised);
    border-color: var(--z-border);
  }

  .hud-btn.active {
    background: var(--z-accent);
    color: #ffffff;
  }

  .hud-titles {
    display: flex;
    flex-direction: column;
    max-width: 45vw;
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
    gap: var(--z-space-3);
  }

  .page-counter {
    font-size: var(--z-text-xs);
    font-weight: 600;
    color: var(--z-text-muted);
    background: var(--z-surface-raised);
    padding: 3px 10px;
    border-radius: var(--z-radius-full);
  }

  .hud-bottom-content {
    max-width: 420px;
    margin: 0 auto;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--z-space-4);
  }

  .bottom-page-indicator {
    font-size: var(--z-text-sm);
    font-weight: 600;
    color: var(--z-text);
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .bottom-sep {
    color: var(--z-text-muted);
    opacity: 0.6;
  }

  .slider-step-btn {
    width: 34px;
    height: 34px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--z-radius-full);
    color: var(--z-text);
    background: var(--z-surface-raised);
    border: 1px solid var(--z-border);
    transition: all var(--z-duration-fast) var(--z-ease-out);
    cursor: pointer;
  }

  .slider-step-btn:hover:not(:disabled) {
    background: var(--z-border-hover);
    border-color: var(--z-accent);
  }

  .slider-step-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .reader-canvas {
    width: 100vw;
    height: 100vh;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex;
    justify-content: center;
    background: var(--z-reader-bg, #1c1a24);
  }

  .reader-canvas.spread-canvas {
    align-items: center;
  }

  /* WEBTOON STRIP */
  .webtoon-strip {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 100%;
    margin: 0 auto;
    padding-bottom: 90px;
  }

  .reader-page-img {
    width: 100%;
    max-width: 900px;
    height: auto;
    display: block;
    margin: 0 auto;
    user-select: none;
  }

  .chapter-end-card {
    margin-top: 48px;
    padding: 32px 24px;
    background: var(--z-surface-raised);
    border: 1px solid var(--z-border);
    border-radius: var(--z-radius-lg);
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    max-width: 400px;
    width: 90%;
  }

  .end-badge {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.8px;
    color: var(--z-accent);
    background: rgba(66, 133, 244, 0.12);
    padding: 3px 10px;
    border-radius: 9999px;
  }

  .end-title {
    font-size: var(--z-text-sm);
    color: var(--z-text);
    font-weight: 600;
  }

  .end-btn {
    padding: 8px 18px;
    background: var(--z-accent);
    color: #ffffff;
    font-size: var(--z-text-xs);
    font-weight: 600;
    border-radius: var(--z-radius-full);
    border: none;
    cursor: pointer;
    transition: opacity 0.2s;
  }

  .end-btn:hover {
    opacity: 0.9;
  }

  /* 2 PAGES DOUBLE SPREAD */
  .double-page-container {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    width: 100vw;
  }

  .double-page-row {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 98vh;
    gap: 4px;
  }

  .double-page-row.single-cover {
    justify-content: center;
  }

  .reader-spread-img {
    max-height: 96vh;
    max-width: 48vw;
    object-fit: contain;
    user-select: none;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.4);
  }

  /* SINGLE PAGE */
  .single-page-wrapper {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    width: 100vw;
  }

  .reader-single-img {
    max-height: 98vh;
    max-width: 98vw;
    object-fit: contain;
    user-select: none;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.4);
  }

  /* SETTINGS DRAWER */
  .settings-drawer {
    position: fixed;
    top: 64px;
    right: 20px;
    width: 320px;
    background: rgba(35, 33, 44, 0.96);
    backdrop-filter: blur(20px);
    border: 1px solid var(--z-border);
    border-radius: var(--z-radius-lg);
    padding: 16px 18px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.55);
    z-index: 120;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .drawer-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--z-border);
    padding-bottom: 10px;
  }

  .drawer-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  :global(.drawer-icon) {
    color: var(--z-accent);
  }

  .drawer-title {
    font-size: var(--z-text-sm);
    font-weight: 700;
    color: var(--z-text);
  }

  .tab-pill {
    font-size: 9px;
    font-weight: 700;
    background: var(--z-surface-raised);
    color: var(--z-text-muted);
    padding: 2px 6px;
    border-radius: 4px;
    letter-spacing: 0.5px;
  }

  .drawer-close-btn {
    font-size: 13px;
    color: var(--z-text-muted);
    background: transparent;
    border: none;
    cursor: pointer;
  }

  .drawer-close-btn:hover {
    color: var(--z-text);
  }

  .drawer-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .drawer-group-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--z-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.4px;
  }

  .segmented-control {
    display: flex;
    background: var(--z-surface-raised);
    padding: 2px;
    border-radius: var(--z-radius-md);
    border: 1px solid var(--z-border);
    gap: 2px;
  }

  .seg-item {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    padding: 6px 8px;
    border-radius: var(--z-radius-sm);
    font-size: var(--z-text-xs);
    font-weight: 500;
    color: var(--z-text-muted);
    background: transparent;
    border: none;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .seg-item:hover {
    color: var(--z-text);
  }

  .seg-item.active {
    background: var(--z-accent);
    color: #ffffff;
    font-weight: 600;
  }

  .slider-label-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .slider-val-badge {
    font-size: 11px;
    font-weight: 600;
    color: var(--z-text);
    background: var(--z-surface-raised);
    padding: 1px 7px;
    border-radius: 9999px;
  }

  .custom-track {
    position: relative;
    height: 6px;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 9999px;
    cursor: pointer;
    margin: 6px 0;
  }

  .track-fill {
    position: absolute;
    height: 100%;
    background: var(--z-accent);
    border-radius: 9999px;
  }

  .track-thumb {
    position: absolute;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 14px;
    height: 14px;
    background: #ffffff;
    border: 2px solid var(--z-accent);
    border-radius: 50%;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.4);
    cursor: grab;
  }

  .toggle-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 4px;
  }

  .switch-btn {
    width: 36px;
    height: 20px;
    background: var(--z-surface-raised);
    border: 1px solid var(--z-border);
    border-radius: 9999px;
    position: relative;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .switch-btn.checked {
    background: var(--z-accent);
    border-color: var(--z-accent);
  }

  .switch-ball {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    background: #ffffff;
    border-radius: 50%;
    transition: transform 0.2s ease;
  }

  .switch-btn.checked .switch-ball {
    transform: translateX(16px);
  }

  .shortcut-legend {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
    border-top: 1px solid var(--z-border);
    padding-top: 10px;
  }

  .sc-item {
    font-size: 11px;
    color: var(--z-text-muted);
  }

  kbd {
    background: var(--z-surface-raised);
    border: 1px solid var(--z-border);
    border-radius: 4px;
    padding: 1px 4px;
    font-size: 10px;
    font-family: inherit;
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
    max-width: 480px;
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
  }

  .cf-solve-btn, .cf-retry-btn, .cf-back-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    border-radius: var(--z-radius-full);
    font-size: var(--z-text-xs);
    font-weight: 600;
    border: 1px solid var(--z-border);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .cf-solve-btn {
    background: #4f46e5;
    color: #ffffff;
    border-color: #4f46e5;
  }

  .cf-solve-btn:hover {
    background: #4338ca;
  }

  .cf-retry-btn {
    background: var(--z-surface-raised);
    color: var(--z-text);
  }

  .cf-retry-btn:hover {
    background: var(--z-border-hover);
  }

  .cf-back-btn {
    background: transparent;
    color: var(--z-text-muted);
  }

  .cf-back-btn:hover {
    color: var(--z-text);
  }
</style>