import { invoke } from '@tauri-apps/api/core';

export const appState = $state({
  currentScreen: 'history',
  previousScreen: 'history',
  railExpanded: false,
  settings: null,
  activeManga: null,
  activeChapter: null,
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

export function openReader(manga, chapter) {
  appState.activeManga = manga;
  appState.activeChapter = chapter;
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

export function applyTheme(settings) {
  if (!settings) return;
  const root = document.documentElement;
  const theme = settings.is_amoled ? 'amoled' : (settings.app_theme?.toLowerCase() || 'dark');
  root.setAttribute('data-theme', theme);
  const accent = settings.color_theme?.toLowerCase() || 'blue';
  root.setAttribute('data-accent', accent);
}
