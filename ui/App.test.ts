import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import App from './App.svelte';
import { timingFixture } from './timing-fixture';
import type { PracticeDocument, Snapshot, Transport } from './types';

const mock = vi.hoisted(() => ({ initialize: vi.fn(), loadDemo: vi.fn(), openSong: vi.fn(),importSeparate:vi.fn(),replaceRecording:vi.fn(),libraryReference:vi.fn(),replaceLibraryReference:vi.fn(), update: vi.fn(), action: vi.fn(), status: vi.fn(), readKeymap: vi.fn(), analysisStatus: vi.fn(), readCachedAnalysis:vi.fn(), readLibrary:vi.fn(),openLibrarySong:vi.fn(),updateLibrarySong:vi.fn(), onExitRequested:vi.fn(),finishExit:vi.fn(), previewMap: vi.fn(), mapView: vi.fn(), countIn:vi.fn(), metronome:vi.fn() }));
vi.mock('./bridge', () => ({ native: true, bridge: mock }));
let document: PracticeDocument;
let transport: Transport;
const response = (): Snapshot => ({ song: null, document: structuredClone(document), transport: { ...transport } });

beforeEach(() => {
  vi.clearAllMocks();
  mock.metronome.mockResolvedValue({});
  mock.countIn.mockResolvedValue({});
  mock.onExitRequested.mockResolvedValue(()=>{});mock.finishExit.mockResolvedValue(undefined);
  mock.readCachedAnalysis.mockResolvedValue(null);
  mock.readLibrary.mockResolvedValue({version:1,songs:[]});
  mock.analysisStatus.mockResolvedValue({id:0,running:false,kind:'idle',phase:'Ready',model_path:'/test/models',progress:null,elapsed_seconds:0,model_ready:false,reference_token:null,map_revision:0,error:null,result:null});
  mock.readKeymap.mockResolvedValue({ path: '/test/keymap.json', configuration: null, warning: null });
  document = { version: 4, notes:'', song_key: 'test',reference_token:'test', controls: { speed: 1, semitones: 0 },
    sections: [{ id: 'verse', name: 'Verse', start: 0, end: 8 }, { id: 'chorus', name: 'Chorus', start: 3, end: 6 }],
    selected_section: null, loop_selection: null, loop_enabled: false, last_position: 0 };
  transport = { song_key: 'test',reference_token:'test', position: 0, playing: false, finished: false, audio_error: null, save_error: null, device: 'Test output' };
  mock.initialize.mockResolvedValue({ ...response(), song: { key: 'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session', title: 'Workshop take', path: '/test/song.wav', duration: 12, rate: 48000, peaks: [[-.2, .2]] } });
  mock.update.mockImplementation(async (value: PracticeDocument) => { document = structuredClone(value); return response(); });
  mock.action.mockImplementation(async (action) => {
    if (action.type === 'seek') {
      transport.position = action.seconds; document.last_position = action.seconds;
      const loop = document.loop_selection;
      if (document.loop_enabled && loop && (action.seconds < loop.start || action.seconds >= loop.end)) document.loop_enabled = false;
    } else if (action.type === 'toggle_play') transport.playing = !transport.playing;
    else if(action.type==='pause')transport.playing=false;
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
  it('keeps the appearance toggle next to shortcut help in the bottom bar', async () => {
    await ready();
    const footer = screen.getByRole('contentinfo');
    const appearance = within(footer).getByRole('button', { name: /^UI: / });
    const shortcuts = within(footer).getByRole('button', { name: /^Shortcuts/ });
    expect(appearance.closest('.statusbar-tools')).toBe(shortcuts.parentElement);
    await fireEvent.click(appearance);
    expect(mock.action).not.toHaveBeenCalled();
    expect(mock.update).not.toHaveBeenCalled();
  });
  it('can save exact dragged boundaries without browser step-validation errors', async () => {
    document.loop_selection = { start: 1.851590106007067, end: 3.5053003533568905, fade_ms: 5 };
    mock.initialize.mockResolvedValue({ ...response(), song: { key: 'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session', title: 'Workshop take', path: '/test/song.wav', duration: 12, rate: 48000, peaks: [[-.2, .2]] } });
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
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'seek', seconds: 3 },'test'));
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
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'seek', seconds: 3 },'test'));
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
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'toggle_play' },'test'));
  });
  it.each([false, true])('jumps to the precise A marker with comma while preserving active state (%s)', async active => {
    document.loop_selection = { start: 2.3456789, end: 5.6789, fade_ms: 5 };
    document.loop_enabled = active; document.selected_section = 'verse';
    transport.playing = active; transport.position = 9;
    const before = structuredClone(document);
    mock.initialize.mockResolvedValue({ ...response(), song: { key: 'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session', title: 'Workshop take', path: '/test/song.wav', duration: 12, rate: 48000, peaks: [[-.2, .2]] } });
    await ready(); await fireEvent.keyDown(window, { key: ',', code: 'Comma' });
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'seek', seconds: 2.3456789 },'test'));
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
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'seek', seconds: 3 },'test'));
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
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'seek', seconds: 3 },'test'));
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
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'seek', seconds: 3 },'test'));
    const previous = within(screen.getByRole('group', { name: 'Playback controls' })).getByRole('button', { name: 'Back to the riff' });
    expect(previous.getAttribute('title')).toBe('Back to the riff (Q)');
    mock.action.mockClear(); await fireEvent.click(previous);
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'seek', seconds: 3 },'test'));
    await fireEvent.click(screen.getByRole('button', { name: /^Shortcuts/ }));
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
    mock.initialize.mockResolvedValue({...response(),song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
    mock.previewMap.mockResolvedValue(timingFixture(48000));await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
    const workspace=screen.getByRole('region',{name:'Measure mapping workspace'});expect(workspace).toBeTruthy();expect(transport.playing).toBe(true);expect(document.loop_enabled).toBe(true);
    await fireEvent.keyDown(window,{key:'x'});await fireEvent.keyDown(window,{key:'a'});expect(mock.update).not.toHaveBeenCalled();
    await fireEvent.input(screen.getByLabelText('Span end (seconds)'),{target:{value:'8'}});await fireEvent.input(screen.getByLabelText('Number of measures'),{target:{value:'2'}});
    await fireEvent.click(screen.getByRole('button',{name:'Preview evenly spaced measures'}));await screen.findByRole('button',{name:'Apply map'});expect(mock.update).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button',{name:'Apply map'}));await waitFor(()=>expect(document.measure_map?.measures).toHaveLength(2));expect(document.loop_enabled).toBe(true);expect(transport.playing).toBe(true);
    await waitFor(()=>expect(screen.getByRole('button',{name:'Close measure map'}).hasAttribute('disabled')).toBe(false));await fireEvent.click(screen.getByRole('button',{name:'Close measure map'}));expect(screen.getByRole('region',{name:'Measure ruler'})).toBeTruthy();
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
  const status={id:4,running:true,kind:'analysis',phase:'Finding beats and downbeats',model_ready:true,model_path:'/test/installed/models',progress:{completed:1,total:4,unit:'segments'},elapsed_seconds:61,reference_token:'test',map_revision:0,error:null,result:null};
  mock.analysisStatus.mockResolvedValueOnce(status).mockResolvedValue({...status,progress:{completed:2,total:4,unit:'segments'},elapsed_seconds:62});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  await screen.findByText('1 / 4 segments completed · 25%');
  await screen.findByText('2 / 4 segments completed · 50%');
  await fireEvent.click(screen.getByText('Analysis model',{selector:'summary'}));
  expect(screen.getByText('/test/installed/models')).toBeTruthy();expect(screen.getByText('Installed model folder')).toBeTruthy();
});

it('uses detected beats by default, derives count from the preview, and prevents applying changed settings',async()=>{
  const result={version:2,reference_token:'test',map_revision:0,rate:48000,frames:576000,model:'test',preprocessing:'test',elapsed_seconds:2,beats:Array.from({length:9},(_,i)=>({frame:i*48000,score:.8,downbeat:i%4===0}))};
  mock.analysisStatus.mockImplementation(async()=>structuredClone({id:3,running:false,kind:'analysis',phase:'Complete',model_ready:true,model_path:'/models',progress:null,elapsed_seconds:3,reference_token:'test',map_revision:0,error:null,result}));
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
  await fireEvent.click(screen.getByRole('button',{name:'Close measure map'}));await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  await waitFor(()=>expect(mock.analysisStatus.mock.calls.length).toBeGreaterThan(1));expect((screen.getByLabelText('Measure source') as HTMLSelectElement).value).toBe('manual');
  expect(showDetected.checked).toBe(true);
  await fireEvent.change(source,{target:{value:'detected'}});
  expect((screen.getByRole('spinbutton',{name:'Beat timing tolerance (ms)'}) as HTMLInputElement).value).toBe('0');
});

