// Input snapping and ruler presentation use the native map's projected source
// ranges. Musical timing and map edits remain in Rust.
import { range } from './model';
import { measureAtFrame, type MeasureView } from './timing';

export function snapLoopPoint(seconds: number, measures: readonly MeasureView[], rate: number): number {
  const frame = Math.round(seconds * rate);
  const measure = measureAtFrame(measures, frame);
  if (!measure) return seconds; // Free time stays freely selectable.
  return (frame - measure.start <= measure.end - frame ? measure.start : measure.end) / rate;
}

// A drag selects the whole measures it touches. The exclusive end belongs to
// the preceding measure, so ending exactly on a barline does not add a bar.
export function measureLoop(a: number, b: number, duration: number, measures: readonly MeasureView[], rate: number) {
  const start = Math.min(a, b), end = Math.max(a, b);
  const first = measureAtFrame(measures, Math.round(start * rate));
  const last = measureAtFrame(measures, Math.ceil(end * rate) - 1);
  return range(first ? first.start / rate : start, last ? last.end / rate : end, duration);
}

export function stepLoopPoint(seconds: number, direction: number, measures: readonly MeasureView[], rate: number): number | null {
  const frame = Math.round(seconds * rate);
  const measure = measureAtFrame(measures, direction > 0 ? frame : frame - 1);
  if (!measure) return null;
  return (direction > 0 ? measure.end : measure.start) / rate;
}
