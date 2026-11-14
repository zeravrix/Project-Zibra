<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { appState, goBack, openReader, notify } from '../stores/app.svelte.js';
  import { 
    ArrowLeft, Heart, Play, Download, BookOpen, 
    Calendar, User, Tag, Check, Sparkles, ShieldAlert 
  } from 'lucide-svelte';

  let manga = appState.activeManga;
  let chapters = [];
  let loading = true;
  let isFavorite = false;
  let progress = null;
  let sortAscending = false;
  let chapterFilter = '';

  async function loadDetails() {
    if (!manga) return;
    loading = true;
    try {
      isFavorite = await invoke('is_favorite', { mangaId: manga.id });
      progress = await invoke('get_reading_progress', { mangaId: manga.id });

      // Fetch enriched details from source
      try {
        const enriched = await invoke('get_manga_details', {
          sourceId: manga.source_id,
          manga,
        });
        manga = enriched;
        appState.activeManga = enriched;
      } catch (e) {
        console.warn('Enriched details fetch note:', e);
      }

      // Fetch chapters
      chapters = await invoke('get_chapters', {
        sourceId: manga.source_id,
        manga,
      });

      // Record in history
      invoke('record_history', { manga });
    } catch (e) {
      console.error('Failed to load manga details:', e);
      notify('Failed to load details: ' + e);
    } finally {
      loading = false;
    }
  }

  async function handleSolveCloudflare() {
    if (!manga) return;
    try {
      const targetUrl = manga.url || 'https://google.com';
      await invoke('open_cloudflare_solver', {
        url: targetUrl,
        title: `${manga.title} — Cloudflare Verification`,
      });
      notify('Opening Cloudflare solver in webview...');
    } catch (e) {
      notify('Failed to open solver: ' + e);
    }
  }

  async function handleToggleFavorite() {
    try {
      isFavorite = await invoke('toggle_favorite', { manga });
      notify(isFavorite ? 'Added to favourites' : 'Removed from favourites');
    } catch (e) {
      notify('Error updating favourite: ' + e);
    }
  }

  function handleStartReading() {
    if (chapters.length === 0) return;
    // If progress exists, resume last chapter; otherwise read first chapter
    let target = chapters[chapters.length - 1]; // First chronological chapter if sorted desc
    if (progress && progress.chapter_id) {
      const found = chapters.find(c => c.id === progress.chapter_id);
      if (found) target = found;
    }
    openReader(manga, target);
  }

  $: displayedChapters = [...chapters]
    .sort((a, b) => {
      const numA = a.number ?? 0;
      const numB = b.number ?? 0;
      return sortAscending ? numA - numB : numB - numA;
    })
    .filter(c => !chapterFilter || c.title.toLowerCase().includes(chapterFilter.toLowerCase()));

  onMount(() => {
    loadDetails();
  });
</script>