it('keeps compact playback over the waveform without turning its controls into seeks',async()=>{
  document.controls={speed:.75,semitones:-3};document.loop_selection={start:3,end:6,fade_ms:5};document.loop_enabled=true;
  mock.initialize.mockResolvedValue({...response(),song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  const waveform=screen.getByRole('region',{name:/Measure-map waveform/}),playback=within(waveform).getByRole('group',{name:'Measure mapping playback'});
  expect(within(screen.getByRole('region',{name:'Measure mapping workspace'})).queryByLabelText('Pitch semitones')).toBeNull();
  expect(within(playback).queryByRole('slider')).toBeNull();
  const speedInput=within(playback).getByRole('spinbutton',{name:'Speed percent'});
  for(const type of ['pointerdown','pointerup']){const event=new MouseEvent(type,{bubbles:true,button:0});Object.defineProperty(event,'pointerId',{value:1});await fireEvent(speedInput,event);}
  await fireEvent.change(speedInput,{target:{value:'50'}});
  await waitFor(()=>expect(document.controls).toEqual({speed:.5,semitones:-3}));
  await fireEvent.click(within(playback).getByRole('button',{name:'Play'}));await waitFor(()=>expect(transport.playing).toBe(true));
  expect(mock.action).toHaveBeenCalledExactlyOnceWith({type:'toggle_play'},'test');
  const wheel=new WheelEvent('wheel',{bubbles:true,cancelable:true,deltaY:-100});await fireEvent(speedInput,wheel);expect(wheel.defaultPrevented).toBe(false);
  await fireEvent.click(within(playback).getByRole('button',{name:'Turn looping off'}));await waitFor(()=>expect(document.loop_enabled).toBe(false));
});

it('edits measure timing and anchors in the panel below the waveform',async()=>{
  const original=timingFixture(48000);document.measure_map=original.map;
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  const moved=structuredClone(original);moved.map.boundaries[0].frame=12000;moved.map.revision=2;moved.view.revision=2;moved.view.measures[0].start=12000;
  mock.previewMap.mockResolvedValue(moved);
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  const tools=screen.getByRole('region',{name:'Measure editing tools'});expect(tools.closest('aside')).toBeNull();
  expect(within(screen.getByRole('complementary',{name:'Measure-map inspector'})).queryByLabelText('Position (seconds)')).toBeNull();
  await fireEvent.click(within(tools).getByText('Advanced',{selector:'summary'}));
  await fireEvent.input(within(tools).getByLabelText('Position (seconds)'),{target:{value:'.25'}});
  await fireEvent.click(within(tools).getByRole('button',{name:'Move boundary'}));
  await waitFor(()=>expect(document.measure_map?.boundaries[0].frame).toBe(12000));
  expect(mock.previewMap.mock.lastCall?.[2]).toEqual({type:'move_boundary',id:'map-1',frame:12000});
  await fireEvent.click(within(tools).getByRole('button',{name:'Join / Split / Shift'}));
  const anchored=structuredClone(moved);anchored.map.revision=3;anchored.view.revision=3;anchored.map.measures[0].beats=[{id:'map-6',frame:57600,offset:{num:1,den:1}}];anchored.map.next_id=7;
  await fireEvent.click(within(tools).getByText('Advanced — beat anchor positions',{selector:'summary'}));
  mock.previewMap.mockResolvedValue(anchored);
  await fireEvent.input(within(tools).getByLabelText('Anchor position (seconds)'),{target:{value:'1.2'}});
  await fireEvent.click(within(tools).getByRole('button',{name:'Add anchor'}));
  await waitFor(()=>expect(document.measure_map?.measures[0].beats[0].frame).toBe(57600));
  expect(mock.previewMap.mock.lastCall?.[2]).toEqual({type:'set_beat',measure:'map-3',id:null,frame:57600,offset:{num:1,den:1}});
});
it('keeps the Measure panel compact and moves global numbering below Create measures',async()=>{
  const original=timingFixture(48000);document.measure_map=structuredClone(original.map);
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  const tools=screen.getByRole('region',{name:'Measure editing tools'}),sidebar=screen.getByRole('complementary',{name:'Measure-map inspector'});
  expect(within(tools).getByRole('spinbutton',{name:'Meter numerator'})).toBeTruthy();
  expect(within(tools).getByRole('spinbutton',{name:'Meter denominator'})).toBeTruthy();
  const advanced=within(tools).getByText('Advanced',{selector:'summary'}).closest('details')!;
  expect(advanced.open).toBe(false);
  for(const name of ['Pulse note value','Beat grouping (optional)','Position (seconds)'])expect(within(advanced).getByLabelText(name).closest('details')).toBe(advanced);
  expect(within(tools).queryByLabelText('First full measure number')).toBeNull();
  const batch=within(sidebar).getByText('Batch tools',{selector:'summary'}).closest('details')!;
  expect(batch.previousElementSibling?.querySelector('summary')?.textContent).toBe('Create measures');
  expect(within(batch).getByRole('spinbutton',{name:'First full measure number'})).toBeTruthy();
  expect(within(batch).getByRole('combobox',{name:'Time signature'})).toBeTruthy();
  expect(within(batch).getByRole('button',{name:'Preview grouping'}).hasAttribute('disabled')).toBe(true);
  await fireEvent.click(within(tools).getByText('Advanced',{selector:'summary'}));
  expect(within(tools).getByRole('combobox',{name:'Pulse note value'})).toBeTruthy();
  expect(within(tools).getByRole('textbox',{name:'Beat grouping (optional)'})).toBeTruthy();
  expect(within(tools).getByRole('spinbutton',{name:'Position (seconds)'})).toBeTruthy();
});

it('saves meter, pulse and grouping together without a preview and restores them with Undo',async()=>{
  const original=timingFixture(48000);original.map.measures[0].beats=[{id:'map-6',frame:48000,offset:{num:1,den:1}}];original.map.next_id=7;
  document.measure_map=structuredClone(original.map);
  const changed=structuredClone(original),meter={numerator:6,denominator:8,pulse:{num:3,den:2},grouping:[3,3]};
  changed.map.measures[0]={...changed.map.measures[0],meter,length:{num:3,den:1},beats:[]};changed.map.revision=2;
  changed.view.measures[0]={...changed.view.measures[0],meter,length:{num:3,den:1}};changed.view.revision=2;
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  mock.previewMap.mockResolvedValue(changed);mock.mapView.mockResolvedValue({...original.view,revision:3});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  const tools=screen.getByRole('region',{name:'Measure editing tools'});
  await fireEvent.input(within(tools).getByRole('spinbutton',{name:'Meter numerator'}),{target:{value:'6'}});
  await fireEvent.input(within(tools).getByRole('spinbutton',{name:'Meter denominator'}),{target:{value:'8'}});
  await fireEvent.click(within(tools).getByText('Advanced',{selector:'summary'}));
  await fireEvent.change(within(tools).getByRole('combobox',{name:'Pulse note value'}),{target:{value:'3/2'}});
  await fireEvent.input(within(tools).getByRole('textbox',{name:'Beat grouping (optional)'}),{target:{value:'3+3'}});
  expect(document.measure_map).toEqual(original.map);
  await fireEvent.click(within(tools).getByRole('button',{name:'Apply changes'}));
  await waitFor(()=>expect(document.measure_map).toEqual(changed.map));
  expect(mock.previewMap).toHaveBeenCalledExactlyOnceWith('test',original.map,{type:'set_notation',id:'map-3',meter,kind:'full',length:{num:4,den:1},following:false});
  expect(screen.queryByRole('button',{name:'Apply map'})).toBeNull();expect(screen.queryByRole('button',{name:'Discard'})).toBeNull();
  await waitFor(()=>expect(screen.getByRole('button',{name:'Undo'}).hasAttribute('disabled')).toBe(false));await fireEvent.click(screen.getByRole('button',{name:'Undo'}));
  await waitFor(()=>expect(document.measure_map?.measures).toEqual(original.map.measures));
  expect(document.measure_map?.boundaries).toEqual(original.map.boundaries);
});

it('reviews batch grouping without saving, discards it, then saves once with Undo and Redo',async()=>{
  const original=timingFixture(48000);
  original.map.measures[0].meter={...original.map.measures[0].meter,grouping:[1,3]};
  original.view.measures[0].meter=original.map.measures[0].meter;
  original.map.measures[0].beats=[{id:'map-6',frame:57600,offset:{num:1,den:1}}];original.map.next_id=7;
  original.map.measures[1].protected=true;original.view.measures[1].protected=true;
  document.measure_map=structuredClone(original.map);
  const changed=structuredClone(original);
  changed.map.measures[0].meter.grouping=[2,2];changed.map.revision=2;changed.view.revision=2;
  changed.view.measures[0].grid=[{id:null,frame:0,quarter:0,label:'1'},{id:'map-6',frame:57600,quarter:1,label:'Anchor'},{id:null,frame:102400,quarter:2,label:'3'}];
  changed.warnings=['1 measure changed; 1 locked measure skipped.'];
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  mock.previewMap.mockResolvedValue(changed);
  mock.mapView.mockImplementation(async map=>({...structuredClone(map.measures[0].meter.grouping[0]===1?original.view:changed.view),revision:map.revision}));
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  const grouping=screen.getByRole('textbox',{name:'Batch beat grouping (optional)'}) as HTMLInputElement;
  await fireEvent.input(grouping,{target:{value:'2+2'}});
  await screen.findByText('1 measure will change; 1 locked measure skipped.');
  await fireEvent.click(screen.getByRole('button',{name:'Preview grouping'}));
  await screen.findByText('1 measure changed; 1 locked measure skipped.');
  expect(mock.previewMap).toHaveBeenCalledExactlyOnceWith('test',original.map,{type:'set_grouping_for_meter',numerator:4,denominator:4,grouping:[2,2]});
  expect(document.measure_map).toEqual(original.map);expect(mock.update).not.toHaveBeenCalled();expect(grouping.disabled).toBe(true);
  await waitFor(()=>expect(mock.metronome.mock.lastCall?.[3]).toEqual(changed.map));
  await fireEvent.click(screen.getByRole('button',{name:'Discard'}));
  expect(document.measure_map).toEqual(original.map);expect(grouping.value).toBe('2+2');expect(grouping.disabled).toBe(false);
  await waitFor(()=>expect(mock.metronome.mock.lastCall?.[3]).toEqual(original.map));
  await fireEvent.click(screen.getByRole('button',{name:'Preview grouping'}));
  await screen.findByRole('button',{name:'Apply map'});
  await fireEvent.click(screen.getByRole('button',{name:'Apply map'}));
  await waitFor(()=>expect(document.measure_map).toEqual(changed.map));
  expect(mock.update).toHaveBeenCalledOnce();expect(screen.getByRole('button',{name:'Preview grouping'}).hasAttribute('disabled')).toBe(true);
  await waitFor(()=>expect(screen.getByRole('button',{name:'Undo'}).hasAttribute('disabled')).toBe(false));
  await fireEvent.click(screen.getByRole('button',{name:'Undo'}));
  await waitFor(()=>expect(document.measure_map?.measures).toEqual(original.map.measures));
  expect(document.measure_map?.boundaries).toEqual(original.map.boundaries);
  await waitFor(()=>expect(screen.getByRole('button',{name:'Redo'}).hasAttribute('disabled')).toBe(false));
  await fireEvent.click(screen.getByRole('button',{name:'Redo'}));
  await waitFor(()=>expect(document.measure_map?.measures).toEqual(changed.map.measures));
  expect(document.measure_map?.revision).toBe(4);
});

it('retains rejected batch grouping input and the accepted map for correction',async()=>{
  const original=timingFixture(48000);document.measure_map=structuredClone(original.map);
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  const grouping=screen.getByRole('textbox',{name:'Batch beat grouping (optional)'}) as HTMLInputElement;
  await fireEvent.input(grouping,{target:{value:'2++2'}});
  await fireEvent.click(screen.getByRole('button',{name:'Preview grouping'}));
  await screen.findByText(/Enter grouping as whole numbers/);expect(mock.previewMap).not.toHaveBeenCalled();
  mock.previewMap.mockRejectedValueOnce(new Error('Grouping must add up to the time-signature numerator'));
  await fireEvent.input(grouping,{target:{value:'2+3'}});
  await fireEvent.click(screen.getByRole('button',{name:'Preview grouping'}));
  await screen.findByText(/Grouping must add up/);expect(grouping.value).toBe('2+3');expect(document.measure_map).toEqual(original.map);
  expect(screen.queryByRole('button',{name:'Apply map'})).toBeNull();expect(mock.update).not.toHaveBeenCalled();
  const changed=structuredClone(original);changed.map.revision=2;changed.view.revision=2;
  for(const m of changed.map.measures)m.meter.grouping=[2,2];
  mock.previewMap.mockResolvedValue(changed);
  await fireEvent.input(grouping,{target:{value:'2+2'}});await fireEvent.click(screen.getByRole('button',{name:'Preview grouping'}));
  await screen.findByRole('button',{name:'Apply map'});expect(document.measure_map).toEqual(original.map);
});

it('keeps a rejected direct meter edit intact for correction and retry',async()=>{
  const original=timingFixture(48000);document.measure_map=structuredClone(original.map);
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  mock.previewMap.mockRejectedValueOnce(new Error('Choose a valid time signature'));
  const accepted={...original,map:{...original.map,revision:2},view:{...original.view,revision:2}};
  mock.previewMap.mockResolvedValue(accepted);
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  const denominator=screen.getByRole('spinbutton',{name:'Meter denominator'}) as HTMLInputElement;
  await fireEvent.input(denominator,{target:{value:'3'}});await fireEvent.click(screen.getByRole('button',{name:'Apply changes'}));
  await screen.findByText('Error: Choose a valid time signature');expect(document.measure_map).toEqual(original.map);expect(denominator.value).toBe('3');
  expect(screen.queryByRole('button',{name:'Apply map'})).toBeNull();
  await fireEvent.input(denominator,{target:{value:'4'}});await fireEvent.click(screen.getByRole('button',{name:'Apply changes'}));
  await waitFor(()=>expect(document.measure_map?.revision).toBe(2));await waitFor(()=>expect(screen.queryByText('Error: Choose a valid time signature')).toBeNull());
});

it('locks and unlocks from the measure title across editing panels',async()=>{
  const original=timingFixture(48000);document.measure_map=structuredClone(original.map);
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  mock.previewMap.mockImplementation(async(_key,map,edit)=>{
    const result=structuredClone(original);result.map=structuredClone(map);result.map.revision++;result.view.revision=result.map.revision;
    result.map.measures[0].protected=edit.protected;result.view.measures[0].protected=edit.protected;return result;
  });
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  const tools=screen.getByRole('region',{name:'Measure editing tools'}),lock=within(tools).getByRole('button',{name:'Lock measure'});
  expect(lock.parentElement).toBe(within(tools).getByRole('heading',{name:'Measure 1'}).parentElement);
  await fireEvent.click(lock);await waitFor(()=>expect(document.measure_map?.measures[0].protected).toBe(true));
  await waitFor(()=>expect(within(tools).getByRole('button',{name:'Unlock measure'}).hasAttribute('disabled')).toBe(false));
  expect(within(tools).getByRole('button',{name:'Unlock measure'}).getAttribute('aria-pressed')).toBe('true');
  expect(within(tools).getByRole('button',{name:'Apply changes'}).hasAttribute('disabled')).toBe(true);
  await fireEvent.click(within(tools).getByRole('button',{name:'Join / Split / Shift'}));
  await fireEvent.click(within(tools).getByRole('button',{name:'Unlock measure'}));
  await waitFor(()=>expect(document.measure_map?.measures[0].protected).toBe(false));
  expect((await within(tools).findByRole('button',{name:'Lock measure'})).getAttribute('aria-pressed')).toBe('false');
  expect(mock.action).not.toHaveBeenCalled();
});

it('joins adjacent measures with a derived meter and no preview gate, and can Undo',async()=>{
  const original=timingFixture(48000),joined=structuredClone(original);document.measure_map=structuredClone(original.map);
  joined.map.measures=[{...joined.map.measures[0],end:'map-4',meter:{...joined.map.measures[0].meter,numerator:8},length:{num:8,den:1},beats:[{id:'map-6',frame:192000,offset:{num:4,den:1}}]}];
  joined.map.boundaries=joined.map.boundaries.filter(b=>b.id!=='map-2');joined.map.next_id=7;joined.map.revision=2;
  joined.view.measures=[{...joined.view.measures[0],end_id:'map-4',end:384000,meter:joined.map.measures[0].meter,length:{num:8,den:1}}];joined.view.revision=2;
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  mock.previewMap.mockResolvedValue(joined);mock.mapView.mockResolvedValue({...original.view,revision:3});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));await fireEvent.click(screen.getByRole('button',{name:'Join / Split / Shift'}));
  expect(screen.queryByRole('button',{name:'Respace or free time'})).toBeNull();expect(screen.queryByRole('button',{name:'Preview spacing'})).toBeNull();
  const advanced=screen.getByText('Advanced — beat anchor positions',{selector:'summary'}).closest('details')!;expect(advanced.open).toBe(false);
  await fireEvent.click(screen.getByRole('button',{name:'Join with next'}));
  await waitFor(()=>expect(document.measure_map).toEqual(joined.map));
  expect(mock.previewMap).toHaveBeenCalledExactlyOnceWith('test',original.map,{type:'merge',id:'map-3'});
  expect(screen.queryByRole('button',{name:'Apply map'})).toBeNull();
  await waitFor(()=>expect(screen.getByRole('button',{name:'Undo'}).hasAttribute('disabled')).toBe(false));await fireEvent.click(screen.getByRole('button',{name:'Undo'}));
  await waitFor(()=>expect(document.measure_map?.measures).toEqual(original.map.measures));
});

