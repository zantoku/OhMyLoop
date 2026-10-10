import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import App from './App.svelte';
import { timingFixture } from './timing-fixture';
import type { PracticeDocument, Snapshot, Transport } from './types';

const mock = vi.hoisted(() => ({ initialize: vi.fn(), loadDemo: vi.fn(), openSong: vi.fn(), update: vi.fn(), action: vi.fn(), status: vi.fn(), readKeymap: vi.fn(), analysisStatus: vi.fn(), previewMap: vi.fn(), mapView: vi.fn(), metronome:vi.fn() }));
vi.mock('./bridge', () => ({ native: true, bridge: mock }));
let document: PracticeDocument;
let transport: Transport;
const response = (): Snapshot => ({ song: null, document: structuredClone(document), transport: { ...transport } });

beforeEach(() => {
  vi.clearAllMocks();
  mock.metronome.mockResolvedValue({});
  mock.analysisStatus.mockResolvedValue({id:0,running:false,kind:'idle',phase:'Ready',model_path:'/test/models',progress:null,elapsed_seconds:0,model_ready:false,song_key:null,map_revision:0,error:null,result:null});
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
    await fireEvent.click(screen.getByRole('button',{name:'Preview evenly spaced measures'}));await screen.findByRole('button',{name:'Apply map'});expect(mock.update).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button',{name:'Apply map'}));await waitFor(()=>expect(document.measure_map?.measures).toHaveLength(2));expect(document.loop_enabled).toBe(true);expect(transport.playing).toBe(true);
    await waitFor(()=>expect(screen.getByRole('button',{name:'Back to practice'}).hasAttribute('disabled')).toBe(false));await fireEvent.click(screen.getByRole('button',{name:'Back to practice'}));expect(screen.getByRole('region',{name:'Measure ruler'})).toBeTruthy();
    await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));expect(screen.getByRole('button',{name:'Undo'}).hasAttribute('disabled')).toBe(false);
  });
});

it('keeps the accepted map on a failed save and can retry the same preview',async()=>{
  mock.previewMap.mockResolvedValue(timingFixture(48000));await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  await fireEvent.click(screen.getByRole('button',{name:'Preview evenly spaced measures'}));const apply=await screen.findByRole('button',{name:'Apply map'});
  const enter=new KeyboardEvent('keydown',{key:'Enter',bubbles:true,cancelable:true});await fireEvent(apply,enter);expect(enter.defaultPrevented).toBe(false);expect(mock.action).not.toHaveBeenCalled();
  mock.update.mockRejectedValueOnce(new Error('Disk full'));await fireEvent.click(apply);await screen.findByText(/Disk full/);expect(document.measure_map).toBeUndefined();
  await fireEvent.click(screen.getByRole('button',{name:'Apply map'}));await waitFor(()=>expect(document.measure_map?.measures).toHaveLength(2));await waitFor(()=>expect(screen.queryByText(/Disk full/)).toBeNull());
});

it('polls visible analysis progress and shows the installed model folder',async()=>{
  const status={id:4,running:true,kind:'analysis',phase:'Finding beats and downbeats',model_ready:true,model_path:'/test/installed/models',progress:{completed:1,total:4,unit:'segments'},elapsed_seconds:61,song_key:'test',map_revision:0,error:null,result:null};
  mock.analysisStatus.mockResolvedValueOnce(status).mockResolvedValue({...status,progress:{completed:2,total:4,unit:'segments'},elapsed_seconds:62});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  await screen.findByText('1 / 4 segments completed · 25%');
  await screen.findByText('2 / 4 segments completed · 50%');
  await fireEvent.click(screen.getByText('Analysis model',{selector:'summary'}));
  expect(screen.getByText('/test/installed/models')).toBeTruthy();expect(screen.getByText('Installed model folder')).toBeTruthy();
});

