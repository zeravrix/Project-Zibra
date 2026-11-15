<script>
  import CoverCard from './CoverCard.svelte';
  import EmptyState from './EmptyState.svelte';
  import { openManga, navigate } from '../stores/app.svelte.js';

  export let mangas = [];
  export let progressMap = {};
  export let favoritesSet = new Set();
  export let onToggleFavorite = null;
  export let onDelete = null;
  export let onSetStatus = null;
  export let emptyMessage = "No titles found";
  export let emptyActionLabel = "Explore Manga";
  export let onEmptyAction = () => navigate('explore');
</script>

{#if mangas.length === 0}
  <EmptyState
    title="Nothing here yet..."
    description={emptyMessage}
    actionLabel={emptyActionLabel}
    onAction={onEmptyAction}
  />
{:else}
  <div class="cover-grid">
    {#each mangas as manga (manga.id)}
      <CoverCard
        {manga}
        progress={progressMap[manga.id] || 0}
        isFavorite={favoritesSet.has(manga.id)}
        onToggleFavorite={onToggleFavorite ? () => onToggleFavorite(manga) : null}
        onDelete={onDelete ? () => onDelete(manga) : null}
        onSetStatus={onSetStatus ? (status) => onSetStatus(manga, status) : null}
        on:click={() => openManga(manga)}
      />
    {/each}
  </div>
{/if}

<style>
  .cover-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(145px, 185px));
    justify-content: start;
    gap: var(--z-space-5);
    padding: var(--z-space-4) 0;
    width: 100%;
  }

  @media (min-width: 1400px) {
    .cover-grid {
      grid-template-columns: repeat(auto-fill, minmax(155px, 195px));
      gap: var(--z-space-6);
    }
  }
</style>
