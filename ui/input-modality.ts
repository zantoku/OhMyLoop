// Tracks whether the person is navigating by keyboard or pointer, on
// `<html data-modality>`. Focus rings are drawn only for keyboard modality.
import { NAVIGATION_KEYS } from './keymap';

export type Modality = 'keyboard' | 'pointer';

const NAVIGATION = new Set(['Tab', ...NAVIGATION_KEYS]);

export function currentModality(doc: Document = document): Modality {
  return doc.documentElement.dataset.modality === 'keyboard' ? 'keyboard' : 'pointer';
}

export function installInputModality(doc: Document = document): () => void {
  const root = doc.documentElement;
  root.dataset.modality ??= 'pointer';
  const pointer = () => { root.dataset.modality = 'pointer'; };
  const keyboard = (event: KeyboardEvent) => { if (NAVIGATION.has(event.key)) root.dataset.modality = 'keyboard'; };
  doc.addEventListener('pointerdown', pointer, true);
  doc.addEventListener('keydown', keyboard, true);
  return () => { doc.removeEventListener('pointerdown', pointer, true); doc.removeEventListener('keydown', keyboard, true); };
}
