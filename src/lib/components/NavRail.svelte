<script>
  import { 
    History, Heart, Compass, Rss, Download, Settings,
    PanelLeftClose, PanelLeftOpen 
  } from 'lucide-svelte';
  import { appState, navigate } from '../stores/app.svelte.js';

  const navItems = [
    { id: 'history', label: 'History', icon: History },
    { id: 'favorites', label: 'Favourites', icon: Heart },
    { id: 'explore', label: 'Explore', icon: Compass },
    { id: 'feed', label: 'Feed', icon: Rss },
    { id: 'downloads', label: 'Downloads', icon: Download },
  ];
</script>

<nav class="nav-rail" class:expanded={appState.railExpanded}>
  <div class="rail-top">
    <div class="logo-area">
      <button 
        class="toggle-btn" 
        on:click={() => appState.railExpanded = !appState.railExpanded}
        title={appState.railExpanded ? "Collapse Sidebar" : "Expand Sidebar"}
      >
        {#if appState.railExpanded}
          <PanelLeftClose size={20} strokeWidth={1.8} />
        {:else}
          <PanelLeftOpen size={20} strokeWidth={1.8} />
        {/if}
      </button>
      {#if appState.railExpanded}
        <span class="logo-title">ZIBRAX</span>
      {/if}
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

  .toggle-btn {
    width: 36px;
    height: 36px;
    border-radius: var(--z-radius-md);
    color: var(--z-text-muted);
    transition: all var(--z-duration-fast) var(--z-ease-out);
  }

  .toggle-btn:hover {
    background: var(--z-surface-raised);
    color: var(--z-text);
  }

  .logo-title {
    font-size: var(--z-text-base);
    font-weight: 700;
    letter-spacing: 1.5px;
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
