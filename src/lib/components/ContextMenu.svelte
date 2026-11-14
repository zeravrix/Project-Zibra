<script>
  export let x = 0;
  export let y = 0;
  export let items = []; // [{ label, icon, danger, action, divider }]
  export let onClose = () => {};

  function handleKeydown(e) {
    if (e.key === 'Escape') onClose();
  }
</script>

<svelte:window on:click={onClose} on:keydown={handleKeydown} />

<div
  class="context-menu"
  style="top: {Math.max(10, Math.min(y, (typeof window !== 'undefined' ? window.innerHeight : 800) - (items.length * 38 + 24)))}px; left: {Math.max(10, Math.min(x, (typeof window !== 'undefined' ? window.innerWidth : 1200) - 200))}px;"
  on:click|stopPropagation
>
  {#each items as item}
    {#if item.divider}
      <div class="menu-divider"></div>
    {:else}
      <button
        class="menu-item"
        class:danger={item.danger}
        on:click={() => { item.action?.(); onClose(); }}
      >
        {#if item.icon}
          <svelte:component this={item.icon} size={15} class="menu-icon" />
        {/if}
        <span>{item.label}</span>
      </button>
    {/if}
  {/each}
</div>

<style>
  .context-menu {
    position: fixed;
    z-index: 10000;
    min-width: 185px;
    background: rgba(18, 18, 28, 0.95);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 12px;
    padding: 6px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.8), 0 0 1px 1px rgba(255, 255, 255, 0.08);
    display: flex;
    flex-direction: column;
    gap: 2px;
    animation: menuIn 0.14s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes menuIn {
    from { opacity: 0; transform: scale(0.95) translateY(-4px); }
    to { opacity: 1; transform: scale(1) translateY(0); }
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    background: transparent;
    border: none;
    border-radius: 8px;
    color: #e4e4e7;
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
    text-align: left;
    width: 100%;
    transition: all 0.15s ease;
  }

  .menu-item:hover {
    background: rgba(59, 130, 246, 0.22);
    color: #ffffff;
  }

  .menu-item.danger {
    color: #fca5a5;
  }

  .menu-item.danger:hover {
    background: rgba(239, 68, 68, 0.22);
    color: #f87171;
  }

  :global(.menu-icon) {
    flex-shrink: 0;
    opacity: 0.85;
  }

  .menu-divider {
    height: 1px;
    background: rgba(255, 255, 255, 0.08);
    margin: 4px 0;
  }
</style>
