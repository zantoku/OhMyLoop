import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import App from './App.svelte';
import type { PracticeDocument, Snapshot, Transport } from './types';

const mock = vi.hoisted(() => ({ initialize: vi.fn(), loadDemo: vi.fn(), openSong: vi.fn(), update: vi.fn(), action: vi.fn(), status: vi.fn() }));
vi.mock('./bridge', () => ({ native: true, bridge: mock }));
let document: PracticeDocument;
let transport: Transport;
const response = (): Snapshot => ({ song: null, document: structuredClone(document), transport: { ...transport } });

beforeEach(() => {
  vi.clearAllMocks();
  document = { version: 2, song_key: 'test', controls: { speed: 1, semitones: 0 },
    sections: [{ id: 'verse', name: 'Verse', start: 0, end: 8 }, { id: 'chorus', name: 'Chorus', start: 3, end: 6 }],
    selected_section: null, loop_selection: null, loop_enabled: false, last_position: 0 };
  transport = { song_key: 'test', position: 0, playing: false, finished: false, audio_error: null, save_error: null, device: 'Test output' };
  mock.initialize.mockResolvedValue({ ...response(), song: { key: 'test', title: 'Workshop take', path: '/test/song.wav', duration: 12, rate: 48000, peaks: [[-.2, .2]] } });
  mock.update.mockImplementation(async (value: PracticeDocument) => { document = structuredClone(value); return response(); });
  mock.action.mockImplementation(async (action) => { if (action.type === 'seek') transport.position = action.seconds; else if (action.type === 'toggle_play') transport.playing = !transport.playing; return response(); });
  mock.status.mockImplementation(async () => ({ ...transport }));
});

async function ready() { render(App); await screen.findByRole('heading', { name: 'Workshop take' }); }

describe('single-song practice workflow', () => {
  it('can save exact dragged boundaries without browser step-validation errors', async () => {
    document.loop_selection = { start: 1.851590106007067, end: 3.5053003533568905, fade_ms: 5 };
    mock.initialize.mockResolvedValue({ ...response(), song: { key: 'test', title: 'Workshop take', path: '/test/song.wav', duration: 12, rate: 48000, peaks: [[-.2, .2]] } });
    await ready(); await fireEvent.click(screen.getByRole('button', { name: 'Save as section' }));
    const start = screen.getByLabelText('Start (seconds)') as HTMLInputElement;
    const end = screen.getByLabelText('End (seconds)') as HTMLInputElement;
    expect(start.checkValidity()).toBe(true); expect(end.checkValidity()).toBe(true);
    await fireEvent.input(screen.getByLabelText('Section name'), { target: { value: 'Exact phrase' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save section' }));
    await waitFor(() => expect(document.sections.find(s => s.name === 'Exact phrase')).toMatchObject({ start: 1.851590106007067, end: 3.5053003533568905 }));
  });
  it('selects a section range without implicitly enabling looping', async () => {
    await ready(); await fireEvent.click(screen.getByRole('button', { name: /Chorus 0:03/ }));
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'seek', seconds: 3 }));
    expect(document.loop_selection).toEqual({ start: 3, end: 6, fade_ms: 5 });
    expect(document.loop_enabled).toBe(false);
    await fireEvent.click(screen.getByRole('button', { name: 'Loop off' }));
    await waitFor(() => expect(document.loop_enabled).toBe(true));
    expect(document.loop_selection?.start).toBe(3);
  });
  it('moves an enabled loop to a selected section without starting playback', async () => {
    await ready(); await fireEvent.click(screen.getByRole('button', { name: 'Loop off' }));
    await screen.findByRole('button', { name: 'Loop on' });
    await fireEvent.click(screen.getByRole('button', { name: /Chorus 0:03/ }));
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'seek', seconds: 3 }));
    expect(document.loop_selection).toEqual({ start: 3, end: 6, fade_ms: 5 });
    expect(document.selected_section).toBe('chorus'); expect(transport.playing).toBe(false);
  });
  it('preserves a pitch adjustment when changing speed quickly afterwards', async () => {
    await ready();
    await fireEvent.change(screen.getByLabelText('Pitch semitones'), { target: { value: '-3' } });
    await fireEvent.click(screen.getByRole('button', { name: '75%' }));
    await waitFor(() => expect(document.controls).toEqual({ speed: .75, semitones: -3 }));
    expect((screen.getByLabelText('Pitch semitones') as HTMLInputElement).value).toBe('-3');
  });
  it('creates a named phrase inside a longer section', async () => {
    await ready(); await fireEvent.click(screen.getByRole('button', { name: 'Add section' }));
    await fireEvent.input(screen.getByLabelText('Section name'), { target: { value: 'Bend and release' } });
    await fireEvent.input(screen.getByLabelText('Start (seconds)'), { target: { value: '2.1' } });
    await fireEvent.input(screen.getByLabelText('End (seconds)'), { target: { value: '2.6' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save section' }));
    await waitFor(() => expect(document.sections.find(s => s.name === 'Bend and release')).toMatchObject({ start: 2.1, end: 2.6 }));
    expect(document.sections.find(s => s.id === 'verse')?.end).toBe(8);
  });
  it('offers Undo and restores the same section identity and bounds', async () => {
    await ready(); await fireEvent.click(screen.getByRole('button', { name: 'Delete Chorus' }));
    await waitFor(() => expect(document.sections.find(s => s.id === 'chorus')).toBeUndefined());
    await fireEvent.click(screen.getByRole('button', { name: 'Undo' }));
    await waitFor(() => expect(document.sections.find(s => s.id === 'chorus')).toEqual({ id: 'chorus', name: 'Chorus', start: 3, end: 6 }));
  });
  it('keeps transport shortcuts out of numeric fields', async () => {
    await ready(); await fireEvent.keyDown(screen.getByLabelText('Pitch semitones'), { key: ' ', code: 'Space' });
    expect(mock.action).not.toHaveBeenCalled();
    await fireEvent.keyDown(window, { key: ' ', code: 'Space' });
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'toggle_play' }));
  });
  it('clears the A–B range with X while preserving saved sections and ongoing playback', async () => {
    await ready(); await fireEvent.click(screen.getByRole('button', { name: /Chorus 0:03/ }));
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'seek', seconds: 3 }));
    await fireEvent.click(screen.getByRole('button', { name: 'Loop off' }));
    await waitFor(() => expect(document.loop_enabled).toBe(true));
    await fireEvent.keyDown(window, { key: ' ', code: 'Space' });
    await waitFor(() => expect(transport.playing).toBe(true));
    const sections = structuredClone(document.sections);
    mock.action.mockClear(); mock.update.mockClear();
    await fireEvent.keyDown(screen.getByLabelText('Pitch semitones'), { key: 'x', code: 'KeyX' });
    expect(mock.update).not.toHaveBeenCalled(); expect(document.loop_selection).not.toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Add section' }));
    await fireEvent.keyDown(window, { key: 'x', code: 'KeyX' });
    expect(mock.update).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    await fireEvent.keyDown(window, { key: 'X', code: 'KeyX', shiftKey: true });
    await waitFor(() => expect(document.loop_selection).toBeNull());
    expect(document.loop_enabled).toBe(false);
    expect(document.sections).toEqual(sections); expect(document.selected_section).toBe('chorus');
    expect(transport.playing).toBe(true); expect(mock.action).not.toHaveBeenCalled();
    expect(screen.queryByRole('button', { name: 'Clear loop selection' })).toBeNull();
  });
});
