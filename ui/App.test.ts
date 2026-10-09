import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import App from './App.svelte';
import { timingFixture } from './timing-fixture';
import type { PracticeDocument, Snapshot, Transport } from './types';

const mock = vi.hoisted(() => ({ initialize: vi.fn(), loadDemo: vi.fn(), openSong: vi.fn(), update: vi.fn(), action: vi.fn(), status: vi.fn(), readKeymap: vi.fn(), analysisStatus: vi.fn(), previewMap: vi.fn(), mapView: vi.fn() }));
vi.mock('./bridge', () => ({ native: true, bridge: mock }));
let document: PracticeDocument;
let transport: Transport;
const response = (): Snapshot => ({ song: null, document: structuredClone(document), transport: { ...transport } });

beforeEach(() => {
  vi.clearAllMocks();
  mock.analysisStatus.mockResolvedValue({id:0,running:false,kind:'idle',phase:'Ready',model_ready:false,song_key:null,map_revision:0,error:null,result:null});
  mock.readKeymap.mockResolvedValue({ path: '/test/keymap.json', configuration: null, warning: null });
  document = { version: 3, song_key: 'test', controls: { speed: 1, semitones: 0 },
    sections: [{ id: 'verse', name: 'Verse', start: 0, end: 8 }, { id: 'chorus', name: 'Chorus', start: 3, end: 6 }],
    selected_section: null, loop_selection: null, loop_enabled: false, last_position: 0 };
  transport = { song_key: 'test', position: 0, playing: false, finished: false, audio_error: null, save_error: null, device: 'Test output' };
  mock.initialize.mockResolvedValue({ ...response(), song: { key: 'test', title: 'Workshop take', path: '/test/song.wav', duration: 12, rate: 48000, peaks: [[-.2, .2]] } });
  mock.update.mockImplementation(async (value: PracticeDocument) => { document = structuredClone(value); return response(); });
  mock.action.mockImplementation(async (action) => {
    if (action.type === 'seek') {
      transport.position = action.seconds; document.last_position = action.seconds;
      const loop = document.loop_selection;
      if (document.loop_enabled && loop && (action.seconds < loop.start || action.seconds >= loop.end)) document.loop_enabled = false;
    } else if (action.type === 'toggle_play') transport.playing = !transport.playing;
    return response();
  });
  mock.status.mockImplementation(async () => ({ ...transport }));
});

