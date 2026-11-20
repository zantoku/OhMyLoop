// What the Measure map toolbar can do with the selected measure, and why not.
// Presentation-side checks on the Rust-projected view only; Rust validates
// every edit again and its rejections are shown as they come.
import { pulseName, type MapView, type MeasureKind, type MeasureView } from './timing';

export type Check = { ok: true; detail: string } | { ok: false; reason: string };

/** `m. 37`, or `Pickup` for an unnumbered measure. */
export const measureName = (measure: Pick<MeasureView, 'number' | 'label'>) => measure.number === null ? measure.label : `m. ${measure.label}`;
export const KIND_NAMES: Record<MeasureKind, string> = { full: 'Full', pickup: 'Pickup', incomplete: 'Incomplete' };
/** The properties button's text: `4/4 · Full`. */
export const propertiesText = (measure: MeasureView) => `${measure.meter.numerator}/${measure.meter.denominator} · ${KIND_NAMES[measure.kind]}`;

/**
 * Split splits before the beat nearest the playhead, so it needs no input.
 * Only beats inside the measure can split it; null when the playhead is
 * outside the measure or the measure has no inner beat.
 */
export function splitTarget(measure: MeasureView, frame: number): { beat: number; frame: number } | null {
  if (frame < measure.start || frame >= measure.end || !measure.split_points.length) return null;
  let best = measure.split_points[0];
  for (const point of measure.split_points) if (Math.abs(point.frame - frame) < Math.abs(best.frame - frame)) best = point;
  return { beat: best.beat, frame: best.frame };
}

export function splitCheck(measure: MeasureView, frame: number): Check {
  if (measure.protected) return { ok: false, reason: 'Unlock this measure to split it' };
  if (!measure.split_points.length) return { ok: false, reason: 'This measure has no inner beat to split at' };
  const target = splitTarget(measure, frame);
  if (!target) return { ok: false, reason: `Move the playhead into ${measureName(measure)} to choose where to split` };
  return { ok: true, detail: `Split at beat ${target.beat}` };
}

const following = (view: MapView | null, measure: MeasureView) => view?.measures[view.measures.findIndex(m => m.id === measure.id) + 1] ?? null;

export function joinCheck(view: MapView | null, measure: MeasureView): Check {
  const next = following(view, measure);
  if (!next) return { ok: false, reason: 'There is no following measure to join' };
  if (measure.end_id !== next.start_id) return { ok: false, reason: 'Free time separates this measure from the next' };
  if (measure.protected || next.protected) return { ok: false, reason: 'Unlock both measures before joining' };
  if (measure.meter.denominator !== next.meter.denominator) return { ok: false, reason: 'Joining needs the same denominator in both measures' };
  const numerator = measure.meter.numerator + next.meter.numerator;
  if (numerator > 99) return { ok: false, reason: 'The joined numerator would exceed 99' };
  return { ok: true, detail: `Join with ${measureName(next)} into one ${numerator}/${measure.meter.denominator} measure` };
}

/** Shifting moves this and every later barline, so the whole run must be one unlocked, gap-free stretch. */
export function shiftCheck(view: MapView | null, measure: MeasureView): Check {
  const index = view?.measures.findIndex(m => m.id === measure.id) ?? -1;
  if (index <= 0) return { ok: false, reason: 'Shifting needs a measure before this one' };
  const run = view!.measures.slice(index - 1);
  if (run.some((m, i) => i > 0 && run[i - 1].end_id !== m.start_id)) return { ok: false, reason: 'Free time interrupts the following measures; count it as a measure first' };
  if (run.some(m => m.protected)) return { ok: false, reason: 'Unlock the affected measures before shifting' };
  return { ok: true, detail: `${pulseName(measure.meter.pulse)}-note pulses` };
}

/** A whole number of pulses, 1–64. */
export const validShiftStep = (pulses: number) => Number.isInteger(pulses) && pulses >= 1 && pulses <= 64;
/** Beat timing tolerance in milliseconds, 0–1000. */
export const validEpsilon = (ms: number) => Number.isFinite(ms) && ms >= 0 && ms <= 1000;
