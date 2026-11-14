<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import CoverGrid from '../components/CoverGrid.svelte';
  import SearchBar from '../components/SearchBar.svelte';
  import { appState, notify } from '../stores/app.svelte.js';
  import { Compass, Globe, Sparkles, Flame, Clock, Filter, ShieldAlert } from 'lucide-svelte';

  async function handleSolveCloudflare() {
    if (!selectedSource) return;
    try {
      await invoke('open_cloudflare_solver', {
        url: selectedSource.base_url,
        title: `${selectedSource.name} — Cloudflare Verification`,
      });
      notify(`Opening Cloudflare solver for ${selectedSource.name}...`);
    } catch (e) {
      notify('Failed to open solver: ' + e);
    }
  }

  let sources = [];
  let selectedSource = null;
  let mangaList = [];
  let loading = false;
  let loadingSources = false;
  let searchQuery = '';
  let activeTab = 'popular'; // 'popular' | 'latest'
  let currentPage = 1;
  let hasNextPage = true;
  let selectedLang = 'ALL';
  let favoritesSet = new Set();
  let searchMode = 'browse'; // 'browse' | 'global'

  const languages = ['ALL', 'en', 'ar', 'es', 'fr', 'id', 'ru', 'pt', 'ja', 'ko'];

  async function loadSources() {
    loadingSources = true;
    try {
      sources = await invoke('get_sources', {
        langFilter: selectedLang,
        query: '',
        nsfwEnabled: appState.settings?.nsfw_enabled ?? false,
      });

      if (!selectedSource && sources.length > 0) {
        selectedSource = sources.find(s => s.id === 'MANGADEX') || sources[0];
      }
    } catch (e) {
      console.error('Failed to load sources:', e);
      notify('Failed to load sources: ' + e);
    } finally {
      loadingSources = false;
    }
  }

  async function loadFavorites() {
    try {
      const favs = await invoke('get_favorites');
      favoritesSet = new Set(favs.map(m => m.id));
    } catch (_) {}
  }

  async function loadMangas(page = 1, append = false) {
    if (!selectedSource && searchMode === 'browse') return;
    loading = true;

    try {
      if (searchMode === 'global' && searchQuery.trim()) {
        const results = await invoke('search_global', {
          query: searchQuery.trim(),
          nsfwEnabled: appState.settings?.nsfw_enabled ?? false,
        });
        mangaList = results;
        hasNextPage = false;
      } else {
        const res = await invoke('fetch_source_page', {
          sourceId: selectedSource.id,
          page,
          latest: activeTab === 'latest',
          query: searchQuery.trim() ? searchQuery.trim() : null,
          nsfwEnabled: appState.settings?.nsfw_enabled ?? false,
        });

        if (append) {
          mangaList = [...mangaList, ...res.items];
        } else {
          mangaList = res.items;
        }
        currentPage = page;
        hasNextPage = res.has_next_page;
      }
    } catch (e) {
      console.error('Failed to load manga page:', e);
      notify('Error fetching from source: ' + e);
    } finally {
      loading = false;
    }
  }

  function handleSourceSelect(source) {
    selectedSource = source;
    appState.selectedSourceId = source.id;
    searchMode = 'browse';
    searchQuery = '';
    loadMangas(1, false);
  }

  function handleSearch(query) {
    searchQuery = query;
    if (query.trim()) {
      loadMangas(1, false);
    } else {
      searchMode = 'browse';
      loadMangas(1, false);
    }
  }

  function handleTabChange(tab) {
    activeTab = tab;
    searchQuery = '';
    loadMangas(1, false);
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
      }
      favoritesSet = new Set(favoritesSet);
    } catch (e) {
      notify('Error toggling favourite: ' + e);
    }
  }

  onMount(async () => {
    await loadFavorites();
    await loadSources();
    if (selectedSource) {
      loadMangas(1, false);
    }
  });
</script>

