// Pure navigation, chapter ordering, and sequential preloading helpers.

export function pushNavHistory(stack, currentScreen, nextScreen, maxLen = 50) {
  if (currentScreen === nextScreen) {
    return { stack, currentScreen };
  }
  const next = [...stack, currentScreen];
  if (next.length > maxLen) {
    next.splice(0, next.length - maxLen);
  }
  return { stack: next, currentScreen: nextScreen };
}

export function popNavHistory(stack, currentScreen, hasActiveManga = false) {
  if (stack.length > 0) {
    const next = stack.slice(0, -1);
    return { stack: next, currentScreen: stack[stack.length - 1] };
  }
  if (currentScreen === 'reader' && hasActiveManga) {
    return { stack: [], currentScreen: 'details' };
  }
  return { stack: [], currentScreen: 'history' };
}

export function extractChapterNumber(chapter) {
  if (!chapter) return null;
  if (typeof chapter.number === 'number' && Number.isFinite(chapter.number)) {
    return chapter.number;
  }
  const title = String(chapter.title || '');
  const match =
    title.match(/(?:ch(?:apter)?\.?\s*|#)(\d+(?:\.\d+)?)/i) ||
    title.match(/(\d+(?:\.\d+)?)/);
  return match ? parseFloat(match[1]) : null;
}

export function toChronologicalChapters(rawChapters) {
  if (!Array.isArray(rawChapters) || rawChapters.length <= 1) {
    return rawChapters ? [...rawChapters] : [];
  }
  let firstNum = null;
  let lastNum = null;
  for (let i = 0; i < rawChapters.length; i++) {
    const n = extractChapterNumber(rawChapters[i]);
    if (n !== null) {
      firstNum = n;
      break;
    }
  }
  for (let i = rawChapters.length - 1; i >= 0; i--) {
    const n = extractChapterNumber(rawChapters[i]);
    if (n !== null) {
      lastNum = n;
      break;
    }
  }
  if (firstNum !== null && lastNum !== null && firstNum < lastNum) {
    return [...rawChapters];
  }
  return [...rawChapters].reverse();
}

export function clampZoom(value, min = 50, max = 250) {
  return Math.max(min, Math.min(max, Math.round(value)));
}

export function needsNextChapterFetch(totalLoadedPages, currentGlobalIdx, preloadAhead = 3, hasMoreChapters = false) {
  if (!hasMoreChapters || totalLoadedPages <= 0) return false;
  return totalLoadedPages - 1 - currentGlobalIdx < preloadAhead;
}

/**
 * Returns the next page index in [currentGlobalIdx .. currentGlobalIdx + preloadAhead]
 * that should start loading so pages always load sequentially starting from currentGlobalIdx.
 * Returns null if a page in the active window is already loading or all window pages are settled.
 */
export function getNextSequentialPageToLoad(flatPages, pageStates, currentGlobalIdx, preloadAhead = 3) {
  if (!flatPages || flatPages.length === 0) return null;
  const start = Math.max(0, Math.min(flatPages.length - 1, currentGlobalIdx));
  const end = Math.min(flatPages.length - 1, start + preloadAhead);

  for (let i = start; i <= end; i++) {
    const state = pageStates[flatPages[i].key];
    if (state === 'loading') {
      return null;
    }
    if (!state || state === 'idle') {
      return i;
    }
  }
  return null;
}
