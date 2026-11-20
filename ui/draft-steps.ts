// The drafting drawer's guided steps (measure-map.md, A1–A7): what step 1
// offers for the detected-beats route, and which steps are current or done.
// Pure presentation of the analysis status, cached suggestions and the draft.
import { time } from './model';
import type { BeatModel } from './settings';

export type DraftSource = 'detected' | 'manual';
export type FindState =
  | 'checking'    // the model's status is not known yet
  | 'install'     // one-time model download needed
  | 'downloading' // the model is downloading
  | 'waiting'     // stem separation holds the shared processing slot
  | 'analyze'     // ready to analyze
  | 'running'     // analysis runs
  | 'found';      // suggestions exist for this recording
export type StepMark = 'current' | 'done' | 'upcoming';

export function findState({ model, analysing, found, separating }:
  { model: BeatModel; analysing: boolean; found: boolean; separating: boolean }): FindState {
  if (analysing) return 'running';
  if (found) return 'found';
  if (model.kind === 'downloading') return 'downloading';
  if (separating) return 'waiting';
  if (model.kind === 'checking') return 'checking';
  if (model.kind === 'none' || model.kind === 'failed') return 'install';
  return 'analyze';
}

/** Step marks for 1, 2 and 3. Manual drafting has no prerequisite step. */
export function stepMarks(source: DraftSource, find: FindState, previewing: boolean): [StepMark, StepMark, StepMark] {
  if (source === 'manual') return previewing ? ['done', 'done', 'current'] : ['current', 'current', 'current'];
  if (find !== 'found') return ['current', 'upcoming', 'upcoming'];
  return ['done', previewing ? 'done' : 'current', 'current'];
}

/**
 * The drafting inputs. They are session state: they survive closing the
 * drawer and switching screens (measure-map.md, fixed constraints).
 */
export type DraftInputs = {
  source: DraftSource;
  numerator: number; denominator: number; pulse: string;
  /** Exact times as typed (`0:03.21` or seconds). */
  first: string; start: string; end: string;
  count: number;
};
export function draftInputs(duration: number): DraftInputs {
  return { source: 'detected', numerator: 4, denominator: 4, pulse: '1', first: '0:00.000', start: '0:00.000', end: time(duration, 3), count: Math.max(1, Math.round(duration / 2)) };
}

/** A draft's rejection in the drawer's words; Rust's message otherwise. */
export function draftError(error: string): string {
  if (/whole unprotected measures/i.test(error)) return 'The span reaches into a locked measure. Choose a span that leaves your locked measures out, or unlock them first.';
  return error;
}
