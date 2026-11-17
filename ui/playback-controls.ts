// Pure rules for the speed, transposition and tuning controls. Limits mirror dsp::Controls::validate.
import { clamp } from './model';
import type { Controls } from './types';

export const SPEED_PERCENT = { min: 25, max: 200 } as const;
export const SEMITONES = { min: -12, max: 12 } as const;
export const TUNING_CENTS = { min: -50, max: 50 } as const;
/** Speed buttons step in percentage points, so 80% + 10 is 90%. */
export const SPEED_STEPS = [-10, -5, 5, 10] as const;
export const ORIGINAL: Controls = { speed: 1, semitones: 0, tuning_cents: 0 };

export const speedPercent = (speed: number) => Math.round(speed * 1000) / 10;

/** Clamps every value to its engine limit; returns null when an entry is not a number. */
export function normalized(next: Controls): Controls | null {
  if (![next.speed, next.semitones, next.tuning_cents].every(Number.isFinite)) return null;
  return {
    speed: clamp(next.speed, SPEED_PERCENT.min / 100, SPEED_PERCENT.max / 100),
    semitones: clamp(next.semitones, SEMITONES.min, SEMITONES.max),
    tuning_cents: clamp(next.tuning_cents, TUNING_CENTS.min, TUNING_CENTS.max),
  };
}

export const stepSpeed = (speed: number, points: number) =>
  clamp(Math.round(speedPercent(speed) + points), SPEED_PERCENT.min, SPEED_PERCENT.max) / 100;

/** Whole-semitone transposition; a fractional value from an older song snaps to the nearest semitone. */
export const transpose = (semitones: number, by: number) => clamp(Math.round(semitones) + by, SEMITONES.min, SEMITONES.max);

export const signed = (value: number) => (value > 0 ? `+${value}` : String(value));