it('uses detected beats by default, derives count from the preview, and prevents applying changed settings',async()=>{
  const result={version:1,song_key:'test',map_revision:0,rate:48000,frames:576000,model:'test',preprocessing:'test',elapsed_seconds:2,beats:Array.from({length:9},(_,i)=>({frame:i*48000,score:.8,downbeat:i%4===0}))};
  mock.analysisStatus.mockImplementation(async()=>structuredClone({id:3,running:false,kind:'analysis',phase:'Complete',model_ready:true,model_path:'/models',progress:null,elapsed_seconds:3,song_key:'test',map_revision:0,error:null,result}));
  mock.previewMap.mockResolvedValue({...timingFixture(48000),draft:{epsilon_ms:30,anchor_count:0,pulse_count:9,pulses_per_measure:4,full_measures:2,partial_pulses:0,measure_count:2,first_frame:0,last_frame:384000}});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  const source=await screen.findByLabelText('Measure source');await waitFor(()=>expect((source as HTMLSelectElement).value).toBe('detected'));
  const sidebar=screen.getByRole('complementary',{name:'Measure-map inspector'}),workspace=screen.getByRole('region',{name:'Measure mapping workspace'});
  expect(within(sidebar).getByRole('button',{name:'Analyze song'})).toBeTruthy();
  const showDetected=within(sidebar).getByRole('checkbox',{name:'Show detected beats'}) as HTMLInputElement;
  expect(showDetected.checked).toBe(true);expect(workspace.querySelectorAll('.candidate-line')).toHaveLength(9);
  expect(screen.queryByLabelText('Number of measures')).toBeNull();
  const epsilon=screen.getByRole('spinbutton',{name:'Beat timing tolerance (ms)'}) as HTMLInputElement;
  expect(epsilon.value).toBe('30');
  await fireEvent.click(screen.getByRole('button',{name:'Preview detected measures'}));await screen.findByText('9 detected pulses, including the end boundary');
  expect(showDetected.checked).toBe(false);expect(workspace.querySelectorAll('.candidate-line')).toHaveLength(0);
  await fireEvent.click(showDetected);expect(showDetected.checked).toBe(true);expect(workspace.querySelectorAll('.candidate-line')).toHaveLength(9);
  expect(document.measure_map).toBeUndefined();
  expect(screen.getByLabelText('Derived measure count').textContent).toBe('2 measures');
  expect(mock.previewMap.mock.lastCall?.[2]).toMatchObject({type:'build_from_pulses',first_downbeat:0,epsilon_ms:30,start:0,end:576000,pulses:result.beats.map(b=>b.frame)});
  expect(screen.getByText('0 timing anchors retained · ε 30 ms')).toBeTruthy();
  await fireEvent.input(epsilon,{target:{value:'15'}});
  expect(screen.getByRole('button',{name:'Apply map'}).hasAttribute('disabled')).toBe(true);
  await fireEvent.click(screen.getByRole('button',{name:'Preview detected measures'}));
  expect(mock.previewMap.mock.lastCall?.[2].epsilon_ms).toBe(15);
  await fireEvent.input(epsilon,{target:{value:''}});
  const calls=mock.previewMap.mock.calls.length;
  await fireEvent.click(screen.getByRole('button',{name:'Preview detected measures'}));
  await screen.findByText('Error: Beat timing tolerance must be between 0 and 1000 ms.');
  expect(mock.previewMap.mock.calls).toHaveLength(calls);expect(document.measure_map).toBeUndefined();
  await fireEvent.input(epsilon,{target:{value:'0'}});
  await fireEvent.click(screen.getByRole('button',{name:'Preview detected measures'}));
  expect(mock.previewMap.mock.lastCall?.[2].epsilon_ms).toBe(0);
  await fireEvent.input(screen.getByLabelText('Draft Meter numerator'),{target:{value:'3'}});
  expect(screen.getByRole('button',{name:'Apply map'}).hasAttribute('disabled')).toBe(true);expect(screen.getByText('Settings changed. Preview again before applying.')).toBeTruthy();
  const regrouped=timingFixture(48000);for(const m of [...regrouped.map.measures,...regrouped.view.measures]){m.meter={...m.meter,numerator:3};m.length={num:3,den:1};}
  mock.previewMap.mockResolvedValue({...regrouped,draft:{epsilon_ms:30,anchor_count:0,pulse_count:9,pulses_per_measure:3,full_measures:2,partial_pulses:2,measure_count:3,first_frame:0,last_frame:384000}});
  await fireEvent.click(screen.getByRole('button',{name:'Preview detected measures'}));expect(mock.previewMap.mock.lastCall?.[2].meter.numerator).toBe(3);
  await waitFor(()=>expect((screen.getByRole('spinbutton',{name:'Meter numerator'}) as HTMLInputElement).value).toBe('3'));
  await fireEvent.change(source,{target:{value:'manual'}});expect(screen.getByLabelText('Number of measures')).toBeTruthy();
  expect(screen.getByText(/This mode does not use detected beats/)).toBeTruthy();
  await fireEvent.click(screen.getByRole('button',{name:'Back to practice'}));await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  await waitFor(()=>expect(mock.analysisStatus.mock.calls.length).toBeGreaterThan(1));expect((screen.getByLabelText('Measure source') as HTMLSelectElement).value).toBe('manual');
  expect(showDetected.checked).toBe(true);
  await fireEvent.change(source,{target:{value:'detected'}});
  expect((screen.getByRole('spinbutton',{name:'Beat timing tolerance (ms)'}) as HTMLInputElement).value).toBe('0');
});

