import { describe, expect, it } from 'vitest';
import { beatsInMeasure, beatText, boundText, musicalPoint, passageSummary, sectionRange } from './practice-readout';
import { timingFixture } from './timing-fixture';

// Two 4/4 measures of 4 s each at 1000 frames per second; 12 s song.
const measures = timingFixture().view.measures;
const rate = 1000;

describe('practice readout', () => {
  it('counts the beat boxes of a measure from its meter and pulse', () => {
    expect(beatsInMeasure(measures[0])).toBe(4);
    expect(beatsInMeasure({ ...measures[0], meter: { numerator: 6, denominator: 8, pulse: { num: 3, den: 2 }, grouping: [] } })).toBe(2);
    expect(beatsInMeasure({ ...measures[0], meter: { numerator: 32, denominator: 4, pulse: { num: 1, den: 2 }, grouping: [] } })).toBe(0);
    expect(beatsInMeasure(null)).toBe(0);
  });
  it('places a source position on a measure and beat, between grid beats too', () => {
    expect(musicalPoint(measures, 0, rate)).toEqual({ label: '1', beat: 1 });
    expect(musicalPoint(measures, 5.5, rate)).toEqual({ label: '2', beat: 2.5 });
    expect(musicalPoint(measures, 10, rate)).toBeNull();
    expect(beatText(2)).toBe('2'); expect(beatText(2.5)).toBe('2.5'); expect(beatText(2.04)).toBe('2');
  });
  it('shows passage bounds as measure and beat, and nothing in free time', () => {
    expect(boundText(measures, 4, rate)).toBe('m. 2 · 1');
    expect(boundText(measures, 9, rate)).toBeNull();
  });
  it('summarises a measure passage in bars and a free passage in seconds', () => {
    expect(passageSummary({ start: 0, end: 8, fade_ms: 5 }, measures, rate)).toEqual({ range: 'm. 1 – 2', length: '2 bars' });
    expect(passageSummary({ start: 4, end: 8, fade_ms: 5 }, measures, rate)).toEqual({ range: 'm. 2', length: '1 bar' });
    expect(passageSummary({ start: 1, end: 6.5, fade_ms: 5 }, measures, rate)).toEqual({ range: 'm. 1 – 2', length: '5.5 s' });
    expect(passageSummary({ start: 1, end: 6.5, fade_ms: 5 }, [], rate)).toEqual({ range: '0:01.0 – 0:06.5', length: '5.5 s' });
  });
  it('lists section extents as measures, or times without a measure map', () => {
    const section = { id: 's', name: 'Verse', start: 0, end: 8 };
    expect(sectionRange(section, measures, rate)).toBe('m. 1–2');
    expect(sectionRange(section, [], rate)).toBe('0:00–0:08');
  });
});