<div class="explore-view">
  <header class="header">
    <div class="header-left">
      <h1 class="page-title">
        <Compass size={26} class="title-icon" />
        Explore
      </h1>
      <span class="source-count">{sources.length} Pure Rust Sources</span>
    </div>

    <div class="header-right">
      <div class="search-mode-toggles">
        <button
          class="mode-btn"
          class:active={searchMode === 'browse'}
          on:click={() => { searchMode = 'browse'; loadMangas(1, false); }}
        >
          Source Browse
        </button>
        <button
          class="mode-btn"
          class:active={searchMode === 'global'}
          on:click={() => { searchMode = 'global'; }}
        >
          Global Search
        </button>
      </div>

      <SearchBar
        bind:value={searchQuery}
        placeholder={searchMode === 'global' ? 'Search all 1,190+ sources...' : `Search in ${selectedSource?.name || 'source'}...`}
        onSearch={handleSearch}
      />
    </div>
  </header>

  <!-- Sources & Language Filter Bar -->
  <div class="sources-strip">
    <div class="lang-selector">
      <Globe size={16} />
      <select bind:value={selectedLang} on:change={() => loadSources()}>
        {#each languages as lang}
          <option value={lang}>{lang.toUpperCase()}</option>
        {/each}
      </select>
    </div>

    <div class="source-chips">
      {#each sources.slice(0, 20) as s (s.id)}
        <button
          class="source-chip"
          class:active={selectedSource?.id === s.id && searchMode === 'browse'}
          on:click={() => handleSourceSelect(s)}
          title="{s.name} ({s.language}) - {s.base_url}"
        >
          {#if s.icon_url}
            <img src={s.icon_url} alt="" class="chip-icon" on:error={(e) => e.target.style.display = 'none'} />
          {/if}
          <span>{s.name}</span>
        </button>
      {/each}
    </div>
  </div>

  {#if searchMode === 'browse'}
    <div class="tabs-bar">
      <button
        class="tab-btn"
        class:active={activeTab === 'popular'}
        on:click={() => handleTabChange('popular')}
      >
        <Flame size={16} />
        Popular
      </button>
      <button
        class="tab-btn"
        class:active={activeTab === 'latest'}
        on:click={() => handleTabChange('latest')}
      >
        <Clock size={16} />
        Latest Updates
      </button>
      {#if selectedSource}
        <button
          class="tab-btn cloudflare-btn"
          title="Open in-app WebView to solve Cloudflare challenge"
          on:click={handleSolveCloudflare}
        >
          <ShieldAlert size={16} />
          Pass Cloudflare
        </button>
      {/if}
    </div>
  {/if}

  <div class="content-area">
    {#if loading}
      <div class="loading-state">
        <div class="spinner"></div>
        <p>Fetching titles from {selectedSource?.name || 'sources'}...</p>
      </div>
    {:else}
      <CoverGrid
        mangas={mangaList}
        {favoritesSet}
        onToggleFavorite={handleToggleFavorite}
        emptyMessage={searchQuery ? 'No manga matching your search.' : 'No titles available.'}
      />

      {#if hasNextPage && !searchQuery && mangaList.length > 0}
        <div class="load-more">
          <button class="load-btn" on:click={() => loadMangas(currentPage + 1, true)}>
            Load More Titles
          </button>
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .explore-view {
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

  :global(.title-icon) {
    color: var(--z-accent);
  }

  .source-count {
    background: var(--z-surface-raised);
    padding: 3px 10px;
    border-radius: var(--z-radius-full);
    font-size: var(--z-text-xs);
    color: var(--z-accent);
    font-weight: 600;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: var(--z-space-3);
    flex-wrap: wrap;
  }

  .search-mode-toggles {
    display: flex;
    background: var(--z-surface-raised);
    padding: 3px;
    border-radius: var(--z-radius-md);
    border: 1px solid var(--z-border);
  }

  .mode-btn {
    padding: 5px 12px;
    border-radius: var(--z-radius-sm);
    font-size: var(--z-text-xs);
    font-weight: 600;
    color: var(--z-text-muted);
    transition: all var(--z-duration-fast) var(--z-ease-out);
  }

  .mode-btn.active {
    background: var(--z-surface);
    color: var(--z-accent);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.2);
  }

  .sources-strip {
    display: flex;
    align-items: center;
    gap: var(--z-space-3);
    margin-bottom: var(--z-space-4);
    overflow-x: auto;
    padding-bottom: var(--z-space-2);
  }

  .lang-selector {
    display: flex;
    align-items: center;
    gap: var(--z-space-2);
    background: var(--z-surface-raised);
    border: 1px solid var(--z-border);
    padding: 6px 12px;
    border-radius: var(--z-radius-full);
    color: var(--z-text-muted);
    flex-shrink: 0;
  }

  .lang-selector select {
    background: transparent;
    border: none;
    color: var(--z-text);
    font-size: var(--z-text-xs);
    font-weight: 600;
    cursor: pointer;
  }

  .source-chips {
    display: flex;
    align-items: center;
    gap: var(--z-space-2);
    overflow-x: auto;
    white-space: nowrap;
  }

  .source-chip {
    display: flex;
    align-items: center;
    gap: var(--z-space-2);
    padding: 6px 12px;
    background: var(--z-surface-raised);
    border: 1px solid var(--z-border);
    border-radius: var(--z-radius-full);
    font-size: var(--z-text-xs);
    font-weight: 500;
    color: var(--z-text-muted);
    transition: all var(--z-duration-fast) var(--z-ease-out);
    flex-shrink: 0;
  }

  .source-chip:hover {
    color: var(--z-text);
    border-color: var(--z-border-hover);
  }

  .source-chip.active {
    background: var(--z-accent);
    border-color: var(--z-accent);
    color: #ffffff;
    font-weight: 600;
  }

  .chip-icon {
    width: 14px;
    height: 14px;
    border-radius: 2px;
  }

  .tabs-bar {
    display: flex;
    align-items: center;
    gap: var(--z-space-3);
    margin-bottom: var(--z-space-4);
    border-bottom: 1px solid var(--z-border);
    padding-bottom: var(--z-space-3);
  }

  .tab-btn {
    display: flex;
    align-items: center;
    gap: var(--z-space-2);
    padding: 6px 14px;
    border-radius: var(--z-radius-full);
    font-size: var(--z-text-sm);
    color: var(--z-text-muted);
    transition: all var(--z-duration-fast) var(--z-ease-out);
    font-weight: 500;
  }

  .tab-btn:hover {
    color: var(--z-text);
    background: var(--z-surface-raised);
  }

  .tab-btn.active {
    background: var(--z-accent-muted);
    color: var(--z-accent);
    font-weight: 600;
  }

  .content-area {
    min-height: 400px;
  }

  .load-more {
    display: flex;
    justify-content: center;
    padding: var(--z-space-6) 0;
  }

  .load-btn {
    padding: 10px 24px;
    background: var(--z-surface-raised);
    border: 1px solid var(--z-border);
    border-radius: var(--z-radius-md);
    color: var(--z-text);
    font-size: var(--z-text-sm);
    font-weight: 600;
    transition: all var(--z-duration-fast) var(--z-ease-out);
  }

  .load-btn:hover {
    background: var(--z-accent);
    color: #ffffff;
    border-color: var(--z-accent);
    transform: translateY(-2px);
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
