import { describe, expect, it } from 'vitest';
import { draftError, findState, stepMarks } from './draft-steps';

const base = { analysing: false, found: false, separating: false };
describe('drafting steps', () => {
  it('offers the one-time download, analysis, progress or the result', () => {
    expect(findState({ ...base, model: { kind: 'checking' } })).toBe('checking');
    expect(findState({ ...base, model: { kind: 'none' } })).toBe('install');
    expect(findState({ ...base, model: { kind: 'failed' } })).toBe('install');
    expect(findState({ ...base, model: { kind: 'downloading', phase: 'Downloading', progress: .3 } })).toBe('downloading');
    expect(findState({ ...base, model: { kind: 'ready' } })).toBe('analyze');
    expect(findState({ ...base, analysing: true, model: { kind: 'busy' } })).toBe('running');
    // Suggestions from an earlier run stay usable without the model.
    expect(findState({ ...base, found: true, model: { kind: 'none' } })).toBe('found');
  });

  it('waits for stem separation, which holds the shared slot', () => {
    expect(findState({ ...base, separating: true, model: { kind: 'ready' } })).toBe('waiting');
    expect(findState({ ...base, separating: true, model: { kind: 'none' } })).toBe('waiting');
    expect(findState({ ...base, separating: true, found: true, model: { kind: 'ready' } })).toBe('found');
  });

  it('marks the steps current or done', () => {
    expect(stepMarks('detected', 'analyze', false)).toEqual(['current', 'upcoming', 'upcoming']);
    expect(stepMarks('detected', 'found', false)).toEqual(['done', 'current', 'current']);
    expect(stepMarks('detected', 'found', true)).toEqual(['done', 'done', 'current']);
    expect(stepMarks('manual', 'install', false)).toEqual(['current', 'current', 'current']);
    expect(stepMarks('manual', 'install', true)).toEqual(['done', 'done', 'current']);
  });

  it('explains a draft rejected by locked measures, and passes other errors through', () => {
    expect(draftError('Error: Build span must contain whole unprotected measures')).toMatch(/^The span reaches into a locked measure/);
    expect(draftError('Error: Disk full')).toBe('Error: Disk full');
  });
});
