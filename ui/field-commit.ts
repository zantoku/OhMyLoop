// Number and short text fields hand focus back after editing:
// Enter commits and returns home, Esc restores the value it had when focused
// and returns home, and leaving the field commits through its change event.
// A closed select also returns home on Esc. Fields in dialogs keep the
// dialog's own Enter/Esc behaviour, and Enter in a form submits the form.
import { focusHome } from './home-focus';
import { isShortField } from './key-routing';

export function installFieldCommit(doc: Document = document, home: () => void = () => focusHome(doc)): () => void {
  const original = new WeakMap<HTMLInputElement, string>();
  const focus = (event: FocusEvent) => { if (isShortField(event.target as Element)) original.set(event.target as HTMLInputElement, (event.target as HTMLInputElement).value); };
  const key = (event: KeyboardEvent) => {
    const field = event.target;
    if (event.defaultPrevented || event.isComposing || !(field instanceof Element) || field.closest('dialog')) return;
    if (field instanceof HTMLSelectElement && event.key === 'Escape') { event.preventDefault(); home(); return; }
    if (!isShortField(field)) return;
    if (event.key === 'Enter' && !field.form && !event.altKey && !event.ctrlKey && !event.metaKey) {
      event.preventDefault(); home();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      const value = original.get(field);
      if (value !== undefined && field.value !== value) {
        field.value = value;
        field.dispatchEvent(new Event('input', { bubbles: true }));
      }
      home();
    }
  };
  doc.addEventListener('focusin', focus);
  doc.addEventListener('keydown', key);
  return () => { doc.removeEventListener('focusin', focus); doc.removeEventListener('keydown', key); };
}
