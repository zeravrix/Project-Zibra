import test from 'node:test';
import assert from 'node:assert/strict';
import {
  pushNavHistory,
  popNavHistory,
  toChronologicalChapters,
  clampZoom,
  needsNextChapterFetch,
  getNextSequentialPageToLoad,
} from './readerNav.js';

test('navigation history pops sequentially without ping-ponging', () => {
  let nav = { stack: [], currentScreen: 'history' };
  nav = pushNavHistory(nav.stack, nav.currentScreen, 'explore');
  nav = pushNavHistory(nav.stack, nav.currentScreen, 'details');
  nav = pushNavHistory(nav.stack, nav.currentScreen, 'reader');

  assert.equal(nav.currentScreen, 'reader');
  assert.deepEqual(nav.stack, ['history', 'explore', 'details']);

  // 1st Back -> details
  nav = popNavHistory(nav.stack, nav.currentScreen, true);
  assert.equal(nav.currentScreen, 'details');
  assert.deepEqual(nav.stack, ['history', 'explore']);

  // 2nd Back -> explore (NOT reader!)
  nav = popNavHistory(nav.stack, nav.currentScreen, true);
  assert.equal(nav.currentScreen, 'explore');
  assert.deepEqual(nav.stack, ['history']);

  // 3rd Back -> history
  nav = popNavHistory(nav.stack, nav.currentScreen, true);
  assert.equal(nav.currentScreen, 'history');
  assert.deepEqual(nav.stack, []);

  // 4th Back on empty stack -> stays on history
  nav = popNavHistory(nav.stack, nav.currentScreen, true);
  assert.equal(nav.currentScreen, 'history');
});

test('toChronologicalChapters normalizes descending and ascending lists', () => {
  const desc = [
    { id: 'c3', title: 'Chapter 3', number: 3 },
    { id: 'c2', title: 'Chapter 2', number: 2 },
    { id: 'c1', title: 'Chapter 1', number: 1 },
  ];
  assert.deepEqual(toChronologicalChapters(desc).map(c => c.id), ['c1', 'c2', 'c3']);

  const asc = [
    { id: 'c1', title: 'Chapter 1', number: 1 },
    { id: 'c2', title: 'Chapter 2', number: 2 },
    { id: 'c3', title: 'Chapter 3', number: 3 },
  ];
  assert.deepEqual(toChronologicalChapters(asc).map(c => c.id), ['c1', 'c2', 'c3']);

  const mixedTextDesc = [
    { id: 'notice', title: 'Hiatus Notice', number: null },
    { id: 'c2', title: 'Ch. 2 - Battle', number: null },
    { id: 'c1', title: 'Ch. 1 - Start', number: null },
    { id: 'pro', title: 'Prologue', number: null },
  ];
  assert.deepEqual(toChronologicalChapters(mixedTextDesc).map(c => c.id), ['pro', 'c1', 'c2', 'notice']);
});

test('sequential page loader loads strictly in forward order up to +3 pages across chapters', () => {
  const flatPages = [
    { key: 'ch1:0' },
    { key: 'ch1:1' },
    { key: 'ch1:2' },
    { key: 'ch2:0' },
    { key: 'ch2:1' },
    { key: 'ch2:2' },
  ];
  const states = {};

  // Starts strictly at index 0
  assert.equal(getNextSequentialPageToLoad(flatPages, states, 0, 3), 0);
  states['ch1:0'] = 'loading';
  // Waits while index 0 is loading
  assert.equal(getNextSequentialPageToLoad(flatPages, states, 0, 3), null);

  // Index 0 finishes -> moves strictly to 1, then 2, then 3 (ch2:0 across chapter boundary!)
  states['ch1:0'] = 'loaded';
  assert.equal(getNextSequentialPageToLoad(flatPages, states, 0, 3), 1);
  states['ch1:1'] = 'loaded';
  assert.equal(getNextSequentialPageToLoad(flatPages, states, 0, 3), 2);
  states['ch1:2'] = 'loaded';
  assert.equal(getNextSequentialPageToLoad(flatPages, states, 0, 3), 3);
  states['ch2:0'] = 'loaded';

  // Window [0..3] complete -> stops until user advances
  assert.equal(getNextSequentialPageToLoad(flatPages, states, 0, 3), null);

  // User moves to index 1 -> preloads index 4 (ch2:1)
  assert.equal(getNextSequentialPageToLoad(flatPages, states, 1, 3), 4);
  states['ch2:1'] = 'error'; // error must not stall queue
  assert.equal(getNextSequentialPageToLoad(flatPages, states, 2, 3), 5);
});

test('needsNextChapterFetch triggers within 3 pages of end', () => {
  assert.equal(needsNextChapterFetch(10, 5, 3, true), false);  // remaining = 4
  assert.equal(needsNextChapterFetch(10, 6, 3, true), false);  // remaining = 3 (pages 7,8,9 already loaded)
  assert.equal(needsNextChapterFetch(10, 7, 3, true), true);   // remaining = 2 < 3 -> fetch next chapter!
  assert.equal(needsNextChapterFetch(10, 9, 3, true), true);   // last page -> fetch next chapter!
  assert.equal(needsNextChapterFetch(10, 9, 3, false), false); // latest chapter -> no more
  assert.equal(needsNextChapterFetch(2, 0, 3, true), true);    // short 2-page chapter -> fetch immediately
});

test('clampZoom bounds properly', () => {
  assert.equal(clampZoom(20), 50);
  assert.equal(clampZoom(135), 135);
  assert.equal(clampZoom(400), 250);
});
