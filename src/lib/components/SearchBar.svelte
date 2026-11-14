<script>
  import { Search, X } from 'lucide-svelte';

  export let value = '';
  export let placeholder = 'Search titles, authors...';
  export let onSearch = null;

  function handleKeydown(e) {
    if (e.key === 'Enter' && onSearch) {
      onSearch(value);
    }
  }

  function handleClear() {
    value = '';
    if (onSearch) {
      onSearch('');
    }
  }
</script>

<div class="search-bar">
  <div class="search-icon">
    <Search size={18} strokeWidth={1.8} />
  </div>
  <input
    type="text"
    bind:value
    {placeholder}
    on:keydown={handleKeydown}
    on:input={() => onSearch && onSearch(value)}
  />
  {#if value}
    <button class="clear-btn" on:click={handleClear} title="Clear search">
      <X size={16} strokeWidth={2} />
    </button>
  {/if}
</div>

<style>
  .search-bar {
    display: flex;
    align-items: center;
    background: var(--z-surface-raised);
    border: 1px solid var(--z-border);
    border-radius: var(--z-radius-md);
    padding: 0 var(--z-space-3);
    height: 40px;
    width: 100%;
    max-width: 480px;
    transition: all var(--z-duration-fast) var(--z-ease-out);
  }

  .search-bar:focus-within {
    border-color: var(--z-accent);
    box-shadow: 0 0 0 2px var(--z-accent-muted);
    background: var(--z-surface);
  }

  .search-icon {
    color: var(--z-text-muted);
    display: flex;
    align-items: center;
    margin-right: var(--z-space-2);
  }

  input {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--z-text);
    font-size: var(--z-text-sm);
  }

  input::placeholder {
    color: var(--z-text-muted);
  }

  .clear-btn {
    color: var(--z-text-muted);
    width: 24px;
    height: 24px;
    border-radius: var(--z-radius-full);
    transition: all var(--z-duration-fast) var(--z-ease-out);
  }

  .clear-btn:hover {
    color: var(--z-text);
    background: var(--z-border);
  }
</style>
