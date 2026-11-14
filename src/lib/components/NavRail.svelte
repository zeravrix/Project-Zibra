<script>
  import { 
    History, Heart, Compass, Globe, Rss, Download, Settings,
    PanelLeftClose, PanelLeftOpen 
  } from 'lucide-svelte';
  import { appState, navigate } from '../stores/app.svelte.js';

  const navItems = [
    { id: 'history', label: 'History', icon: History },
    { id: 'favorites', label: 'Favourites', icon: Heart },
    { id: 'explore', label: 'Explore', icon: Compass },
    { id: 'sources', label: 'Sources', icon: Globe },
    { id: 'feed', label: 'Feed', icon: Rss },
    { id: 'downloads', label: 'Downloads', icon: Download },
  ];
</script>

<nav class="nav-rail" class:expanded={appState.railExpanded}>
  <div class="rail-top">
    <div class="logo-area" on:click={() => navigate('explore')}>
      <img src="/logo.png" alt="ZX Logo" class="brand-logo-img" />
      {#if appState.railExpanded}
        <div class="brand-text">
          <span class="logo-title">ZIBRA<span class="logo-x">X</span></span>
          <span class="logo-sub">READER</span>
        </div>
      {/if}
      <button 
        class="toggle-btn" 
        on:click|stopPropagation={() => appState.railExpanded = !appState.railExpanded}
        title={appState.railExpanded ? "Collapse Sidebar" : "Expand Sidebar"}
      >
        {#if appState.railExpanded}
          <PanelLeftClose size={18} strokeWidth={1.8} />
        {:else}
          <PanelLeftOpen size={18} strokeWidth={1.8} />
        {/if}
      </button>
    </div>

    <div class="items-list">
      {#each navItems as item (item.id)}
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

  <div class="rail-bottom">
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
  }

  .nav-rail.expanded {
    width: 210px;
  }

  .rail-top {
    display: flex;
    flex-direction: column;
    gap: var(--z-space-3);
  }

  .logo-area {
    display: flex;
    align-items: center;
    gap: var(--z-space-3);
    padding: var(--z-space-1) var(--z-space-2);
    height: 40px;
  }

  .brand-logo-img {
    width: 32px;
    height: 32px;
    object-fit: contain;
    filter: drop-shadow(0 0 10px rgba(59, 130, 246, 0.5));
    transition: transform 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    cursor: pointer;
    flex-shrink: 0;
  }

  .brand-logo-img:hover {
    transform: scale(1.1);
  }

  .brand-text {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    white-space: nowrap;
    cursor: pointer;
    flex: 1;
  }

  .logo-title {
    font-size: 14px;
    font-weight: 800;
    letter-spacing: 0.8px;
    color: var(--z-text);
    line-height: 1.1;
  }

  .logo-x {
    color: #3b82f6;
  }

  .logo-sub {
    font-size: 8px;
    font-weight: 700;
    letter-spacing: 1.8px;
    color: var(--z-text-muted);
  }

  .toggle-btn {
    width: 32px;
    height: 32px;
    border-radius: var(--z-radius-md);
    color: var(--z-text-muted);
    transition: all var(--z-duration-fast) var(--z-ease-out);
    margin-left: auto;
  }

  .toggle-btn:hover {
    background: var(--z-surface-raised);
    color: var(--z-text);
  }

  .items-list {
    display: flex;
    flex-direction: column;
    gap: var(--z-space-1);
    margin-top: var(--z-space-2);
  }

  .rail-item {
    display: flex;
    align-items: center;
    gap: var(--z-space-3);
    padding: var(--z-space-2) var(--z-space-2);
    border-radius: var(--z-radius-md);
    color: var(--z-text-muted);
    transition: all var(--z-duration-fast) var(--z-ease-out);
    position: relative;
    height: 42px;
  }

  .rail-item:hover {
    background: var(--z-surface-raised);
    color: var(--z-text);
  }

  .rail-item.active {
    background: var(--z-accent-muted);
    color: var(--z-accent);
    font-weight: 600;
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
    padding-top: var(--z-space-2);
    border-top: 1px solid var(--z-border);
  }
</style>
