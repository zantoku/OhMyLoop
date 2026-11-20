import { describe, expect, it } from 'vitest';
import { joinCheck, measureName, propertiesText, shiftCheck, splitCheck, splitTarget, validEpsilon, validShiftStep } from './measure-tools';
import { timingFixture } from './timing-fixture';

const view = () => timingFixture(1000).view;

describe('measure toolbar rules', () => {
  it('names measures and their properties', () => {
    const [first] = view().measures;
    expect(measureName(first)).toBe('m. 1');
    expect(measureName({ number: null, label: 'Pickup' })).toBe('Pickup');
    expect(propertiesText(first)).toBe('4/4 · Full');
    expect(propertiesText({ ...first, kind: 'pickup' })).toBe('4/4 · Pickup');
  });

  it('splits before the inner beat nearest the playhead, only inside the measure', () => {
    const [first] = view().measures; // Beats at 1000, 2000, 3000.
    expect(splitTarget(first, 1400)).toEqual({ beat: 2, frame: 1000 });
    expect(splitTarget(first, 2600)).toEqual({ beat: 4, frame: 3000 });
    expect(splitTarget(first, 100)).toEqual({ beat: 2, frame: 1000 });
    expect(splitTarget(first, 3999)).toEqual({ beat: 4, frame: 3000 });
    expect(splitTarget(first, 4000)).toBeNull();
    expect(splitCheck(first, 4500)).toEqual({ ok: false, reason: 'Move the playhead into m. 1 to choose where to split' });
    expect(splitCheck(first, 2100)).toEqual({ ok: true, detail: 'Split at beat 3' });
    expect(splitCheck({ ...first, protected: true }, 2100).ok).toBe(false);
    expect(splitCheck({ ...first, split_points: [] }, 2100)).toEqual({ ok: false, reason: 'This measure has no inner beat to split at' });
  });

  it('joins only adjacent, unlocked measures with the same denominator', () => {
    const map = view();
    expect(joinCheck(map, map.measures[0])).toEqual({ ok: true, detail: 'Join with m. 2 into one 8/4 measure' });
    expect(joinCheck(map, map.measures[1])).toEqual({ ok: false, reason: 'There is no following measure to join' });
    const eighths = view(); eighths.measures[1].meter = { ...eighths.measures[1].meter, denominator: 8 };
    expect(joinCheck(eighths, eighths.measures[0]).ok).toBe(false);
    const gap = view(); gap.measures[1].start_id = 'other';
    expect(joinCheck(gap, gap.measures[0])).toEqual({ ok: false, reason: 'Free time separates this measure from the next' });
    const locked = view(); locked.measures[1].protected = true;
    expect(joinCheck(locked, locked.measures[0])).toEqual({ ok: false, reason: 'Unlock both measures before joining' });
    const long = view(); long.measures[0].meter = { ...long.measures[0].meter, numerator: 97 };
    expect(joinCheck(long, long.measures[0])).toEqual({ ok: false, reason: 'The joined numerator would exceed 99' });
  });

  it('shifts only a gap-free, unlocked run with a preceding measure', () => {
    const map = view();
    expect(shiftCheck(map, map.measures[0])).toEqual({ ok: false, reason: 'Shifting needs a measure before this one' });
    expect(shiftCheck(map, map.measures[1])).toEqual({ ok: true, detail: 'quarter-note pulses' });
    const locked = view(); locked.measures[0].protected = true;
    expect(shiftCheck(locked, locked.measures[1]).ok).toBe(false);
    const gap = view(); gap.measures[1].start_id = 'other';
    expect(shiftCheck(gap, gap.measures[1])).toMatchObject({ ok: false, reason: expect.stringMatching(/^Free time/) });
    expect(shiftCheck(null, map.measures[1]).ok).toBe(false);
  });

  it('validates the shift step and ε', () => {
    for (const value of [1, 4, 64]) expect(validShiftStep(value)).toBe(true);
    for (const value of [0, 1.5, 65, NaN]) expect(validShiftStep(value)).toBe(false);
    for (const value of [0, 30, 1000]) expect(validEpsilon(value)).toBe(true);
    for (const value of [-1, 1001, NaN]) expect(validEpsilon(value)).toBe(false);
  });
});
