import { describe, expect, it } from 'vitest';
import { centsForReference, hzText, normalized, REFERENCE_HZ, referenceHz, semitoneText, speedPercent, stepSpeed, transpose } from './playback-controls';

describe('playback control rules', () => {
  it('steps speed in percentage points within 25–200%', () => {
    expect(stepSpeed(.8, 10)).toBe(.9);
    expect(stepSpeed(.8, -5)).toBe(.75);
    expect(stepSpeed(.875, 5)).toBe(.93);
    expect(stepSpeed(.3, -10)).toBe(.25);
    expect(stepSpeed(1.95, 10)).toBe(2);
  });
  it('shows freely entered speeds to one decimal place', () => {
    expect(speedPercent(.875)).toBe(87.5);
    expect(speedPercent(1 / 3)).toBe(33.3);
  });
  it('converts a recording\'s reference pitch to the engine\'s cents and back', () => {
    expect(centsForReference(440)).toBe(0);
    expect(centsForReference(432)).toBeCloseTo(31.77, 2);
    expect(hzText(centsForReference(432))).toBe('432');
    expect(hzText(0)).toBe('440');
    expect(referenceHz(centsForReference(445.5))).toBeCloseTo(445.5, 6);
    // The engine accepts ±50 ¢, about A = 427.5–452.9 Hz.
    expect(centsForReference(400)).toBe(50);
    expect(centsForReference(500)).toBe(-50);
    expect(REFERENCE_HZ.min).toBeCloseTo(427.47, 2);
    expect(REFERENCE_HZ.max).toBeCloseTo(452.89, 2);
  });
  it('labels transposition with a real minus sign', () => {
    expect(semitoneText(-2)).toBe('−2 st');
    expect(semitoneText(0)).toBe('0 st');
    expect(semitoneText(3)).toBe('+3 st');
  });
  it('transposes in whole semitones within an octave either way', () => {
    expect(transpose(2, 1)).toBe(3);
    expect(transpose(12, 1)).toBe(12);
    expect(transpose(1.3, -1)).toBe(0);
  });
  it('clamps every control to the engine limits and rejects non-numbers', () => {
    expect(normalized({ speed: 3, semitones: -20, tuning_cents: 80 })).toEqual({ speed: 2, semitones: -12, tuning_cents: 50 });
    expect(normalized({ speed: 1, semitones: 0, tuning_cents: NaN })).toBeNull();
  });
});