it('chooses a numbered fixed beat for splitting without typing source time or offsets',async()=>{
  const original=timingFixture(48000);original.map.measures[0].beats=[{id:'map-6',frame:72000,offset:{num:2,den:1}}];original.map.next_id=7;
  original.view.measures[0].split_points[1]={beat:3,frame:72000,anchor:true};document.measure_map=structuredClone(original.map);
  const split=structuredClone(original),first={...split.map.measures[0],end:'map-7',kind:'incomplete' as const,length:{num:2,den:1},beats:[]};
  split.map.measures=[first,{...first,id:'map-8',start:'map-7',end:'map-2'},split.map.measures[1]];split.map.boundaries.push({id:'map-7',frame:72000});split.map.next_id=9;split.map.revision=2;
  const left={...split.view.measures[0],end:72000,end_id:'map-7',kind:'incomplete' as const,length:{num:2,den:1},split_points:[{beat:2,frame:36000,anchor:false}]};
  split.view.measures=[left,{...left,id:'map-8',start:72000,start_id:'map-7',end:192000,end_id:'map-2',label:'2',number:2}, {...split.view.measures[1],label:'3',number:3}];split.view.revision=2;
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});mock.previewMap.mockResolvedValue(split);
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));await fireEvent.click(screen.getByRole('button',{name:'Join / Split / Shift'}));
  await fireEvent.change(screen.getByRole('combobox',{name:'Split before beat'}),{target:{value:'3'}});
  await fireEvent.click(screen.getByRole('button',{name:'Split measure'}));
  await waitFor(()=>expect(document.measure_map?.measures).toHaveLength(3));
  expect(mock.previewMap).toHaveBeenCalledExactlyOnceWith('test',original.map,{type:'split_at_beat',id:'map-3',beat:3});
  expect(document.measure_map?.measures.slice(0,2).map(m=>[m.meter.numerator,m.meter.denominator,m.kind])).toEqual([[4,4,'incomplete'],[4,4,'incomplete']]);
  expect(screen.queryByRole('button',{name:'Apply map'})).toBeNull();expect(mock.action).not.toHaveBeenCalled();
});

