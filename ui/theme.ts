export const themePreferences = ['light', 'dark', 'auto'] as const;
export type ThemePreference = typeof themePreferences[number];

const storageKey = 'ohmyloop.ui.theme';

export function readThemePreference(): ThemePreference {
  try {
    const value = localStorage.getItem(storageKey);
    return themePreferences.find(theme => theme === value) ?? 'auto';
  } catch {
    return 'auto';
  }
}

export function applyTheme(preference: ThemePreference) {
  document.documentElement.dataset.theme = preference;
}

export function saveThemePreference(preference: ThemePreference): boolean {
  applyTheme(preference);
  try {
    localStorage.setItem(storageKey, preference);
    return true;
  } catch {
    return false;
  }
}