it('keeps compact playback over the waveform without turning its controls into seeks',async()=>{
  document.controls={speed:.75,semitones:-3};document.loop_selection={start:3,end:6,fade_ms:5};document.loop_enabled=true;
  mock.initialize.mockResolvedValue({...response(),song:{key:'test',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  const waveform=screen.getByRole('region',{name:/Measure-map waveform/}),playback=within(waveform).getByRole('group',{name:'Measure mapping playback'});
  expect(within(screen.getByRole('region',{name:'Measure mapping workspace'})).queryByLabelText('Pitch semitones')).toBeNull();
  expect(within(playback).queryByRole('slider')).toBeNull();
  const speedInput=within(playback).getByRole('spinbutton',{name:'Speed percent'});
  for(const type of ['pointerdown','pointerup']){const event=new MouseEvent(type,{bubbles:true,button:0});Object.defineProperty(event,'pointerId',{value:1});await fireEvent(speedInput,event);}
  await fireEvent.change(speedInput,{target:{value:'50'}});
  await waitFor(()=>expect(document.controls).toEqual({speed:.5,semitones:-3}));
  await fireEvent.click(within(playback).getByRole('button',{name:'Play'}));await waitFor(()=>expect(transport.playing).toBe(true));
  expect(mock.action).toHaveBeenCalledExactlyOnceWith({type:'toggle_play'});
  const wheel=new WheelEvent('wheel',{bubbles:true,cancelable:true,deltaY:-100});await fireEvent(speedInput,wheel);expect(wheel.defaultPrevented).toBe(false);
  await fireEvent.click(within(playback).getByRole('button',{name:'Turn looping off'}));await waitFor(()=>expect(document.loop_enabled).toBe(false));
});

it('edits measure timing and anchors in the panel below the waveform',async()=>{
  const original=timingFixture(48000);document.measure_map=original.map;
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  const moved=structuredClone(original);moved.map.boundaries[0].frame=12000;moved.map.revision=2;moved.view.revision=2;moved.view.measures[0].start=12000;
  mock.previewMap.mockResolvedValue(moved);
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  const tools=screen.getByRole('region',{name:'Measure editing tools'});expect(tools.closest('aside')).toBeNull();
  expect(within(screen.getByRole('complementary',{name:'Measure-map inspector'})).queryByLabelText('Position (seconds)')).toBeNull();
  await fireEvent.input(within(tools).getByLabelText('Position (seconds)'),{target:{value:'.25'}});
  await fireEvent.click(within(tools).getByRole('button',{name:'Move boundary'}));
  await waitFor(()=>expect(document.measure_map?.boundaries[0].frame).toBe(12000));
  expect(mock.previewMap.mock.lastCall?.[2]).toEqual({type:'move_boundary',id:'map-1',frame:12000});
  await fireEvent.click(within(tools).getByRole('button',{name:'Beat anchors and splits'}));
  const anchored=structuredClone(moved);anchored.map.revision=3;anchored.view.revision=3;anchored.map.measures[0].beats=[{id:'map-6',frame:57600,offset:{num:1,den:1}}];anchored.map.next_id=7;
  mock.previewMap.mockResolvedValue(anchored);
  await fireEvent.input(within(tools).getByLabelText('Anchor position (seconds)'),{target:{value:'1.2'}});
  await fireEvent.click(within(tools).getByRole('button',{name:'Add anchor'}));
  await waitFor(()=>expect(document.measure_map?.measures[0].beats[0].frame).toBe(57600));
  expect(mock.previewMap.mock.lastCall?.[2]).toEqual({type:'set_beat',measure:'map-3',id:null,frame:57600,offset:{num:1,den:1}});
});
it.each([
  {playing:false,draft:false},{playing:true,draft:false},{playing:false,draft:true}
])('seeks to a manually selected measure with playing=$playing and draft=$draft',async({playing,draft})=>{
  const original=timingFixture(48000),preview=structuredClone(original);
  preview.map.boundaries[1].frame=252000;preview.map.revision=2;
  preview.view.measures[0].end=252000;preview.view.measures[1].start=252000;preview.view.revision=2;
  document.measure_map=structuredClone(original.map);document.controls={speed:.75,semitones:-3};
  document.loop_selection={start:1,end:3,fade_ms:5};document.loop_enabled=true;
  transport.playing=playing;
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  mock.previewMap.mockResolvedValue(preview);
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  if(draft){await fireEvent.click(screen.getByRole('button',{name:'Preview numbering'}));await screen.findByRole('button',{name:'Apply map'});}
  expect(mock.action).not.toHaveBeenCalled();
  const lane=screen.getByLabelText('Measures on timeline'),measure=within(lane).getByRole('button',{name:'2 4/4'});
  await fireEvent.click(measure);
  const seconds=draft?5.25:4;
  await waitFor(()=>expect(mock.action).toHaveBeenCalledExactlyOnceWith({type:'seek',seconds}));
  await waitFor(()=>expect(screen.getByRole('slider',{name:'Measure-map song position'}).getAttribute('aria-valuenow')).toBe(String(seconds)));
  expect(measure.classList.contains('selected')).toBe(true);
  expect(transport.playing).toBe(playing);expect(document.loop_enabled).toBe(false);
  expect(document.loop_selection).toEqual({start:1,end:3,fade_ms:5});
  expect(document.controls).toEqual({speed:.75,semitones:-3});expect(document.measure_map).toEqual(original.map);
  expect(screen.getByRole('button',{name:'Follow playhead on'}).getAttribute('aria-pressed')).toBe('true');
  await fireEvent.keyDown(window,{key:'Enter',code:'Enter'});
  await waitFor(()=>expect(transport.position).toBe(0));
  await fireEvent.click(measure);await waitFor(()=>expect(transport.position).toBe(seconds));
});

it.each([false,true])('selects the measure under a waveform click without snapping time with draft=%s',async(draft)=>{
  const original=timingFixture(48000),preview=structuredClone(original);
  preview.map.boundaries[1].frame=336000;preview.map.revision=2;
  preview.view.measures[0].end=336000;preview.view.measures[1].start=336000;preview.view.revision=2;
  document.measure_map=structuredClone(original.map);
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  mock.previewMap.mockResolvedValue(preview);
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  if(draft){await fireEvent.click(screen.getByRole('button',{name:'Preview numbering'}));await screen.findByRole('button',{name:'Apply map'});}
  const waveform=screen.getByRole('region',{name:/Measure-map waveform/});
  vi.spyOn(waveform,'getBoundingClientRect').mockReturnValue({left:0,width:1000,height:200,top:0,right:1000,bottom:200,x:0,y:0,toJSON(){}});
  waveform.setPointerCapture=vi.fn();waveform.hasPointerCapture=vi.fn(()=>false);
  for(const type of ['pointerdown','pointerup']){const event=new MouseEvent(type,{bubbles:true,clientX:500,button:0});Object.defineProperty(event,'pointerId',{value:1});await fireEvent(waveform,event);}
  await waitFor(()=>expect(mock.action).toHaveBeenCalledExactlyOnceWith({type:'seek',seconds:6}));
  const lane=screen.getByLabelText('Measures on timeline');
  expect(within(lane).getByRole('button',{name:draft?'1 4/4':'2 4/4',pressed:true})).toBeTruthy();
  expect(screen.getByRole('heading',{name:draft?'Measure 1':'Measure 2'})).toBeTruthy();
  expect(document.measure_map).toEqual(original.map);expect(transport.playing).toBe(false);
});

it('selects the playing measure independently of viewport following, including pickups, gaps and backwards wraps',async()=>{
  const original=timingFixture(48000);
  Object.assign(original.map.measures[0],{kind:'pickup',length:{num:1,den:1},offset:{num:3,den:1}});
  original.map.boundaries[1].frame=96000;
  original.map.boundaries.push({id:'map-6',frame:192000});original.map.measures[1].start='map-6';original.map.next_id=7;
  Object.assign(original.view.measures[0],{kind:'pickup',length:{num:1,den:1},label:'Pickup',number:null,end:96000,grid:[]});
  Object.assign(original.view.measures[1],{start_id:'map-6',label:'1',number:1});
  document.measure_map=structuredClone(original.map);transport.playing=true;
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  await fireEvent.click(screen.getByRole('button',{name:'Follow playhead on'}));
  const lane=screen.getByLabelText('Measures on timeline');
  expect(within(lane).getByRole('button',{name:/Pickup/,pressed:true})).toBeTruthy();
  transport.position=2;
  await waitFor(()=>expect(within(lane).queryByRole('button',{pressed:true})).toBeNull());
  transport.position=4;
  await waitFor(()=>expect(within(lane).getByRole('button',{name:'1 4/4',pressed:true})).toBeTruthy());
  transport.position=8;
  await waitFor(()=>expect(within(lane).queryByRole('button',{pressed:true})).toBeNull());
  transport.position=.5;
  await waitFor(()=>expect(within(lane).getByRole('button',{name:/Pickup/,pressed:true})).toBeTruthy());
  expect(mock.action).not.toHaveBeenCalled();
  expect(screen.getByRole('button',{name:'Follow playhead off'}).getAttribute('aria-pressed')).toBe('false');
  await fireEvent.click(screen.getByRole('button',{name:'Pause'}));await waitFor(()=>expect(transport.playing).toBe(false));
  await fireEvent.click(within(lane).getByRole('button',{name:'1 4/4'}));await waitFor(()=>expect(transport.position).toBe(4));
  transport.position=1;
  await within(screen.getByRole('group',{name:'Measure mapping playback'})).findByText('0:01.00');
  expect(within(lane).getByRole('button',{name:'1 4/4',pressed:true})).toBeTruthy();
});

it('resizes a shared boundary as one undoable edit that restores both measures anchors',async()=>{
  const original=timingFixture(48000);
  original.map.measures[0].beats=[{id:'map-6',frame:48000,offset:{num:1,den:1}}];
  original.map.measures[1].beats=[{id:'map-7',frame:240000,offset:{num:2,den:1}}];original.map.next_id=8;
  const resized=structuredClone(original);resized.map.revision=2;resized.view.revision=2;resized.map.boundaries[1].frame=288000;
  resized.view.measures[0].end=288000;resized.view.measures[1].start=288000;for(const m of resized.map.measures)m.beats=[];
  document.measure_map=structuredClone(original.map);
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  mock.previewMap.mockResolvedValue(resized);mock.mapView.mockResolvedValue({...original.view,revision:3});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  const waveform=screen.getByRole('region',{name:/Measure-map waveform/});
  vi.spyOn(waveform,'getBoundingClientRect').mockReturnValue({left:0,width:1000,height:200,top:0,right:1000,bottom:200,x:0,y:0,toJSON(){}});
  waveform.setPointerCapture=vi.fn();waveform.hasPointerCapture=vi.fn(()=>false);
  for(const [target,type,x] of [[screen.getByRole('button',{name:'Measure 2 start'}),'pointerdown',1000/3],[waveform,'pointerup',500]] as const){
    const event=new MouseEvent(type,{bubbles:true,cancelable:true,clientX:x,button:0});Object.defineProperty(event,'pointerId',{value:1});await fireEvent(target,event);
  }
  await waitFor(()=>expect(document.measure_map?.measures.every(m=>!m.beats.length)).toBe(true));
  expect(mock.previewMap).toHaveBeenCalledExactlyOnceWith('test',original.map,{type:'move_boundary',id:'map-2',frame:288000});
  expect(mock.action).not.toHaveBeenCalled();
  await waitFor(()=>expect(screen.getByRole('button',{name:'Undo'}).hasAttribute('disabled')).toBe(false));
  await fireEvent.click(screen.getByRole('button',{name:'Undo'}));
  await waitFor(()=>expect(document.measure_map?.measures).toEqual(original.map.measures));
  expect(document.measure_map?.boundaries).toEqual(original.map.boundaries);
});

it('clears a protected map, keeps practice settings, and restores the exact map through Undo',async()=>{
  const original=timingFixture(48000);original.map.measures[0].protected=true;
  document.measure_map=structuredClone(original.map);document.controls={speed:.75,semitones:-3};document.loop_selection={start:3,end:6,fade_ms:5};document.loop_enabled=true;
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  const empty={...original,map:{...original.map,revision:2,first_number:1,measures:[],boundaries:[]},view:{revision:2,measures:[]}};
  mock.previewMap.mockResolvedValue(empty);mock.mapView.mockResolvedValue({...original.view,revision:3});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));await fireEvent.click(screen.getByRole('button',{name:'Clear map'}));
  await waitFor(()=>expect(document.measure_map?.measures).toHaveLength(0));expect(mock.previewMap.mock.lastCall?.[2]).toEqual({type:'clear'});
  await waitFor(()=>expect(screen.getByRole('button',{name:'Undo'}).hasAttribute('disabled')).toBe(false));await fireEvent.click(screen.getByRole('button',{name:'Undo'}));
  await waitFor(()=>expect(document.measure_map?.measures).toEqual(original.map.measures));expect(document.measure_map?.boundaries).toEqual(original.map.boundaries);
  expect(document.controls).toEqual({speed:.75,semitones:-3});expect(document.loop_enabled).toBe(true);expect(document.loop_selection).toEqual({start:3,end:6,fade_ms:5});expect(document.sections[0].id).toBe('verse');
});

it('auditions the displayed draft, reverts on discard, and disables clicks when returning to practice',async()=>{
  const original=timingFixture(48000),draft=structuredClone(original);draft.map.boundaries[1].frame=3*48000;draft.map.revision=2;draft.view.revision=2;
  document.measure_map=structuredClone(original.map);
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  mock.previewMap.mockResolvedValue(draft);
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  expect(screen.getByRole('button',{name:'Metronome on'}).getAttribute('aria-pressed')).toBe('true');
  expect(screen.getByRole('button',{name:'Follow playhead on'}).getAttribute('aria-pressed')).toBe('true');
  await waitFor(()=>expect(mock.metronome.mock.lastCall).toEqual(['test',true,.5,original.map]));
  await fireEvent.click(screen.getByRole('button',{name:'Metronome on'}));
  await waitFor(()=>expect(mock.metronome.mock.lastCall).toEqual(['test',false,.5,original.map]));
  await fireEvent.click(screen.getByRole('button',{name:'Metronome off'}));
  await waitFor(()=>expect(mock.metronome.mock.lastCall).toEqual(['test',true,.5,original.map]));
  await fireEvent.click(screen.getByRole('button',{name:'Preview numbering'}));
  await screen.findByRole('button',{name:'Apply map'});
  await waitFor(()=>expect(mock.metronome.mock.lastCall).toEqual(['test',true,.5,draft.map]));
  expect(document.measure_map).toEqual(original.map);
  await fireEvent.click(screen.getByRole('button',{name:'Discard'}));
  await waitFor(()=>expect(mock.metronome.mock.lastCall?.[3]).toEqual(original.map));
  await fireEvent.input(screen.getByLabelText('Metronome volume'),{target:{value:'80'}});
  await waitFor(()=>expect(mock.metronome.mock.lastCall?.[2]).toBe(.8));
  transport.audio_error='Output disconnected';
  await waitFor(()=>expect(mock.metronome.mock.lastCall).toEqual(['test',false,.8,original.map]));
  transport.audio_error=null;
  await waitFor(()=>expect(mock.metronome.mock.lastCall).toEqual(['test',true,.8,original.map]));
  await fireEvent.click(screen.getByRole('button',{name:'Back to practice'}));
  await waitFor(()=>expect(mock.metronome.mock.lastCall).toEqual(['test',false,.8,null]));
  expect(transport.playing).toBe(false);expect(document.measure_map).toEqual(original.map);
});

it('keeps following through navigation and pages until explicitly toggled off',async()=>{
  const fixture=timingFixture(48000);document.measure_map=fixture.map;
  mock.initialize.mockResolvedValue({...response(),map_view:fixture.view,song:{key:'test',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  await fireEvent.click(screen.getByRole('button',{name:'Fit measure'}));
  const viewport=documentForViewport();
  expect(screen.getByRole('button',{name:'Follow playhead on'}).getAttribute('aria-pressed')).toBe('true');
  await fireEvent.click(screen.getByRole('button',{name:'Whole song'}));
  await fireEvent.click(screen.getByRole('button',{name:'Fit measure'}));
  expect(screen.getByRole('button',{name:'Follow playhead on'}).getAttribute('aria-pressed')).toBe('true');
  await fireEvent(screen.getByRole('region',{name:/Measure-map waveform/}),new WheelEvent('wheel',{bubbles:true,cancelable:true,deltaY:-60,clientX:0}));
  expect(screen.getByRole('button',{name:'Follow playhead on'}).getAttribute('aria-pressed')).toBe('true');
  const width=parseFloat(viewport.style.width);
  const waveform=screen.getByRole('region',{name:/Measure-map waveform/});
  vi.spyOn(waveform,'getBoundingClientRect').mockReturnValue({left:0,width:1000,height:200,top:0,right:1000,bottom:200,x:0,y:0,toJSON(){}});
  waveform.setPointerCapture=vi.fn();waveform.hasPointerCapture=vi.fn(()=>false);
  for(const [type,x] of [['pointerdown',500],['pointermove',300],['pointerup',300]] as const){
    const event=new MouseEvent(type,{bubbles:true,cancelable:true,clientX:x,button:1});Object.defineProperty(event,'pointerId',{value:1});await fireEvent(waveform,event);
  }
  expect(parseFloat(viewport.style.left)).toBeGreaterThan(0);
  expect(screen.getByRole('button',{name:'Follow playhead on'}).getAttribute('aria-pressed')).toBe('true');
  await fireEvent.click(screen.getByRole('button',{name:'Play'}));
  const initial=viewport.style.left;transport.position=5;
  await waitFor(()=>expect(viewport.style.left).not.toBe(initial));
  expect(parseFloat(viewport.style.width)).toBeCloseTo(width);
  transport.position=.5;await waitFor(()=>expect(viewport.style.left).toBe('0%'));
  await fireEvent.click(screen.getByRole('button',{name:'Move measure-map view'}),{clientX:300});
  expect(screen.getByRole('button',{name:'Follow playhead on'}).getAttribute('aria-pressed')).toBe('true');
  await fireEvent.click(screen.getByRole('button',{name:'Follow playhead on'}));
  await fireEvent.click(screen.getByRole('button',{name:'Fit measure'}));
  await fireEvent.click(screen.getByRole('button',{name:'Move measure-map view'}),{clientX:300});
  const manual=viewport.style.left;transport.position=1;
  await within(screen.getByRole('group',{name:'Measure mapping playback'})).findByText('0:01.00');
  expect(viewport.style.left).toBe(manual);
  expect(screen.getByRole('button',{name:'Follow playhead off'}).getAttribute('aria-pressed')).toBe('false');
  await fireEvent.click(screen.getByRole('button',{name:'Follow playhead off'}));
  await waitFor(()=>expect(viewport.style.left).not.toBe(manual));
});
function documentForViewport(){return screen.getByRole('button',{name:'Move measure-map view'}).querySelector('.map-viewport') as HTMLElement;}

it.each([
  {context:'mapping',playing:false},{context:'mapping',playing:true},
  {context:'practice',playing:false},{context:'practice',playing:true}
])('reveals keyboard seeks in $context while playing=$playing, preserving zoom and manual follow state',async({context,playing})=>{
  await ready();
  if(context==='mapping')await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  const waveform=screen.getByRole('region',{name:context==='mapping'?/Measure-map waveform/:/^Waveform\./});
  const box={left:0,width:1000,height:200,top:0,right:1000,bottom:200,x:0,y:0,toJSON(){}};
  vi.spyOn(waveform,'getBoundingClientRect').mockReturnValue(box);
  for(let i=0;i<2;i++)await fireEvent(waveform,new WheelEvent('wheel',{bubbles:true,cancelable:true,deltaY:-240,clientX:0}));
  const overview=screen.getByRole('button',{name:context==='mapping'?'Move measure-map view':'Move waveform view'});
  vi.spyOn(overview,'getBoundingClientRect').mockReturnValue(box);
  const viewport=overview.querySelector(context==='mapping'?'.map-viewport':'.viewport') as HTMLElement;
  const width=parseFloat(viewport.style.width);
  if(context==='mapping')await fireEvent.click(screen.getByRole('button',{name:'Follow playhead on'}));
  if(playing){await fireEvent.keyDown(window,{key:' ',code:'Space'});await waitFor(()=>expect(transport.playing).toBe(true));}
  await fireEvent.keyDown(window,{key:'ArrowRight',code:'ArrowRight'});
  await waitFor(()=>expect(transport.position).toBe(5));await waitFor(()=>expect(parseFloat(viewport.style.left)).toBeGreaterThan(0));
  const firstPage=parseFloat(viewport.style.left);
  await fireEvent.keyDown(window,{key:'ArrowRight',code:'ArrowRight'});
  await waitFor(()=>expect(transport.position).toBe(10));await waitFor(()=>expect(parseFloat(viewport.style.left)).toBeGreaterThan(firstPage));
  await fireEvent.keyDown(window,{key:'ArrowLeft',code:'ArrowLeft'});
  await waitFor(()=>expect(transport.position).toBe(5));await waitFor(()=>expect(parseFloat(viewport.style.left)).toBeCloseTo(firstPage));
  await fireEvent.keyDown(window,{key:'Enter',code:'Enter'});
  await waitFor(()=>expect(transport.position).toBe(0));await waitFor(()=>expect(viewport.style.left).toBe('0%'));
  // Rewind must also reveal zero when the playhead is already there and the
  // user has manually panned elsewhere with continuous following toggled off.
  await fireEvent.click(overview,{clientX:300});expect(parseFloat(viewport.style.left)).toBeGreaterThan(0);
  if(context==='mapping')expect(screen.getByRole('button',{name:'Follow playhead off'}).getAttribute('aria-pressed')).toBe('false');
  await fireEvent.keyDown(window,{key:'Enter',code:'Enter'});
  await waitFor(()=>expect(viewport.style.left).toBe('0%'));
  if(context==='mapping')expect(screen.getByRole('button',{name:'Follow playhead off'}).getAttribute('aria-pressed')).toBe('false');
  expect(parseFloat(viewport.style.width)).toBeCloseTo(width);expect(transport.playing).toBe(playing);
});