it('disables joining unequal denominators and does not modify the map',async()=>{
  const original=timingFixture(48000);original.map.measures[1].meter={...original.map.measures[1].meter,denominator:8};original.view.measures[1].meter=original.map.measures[1].meter;
  document.measure_map=structuredClone(original.map);
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));await fireEvent.click(screen.getByRole('button',{name:'Join / Split / Shift'}));
  expect(screen.getByRole('button',{name:'Join with next'}).hasAttribute('disabled')).toBe(true);
  await fireEvent.click(screen.getByRole('button',{name:'Join with next'}));expect(mock.previewMap).not.toHaveBeenCalled();
});

it('marks a measure as free time, selects its waveform gap and reclaims it without disturbing its neighbor',async()=>{
  const original=timingFixture(48000);document.measure_map=structuredClone(original.map);
  const removed=structuredClone(original);removed.map.measures.shift();removed.map.boundaries.shift();removed.map.revision=2;removed.view.measures.shift();removed.view.measures[0].label='1';removed.view.revision=2;
  const reclaimed=structuredClone(original);reclaimed.map.measures[0].id='map-7';reclaimed.map.measures[0].start='map-6';reclaimed.map.boundaries[0].id='map-6';reclaimed.map.next_id=8;reclaimed.map.revision=3;reclaimed.view.measures[0].id='map-7';reclaimed.view.measures[0].start_id='map-6';reclaimed.view.revision=3;
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  mock.previewMap.mockResolvedValueOnce(removed).mockResolvedValueOnce(reclaimed);mock.mapView.mockResolvedValue({...removed.view,revision:4});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));await fireEvent.click(screen.getByRole('button',{name:'Free time'}));
  await fireEvent.click(screen.getByRole('button',{name:'Mark measure 1 as free time'}));
  await waitFor(()=>expect(document.measure_map?.measures).toHaveLength(1));
  await waitFor(()=>expect((screen.getByRole('combobox',{name:'Uncounted range'}) as HTMLSelectElement).value).toBe('0:192000'));
  const waveform=screen.getByRole('region',{name:/Measure-map waveform/});
  vi.spyOn(waveform,'getBoundingClientRect').mockReturnValue({left:0,width:1000,height:200,top:0,right:1000,bottom:200,x:0,y:0,toJSON(){}});waveform.setPointerCapture=vi.fn();waveform.hasPointerCapture=vi.fn(()=>false);
  for(const type of ['pointerdown','pointerup']){const event=new MouseEvent(type,{bubbles:true,clientX:125,button:0});Object.defineProperty(event,'pointerId',{value:1});await fireEvent(waveform,event);}
  await screen.findByRole('heading',{name:'Free time'});await waitFor(()=>expect(transport.position).toBe(1.5));
  expect((screen.getByRole('combobox',{name:'Uncounted range'}) as HTMLSelectElement).value).toBe('0:192000');
  await waitFor(()=>expect(screen.getByRole('button',{name:'Create measure from range'}).hasAttribute('disabled')).toBe(false));
  await fireEvent.click(screen.getByRole('button',{name:'Create measure from range'}));
  await waitFor(()=>expect(document.measure_map?.measures).toHaveLength(2));
  expect(mock.previewMap.mock.lastCall?.[2]).toEqual({type:'reclaim',start:0,end:192000,meter:original.map.measures[0].meter});
  expect(document.measure_map?.measures[1]).toEqual(original.map.measures[1]);
  await screen.findByRole('heading',{name:'Measure 1'});
  await waitFor(()=>expect(screen.getByRole('button',{name:'Measure',pressed:true})).toBeTruthy());
  expect(screen.queryByRole('button',{name:'Apply map'})).toBeNull();
  await waitFor(()=>expect(screen.getByRole('button',{name:'Undo'}).hasAttribute('disabled')).toBe(false));await fireEvent.click(screen.getByRole('button',{name:'Undo'}));
  await waitFor(()=>expect(document.measure_map?.measures).toHaveLength(1));
});

it.each(['left','right'] as const)('shifts the remaining map %s in one direct edit with Undo and preserved practice settings',async(direction)=>{
  const original=timingFixture(48000),changed=structuredClone(original);
  document.measure_map=structuredClone(original.map);document.controls={speed:.75,semitones:-3};document.loop_selection={start:1,end:3,fade_ms:5};document.loop_enabled=true;
  const frame=direction==='right'?288000:96000;
  changed.map.boundaries[1].frame=frame;changed.map.revision=2;changed.view.revision=2;
  changed.map.measures[0]={...changed.map.measures[0],meter:{...changed.map.measures[0].meter,numerator:direction==='right'?6:4},length:{num:direction==='right'?6:2,den:1},kind:direction==='right'?'full':'incomplete'};
  changed.map.measures[1]={...changed.map.measures[1],meter:{...changed.map.measures[1].meter,numerator:direction==='left'?6:4},length:{num:direction==='left'?6:2,den:1},kind:direction==='left'?'full':'incomplete'};
  changed.view.measures[0]={...changed.view.measures[0],end:frame,meter:changed.map.measures[0].meter,length:changed.map.measures[0].length,kind:changed.map.measures[0].kind};
  changed.view.measures[1]={...changed.view.measures[1],start:frame,meter:changed.map.measures[1].meter,length:changed.map.measures[1].length,kind:changed.map.measures[1].kind};
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  mock.previewMap.mockResolvedValue(changed);mock.mapView.mockResolvedValue({...original.view,revision:3});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));await fireEvent.click(screen.getByRole('button',{name:'Join / Split / Shift'}));
  expect(screen.getByRole('button',{name:'Shift right'}).hasAttribute('disabled')).toBe(true);
  await fireEvent.click(within(screen.getByLabelText('Measures on timeline')).getByRole('button',{name:'2 4/4'}));
  await waitFor(()=>expect(transport.position).toBe(4));mock.action.mockClear();
  await fireEvent.click(screen.getByRole('checkbox',{name:'Re-snap to detected beats'}));
  expect((screen.getByRole('spinbutton',{name:'Shift by pulses'}) as HTMLInputElement).value).toBe('2');
  await fireEvent.click(screen.getByRole('button',{name:direction==='right'?'Shift right':'Shift left'}));
  await waitFor(()=>expect(document.measure_map).toEqual(changed.map));
  expect(mock.previewMap).toHaveBeenCalledExactlyOnceWith('test',original.map,{type:'shift',id:'map-5',pulses:2,direction});
  expect(document.controls).toEqual({speed:.75,semitones:-3});expect(document.loop_selection).toEqual({start:1,end:3,fade_ms:5});expect(mock.action).not.toHaveBeenCalled();
  expect(screen.queryByRole('button',{name:'Apply map'})).toBeNull();
  await waitFor(()=>expect(screen.getByRole('button',{name:'Undo'}).hasAttribute('disabled')).toBe(false));await fireEvent.click(screen.getByRole('button',{name:'Undo'}));
  await waitFor(()=>expect(document.measure_map?.measures).toEqual(original.map.measures));expect(document.measure_map?.boundaries).toEqual(original.map.boundaries);
});

it('validates shift pulse counts and keeps the map and typed amount after a rejected meter-overflow shift',async()=>{
  const original=timingFixture(48000);document.measure_map=structuredClone(original.map);
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  mock.previewMap.mockRejectedValue(new Error('Shift would require a meter numerator above 99; choose fewer pulses'));
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));await fireEvent.click(within(screen.getByLabelText('Measures on timeline')).getByRole('button',{name:'2 4/4'}));await fireEvent.click(screen.getByRole('button',{name:'Join / Split / Shift'}));
  await fireEvent.click(screen.getByRole('checkbox',{name:'Re-snap to detected beats'}));
  const pulses=screen.getByRole('spinbutton',{name:'Shift by pulses'}) as HTMLInputElement;
  for(const value of ['','0','1.5','65']){await fireEvent.input(pulses,{target:{value}});expect(screen.getByRole('button',{name:'Shift right'}).hasAttribute('disabled')).toBe(true);}
  await fireEvent.input(pulses,{target:{value:'4'}});await fireEvent.click(screen.getByRole('button',{name:'Shift right'}));
  await screen.findByText(/Error: Shift would require/);expect(document.measure_map).toEqual(original.map);expect(pulses.value).toBe('4');expect(screen.queryByRole('button',{name:'Apply map'})).toBeNull();
});

