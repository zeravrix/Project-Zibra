import { invoke } from '@tauri-apps/api/core';

export const appState = $state({
  currentScreen: 'history',
  previousScreen: 'history',
  railExpanded: false,
  settings: null,
  activeManga: null,
  activeChapter: null,
  initialPageIndex: 0,
  selectedSourceId: 'MANGADEX',
  notification: null,
});

export function notify(message, duration = 3000) {
  appState.notification = message;
  setTimeout(() => {
    if (appState.notification === message) {
      appState.notification = null;
    }
  }, duration);
}

export function navigate(screen) {
  if (appState.currentScreen !== screen) {
    appState.previousScreen = appState.currentScreen;
    appState.currentScreen = screen;
  }
}

export function openManga(manga) {
  appState.activeManga = manga;
  navigate('details');
}

export function openReader(manga, chapter, page = 0) {
  appState.activeManga = manga;
  appState.activeChapter = chapter;
  appState.initialPageIndex = page;
  navigate('reader');
}

export function goBack() {
  if (appState.currentScreen === 'reader') {
    navigate('details');
  } else if (appState.currentScreen === 'details') {
    navigate(appState.previousScreen || 'history');
  }
}

export async function loadSettings() {
  try {
    const s = await invoke('get_settings');
    appState.settings = s;
    applyTheme(s);
    return s;
  } catch (e) {
    console.error('Failed to load settings:', e);
  }
}

export async function updateSettings(newSettings) {
  try {
    await invoke('save_settings', { settings: newSettings });
    appState.settings = newSettings;
    applyTheme(newSettings);
  } catch (e) {
    console.error('Failed to save settings:', e);
  }
}

const PALETTE_COLORS = {
  totoro: '#4285f4',
  monet: '#38bdf8',
  expressive: '#c084fc',
  miku: '#06b6d4',
  asuka: '#f87171',
  mion: '#34d399',
  rikka: '#a78bfa',
  sakura: '#fb7185',
  mamimi: '#60a5fa',
  kanade: '#cbd5e1',
  itsuka: '#fbbf24',
  amoledvoid: '#3b82f6',
  highcontrast: '#ffffff',
  blue: '#4285f4',
  purple: '#a855f7',
  teal: '#14b8a6',
  green: '#22c55e',
  red: '#ef4444',
  orange: '#f97316',
  pink: '#ec4899',
  yellow: '#eab308',
};

export function applyTheme(settings) {
  if (!settings) return;
  const root = document.documentElement;
  const theme = settings.is_amoled ? 'amoled' : (settings.theme?.toLowerCase() || settings.app_theme?.toLowerCase() || 'dark');
  root.setAttribute('data-theme', theme);
  const themeKey = String(settings.color_theme || 'totoro').toLowerCase();
  const hex = PALETTE_COLORS[themeKey] || (themeKey.startsWith('#') ? themeKey : '#4285f4');
  root.setAttribute('data-accent', themeKey);
  root.style.setProperty('--z-accent', hex);
  root.style.setProperty('--z-accent-glow', hex + '55');
  root.style.setProperty('--z-accent-muted', hex + '22');
}
