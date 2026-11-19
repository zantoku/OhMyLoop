import { describe, expect, it } from 'vitest';
import { beatModel, sectionStatus, settingsBlockedReason, settingsDownload, stemTools, type SettingsSummary } from './settings';
import type { SeparationStatus } from './separation';
import type { AnalysisStatus } from './timing';

const attempt = { id: '', running: false, phase: 'Idle', reference_token: null, progress: null, error: null, details: '', elapsed_seconds: 0, cancellable: false, result: null };
const separation = (change: Partial<SeparationStatus> = {}): SeparationStatus => ({ package_id: 'p', profile_id: 'c', model_ready: true, model_path: '/m', package_bytes: 699416805,
  runtime: { ready: true, message: 'Ready', path: '/python', supported: true }, heavy_work: null, install: { ...attempt }, job: { ...attempt }, ...change });
const analysis = (change: Partial<AnalysisStatus> = {}): AnalysisStatus => ({ id: 1, running: false, kind: 'idle', phase: 'Ready', model_ready: false, model_path: '/b', progress: null, elapsed_seconds: 0, reference_token: null, map_revision: 0, error: null, result: null, ...change });
const summary = (change: Partial<SettingsSummary> = {}): SettingsSummary => ({
  library: { selected: { id: 'l', root: '/Users/me/Music/OhMyLoop Library' }, available: true, error: null, import_notice_seen: true }, songCount: 12,
  theme: 'auto', notesFont: 'proportional', device: 'MacBook Pro Speakers', audioError: null, native: true, shortcuts: 'builtin',
  separation: separation(), analysis: analysis(), ...change });

describe('sidebar status lines', () => {
  it('names the library and its songs, and flags a missing one', () => {
    expect(sectionStatus('library', summary())).toEqual({ text: 'OhMyLoop Library · 12 songs', tone: 'normal' });
    expect(sectionStatus('library', summary({ songCount: 1 })).text).toBe('OhMyLoop Library · 1 song');
    expect(sectionStatus('library', summary({ library: { selected: { id: 'l', root: 'D:\\Music\\Lib' }, available: false, error: 'gone', import_notice_seen: true } }))).toEqual({ text: 'Not found', tone: 'warning' });
    expect(sectionStatus('library', summary({ library: { selected: null, available: false, error: null, import_notice_seen: false } })).text).toBe('No library yet');
  });
  it('summarises appearance, audio and shortcuts', () => {
    expect(sectionStatus('appearance', summary({ theme: 'dark', notesFont: 'monospace' })).text).toBe('Dark · monospaced notes');
    expect(sectionStatus('audio', summary()).text).toBe('MacBook Pro Speakers');
    expect(sectionStatus('audio', summary({ audioError: 'Device disconnected' }))).toEqual({ text: 'Device lost', tone: 'error' });
    expect(sectionStatus('audio', summary({ device: null })).text).toBe('No audio output');
    expect(sectionStatus('shortcuts', summary({ shortcuts: 'custom' })).text).toBe('Custom file');
    expect(sectionStatus('shortcuts', summary({ shortcuts: 'rejected' }))).toEqual({ text: 'File rejected', tone: 'error' });
  });
  it('shows each tool’s state, with download progress', () => {
    expect(sectionStatus('stems', summary())).toEqual({ text: 'Ready · 699.4 MB model', tone: 'success' });
    const installing = separation({ install: { ...attempt, running: true, phase: 'Downloading model', progress: { stage: 'model', completed: 59, total: 100, workflow: null } } });
    expect(sectionStatus('stems', summary({ separation: installing })).text).toBe('Downloading model · 59 %');
    expect(sectionStatus('beats', summary()).text).toBe('Not installed');
    expect(sectionStatus('beats', summary({ analysis: analysis({ running: true, kind: 'install', phase: 'Downloading beat model', progress: { completed: 42, total: 100, unit: 'bytes' } }) })).text).toBe('Downloading · 42 %');
    expect(sectionStatus('beats', summary({ analysis: analysis({ kind: 'install', phase: 'Failed', error: 'offline' }) }))).toEqual({ text: 'Download failed', tone: 'error' });
  });
});

describe('tool states', () => {
  it('tells damaged tools from missing ones', () => {
    expect(stemTools(null).kind).toBe('checking');
    expect(stemTools(separation({ model_ready: false, runtime: { ready: false, message: 'Set up', path: null, supported: true } })).kind).toBe('none');
    expect(stemTools(separation({ runtime: { ready: false, message: 'A file failed verification', path: '/python', supported: true } })).kind).toBe('damaged');
    expect(stemTools(separation({ runtime: { ready: false, message: 'No AVX', path: null, supported: false } })).kind).toBe('unsupported');
    expect(stemTools(separation({ model_ready: false, runtime: { ready: false, message: '', path: null, supported: true }, install: { ...attempt, phase: 'Failed', error: 'offline' } })).kind).toBe('failed');
  });
  it('separates a model download from an analysis run', () => {
    expect(beatModel(analysis({ running: true, kind: 'analysis' })).kind).toBe('busy');
    expect(beatModel(analysis({ model_ready: true })).kind).toBe('ready');
    expect(settingsDownload(separation(), analysis({ running: true, kind: 'install' }))).toBe('beats');
    expect(settingsDownload(separation({ install: { ...attempt, running: true } }), analysis())).toBe('stems');
    expect(settingsDownload(separation(), analysis({ running: true, kind: 'analysis' }))).toBeNull();
  });
});

describe('Settings blocked by background work (decision 6)', () => {
  const idle = { separating: false, stemSetup: false, beatDownload: false, analysing: false, busy: '' };
  it('opens when nothing runs, and says why otherwise', () => {
    expect(settingsBlockedReason(idle)).toBeNull();
    expect(settingsBlockedReason({ ...idle, separating: true })).toMatch(/split into stems/);
    expect(settingsBlockedReason({ ...idle, stemSetup: true })).toMatch(/stem separation tools download/);
    expect(settingsBlockedReason({ ...idle, beatDownload: true })).toMatch(/beat analysis model downloads/);
    expect(settingsBlockedReason({ ...idle, analysing: true })).toMatch(/beat analysis runs/);
    expect(settingsBlockedReason({ ...idle, busy: 'Importing song… 40%' })).toMatch(/Importing song… 40%/);
    expect(settingsBlockedReason({ ...idle, busy: 'Opening song…' })).toMatch(/Opening song…/);
  });
});