it.each(['protected','gap'])('blocks shifting a remaining span with a %s',async(reason)=>{
  const original=timingFixture(48000);
  if(reason==='protected'){original.map.measures[0].protected=true;original.view.measures[0].protected=true;}
  else{original.map.boundaries[1].frame=96000;original.view.measures[0].end=96000;original.map.boundaries.push({id:'map-6',frame:192000});original.map.measures[1].start='map-6';original.map.next_id=7;original.view.measures[1].start_id='map-6';}
  document.measure_map=structuredClone(original.map);
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));await fireEvent.click(within(screen.getByLabelText('Measures on timeline')).getByRole('button',{name:'2 4/4'}));await fireEvent.click(screen.getByRole('button',{name:'Join / Split / Shift'}));
  expect(screen.getByRole('button',{name:'Shift right'}).hasAttribute('disabled')).toBe(true);expect(screen.getByRole('button',{name:'Shift left'}).hasAttribute('disabled')).toBe(true);expect(mock.previewMap).not.toHaveBeenCalled();
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
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  mock.previewMap.mockResolvedValue(preview);
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  if(draft){await fireEvent.click(screen.getByRole('button',{name:'Preview numbering'}));await screen.findByRole('button',{name:'Apply map'});}
  expect(mock.action).not.toHaveBeenCalled();
  const lane=screen.getByLabelText('Measures on timeline'),measure=within(lane).getByRole('button',{name:'2 4/4'});
  await fireEvent.click(measure);
  const seconds=draft?5.25:4;
  await waitFor(()=>expect(mock.action).toHaveBeenCalledExactlyOnceWith({type:'seek',seconds},'test'));
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
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  mock.previewMap.mockResolvedValue(preview);
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  if(draft){await fireEvent.click(screen.getByRole('button',{name:'Preview numbering'}));await screen.findByRole('button',{name:'Apply map'});}
  const waveform=screen.getByRole('region',{name:/Measure-map waveform/});
  vi.spyOn(waveform,'getBoundingClientRect').mockReturnValue({left:0,width:1000,height:200,top:0,right:1000,bottom:200,x:0,y:0,toJSON(){}});
  waveform.setPointerCapture=vi.fn();waveform.hasPointerCapture=vi.fn(()=>false);
  for(const type of ['pointerdown','pointerup']){const event=new MouseEvent(type,{bubbles:true,clientX:500,button:0});Object.defineProperty(event,'pointerId',{value:1});await fireEvent(waveform,event);}
  await waitFor(()=>expect(mock.action).toHaveBeenCalledExactlyOnceWith({type:'seek',seconds:6},'test'));
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
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
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
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
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
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
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
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
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
  await fireEvent.click(screen.getByRole('button',{name:'Close measure map'}));
  await waitFor(()=>expect(mock.metronome.mock.lastCall).toEqual(['test',true,.8,original.map]));
  expect(transport.playing).toBe(false);expect(document.measure_map).toEqual(original.map);
});

