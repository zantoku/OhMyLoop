import { readChoice, writeChoice } from './stored-choice';

export const themePreferences = ['auto', 'light', 'dark'] as const;
export type ThemePreference = typeof themePreferences[number];

const storageKey = 'ohmyloop.ui.theme';

export function readThemePreference(): ThemePreference {
  return readChoice(storageKey, themePreferences, 'auto');
}

export function applyTheme(preference: ThemePreference) {
  document.documentElement.dataset.theme = preference;
}

/** Applies the theme at once; false when it couldn't be saved for next time. */
export function saveThemePreference(preference: ThemePreference): boolean {
  applyTheme(preference);
  return writeChoice(storageKey, preference);
}
