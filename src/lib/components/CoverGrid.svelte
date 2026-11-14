<script>
  import CoverCard from './CoverCard.svelte';
  import { openManga } from '../stores/app.svelte.js';
  import { BookOpen } from 'lucide-svelte';

  export let mangas = [];
  export let progressMap = {};
  export let favoritesSet = new Set();
  export let onToggleFavorite = null;
  export let emptyMessage = "No titles found";
</script>

{#if mangas.length === 0}
  <div class="empty-state">
    <div class="empty-icon">
      <BookOpen size={48} strokeWidth={1.2} />
    </div>
    <p class="empty-text">{emptyMessage}</p>
  </div>
{:else}
  <div class="cover-grid">
    {#each mangas as manga (manga.id)}
      <CoverCard
        {manga}
        progress={progressMap[manga.id] || 0}
        isFavorite={favoritesSet.has(manga.id)}
        onToggleFavorite={onToggleFavorite ? () => onToggleFavorite(manga) : null}
        on:click={() => openManga(manga)}
      />
    {/each}
  </div>
{/if}

<style>
  .cover-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(135px, 1fr));
    gap: var(--z-space-5);
    padding: var(--z-space-4) 0;
    width: 100%;
  }

  @media (min-width: 1400px) {
    .cover-grid {
      grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
      gap: var(--z-space-6);
    }
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 350px;
    gap: var(--z-space-3);
    color: var(--z-text-muted);
  }

  .empty-icon {
    opacity: 0.35;
  }

  .empty-text {
    font-size: var(--z-text-base);
    font-weight: 500;
  }
</style>
