<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import CoverGrid from '../components/CoverGrid.svelte';
  import SearchBar from '../components/SearchBar.svelte';
  import { appState, notify } from '../stores/app.svelte.js';
  import { Heart, History, Trash2, Rss, Download } from 'lucide-svelte';

  export let mode = 'history'; // 'history' | 'favorites' | 'feed' | 'downloads'

  let mangas = [];
  let loading = true;
  let searchQuery = '';
  let activeFilter = 'ALL'; // 'ALL' | 'READING' | 'COMPLETED'
  let progressMap = {};
  let favoritesSet = new Set();

  async function loadLibraryData() {
    loading = true;
    try {
      const favs = await invoke('get_favorites');
      favoritesSet = new Set(favs.map(m => m.id));

      if (mode === 'favorites') {
        mangas = favs;
      } else if (mode === 'feed') {
        mangas = favs;
      } else if (mode === 'downloads') {
        try {
          const dls = await invoke('get_downloads');
          mangas = dls.map(d => ({
            id: d.manga_id,
            title: d.manga_title || 'Downloaded Title',
            cover_url: d.cover_url || '',
            source_id: d.source_id,
            url: ''
          }));
        } catch (_) {
          mangas = [];
        }
      } else {
        mangas = await invoke('get_history');
      }

      // Load progress for each manga
      const pMap = {};
      await Promise.all(
        mangas.map(async m => {
          try {
            const p = await invoke('get_reading_progress', { mangaId: m.id });
            if (p && p.total_pages > 0) {
              pMap[m.id] = p.page_index / p.total_pages;
            }
          } catch (_) {}
        })
      );
      progressMap = pMap;
    } catch (e) {
      console.error('Failed to load library:', e);
      notify('Failed to load library data: ' + e);
    } finally {
      loading = false;
    }
  }

  async function handleToggleFavorite(manga) {
    try {
      const isFav = await invoke('toggle_favorite', { manga });
      if (isFav) {
        favoritesSet.add(manga.id);
        notify(`Added "${manga.title}" to favourites`);
      } else {
        favoritesSet.delete(manga.id);
        notify(`Removed "${manga.title}" from favourites`);
        if (mode === 'favorites') {
          mangas = mangas.filter(m => m.id !== manga.id);
        }
      }
      favoritesSet = new Set(favoritesSet);
    } catch (e) {
      notify('Error toggling favourite: ' + e);
    }
  }

  $: filteredMangas = mangas.filter(m => {
    const matchesSearch = !searchQuery || 
      m.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (m.author && m.author.toLowerCase().includes(searchQuery.toLowerCase()));

    const prog = progressMap[m.id] || 0;
    if (activeFilter === 'READING') {
      return matchesSearch && prog > 0 && prog < 0.95;
    } else if (activeFilter === 'COMPLETED') {
      return matchesSearch && prog >= 0.95;
    }
    return matchesSearch;
  });

  onMount(() => {
    loadLibraryData();
  });
</script>

<div class="library-view">
  <header class="header">
    <div class="header-left">
      <h1 class="page-title">
        {#if mode === 'favorites'}
          <Heart size={26} class="title-icon fav" fill="currentColor" />
          Favourites
        {:else if mode === 'feed'}
          <Rss size={26} class="title-icon feed" />
          Feed
        {:else if mode === 'downloads'}
          <Download size={26} class="title-icon dl" />
          Downloads
        {:else}
          <History size={26} class="title-icon hist" />
          History
        {/if}
      </h1>
      <span class="count-badge">{filteredMangas.length} titles</span>
    </div>

    <div class="header-right">
      <SearchBar bind:value={searchQuery} placeholder="Filter your shelf..." />
    </div>
  </header>

  <div class="filter-bar">
    <button 
      class="chip" 
      class:active={activeFilter === 'ALL'} 
      on:click={() => activeFilter = 'ALL'}
    >
      All Titles
    </button>
    <button 
      class="chip" 
      class:active={activeFilter === 'READING'} 
      on:click={() => activeFilter = 'READING'}
    >
      Reading
    </button>
    <button 
      class="chip" 
      class:active={activeFilter === 'COMPLETED'} 
      on:click={() => activeFilter = 'COMPLETED'}
    >
      Completed
    </button>
  </div>

  <div class="content-area">
    {#if loading}
      <div class="loading-state">
        <div class="spinner"></div>
        <p>Loading shelf...</p>
      </div>
    {:else}
      <CoverGrid
        mangas={filteredMangas}
        {progressMap}
        {favoritesSet}
        onToggleFavorite={handleToggleFavorite}
        emptyMessage={
          mode === 'favorites' 
            ? 'No favourite manga yet. Explore to add some!' 
            : mode === 'feed'
            ? 'No updates in your feed yet.'
            : mode === 'downloads'
            ? 'No downloaded chapters found.'
            : 'No reading history yet.'
        }
      />
    {/if}
  </div>
</div>

<style>
  .library-view {
    flex: 1;
    height: 100vh;
    overflow-y: auto;
    padding: var(--z-space-6) var(--z-space-8);
    background: var(--z-bg);
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--z-space-4);
    margin-bottom: var(--z-space-4);
    flex-wrap: wrap;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: var(--z-space-3);
  }

  .page-title {
    font-size: var(--z-text-2xl);
    font-weight: 700;
    color: var(--z-text);
    display: flex;
    align-items: center;
    gap: var(--z-space-3);
  }

  :global(.title-icon.fav) {
    color: #ef4444;
  }

  :global(.title-icon.feed) {
    color: #f59e0b;
  }

  :global(.title-icon.dl) {
    color: #10b981;
  }

  :global(.title-icon.hist) {
    color: var(--z-accent);
  }

  .count-badge {
    background: var(--z-surface-raised);
    padding: 3px 10px;
    border-radius: var(--z-radius-full);
    font-size: var(--z-text-xs);
    color: var(--z-text-muted);
    font-weight: 600;
  }

  .filter-bar {
    display: flex;
    align-items: center;
    gap: var(--z-space-2);
    margin-bottom: var(--z-space-4);
    border-bottom: 1px solid var(--z-border);
    padding-bottom: var(--z-space-3);
  }

  .chip {
    padding: 6px 14px;
    border-radius: var(--z-radius-full);
    font-size: var(--z-text-sm);
    color: var(--z-text-muted);
    background: var(--z-surface-raised);
    transition: all var(--z-duration-fast) var(--z-ease-out);
    font-weight: 500;
  }

  .chip:hover {
    color: var(--z-text);
    background: var(--z-border-hover);
  }

  .chip.active {
    background: var(--z-accent);
    color: #ffffff;
    font-weight: 600;
  }

  .content-area {
    min-height: 400px;
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 300px;
    gap: var(--z-space-3);
    color: var(--z-text-muted);
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--z-border);
    border-top-color: var(--z-accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
