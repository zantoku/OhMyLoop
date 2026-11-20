// Presentation of musical positions for the Practice readout, passage bounds
// and section list. Positions are source seconds; the Rust-projected map view
// supplies measure ranges and beat grid points. No timing rules live here.
import { time } from './model';
import { measureAtFrame, type MeasureView } from './timing';
import type { Loop, Section } from './types';

/** Beat boxes shown for a measure: its notated pulses, or none for very long bars. */
export function beatsInMeasure(measure: MeasureView | null): number {
  if (!measure) return 0;
  const pulse = measure.meter.pulse.num / measure.meter.pulse.den;
  const quarters = measure.meter.numerator * 4 / measure.meter.denominator;
  const beats = Math.ceil(quarters / pulse - 1e-9);
  return beats >= 1 && beats <= 12 ? beats : 0;
}

/** `2`, or `2.5` between grid beats. */
export const beatText = (beat: number) => Number.isInteger(Math.round(beat * 10) / 10) ? String(Math.round(beat)) : (Math.round(beat * 10) / 10).toFixed(1);

/** Measure label and beat at a source position, interpolated between numbered grid beats. */
export function musicalPoint(measures: readonly MeasureView[], seconds: number, rate: number): { label: string; beat: number } | null {
  const frame = Math.round(seconds * rate);
  const measure = measureAtFrame(measures, frame);
  if (!measure) return null;
  const beats = measure.grid.filter(point => Number.isFinite(Number(point.label)));
  const index = beats.filter(point => point.frame <= frame).length - 1;
  if (index < 0) return { label: measure.label, beat: Number(beats[0]?.label ?? 1) };
  const from = beats[index], to = beats[index + 1];
  const end = to?.frame ?? measure.end;
  const fraction = end > from.frame ? (frame - from.frame) / (end - from.frame) : 0;
  const span = to ? Number(to.label) - Number(from.label) : 1;
  return { label: measure.label, beat: Number(from.label) + fraction * span };
}

/** A passage bound as `m. 33 · 1`; null in free time or without a measure map. */
export function boundText(measures: readonly MeasureView[], seconds: number, rate: number): string | null {
  const point = musicalPoint(measures, seconds, rate);
  return point ? `m. ${point.label} · ${beatText(point.beat)}` : null;
}

/** The measures a range touches; its exclusive end belongs to the preceding measure. */
function spannedMeasures(measures: readonly MeasureView[], start: number, end: number, rate: number) {
  const first = measureAtFrame(measures, Math.round(start * rate));
  const last = measureAtFrame(measures, Math.ceil(end * rate) - 1);
  return first && last ? { first, last } : null;
}

const measureRange = (first: MeasureView, last: MeasureView, separator: string) =>
  first === last ? `m. ${first.label}` : `m. ${first.label}${separator}${last.label}`;

/** The passage as measures (`m. 33 – 40`, `8 bars`) or times (`1:16.2 – 1:39.0`, `22.8 s`). */
export function passageSummary(loop: Loop, measures: readonly MeasureView[], rate: number): { range: string; length: string } {
  const seconds = `${(loop.end - loop.start).toFixed(1)} s`;
  const spanned = spannedMeasures(measures, loop.start, loop.end, rate);
  if (!spanned) return { range: `${time(loop.start)} – ${time(loop.end)}`, length: seconds };
  const { first, last } = spanned;
  const onBarlines = Math.abs(first.start - loop.start * rate) <= 1 && Math.abs(last.end - loop.end * rate) <= 1;
  const bars = measures.indexOf(last) - measures.indexOf(first) + 1;
  return { range: measureRange(first, last, ' – '), length: onBarlines ? `${bars} ${bars === 1 ? 'bar' : 'bars'}` : seconds };
}

/** A section's extent for the list: `m. 1–8`, or `0:00–0:18` without a measure map. */
export function sectionRange(section: Section, measures: readonly MeasureView[], rate: number): string {
  const spanned = spannedMeasures(measures, section.start, section.end, rate);
  return spanned ? measureRange(spanned.first, spanned.last, '–') : `${time(section.start, 0)}–${time(section.end, 0)}`;
}
