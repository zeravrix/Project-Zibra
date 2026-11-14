<script>
  import { 
    History, Heart, Compass, Rss, Download, Settings,
    PanelLeftClose, PanelLeftOpen 
  } from 'lucide-svelte';
  import { appState, navigate } from '../stores/app.svelte.js';

  const primaryNav = [
    { id: 'history', label: 'History', icon: History },
    { id: 'favorites', label: 'Favourites', icon: Heart },
    { id: 'explore', label: 'Explore', icon: Compass },
  ];

  const secondaryNav = [
    { id: 'feed', label: 'Feed', icon: Rss },
    { id: 'downloads', label: 'Downloads', icon: Download },
  ];
</script>

<nav class="nav-rail" class:expanded={appState.railExpanded}>
  <div class="rail-top">
    <!-- BRAND LOGO ONLY (NO TITLE TEXT) & SIDEBAR TOGGLE -->
    <div class="rail-header" class:expanded={appState.railExpanded}>
      <button class="logo-btn" on:click={() => navigate('explore')} title="Explore">
        <img src="/logo.png" alt="Logo" class="brand-logo-img" />
      </button>

      <button 
        class="toggle-btn" 
        on:click={() => appState.railExpanded = !appState.railExpanded}
        title={appState.railExpanded ? "Collapse Sidebar" : "Expand Sidebar"}
      >
        {#if appState.railExpanded}
          <PanelLeftClose size={18} strokeWidth={1.8} />
        {:else}
          <PanelLeftOpen size={18} strokeWidth={1.8} />
        {/if}
      </button>
    </div>

    <!-- PRIMARY NAV ITEMS -->
    <div class="items-list">
      {#each primaryNav as item (item.id)}
        <button
          class="rail-item"
          class:active={appState.currentScreen === item.id}
          on:click={() => navigate(item.id)}
          title={!appState.railExpanded ? item.label : undefined}
        >
          <div class="icon-wrap">
            <svelte:component this={item.icon} size={20} strokeWidth={1.8} />
          </div>
          {#if appState.railExpanded}
            <span class="label">{item.label}</span>
          {/if}
        </button>
      {/each}
    </div>
  </div>

  <!-- RAIL BOTTOM: FEED & DOWNLOADS ABOVE THE LINE, SETTINGS BELOW -->
  <div class="rail-bottom">
    <div class="items-list">
      {#each secondaryNav as item (item.id)}
        <button
          class="rail-item"
          class:active={appState.currentScreen === item.id}
          on:click={() => navigate(item.id)}
          title={!appState.railExpanded ? item.label : undefined}
        >
          <div class="icon-wrap">
            <svelte:component this={item.icon} size={20} strokeWidth={1.8} />
          </div>
          {#if appState.railExpanded}
            <span class="label">{item.label}</span>
          {/if}
        </button>
      {/each}
    </div>

    <div class="rail-divider"></div>

    <button
      class="rail-item"
      class:active={appState.currentScreen === 'settings'}
      on:click={() => navigate('settings')}
      title={!appState.railExpanded ? "Settings" : undefined}
    >
      <div class="icon-wrap">
        <Settings size={20} strokeWidth={1.8} />
      </div>
      {#if appState.railExpanded}
        <span class="label">Settings</span>
      {/if}
    </button>
  </div>
</nav>

<style>
  .nav-rail {
    width: 68px;
    height: 100vh;
    background: var(--z-surface);
    border-right: 1px solid var(--z-border);
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: var(--z-space-3) var(--z-space-2);
    transition: width var(--z-duration-normal) var(--z-ease-out);
    z-index: 50;
    flex-shrink: 0;
    box-shadow: 4px 0 20px rgba(0, 0, 0, 0.35);
  }

  .nav-rail.expanded {
    width: 200px;
  }

  .rail-top {
    display: flex;
    flex-direction: column;
    gap: var(--z-space-3);
  }

  .rail-header {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 4px 0;
  }

  .rail-header.expanded {
    flex-direction: row;
    justify-content: space-between;
    align-items: center;
    padding: 4px 8px;
  }

  .logo-btn {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .brand-logo-img {
    width: 32px;
    height: 32px;
    object-fit: contain;
    filter: drop-shadow(0 0 12px rgba(59, 130, 246, 0.5));
    transition: transform 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .brand-logo-img:hover {
    transform: scale(1.1);
  }

  .toggle-btn {
    width: 32px;
    height: 32px;
    border-radius: var(--z-radius-md);
    color: var(--z-text-muted);
    background: transparent;
    border: none;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all var(--z-duration-fast) var(--z-ease-out);
  }

  .toggle-btn:hover {
    background: var(--z-surface-raised);
    color: var(--z-text);
  }

  .items-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .rail-item {
    display: flex;
    align-items: center;
    gap: var(--z-space-3);
    padding: var(--z-space-2);
    border-radius: var(--z-radius-md);
    color: var(--z-text-muted);
    transition: all var(--z-duration-fast) var(--z-ease-out);
    position: relative;
    height: 42px;
    width: 100%;
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
  }

  .rail-item:hover {
    background: var(--z-surface-raised);
    color: var(--z-text);
  }

  .rail-item.active {
    background: var(--z-accent-muted);
    color: var(--z-accent);
    font-weight: 600;
    box-shadow: inset 0 0 10px var(--z-accent-muted);
  }

  .icon-wrap {
    width: 34px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .label {
    font-size: var(--z-text-sm);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .rail-bottom {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .rail-divider {
    height: 1px;
    background: var(--z-border);
    margin: 6px 4px;
  }
</style>