import type { Loop, Section, View } from './types';

export const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));
export function time(value: number, decimals = 1): string {
  const seconds = Math.max(0, Number.isFinite(value) ? value : 0);
  const precision = 10 ** decimals;
  const rounded = Math.round(seconds * precision) / precision;
  return `${Math.floor(rounded / 60)}:${(rounded % 60).toFixed(decimals).padStart(decimals ? 3 + decimals : 2, '0')}`;
}
export function range(a: number, b: number, duration: number): Loop | null {
  const start = clamp(Math.min(a, b), 0, duration), end = clamp(Math.max(a, b), 0, duration);
  return end - start + 1e-9 >= 0.1 ? { start, end, fade_ms: 5 } : null;
}
export function zoom(view: View, duration: number, factor: number, center: number, anchor = 0.5): View {
  const span = clamp((view.end - view.start) * factor, Math.min(0.25, duration), duration);
  const start = clamp(center - span * clamp(anchor, 0, 1), 0, duration - span);
  return { start, end: start + span };
}
export function ordered(sections: Section[]): Section[] { return [...sections].sort((a, b) => a.start - b.start || a.end - b.end); }
export function sectionColor(id: string): string {
  const colors = ['#b9c6eb', '#dfbb7f', '#a7c9bd', '#d3a8c2', '#a9c9dc'];
  let hash = 0; for (const c of id) hash = (hash * 31 + c.charCodeAt(0)) >>> 0;
  return colors[hash % colors.length];
}
export function editing(target: EventTarget | null): boolean {
  return target instanceof Element && Boolean(target.closest('input, textarea, select, [contenteditable="true"], dialog'));
}