{#if manga}
  <div class="details-view">
    <!-- Top Bar -->
    <div class="top-nav">
      <button class="nav-btn" on:click={goBack} title="Back">
        <ArrowLeft size={20} strokeWidth={2} />
      </button>
      <span class="nav-title">{manga.title}</span>
      <button class="nav-btn" class:active={isFavorite} on:click={handleToggleFavorite} title="Favourite">
        <Heart size={20} fill={isFavorite ? "currentColor" : "none"} strokeWidth={2} />
      </button>
    </div>

    <!-- Hero Backdrop Banner -->
    <div class="hero-section">
      {#if manga.cover_url}
        <div class="backdrop" style:background-image="url('{manga.cover_url}')" />
      {/if}
      <div class="backdrop-overlay" />

      <div class="hero-content">
        <div class="cover-card">
          {#if manga.cover_url}
            <img src={manga.cover_url} alt={manga.title} />
          {:else}
            <div class="placeholder"><BookOpen size={40} /></div>
          {/if}
        </div>

        <div class="hero-info">
          <div class="tags-row">
            {#if manga.is_manhwa}
              <span class="badge manhwa">Manhwa</span>
            {/if}
            {#if manga.status}
              <span class="badge status">{manga.status}</span>
            {/if}
            <span class="badge source">{manga.source_id.replace(/^[^:]+:/, '')}</span>
          </div>

          <h1 class="manga-title">{manga.title}</h1>

          <div class="meta-row">
            {#if manga.author}
              <span class="meta-item"><User size={15} /> {manga.author}</span>
            {/if}
            <span class="meta-item"><BookOpen size={15} /> {chapters.length} Chapters</span>
          </div>

          <div class="genres-row">
            {#each (manga.genres || []) as genre}
              <span class="genre-pill">{genre}</span>
            {/each}
          </div>

          <div class="actions-row">
            <button class="primary-btn" on:click={handleStartReading} disabled={chapters.length === 0}>
              <Play size={18} fill="currentColor" />
              <span>{progress ? 'Continue Reading' : 'Start Reading'}</span>
            </button>
            <button class="icon-action-btn" class:fav={isFavorite} on:click={handleToggleFavorite}>
              <Heart size={18} fill={isFavorite ? "currentColor" : "none"} />
            </button>
            <button class="icon-action-btn" title="Pass Cloudflare Challenge" on:click={handleSolveCloudflare}>
              <ShieldAlert size={18} />
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Description & Chapters -->
    <div class="body-section">
      {#if manga.description}
        <div class="description-box">
          <h3 class="section-title">Synopsis</h3>
          <p class="description-text">{manga.description}</p>
        </div>
      {/if}

      <div class="chapters-header">
        <h3 class="section-title">Chapters ({displayedChapters.length})</h3>
        <div class="chapter-controls">
          <button class="sort-toggle" on:click={() => sortAscending = !sortAscending}>
            {sortAscending ? 'Oldest First' : 'Newest First'}
          </button>
        </div>
      </div>

      {#if loading}
        <div class="loading-chapters">
          <div class="spinner" />
          <p>Fetching chapter list...</p>
        </div>
      {:else}
        <div class="chapters-list">
          {#each displayedChapters as chapter (chapter.id)}
            <button
              class="chapter-row"
              class:current={progress?.chapter_id === chapter.id}
              on:click={() => openReader(manga, chapter)}
            >
              <div class="chapter-info">
                <span class="chapter-name">{chapter.title}</span>
                {#if chapter.upload_date}
                  <span class="chapter-date">
                    {new Date(chapter.upload_date).toLocaleDateString()}
                  </span>
                {/if}
              </div>
              <div class="chapter-actions">
                {#if progress?.chapter_id === chapter.id}
                  <span class="reading-badge">Reading</span>
                {/if}
                <Play size={16} class="play-icon" />
              </div>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .details-view {
    flex: 1;
    height: 100vh;
    overflow-y: auto;
    background: var(--z-bg);
  }

  .top-nav {
    position: sticky;
    top: 0;
    left: 0;
    right: 0;
    height: 56px;
    background: rgba(15, 19, 26, 0.75);
    backdrop-filter: blur(16px);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--z-space-4);
    z-index: 40;
    border-bottom: 1px solid var(--z-border);
  }

  .nav-btn {
    width: 38px;
    height: 38px;
    border-radius: var(--z-radius-full);
    color: var(--z-text);
    transition: all var(--z-duration-fast) var(--z-ease-out);
  }

  .nav-btn:hover {
    background: var(--z-surface-raised);
  }

  .nav-btn.active {
    color: #ef4444;
  }

  .nav-title {
    font-size: var(--z-text-sm);
    font-weight: 600;
    color: var(--z-text);
    max-width: 60%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .hero-section {
    position: relative;
    padding: var(--z-space-8) var(--z-space-8) var(--z-space-6);
    overflow: hidden;
  }

  .backdrop {
    position: absolute;
    inset: 0;
    background-size: cover;
    background-position: center;
    filter: blur(35px) brightness(0.25);
    transform: scale(1.1);
  }

  .backdrop-overlay {
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg, rgba(15, 19, 26, 0.4) 0%, var(--z-bg) 100%);
  }

  .hero-content {
    position: relative;
    display: flex;
    gap: var(--z-space-6);
    max-width: 1100px;
    margin: 0 auto;
  }

  .cover-card {
    width: 220px;
    aspect-ratio: 13 / 18;
    border-radius: var(--z-radius-lg);
    overflow: hidden;
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.6);
    flex-shrink: 0;
    background: var(--z-surface-raised);
    border: 1px solid var(--z-border);
  }

  .cover-card img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .hero-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--z-space-3);
  }

  .tags-row {
    display: flex;
    gap: var(--z-space-2);
  }

  .badge {
    padding: 3px 10px;
    border-radius: var(--z-radius-full);
    font-size: var(--z-text-xs);
    font-weight: 600;
    text-transform: uppercase;
  }

  .badge.manhwa {
    background: var(--z-accent-muted);
    color: var(--z-accent);
  }

  .badge.status {
    background: rgba(34, 197, 94, 0.15);
    color: #22c55e;
  }

  .badge.source {
    background: var(--z-surface-raised);
    color: var(--z-text-muted);
  }

  .manga-title {
    font-size: var(--z-text-2xl);
    font-weight: 800;
    color: var(--z-text);
    line-height: 1.25;
  }

  .meta-row {
    display: flex;
    align-items: center;
    gap: var(--z-space-4);
    color: var(--z-text-muted);
    font-size: var(--z-text-sm);
  }

  .meta-item {
    display: flex;
    align-items: center;
    gap: var(--z-space-1);
  }

  .genres-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--z-space-2);
    margin: var(--z-space-1) 0;
  }

  .genre-pill {
    padding: 4px 12px;
    background: var(--z-surface-raised);
    border: 1px solid var(--z-border);
    border-radius: var(--z-radius-full);
    font-size: var(--z-text-xs);
    color: var(--z-text-muted);
  }

  .actions-row {
    display: flex;
    align-items: center;
    gap: var(--z-space-3);
    margin-top: var(--z-space-2);
  }

  .primary-btn {
    display: flex;
    align-items: center;
    gap: var(--z-space-2);
    padding: 12px 28px;
    background: var(--z-accent);
    color: #ffffff;
    font-weight: 600;
    border-radius: var(--z-radius-md);
    box-shadow: 0 4px 16px var(--z-accent-glow);
    transition: all var(--z-duration-fast) var(--z-ease-out);
  }

  .primary-btn:hover:not(:disabled) {
    transform: translateY(-2px);
    box-shadow: 0 6px 20px var(--z-accent-glow);
  }

  .icon-action-btn {
    width: 44px;
    height: 44px;
    border-radius: var(--z-radius-md);
    background: var(--z-surface-raised);
    border: 1px solid var(--z-border);
    color: var(--z-text);
    transition: all var(--z-duration-fast) var(--z-ease-out);
  }

  .icon-action-btn.fav {
    color: #ef4444;
  }

  .icon-action-btn:hover {
    background: var(--z-border-hover);
  }

  .body-section {
    max-width: 1100px;
    margin: 0 auto;
    padding: var(--z-space-6) var(--z-space-8) var(--z-space-8);
  }

  .description-box {
    margin-bottom: var(--z-space-6);
  }

  .section-title {
    font-size: var(--z-text-lg);
    font-weight: 700;
    margin-bottom: var(--z-space-3);
  }

  .description-text {
    font-size: var(--z-text-sm);
    color: var(--z-text-muted);
    line-height: 1.7;
    white-space: pre-line;
  }

  .chapters-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--z-space-3);
    border-bottom: 1px solid var(--z-border);
    padding-bottom: var(--z-space-2);
  }

  .sort-toggle {
    font-size: var(--z-text-xs);
    color: var(--z-accent);
    font-weight: 600;
    padding: 4px 8px;
    border-radius: var(--z-radius-sm);
  }

  .chapters-list {
    display: flex;
    flex-direction: column;
    gap: var(--z-space-2);
  }

  .chapter-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--z-space-3) var(--z-space-4);
    background: var(--z-surface);
    border: 1px solid var(--z-border);
    border-radius: var(--z-radius-md);
    transition: all var(--z-duration-fast) var(--z-ease-out);
  }

  .chapter-row:hover {
    background: var(--z-surface-raised);
    border-color: var(--z-border-hover);
    transform: translateX(4px);
  }

  .chapter-row.current {
    border-color: var(--z-accent);
    background: var(--z-accent-muted);
  }

  .chapter-info {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 3px;
  }

  .chapter-name {
    font-size: var(--z-text-sm);
    font-weight: 500;
    color: var(--z-text);
  }

  .chapter-date {
    font-size: var(--z-text-xs);
    color: var(--z-text-muted);
  }

  .chapter-actions {
    display: flex;
    align-items: center;
    gap: var(--z-space-3);
    color: var(--z-text-muted);
  }

  .reading-badge {
    font-size: var(--z-text-xs);
    background: var(--z-accent);
    color: #ffffff;
    padding: 2px 8px;
    border-radius: var(--z-radius-full);
    font-weight: 600;
  }

  .loading-chapters {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 200px;
    gap: var(--z-space-3);
    color: var(--z-text-muted);
  }

  .spinner {
    width: 28px;
    height: 28px;
    border: 3px solid var(--z-border);
    border-top-color: var(--z-accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