async function ready() {
  render(App);
  await screen.findByRole('heading', { name: 'Workshop take' });
  await screen.findByText('Practice settings saved');
}

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
  it.each([false, true])('jumps to the precise A marker with comma while preserving active state (%s)', async active => {
    document.loop_selection = { start: 2.3456789, end: 5.6789, fade_ms: 5 };
    document.loop_enabled = active; document.selected_section = 'verse';
    transport.playing = active; transport.position = 9;
    const before = structuredClone(document);
    mock.initialize.mockResolvedValue({ ...response(), song: { key: 'test', title: 'Workshop take', path: '/test/song.wav', duration: 12, rate: 48000, peaks: [[-.2, .2]] } });
    await ready(); await fireEvent.keyDown(window, { key: ',', code: 'Comma' });
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'seek', seconds: 2.3456789 }));
    expect(transport.position).toBe(2.3456789); expect(transport.playing).toBe(active);
    expect(document.loop_enabled).toBe(active); expect(document.loop_selection).toEqual(before.loop_selection);
    expect(document.sections).toEqual(before.sections); expect(document.selected_section).toBe('verse');
    expect(mock.update).not.toHaveBeenCalled();
  });
  it('ignores A/B shortcuts without a range and while entering decimal boundaries or editing a dialog', async () => {
    await ready(); await fireEvent.keyDown(window, { key: '.', code: 'Period' });
    await fireEvent.keyDown(window, { key: ',', code: 'Comma' });
    expect(mock.action).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button', { name: /Chorus 0:03/ }));
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'seek', seconds: 3 }));
    mock.action.mockClear();
    const decimal = new KeyboardEvent('keydown', { key: '.', code: 'Period', bubbles: true, cancelable: true });
    await fireEvent(screen.getByLabelText('Loop start seconds'), decimal);
    expect(decimal.defaultPrevented).toBe(false);
    await fireEvent.click(screen.getByRole('button', { name: 'Add section' }));
    await fireEvent.keyDown(window, { key: '.', code: 'Period' });
    expect(mock.action).not.toHaveBeenCalled();
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
  it('jumps to B and turns looping off without pausing playback or clearing the range', async () => {
    await ready(); await fireEvent.click(screen.getByRole('button', { name: /Chorus 0:03/ }));
    await waitFor(() => expect(transport.position).toBe(3));
    await fireEvent.click(screen.getByRole('button', { name: 'Loop off' }));
    await waitFor(() => expect(document.loop_enabled).toBe(true));
    await fireEvent.keyDown(window, { key: ' ', code: 'Space' });
    await waitFor(() => expect(transport.playing).toBe(true));
    await fireEvent.keyDown(window, { key: '.', code: 'Period' });
    await waitFor(() => expect(transport.position).toBe(6));
    expect(document.loop_enabled).toBe(false); expect(transport.playing).toBe(true);
    expect(document.loop_selection).toEqual({ start: 3, end: 6, fade_ms: 5 });
  });
  it('rewinds to the song start with Enter, including outside an active loop', async () => {
    await ready(); await fireEvent.click(screen.getByRole('button', { name: /Chorus 0:03/ }));
    await waitFor(() => expect(transport.position).toBe(3));
    await fireEvent.click(screen.getByRole('button', { name: 'Loop off' }));
    await waitFor(() => expect(document.loop_enabled).toBe(true));
    await fireEvent.keyDown(window, { key: 'Enter', code: 'Enter' });
    await waitFor(() => expect(transport.position).toBe(0));
    expect(document.loop_enabled).toBe(false); expect(transport.playing).toBe(false);
    expect(document.loop_selection).not.toBeNull();
  });
  it('uses disk overrides for keyboard, buttons, hover text, and shortcut help together', async () => {
    mock.readKeymap.mockResolvedValue({ path: '/test/keymap.json', warning: null, configuration: {
      version: 1, keys: { jump_loop_start: ['q'] }, buttons: { previous_section: 'jump_loop_start' },
      descriptions: { jump_loop_start: 'Back to the riff' }
    } });
    await ready(); await fireEvent.click(screen.getByRole('button', { name: /Chorus 0:03/ }));
    await waitFor(() => expect(transport.position).toBe(3));
    mock.action.mockClear();
    await fireEvent.keyDown(window, { key: ',', code: 'Comma' });
    expect(mock.action).not.toHaveBeenCalled();
    await fireEvent.keyDown(window, { key: 'q', code: 'KeyQ' });
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'seek', seconds: 3 }));
    const previous = within(screen.getByRole('group', { name: 'Playback controls' })).getByRole('button', { name: 'Back to the riff' });
    expect(previous.getAttribute('title')).toBe('Back to the riff (Q)');
    mock.action.mockClear(); await fireEvent.click(previous);
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'seek', seconds: 3 }));
    await fireEvent.click(screen.getByRole('button', { name: 'Keyboard shortcuts' }));
    expect(screen.getByText('Back to the riff', { selector: 'dd' })).toBeTruthy();
    expect(screen.getByText('Q', { selector: 'dt' })).toBeTruthy();
  });
  it('shows a configuration warning and can reload a corrected file without restarting', async () => {
    mock.readKeymap.mockResolvedValue({ path: '/test/keymap.json', configuration: { version: 99 }, warning: null });
    await ready(); expect(screen.getByRole('alert').textContent).toContain('version 1');
    mock.readKeymap.mockResolvedValue({ path: '/test/keymap.json', configuration: { version: 1, keys: { play_pause: ['p'] } }, warning: null });
    await fireEvent.click(screen.getByRole('button', { name: 'Reload configuration' }));
    await waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
    expect(screen.getByRole('button', { name: 'Play' }).getAttribute('title')).toBe('Play / pause (P)');
    await fireEvent.keyDown(window, { key: 'p', code: 'KeyP' });
    await waitFor(() => expect(transport.playing).toBe(true));
  });
});

describe('measure-map screen integration',()=>{
  it('preserves transport and loops across screen changes and saves only an applied preview',async()=>{
    document.loop_selection={start:3,end:6,fade_ms:5};document.loop_enabled=true;transport.playing=true;transport.position=4;
    mock.initialize.mockResolvedValue({...response(),song:{key:'test',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
    mock.previewMap.mockResolvedValue(timingFixture(48000));await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
    const workspace=screen.getByRole('region',{name:'Measure mapping workspace'});expect(workspace).toBeTruthy();expect(transport.playing).toBe(true);expect(document.loop_enabled).toBe(true);
    await fireEvent.keyDown(window,{key:'x'});await fireEvent.keyDown(window,{key:'a'});expect(mock.update).not.toHaveBeenCalled();
    await fireEvent.input(screen.getByLabelText('Span end (seconds)'),{target:{value:'8'}});await fireEvent.input(screen.getByLabelText('Number of measures'),{target:{value:'2'}});
    await fireEvent.click(screen.getByRole('button',{name:'Preview measures'}));await screen.findByRole('button',{name:'Apply map'});expect(mock.update).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button',{name:'Apply map'}));await waitFor(()=>expect(document.measure_map?.measures).toHaveLength(2));expect(document.loop_enabled).toBe(true);expect(transport.playing).toBe(true);
    await fireEvent.click(screen.getByRole('button',{name:'Back to practice'}));expect(screen.getByRole('region',{name:'Measure ruler'})).toBeTruthy();
    await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));expect(screen.getByRole('button',{name:'Undo'}).hasAttribute('disabled')).toBe(false);
  });
});