it('keeps following through navigation and pages until explicitly toggled off',async()=>{
  const fixture=timingFixture(48000);document.measure_map=fixture.map;
  mock.initialize.mockResolvedValue({...response(),map_view:fixture.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
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
  await fireEvent.click(screen.getByRole('button',{name:'Follow playhead on'}));
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

it.each(['left','right'] as const)('absorbs a consumed measure on a %s shift and restores it with Undo',async(direction)=>{
  const original=timingFixture(48000),changed=structuredClone(original);
  const surviving=direction==='right'?0:1;
  const measure={...changed.map.measures[surviving],start:original.map.measures[0].start,end:original.map.measures[1].end,meter:{...changed.map.measures[surviving].meter,numerator:8},length:{num:8,den:1}};
  changed.map.measures=[measure];changed.map.boundaries=[changed.map.boundaries[0],changed.map.boundaries[2]];changed.map.revision=2;
  changed.view={revision:2,measures:[{...changed.view.measures[surviving],label:'1',number:1,start_id:measure.start,end_id:measure.end,start:0,end:384000,meter:measure.meter,length:measure.length}]};
  document.measure_map=structuredClone(original.map);
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  mock.previewMap.mockResolvedValue(changed);mock.mapView.mockResolvedValue({...original.view,revision:3});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));await fireEvent.click(within(screen.getByLabelText('Measures on timeline')).getByRole('button',{name:'2 4/4'}));await fireEvent.click(screen.getByRole('button',{name:'Join / Split / Shift'}));
  await fireEvent.click(screen.getByRole('checkbox',{name:'Re-snap to detected beats'}));await fireEvent.input(screen.getByRole('spinbutton',{name:'Shift by pulses'}),{target:{value:'4'}});
  await fireEvent.click(screen.getByRole('button',{name:direction==='right'?'Shift right':'Shift left'}));
  await waitFor(()=>expect(document.measure_map?.measures).toEqual(changed.map.measures));
  expect(await screen.findByRole('heading',{name:'Measure 1'})).toBeTruthy();expect(screen.queryByRole('button',{name:'Apply map'})).toBeNull();
  expect(mock.previewMap).toHaveBeenCalledExactlyOnceWith('test',original.map,{type:'shift',id:'map-5',pulses:4,direction});
  await waitFor(()=>expect(screen.getByRole('button',{name:'Undo'}).hasAttribute('disabled')).toBe(false));await fireEvent.click(screen.getByRole('button',{name:'Undo'}));
  await waitFor(()=>expect(document.measure_map?.measures).toEqual(original.map.measures));expect(document.measure_map?.boundaries).toEqual(original.map.boundaries);
});


it('shares metronome enable/level across screens and listens to saved map on returning from a draft',async()=>{
  const original=timingFixture(48000),draft=structuredClone(original);draft.map.first_number=7;
  document.measure_map=original.map;
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  mock.previewMap.mockResolvedValue(draft);
  await ready();
  await waitFor(()=>expect(mock.metronome.mock.lastCall).toEqual(['test',true,.5,original.map]));
  await fireEvent.click(screen.getByRole('button',{name:'Metronome on'}));
  await fireEvent.input(screen.getByLabelText('Metronome volume'),{target:{value:'65'}});
  await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  expect(screen.getByRole('button',{name:'Metronome off'}).getAttribute('aria-pressed')).toBe('false');
  expect((screen.getByLabelText('Metronome volume') as HTMLInputElement).value).toBe('65');
  await fireEvent.click(screen.getByRole('button',{name:'Metronome off'}));
  await fireEvent.click(screen.getByRole('button',{name:'Preview numbering'}));
  await waitFor(()=>expect(mock.metronome.mock.lastCall).toEqual(['test',true,.65,draft.map]));
  await fireEvent.click(screen.getByRole('button',{name:'Close measure map'}));
  await waitFor(()=>expect(mock.metronome.mock.lastCall).toEqual(['test',true,.65,original.map]));
  expect(screen.getByRole('button',{name:'Metronome on'}).getAttribute('aria-pressed')).toBe('true');
  expect(document.measure_map).toEqual(original.map);
});

it('defaults to measure loops and snaps A/B shortcuts, with exact inputs and a free selection option',async()=>{
  const original=timingFixture(48000);document.measure_map=original.map;transport.position=5;
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();expect((screen.getByRole('checkbox',{name:'Snap to measures'}) as HTMLInputElement).checked).toBe(true);
  await fireEvent.click(screen.getByRole('button',{name:'Loop off'}));
  await waitFor(()=>expect(document.loop_selection).toEqual({start:4,end:8,fade_ms:5}));
  await fireEvent.keyDown(window,{key:'ArrowLeft'});await waitFor(()=>expect(transport.position).toBe(0));
  await fireEvent.keyDown(window,{key:'a'});await waitFor(()=>expect(document.loop_selection?.start).toBe(0));
  await fireEvent.keyDown(window,{key:'ArrowRight'});await waitFor(()=>expect(transport.position).toBe(5));
  await fireEvent.keyDown(window,{key:'b'});await waitFor(()=>expect(document.loop_selection?.end).toBe(4));
  await fireEvent.change(screen.getByLabelText('Loop end seconds'),{target:{value:'5.25'}});
  await waitFor(()=>expect(document.loop_selection?.end).toBe(5.25));
  await fireEvent.click(screen.getByRole('checkbox',{name:'Snap to measures'}));
  await fireEvent.keyDown(window,{key:'b'});await waitFor(()=>expect(document.loop_selection?.end).toBe(5));
});


it('sets an initial mapped loop to one measure near a barline, preserving free-time A/B behavior',async()=>{
  const original=timingFixture(48000);document.measure_map=original.map;transport.position=3.99;
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();await fireEvent.keyDown(window,{key:'a'});
  await waitFor(()=>expect(document.loop_selection).toEqual({start:0,end:4,fade_ms:5}));
  await fireEvent.keyDown(window,{key:'x'});await waitFor(()=>expect(document.loop_selection).toBeNull());
  await fireEvent.keyDown(window,{key:'ArrowRight'});await waitFor(()=>expect(transport.position).toBe(8.99));
  await fireEvent.keyDown(window,{key:'b'});
  await waitFor(()=>expect(document.loop_selection).toEqual({start:0,end:8.99,fade_ms:5}));
});


it.each([false,true])('pages practice playback with mapped=%s, retaining zoom through boundaries, loop wraps and EOF',async(mapped)=>{
  if(mapped){
    const fixture=timingFixture(48000);document.measure_map=fixture.map;
    mock.initialize.mockResolvedValue({...response(),map_view:fixture.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  }
  await ready();
  expect(screen.getByRole('button',{name:'Follow playhead on'}).getAttribute('aria-pressed')).toBe('true');
  await fireEvent.click(screen.getByRole('button',{name:'Zoom in'})); // 0..6 s.
  const viewport=screen.getByRole('button',{name:'Move waveform view'}).querySelector('.viewport') as HTMLElement;
  expect(viewport.style.width).toBe('50%');expect(viewport.style.left).toBe('0%');
  transport.position=7;await screen.findByText('0:07.0');
  expect(viewport.style.left).toBe('0%'); // Paused playback does not follow.
  await fireEvent.click(screen.getByRole('button',{name:'Play'}));
  await waitFor(()=>expect(viewport.style.left).toBe('50%'));
  transport.position=5.999;await waitFor(()=>expect(viewport.style.left).toBe('0%')); // Backwards wrap.
  transport.position=6;await waitFor(()=>expect(viewport.style.left).toBe('50%')); // Exclusive page end.
  transport.position=12;await waitFor(()=>expect(screen.getByRole('region',{name:'Practice workspace'}).querySelector('.transport-time strong')?.textContent).toBe('0:12.0'));
  expect(viewport.style.left).toBe('50%');expect(viewport.style.width).toBe('50%');
  expect(mock.action.mock.calls.map(c=>c[0].type)).toEqual(['toggle_play']); // Paging never seeks audio.
});

it('keeps practice following through gestures until manually switched off, and preserves its preference across screens',async()=>{
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Zoom in'}));
  const overview=screen.getByRole('button',{name:'Move waveform view'}),field=screen.getByRole('region',{name:/^Waveform/});
  const box={left:0,width:1000,height:200,top:0,right:1000,bottom:200,x:0,y:0,toJSON(){}};
  for(const element of [overview,field])vi.spyOn(element,'getBoundingClientRect').mockReturnValue(box);
  field.setPointerCapture=vi.fn();field.hasPointerCapture=vi.fn(()=>false);
  const viewport=overview.querySelector('.viewport') as HTMLElement;
  await fireEvent.click(overview,{clientX:900});expect(viewport.style.left).toBe('50%');
  for(const [type,x] of [['pointerdown',500],['pointermove',700],['pointerup',700]] as const){
    const event=new MouseEvent(type,{bubbles:true,cancelable:true,clientX:x,button:1});Object.defineProperty(event,'pointerId',{value:1});await fireEvent(field,event);
  }
  expect(screen.getByRole('button',{name:'Follow playhead on'}).getAttribute('aria-pressed')).toBe('true');
  await fireEvent(field,new WheelEvent('wheel',{bubbles:true,cancelable:true,deltaY:-60,clientX:500}));
  expect(screen.getByRole('button',{name:'Follow playhead on'}).getAttribute('aria-pressed')).toBe('true');
  await fireEvent.click(screen.getByRole('button',{name:'Follow playhead on'}));
  await fireEvent.click(overview,{clientX:900});const manual=viewport.style.left,width=viewport.style.width;
  await fireEvent.click(screen.getByRole('button',{name:'Play'}));transport.position=1;
  await screen.findByText('0:01.0');expect(viewport.style.left).toBe(manual);
  await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  expect(screen.getByRole('button',{name:'Follow playhead on'}).getAttribute('aria-pressed')).toBe('true');
  await fireEvent.click(screen.getByRole('button',{name:'Close measure map'}));
  expect(screen.getByRole('button',{name:'Follow playhead off'}).getAttribute('aria-pressed')).toBe('false');
  expect(viewport.style.left).toBe(manual);
  await fireEvent.click(screen.getByRole('button',{name:'Follow playhead off'}));
  await waitFor(()=>expect(viewport.style.left).toBe('0%'));expect(viewport.style.width).toBe(width);
});

it('shares epsilon between measure creation and default shift refitting, with one Undo',async()=>{
  const original=timingFixture(48000),changed=structuredClone(original);
  changed.map.revision=2;changed.view.revision=2;
  document.measure_map=structuredClone(original.map);
  const result={version:2,reference_token:'test',map_revision:1,rate:48000,frames:576000,model:'test',preprocessing:'test',elapsed_seconds:2,beats:Array.from({length:9},(_,i)=>({frame:i*48000,score:.8,downbeat:i%4===0}))};
  mock.analysisStatus.mockResolvedValue({id:3,running:false,kind:'analysis',phase:'Complete',model_ready:true,model_path:'/models',progress:null,elapsed_seconds:3,reference_token:'test',map_revision:1,error:null,result});
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  mock.previewMap.mockResolvedValue(changed);mock.mapView.mockResolvedValue(original.view);
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  const epsilon=await screen.findByRole('spinbutton',{name:'Beat timing tolerance (ms)'});
  await fireEvent.input(epsilon,{target:{value:'15'}});
  await fireEvent.click(within(screen.getByLabelText('Measures on timeline')).getByRole('button',{name:'2 4/4'}));
  await fireEvent.click(screen.getByRole('button',{name:'Join / Split / Shift'}));
  const checkbox=screen.getByRole('checkbox',{name:'Re-snap to detected beats'}) as HTMLInputElement;
  expect(checkbox.checked).toBe(true);
  const shiftEpsilon=screen.getByRole('spinbutton',{name:'Shift beat timing tolerance (ms)'}) as HTMLInputElement;
  expect(shiftEpsilon.value).toBe('15');
  for(const value of ['', '-1','1001']) {
    await fireEvent.input(shiftEpsilon,{target:{value}});
    expect(screen.getByRole('button',{name:'Shift right'}).hasAttribute('disabled')).toBe(true);
  }
  await fireEvent.input(shiftEpsilon,{target:{value:'0'}});
  expect((epsilon as HTMLInputElement).value).toBe('0');
  await fireEvent.click(screen.getByRole('button',{name:'Shift right'}));
  await waitFor(()=>expect(document.measure_map).toEqual(changed.map));
  expect(mock.previewMap).toHaveBeenCalledExactlyOnceWith('test',original.map,{type:'shift',id:'map-5',pulses:2,direction:'right',resnap:{pulses:result.beats.map(b=>b.frame),epsilon_ms:0}});
  expect(screen.queryByRole('button',{name:'Apply map'})).toBeNull();
  await fireEvent.click(screen.getByRole('button',{name:'Undo'}));
  await waitFor(()=>expect(document.measure_map?.measures).toEqual(original.map.measures));
});

it('defaults re-snapping on without detection and allows toggling either way',async()=>{
  const original=timingFixture(48000);document.measure_map=structuredClone(original.map);
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));await fireEvent.click(screen.getByRole('button',{name:'Join / Split / Shift'}));
  const checkbox=screen.getByRole('checkbox',{name:'Re-snap to detected beats'}) as HTMLInputElement;
  expect(checkbox.disabled).toBe(false);expect(checkbox.checked).toBe(true);
  await fireEvent.click(within(screen.getByLabelText('Measures on timeline')).getByRole('button',{name:'2 4/4'}));
  expect(screen.getByRole('button',{name:'Shift right'}).hasAttribute('disabled')).toBe(true);
  await fireEvent.click(checkbox);expect(checkbox.checked).toBe(false);
  expect(screen.getByRole('button',{name:'Shift right'}).hasAttribute('disabled')).toBe(false);
  await fireEvent.keyDown(screen.getByRole('slider',{name:'Measure-map song position'}),{key:'ArrowRight'});
  await screen.findByRole('heading',{name:'Free time'});
  await fireEvent.click(within(screen.getByLabelText('Measures on timeline')).getByRole('button',{name:'2 4/4'}));
  await fireEvent.click(screen.getByRole('button',{name:'Join / Split / Shift'}));
  const restored=screen.getByRole('checkbox',{name:'Re-snap to detected beats'}) as HTMLInputElement;
  expect(restored.checked).toBe(false);
  await fireEvent.click(restored);expect(restored.checked).toBe(true);
  expect(screen.getByRole('button',{name:'Shift right'}).hasAttribute('disabled')).toBe(true);
});

it('reloads cached beats after restart and shifts with re-snapping enabled by default',async()=>{
  const original=timingFixture(48000),changed=structuredClone(original);changed.map.revision=2;changed.view.revision=2;
  document.measure_map=structuredClone(original.map);
  const cached={version:2,reference_token:'test',map_revision:0,rate:48000,frames:576000,model:'test',preprocessing:'test',elapsed_seconds:2,beats:Array.from({length:9},(_,i)=>({frame:i*48000,score:.8,downbeat:i%4===0}))};
  mock.readCachedAnalysis.mockResolvedValue(cached);
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});mock.previewMap.mockResolvedValue(changed);
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));await screen.findByText(/9 suggested beats/);
  await fireEvent.click(within(screen.getByLabelText('Measures on timeline')).getByRole('button',{name:'2 4/4'}));await fireEvent.click(screen.getByRole('button',{name:'Join / Split / Shift'}));
  const checkbox=screen.getByRole('checkbox',{name:'Re-snap to detected beats'}) as HTMLInputElement;expect(checkbox.disabled).toBe(false);expect(checkbox.checked).toBe(true);
  await fireEvent.click(screen.getByRole('button',{name:'Shift right'}));await waitFor(()=>expect(document.measure_map).toEqual(changed.map));
  expect(mock.previewMap.mock.lastCall?.[2]).toMatchObject({resnap:{pulses:cached.beats.map(b=>b.frame),epsilon_ms:30}});
  expect(mock.readCachedAnalysis).toHaveBeenCalledExactlyOnceWith('test');
});

const libraryEntry=(key:string,title:string,artist:string,added:number,opened:number)=>({song_key:key,metadata:{title,artist,album:'Workshop'},file_location:`/music/${title}.wav`,date_added:added,last_opened:opened});
it('opens the searchable library, pauses playback, sorts columns and edits metadata',async()=>{
  const current=libraryEntry('test','Workshop take','Zed',1000,2000),other=libraryEntry('other','Riff 2','Alpha',2000,1000);
  mock.readLibrary.mockResolvedValue({version:1,songs:[current,other]});mock.updateLibrarySong.mockImplementation(async(key,metadata)=>({version:1,songs:[{...current,metadata},other]}));
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Play'}));await fireEvent.click(screen.getByRole('button',{name:'Library'}));
  await screen.findByRole('button',{name:'Riff 2'});expect(mock.action).toHaveBeenLastCalledWith({type:'pause'},'test');
  const list=screen.getByRole('table',{name:'Practice songs'});
  expect(within(list).getAllByRole('row').slice(1).map(row=>row.textContent)).toEqual([expect.stringContaining('Workshop take'),expect.stringContaining('Riff 2')]);
  await fireEvent.click(within(list).getByRole('button',{name:'Artist'}));expect(within(list).getAllByRole('row')[1].textContent).toContain('Riff 2');
  await fireEvent.input(screen.getByRole('searchbox',{name:'Search library'}),{target:{value:'Zed Workshop'}});expect(screen.queryByRole('button',{name:'Riff 2'})).toBeNull();
  await fireEvent.click(screen.getByRole('button',{name:'Edit details for Workshop take'}));await fireEvent.input(screen.getByRole('textbox',{name:'Library song title'}),{target:{value:'My guitar take'}});
  await fireEvent.click(screen.getByRole('button',{name:'Save details'}));await screen.findByRole('heading',{name:'My guitar take'});expect(mock.updateLibrarySong).toHaveBeenCalledExactlyOnceWith('test',{title:'My guitar take',artist:'Zed',album:'Workshop'});
  expect(screen.queryByRole('textbox',{name:'Library song title'})).toBeNull();expect(mock.update).not.toHaveBeenCalled();
});
it('reopens a song from the library and leaves existing practice intact on open failure',async()=>{
  const entry=libraryEntry('other','Another take','Player',1000,2000);mock.readLibrary.mockResolvedValue({version:1,songs:[entry]});mock.openLibrarySong.mockRejectedValueOnce(new Error('File missing; use Open song'));
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Library'}));await screen.findByRole('button',{name:'Another take'});await fireEvent.click(screen.getByRole('button',{name:'Another take'}));await screen.findByText(/File missing/);expect(screen.getByRole('heading',{name:'Workshop take'})).toBeTruthy();expect(mock.openLibrarySong).toHaveBeenCalledWith('other');
  mock.openLibrarySong.mockResolvedValue({...response(),song:{key:'other', reference_token:'other',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Another take',path:entry.file_location,duration:12,rate:48000,peaks:[[-.2,.2]]}});await fireEvent.click(screen.getByRole('button',{name:'Another take'}));await screen.findByRole('heading',{name:'Another take'});expect(screen.getByRole('region',{name:'Practice workspace'})).toBeTruthy();
});
it('shows library load failures with retry and preserves typed metadata after a failed save',async()=>{
  mock.readLibrary.mockRejectedValueOnce(new Error('Invalid library; original file left intact'));
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Library'}));await screen.findByText(/Invalid library/);
  mock.readLibrary.mockResolvedValue({version:1,songs:[libraryEntry('test','Workshop take','Player',1,2)]});await fireEvent.click(screen.getByRole('button',{name:'Retry'}));await screen.findByRole('button',{name:'Edit details for Workshop take'});
  await fireEvent.click(screen.getByRole('button',{name:'Edit details for Workshop take'}));await fireEvent.input(screen.getByRole('textbox',{name:'Library song artist'}),{target:{value:'My artist'}});mock.updateLibrarySong.mockRejectedValue(new Error('Disk full'));await fireEvent.click(screen.getByRole('button',{name:'Save details'}));await screen.findByText(/Disk full/);expect((screen.getByRole('textbox',{name:'Library song artist'}) as HTMLInputElement).value).toBe('My artist');
});

it.each(['practice','mapping','library'] as const)('routes opening a song through the library from %s and retains the current song on cancel',async(context)=>{
  mock.openSong.mockResolvedValue(null);
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Play'}));
  if(context==='mapping')await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  if(context==='library')await fireEvent.click(screen.getByRole('button',{name:'Library'}));
  await fireEvent.keyDown(window,{key:'o',ctrlKey:true});
  await waitFor(()=>expect(mock.openSong).toHaveBeenCalledOnce());
  await screen.findByRole('main',{name:'Music library'});
  expect(screen.getByRole('button',{name:'Practice'})).toBeTruthy();
  expect(screen.getAllByRole('button',{name:'Open song'})).toHaveLength(1);
  expect(transport.playing).toBe(false);expect(screen.getByRole('heading',{name:'Workshop take'})).toBeTruthy();
  await fireEvent.click(screen.getByRole('button',{name:'Practice'}));
  expect(screen.getByRole('region',{name:'Practice workspace'})).toBeTruthy();
});

it('saves plain-text notes before closing and keeps transport shortcuts out of the textarea',async()=>{
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Notes'}));
  const notes=screen.getByRole('textbox',{name:'Notes for this song'});mock.action.mockClear();
  const text='  Guitar in drop D\n\nWork on measures 15–17. **Plain text**  ';
  await fireEvent.input(notes,{target:{value:text}});await fireEvent.keyDown(notes,{key:'Enter'});expect(mock.action).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button',{name:'Close notes'}));await screen.findByRole('region',{name:'Practice workspace'});
  expect(document.notes).toBe(text);expect(document.sections).toHaveLength(2);
  await fireEvent.click(screen.getByRole('button',{name:'Notes'}));expect((screen.getByRole('textbox',{name:'Notes for this song'}) as HTMLTextAreaElement).value).toBe(text);
  expect(screen.getByRole('button',{name:'Library'})).toBeTruthy();
});
it('keeps notes open and retains text on failed save, then closes after retry',async()=>{
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Notes'}));const main=screen.getByRole('main',{name:'Song notes'});
  mock.update.mockRejectedValueOnce(new Error('Disk full'));await fireEvent.input(screen.getByRole('textbox',{name:'Notes for this song'}),{target:{value:'Do not lose this'}});
  await fireEvent.click(screen.getByRole('button',{name:'Close notes'}));await within(main).findByRole('alert');expect((screen.getByRole('textbox',{name:'Notes for this song'}) as HTMLTextAreaElement).value).toBe('Do not lose this');
  await fireEvent.click(screen.getByRole('button',{name:'Retry save'}));await waitFor(()=>expect(document.notes).toBe('Do not lose this'));
  await fireEvent.click(screen.getByRole('button',{name:'Close notes'}));expect(await screen.findByRole('region',{name:'Practice workspace'})).toBeTruthy();
});
it('flushes pending notes before a normal app exit and keeps the app open on save failure',async()=>{
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Notes'}));
  await fireEvent.input(screen.getByRole('textbox',{name:'Notes for this song'}),{target:{value:'Saved on quit'}});
  const onExit=mock.onExitRequested.mock.lastCall![0] as ()=>Promise<void>;mock.update.mockRejectedValueOnce(new Error('Disk full'));await onExit();expect(mock.finishExit).not.toHaveBeenCalled();expect(screen.getByRole('main',{name:'Song notes'})).toBeTruthy();
  await onExit();expect(document.notes).toBe('Saved on quit');expect(mock.finishExit).toHaveBeenCalledOnce();
});
it('loads stored notes and flushes them when navigating to the library',async()=>{
  document.notes='Remember the bridge';mock.initialize.mockResolvedValue({...response(),song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Notes'}));const field=screen.getByRole('textbox',{name:'Notes for this song'}) as HTMLTextAreaElement;expect(field.value).toBe('Remember the bridge');await fireEvent.input(field,{target:{value:'Remember the bridge\nRepeat slowly'}});
  await fireEvent.click(screen.getByRole('button',{name:'Library'}));await screen.findByRole('main',{name:'Music library'});expect(document.notes).toBe('Remember the bridge\nRepeat slowly');
});

it('requires an accepted measure map for count-in and highlights enabled metronome controls',async()=>{
  await ready();
  const count=screen.getByRole('button',{name:'Count-in off'});
  expect(count.hasAttribute('disabled')).toBe(true);
  expect(count.getAttribute('aria-pressed')).toBe('false');
  const playback=screen.getByRole('group',{name:'Playback controls'});
  expect(playback.contains(count)).toBe(true);
  expect(playback.contains(screen.getByRole('checkbox',{name:'Before each loop repetition'}))).toBe(true);
  expect(screen.getByRole('checkbox',{name:'Before each loop repetition'}).hasAttribute('disabled')).toBe(true);
  expect(screen.getByRole('button',{name:'Metronome on'}).classList.contains('active')).toBe(true);
  await fireEvent.click(screen.getByRole('button',{name:'Metronome on'}));
  expect(screen.getByRole('button',{name:'Metronome off'}).classList.contains('active')).toBe(false);
  expect(mock.countIn.mock.lastCall).toEqual(['test',false,false,.5]);
});

it('offers count-in and independent loop repetitions only in Practice, using the shared click level',async()=>{
  const original=timingFixture(48000);document.measure_map=original.map;
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();
  await fireEvent.keyDown(window,{key:'c'});
  const count=screen.getByRole('button',{name:'Count-in on'});
  expect(count.classList.contains('active')).toBe(true);expect(count.getAttribute('aria-pressed')).toBe('true');
  const repeat=screen.getByRole('checkbox',{name:'Before each loop repetition'}) as HTMLInputElement;
  expect(repeat.disabled).toBe(false);expect(repeat.checked).toBe(false);
  await fireEvent.click(repeat);await fireEvent.input(screen.getByLabelText('Metronome volume'),{target:{value:'70'}});
  await waitFor(()=>expect(mock.countIn.mock.lastCall).toEqual(['test',true,true,.7]));
  await fireEvent.click(screen.getByRole('button',{name:'Metronome on'}));
  expect(screen.getByRole('button',{name:'Count-in on'}).classList.contains('active')).toBe(true);
  await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  expect(screen.queryByRole('button',{name:'Count-in on'})).toBeNull();
  expect(screen.queryByRole('checkbox',{name:'Before each loop repetition'})).toBeNull();
  await waitFor(()=>expect(mock.countIn.mock.lastCall).toEqual(['test',false,true,.7]));
  expect(screen.getByRole('button',{name:'Metronome off'}).classList.contains('active')).toBe(false);
  await fireEvent.click(screen.getByRole('button',{name:'Metronome off'}));
  expect(screen.getByRole('button',{name:'Metronome on'}).classList.contains('active')).toBe(true);
  await fireEvent.click(screen.getByRole('button',{name:'Close measure map'}));
  expect((screen.getByRole('checkbox',{name:'Before each loop repetition'}) as HTMLInputElement).checked).toBe(true);
  await waitFor(()=>expect(mock.countIn.mock.lastCall).toEqual(['test',true,true,.7]));
  await fireEvent.keyDown(window,{key:'C',shiftKey:true});
  expect(screen.getByRole('button',{name:'Count-in off'}).classList.contains('active')).toBe(false);
  expect(screen.getByRole('checkbox',{name:'Before each loop repetition'}).hasAttribute('disabled')).toBe(true);
  expect(document.measure_map).toEqual(original.map);
});

it('waits for native count-in configuration before Play and reports counting without moving the playhead',async()=>{
  const original=timingFixture(48000);document.measure_map=original.map;
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();await waitFor(()=>expect(mock.countIn).toHaveBeenCalled());
  let finish!:(value:object)=>void;mock.countIn.mockReturnValueOnce(new Promise(resolve=>{finish=resolve;}));
  await fireEvent.click(screen.getByRole('button',{name:'Count-in off'}));
  await waitFor(()=>expect(mock.countIn.mock.lastCall).toEqual(['test',true,false,.5]));
  await fireEvent.click(screen.getByRole('button',{name:'Play'}));expect(mock.action).not.toHaveBeenCalled();
  transport.counting_in=true;finish({});await screen.findByText('Counting in…');
  expect(screen.getByRole('button',{name:'Pause'})).toBeTruthy();
  expect(screen.getByRole('region',{name:'Practice workspace'}).querySelector('.transport-time strong')?.textContent).toBe('0:00.0');
  await fireEvent.click(screen.getByRole('button',{name:'Pause'}));await screen.findByText('Paused');
});

async function openRecordingDetails(title='Workshop take') {
  mock.readLibrary.mockResolvedValue({version:2,songs:[libraryEntry('test',title,'Player',1,2)]});
  expect(screen.queryByRole('button',{name:'Replace recording…'})).toBeNull();
  await fireEvent.click(screen.getByRole('button',{name:'Library'}));
  await screen.findByRole('button',{name:`Edit details for ${title}`});
  expect(screen.queryByRole('button',{name:`Replace recording for ${title}`})).toBeNull();
  await fireEvent.click(screen.getByRole('button',{name:`Edit details for ${title}`}));
  const details=screen.getByRole('complementary',{name:'Song details'});
  const replace=within(details).getByRole('button',{name:`Replace recording for ${title}`});
  expect(replace.closest('dd')?.textContent).toContain(`/music/${title}.wav`);
  await fireEvent.click(replace);
}

it('requires an explicit timing decision when replacing a recording and refreshes the same song session',async()=>{
  await ready();
  const original=await mock.initialize.mock.results[0].value;
  mock.replaceRecording.mockResolvedValue({...response(),song:{...original.song,reference_token:'test:new-asset:session-2',asset_id:'new-asset',session_id:'session-2'},document:{...document,reference_token:'test:new-asset:session-2'},transport:{...transport,reference_token:'test:new-asset:session-2'}});
  await openRecordingDetails();
  const chooser=screen.getByRole('dialog',{name:'Replace reference recording'});
  expect(mock.replaceRecording).not.toHaveBeenCalled();
  await fireEvent.click(within(chooser).getByRole('button',{name:'Choose timing-compatible recording'}));
  await waitFor(()=>expect(mock.replaceRecording).toHaveBeenCalledExactlyOnceWith('test',true));
  await waitFor(()=>expect(screen.getByRole('button',{name:'Play'}).hasAttribute('disabled')).toBe(false));
  await fireEvent.click(screen.getByRole('button',{name:'Play'}));
  await waitFor(()=>expect(mock.action).toHaveBeenCalledWith({type:'toggle_play'},'test:new-asset:session-2'));
});

it('ignores a late transport reply from the previous asset while preserving the song UUID',async()=>{
  await ready();
  const original=await mock.initialize.mock.results[0].value;
  let finish!:(value:Transport)=>void;
  mock.status.mockReturnValue(new Promise(resolve=>{finish=resolve;}));
  await waitFor(()=>expect(mock.status).toHaveBeenCalled());
  mock.replaceRecording.mockResolvedValue({...response(),song:{...original.song,reference_token:'new-asset-session'},document:{...document,reference_token:'new-asset-session'},transport:{...transport,reference_token:'new-asset-session',position:0}});
  await openRecordingDetails();
  await fireEvent.click(screen.getByRole('button',{name:'Choose recording with changed timing'}));
  await waitFor(()=>expect(mock.replaceRecording).toHaveBeenCalledWith('test',false));
  finish({...transport,position:9,playing:true,reference_token:'test'});
  await new Promise(resolve=>setTimeout(resolve,75));
  expect(screen.getByRole('button',{name:'Play'})).toBeTruthy();
  expect(screen.queryByRole('button',{name:'Pause'})).toBeNull();
});

it('offers independent import and closes replacement from its leading icon',async()=>{
  await ready();mock.importSeparate.mockResolvedValue(null);
  await fireEvent.click(screen.getByRole('button',{name:'Library'}));
  await fireEvent.click(screen.getByRole('button',{name:'Import separate song'}));
  await waitFor(()=>expect(mock.importSeparate).toHaveBeenCalledOnce());
  await waitFor(()=>expect(screen.getByRole('button',{name:'Practice'}).hasAttribute('disabled')).toBe(false));
  await fireEvent.click(screen.getByRole('button',{name:'Practice'}));
  await openRecordingDetails();
  const chooser=screen.getByRole('dialog',{name:'Replace reference recording'});
  expect(within(chooser).queryByRole('button',{name:'Cancel'})).toBeNull();
  expect(within(chooser).queryByRole('combobox')).toBeNull();
  expect(within(chooser).getByText(/Changed timing clears the measure map/)).toBeTruthy();
  const close=within(chooser).getByRole('button',{name:'Close recording replacement'});
  expect(close.parentElement?.firstElementChild).toBe(close);
  await fireEvent.click(close);
  expect(chooser.hasAttribute('open')).toBe(false);
  expect(mock.replaceRecording).not.toHaveBeenCalled();
});

it('recovers an unavailable library recording without opening it or changing its song UUID',async()=>{
  const entry=libraryEntry('unavailable-song','Missing recording','Player',1,2);
  mock.initialize.mockResolvedValue({song:null,document:null,transport:{...transport,song_key:null,reference_token:null},startup_notice:'The previous recording is unavailable.'});
  mock.readLibrary.mockResolvedValue({version:2,songs:[entry]});
  mock.libraryReference.mockResolvedValue('persisted-reference-before-picker');
  const token='replacement-reference-session';
  mock.replaceLibraryReference.mockResolvedValue({song:{key:entry.song_key,reference_token:token,asset_id:'replacement',timeline_id:'new-timeline',reference_revision:2,session_id:'session',title:entry.metadata.title,path:'/replacement.wav',duration:12,rate:48000,peaks:[[-.2,.2]]},document:{...document,song_key:entry.song_key,reference_token:token},transport:{...transport,song_key:entry.song_key,reference_token:token}});
  render(App);
  await screen.findByText('The previous recording is unavailable.');
  await waitFor(()=>expect(screen.getByRole('button',{name:'Library'}).hasAttribute('disabled')).toBe(false));
  await fireEvent.click(screen.getByRole('button',{name:'Library'}));
  await screen.findByRole('button',{name:'Missing recording'});
  expect(screen.queryByRole('button',{name:'Replace recording for Missing recording'})).toBeNull();
  await fireEvent.click(screen.getByRole('button',{name:'Edit details for Missing recording'}));
  await fireEvent.click(within(screen.getByRole('complementary',{name:'Song details'})).getByRole('button',{name:'Replace recording for Missing recording'}));
  await screen.findByRole('dialog',{name:'Replace reference recording'});
  await waitFor(()=>expect(mock.libraryReference).toHaveBeenCalledExactlyOnceWith(entry.song_key));
  await fireEvent.click(screen.getByRole('button',{name:'Choose recording with changed timing'}));
  await waitFor(()=>expect(mock.replaceLibraryReference).toHaveBeenCalledExactlyOnceWith(entry.song_key,'persisted-reference-before-picker',false));
  await screen.findByRole('heading',{name:'Missing recording'});
  expect(mock.openLibrarySong).not.toHaveBeenCalled();
});
