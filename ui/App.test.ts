import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import App from './App.svelte';
import { installNativeChrome } from './native-chrome';
import { installFieldCommit } from './field-commit';
import { timingFixture } from './timing-fixture';
import type { PracticeDocument, Snapshot, Transport } from './types';
import type {StemProgress} from './stems';
import type {SeparationStatus} from './separation';

const mock = vi.hoisted(() => ({ separationEstimate:vi.fn(),separationStatus:vi.fn(),installSeparationModel:vi.fn(),reinstallSeparationModel:vi.fn(),reinstallSeparationRuntime:vi.fn(),removeSeparationRuntime:vi.fn(),removeSeparationModel:vi.fn(),refreshSeparationRuntime:vi.fn(),startSeparation:vi.fn(),cancelSeparation:vi.fn(), stemsStatus:vi.fn(),setStemMix:vi.fn(),attachStems:vi.fn(),removeStems:vi.fn(),pickStemsDirectory:vi.fn(),onStemProgress:vi.fn(), libraryStatus:vi.fn(),chooseLibrary:vi.fn(),retryLibrary:vi.fn(),acknowledgeImportNotice:vi.fn(),pickImportSong:vi.fn(),inspectImportSong:vi.fn(),onFileDrop:vi.fn(),importSong:vi.fn(),deleteLibrarySong:vi.fn(),onImportProgress:vi.fn(), initialize: vi.fn(), openSong: vi.fn(),importSeparate:vi.fn(),replaceRecording:vi.fn(),libraryReference:vi.fn(),replaceLibraryReference:vi.fn(), update: vi.fn(), action: vi.fn(), status: vi.fn(), readKeymap: vi.fn(), analysisStatus: vi.fn(), readCachedAnalysis:vi.fn(), readLibrary:vi.fn(),openLibrarySong:vi.fn(),updateLibrarySong:vi.fn(), onExitRequested:vi.fn(),finishExit:vi.fn(), previewMap: vi.fn(), mapView: vi.fn(), countIn:vi.fn(), metronome:vi.fn() }));
vi.mock('./bridge', () => ({ native: true, bridge: mock }));
const menu = vi.hoisted(() => {
  const items: { id?: string; action?: () => void; setEnabled: unknown; close: unknown }[] = [];
  const resource = (options: object) => { const item = { ...options, setEnabled: vi.fn(async () => {}), close: vi.fn(async () => {}) }; items.push(item); return item; };
  return { items, module: {
    Menu: { new: vi.fn(async (options: object) => ({ ...resource(options), setAsAppMenu: vi.fn(async () => {}) })) },
    Submenu: { new: vi.fn(async (options: object) => resource(options)) },
    PredefinedMenuItem: { new: vi.fn(async (options: object) => resource(options)) },
    MenuItem: { new: vi.fn(async (options: object) => resource(options)) }
  } };
});
vi.mock('@tauri-apps/api/menu', () => menu.module);
let document: PracticeDocument;
let transport: Transport;
const response = (): Snapshot => ({ song: null, document: structuredClone(document), transport: { ...transport } });

function separationStatus():SeparationStatus {const attempt={id:'',running:false,phase:'Idle',reference_token:null,progress:null,error:null,details:'',elapsed_seconds:0,cancellable:false,result:null};return {package_id:'roformer-sw-v1',profile_id:'roformer-sw-cpu2-v1',model_ready:false,model_path:'/models',package_bytes:699416805,runtime:{ready:true,message:'Ready',path:'/python'},heavy_work:null,install:{...attempt},job:{...attempt}};}

beforeEach(() => {
  vi.clearAllMocks();
  delete window.document.documentElement.dataset.modality;
  mock.separationEstimate.mockResolvedValue(1950628864);mock.separationStatus.mockResolvedValue(separationStatus());
  mock.reinstallSeparationModel.mockResolvedValue(separationStatus());mock.reinstallSeparationRuntime.mockResolvedValue(separationStatus());mock.removeSeparationRuntime.mockResolvedValue(separationStatus());
  mock.installSeparationModel.mockResolvedValue(separationStatus());mock.removeSeparationModel.mockResolvedValue(separationStatus());mock.refreshSeparationRuntime.mockResolvedValue(separationStatus());mock.startSeparation.mockResolvedValue(separationStatus());mock.cancelSeparation.mockResolvedValue(separationStatus());
  mock.stemsStatus.mockImplementation(async(reference:string)=>({reference_token:reference,set_id:null,tracks:[],available:false,warning:null,attenuation:null,model:null}));mock.onStemProgress.mockResolvedValue(()=>{});
  mock.libraryStatus.mockResolvedValue({selected:{id:'library',root:'/library'},available:true,error:null,import_notice_seen:true});mock.onImportProgress.mockResolvedValue(()=>{});mock.onFileDrop.mockResolvedValue(()=>{});mock.pickImportSong.mockResolvedValue({path:'/song.wav',candidates:[]});mock.importSong.mockImplementation(()=>mock.openSong());
  mock.metronome.mockResolvedValue({});
  mock.countIn.mockResolvedValue({});
  mock.onExitRequested.mockResolvedValue(()=>{});mock.finishExit.mockResolvedValue(undefined);
  mock.readCachedAnalysis.mockResolvedValue(null);
  mock.readLibrary.mockResolvedValue({version:1,songs:[libraryEntry('test','Workshop take','Player',1,2)]});
  mock.analysisStatus.mockResolvedValue({id:0,running:false,kind:'idle',phase:'Ready',model_path:'/test/models',progress:null,elapsed_seconds:0,model_ready:false,reference_token:null,map_revision:0,error:null,result:null});
  mock.readKeymap.mockResolvedValue({ path: '/test/keymap.json', configuration: null, warning: null });
  document = { version: 4, notes:'', song_key: 'test',reference_token:'test', controls: { speed: 1, semitones: 0, tuning_cents: 0 },
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

// Seeks run through the app's action queue, so wait for the rendered slider as
// well as the mock before issuing a dependent keyboard command.
async function shown(seconds: number, name = 'Song position') {
  await waitFor(() => expect(transport.position).toBe(seconds));
  await waitFor(() => expect(screen.getByRole('slider', { name }).getAttribute('aria-valuenow')).toBe(String(seconds)));
}

const speedField = () => within(screen.getByRole('group', { name: 'Speed' })).getByLabelText('Speed percent') as HTMLInputElement;
async function openPitch() {
  if (!screen.queryByRole('dialog', { name: 'Pitch' })) await fireEvent.click(screen.getByRole('button', { name: /^Pitch / }));
  return screen.getByRole('dialog', { name: 'Pitch' });
}

// The Library header has no song views; the current song's → returns to Practice.
async function backToPractice(title = 'Workshop take') {
  const row = await screen.findByRole('button', { name: `Back to ${title}` });
  await waitFor(() => expect(row.hasAttribute('disabled')).toBe(false));
  await fireEvent.click(row);
  await screen.findByRole('main', { name: 'Practice workspace' });
}

async function openBounds() {
  if (!screen.queryByRole('dialog', { name: 'Passage bounds' })) await fireEvent.click(screen.getByRole('button', { name: /^(Looping|Passage · loop off) / }));
  return screen.getByRole('dialog', { name: 'Passage bounds' });
}

async function ready() {
  render(App);
  await screen.findByRole('heading', { name: 'Workshop take' });
  await screen.findByText('Saved');
  await waitFor(() => expect(screen.queryByText('Restoring practice…')).toBeNull());
}

describe('single-song practice workflow', () => {
  it('shows save state and shortcut help in the header, and appearance and the audio device in the Library header', async () => {
    await ready();
    const header = screen.getByRole('banner');
    expect(within(header).getByRole('status').textContent).toBe('Saved');
    expect(within(header).getByRole('button', { name: 'Keyboard shortcuts' })).toBeTruthy();
    expect(within(header).queryByRole('button', { name: /^UI: / })).toBeNull();
    expect(screen.queryByRole('contentinfo')).toBeNull();
    await fireEvent.click(within(header).getByRole('button', { name: 'Library' }));
    const appearance = await within(header).findByRole('button', { name: /^UI: / });
    expect(within(header).getByText('Test output')).toBeTruthy();
    mock.action.mockClear(); mock.update.mockClear();
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
    await fireEvent.click(screen.getByRole('button', { name: 'Loop', pressed: false }));
    await waitFor(() => expect(document.loop_enabled).toBe(true));
    expect(document.loop_selection?.start).toBe(3);
  });
  it('moves an enabled loop to a selected section without starting playback', async () => {
    await ready(); await fireEvent.click(screen.getByRole('button', { name: 'Loop', pressed: false }));
    await screen.findByRole('button', { name: 'Loop', pressed: true });
    await fireEvent.click(screen.getByRole('button', { name: /Chorus 0:03/ }));
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'seek', seconds: 3 },'test'));
    expect(document.loop_selection).toEqual({ start: 3, end: 6, fade_ms: 5 });
    expect(document.selected_section).toBe('chorus'); expect(transport.playing).toBe(false);
  });
  it('preserves a pitch adjustment when changing speed quickly afterwards', async () => {
    await ready();
    const pitch = await openPitch();
    for (let i = 0; i < 3; i++) await fireEvent.click(within(pitch).getByRole('button', { name: 'Transpose down' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Slower by 10 percent' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Slower by 5 percent' }));
    await waitFor(() => expect(document.controls).toEqual({ speed: .85, semitones: -3, tuning_cents: 0 }));
    expect(screen.getByRole('button', { name: /^Pitch / }).getAttribute('aria-label')).toBe('Pitch −3 st, recording tuned to A = 440 Hz');
  });
  it('keeps the reference tuning separate from whole-semitone transposition and resets each on its own', async () => {
    await ready();
    const pitch = await openPitch();
    const hz = within(pitch).getByLabelText('Recording tuned to A (Hz)') as HTMLInputElement;
    expect(hz.value).toBe('440');
    await fireEvent.change(hz, { target: { value: '432' } });
    await fireEvent.click(within(pitch).getByRole('button', { name: 'Transpose up' }));
    await fireEvent.click(within(pitch).getByRole('button', { name: 'Transpose up' }));
    await fireEvent.change(speedField(), { target: { value: '87.5' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Faster by 5 percent' }));
    await waitFor(() => expect(document.controls).toEqual({ speed: .93, semitones: 2, tuning_cents: 1200 * Math.log2(440 / 432) }));
    // The tile shows both changes; the popover explains the correction.
    expect(screen.getByRole('button', { name: /^Pitch / }).getAttribute('aria-label')).toBe('Pitch +2 st, recording tuned to A = 432 Hz');
    expect(pitch.textContent).toContain('Plays +31.8 ¢ to match A = 440 Hz');
    await fireEvent.click(within(pitch).getByRole('button', { name: 'Reset transpose' }));
    await waitFor(() => expect(document.controls.semitones).toBe(0));
    expect(document.controls.tuning_cents).toBeCloseTo(31.77, 2);
    // The engine accepts ±50 ¢ (about A = 427.5–452.9 Hz); entries outside clamp.
    await fireEvent.change(hz, { target: { value: '400' } });
    await waitFor(() => expect(document.controls.tuning_cents).toBe(50));
    await fireEvent.change(speedField(), { target: { value: '' } });
    expect(speedField().value).toBe('93');
    await fireEvent.change(hz, { target: { value: 'abc' } });
    expect(hz.value).toBe('427.5');
    await fireEvent.click(within(pitch).getByRole('button', { name: 'Reset tuning' }));
    await waitFor(() => expect(document.controls).toEqual({ speed: .93, semitones: 0, tuning_cents: 0 }));
    expect(pitch.textContent).toContain('Standard tuning');
  });
  it('creates a named phrase inside a longer section', async () => {
    await ready(); await fireEvent.keyDown(window, { key: 's', code: 'KeyS' });
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
    await ready(); await fireEvent.keyDown(speedField(), { key: ' ', code: 'Space' });
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
    await fireEvent(within(await openBounds()).getByLabelText('Loop start seconds'), decimal);
    expect(decimal.defaultPrevented).toBe(false);
    await fireEvent.click(screen.getByRole('button', { name: 'Save as section' }));
    await fireEvent.keyDown(window, { key: '.', code: 'Period' });
    expect(mock.action).not.toHaveBeenCalled();
  });
  it('clears the A–B range with X while preserving saved sections and ongoing playback', async () => {
    await ready(); await fireEvent.click(screen.getByRole('button', { name: /Chorus 0:03/ }));
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'seek', seconds: 3 },'test'));
    await fireEvent.click(screen.getByRole('button', { name: 'Loop', pressed: false }));
    await waitFor(() => expect(document.loop_enabled).toBe(true));
    await fireEvent.keyDown(window, { key: ' ', code: 'Space' });
    await waitFor(() => expect(transport.playing).toBe(true));
    const sections = structuredClone(document.sections);
    mock.action.mockClear(); mock.update.mockClear();
    await fireEvent.keyDown(speedField(), { key: 'x', code: 'KeyX' });
    expect(mock.update).not.toHaveBeenCalled(); expect(document.loop_selection).not.toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Save as section' }));
    await fireEvent.keyDown(window, { key: 'x', code: 'KeyX' });
    expect(mock.update).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    await fireEvent.keyDown(window, { key: 'X', code: 'KeyX', shiftKey: true });
    await waitFor(() => expect(document.loop_selection).toBeNull());
    expect(document.loop_enabled).toBe(false);
    expect(document.sections).toEqual(sections); expect(document.selected_section).toBe('chorus');
    expect(transport.playing).toBe(true); expect(mock.action).not.toHaveBeenCalled();
    expect(screen.queryByRole('button', { name: 'Clear passage' })).toBeNull();
  });
  it('jumps to B and turns looping off without pausing playback or clearing the range', async () => {
    await ready(); await fireEvent.click(screen.getByRole('button', { name: /Chorus 0:03/ }));
    await waitFor(() => expect(transport.position).toBe(3));
    await fireEvent.click(screen.getByRole('button', { name: 'Loop', pressed: false }));
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
    await fireEvent.click(screen.getByRole('button', { name: 'Loop', pressed: false }));
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
    const previous = within(screen.getByRole('complementary', { name: 'Sections' })).getByRole('button', { name: 'Back to the riff' });
    expect(previous.getAttribute('title')).toBe('Back to the riff (Q)');
    mock.action.mockClear(); await fireEvent.click(previous);
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'seek', seconds: 3 },'test'));
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
  // A Tab-focused button owns Enter (keyboard modality), so it does not also jump to the start.
  window.document.documentElement.dataset.modality='keyboard';
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
  document.controls={speed:.75,semitones:-3, tuning_cents: 0};document.loop_selection={start:3,end:6,fade_ms:5};document.loop_enabled=true;
  mock.initialize.mockResolvedValue({...response(),song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  const waveform=screen.getByRole('region',{name:/Measure-map waveform/}),playback=within(waveform).getByRole('group',{name:'Measure mapping playback'});
  expect(within(screen.getByRole('region',{name:'Measure mapping workspace'})).queryByLabelText('Pitch semitones')).toBeNull();
  expect(within(playback).queryByRole('slider')).toBeNull();
  const speedInput=within(playback).getByRole('spinbutton',{name:'Speed percent'});
  for(const type of ['pointerdown','pointerup']){const event=new MouseEvent(type,{bubbles:true,button:0});Object.defineProperty(event,'pointerId',{value:1});await fireEvent(speedInput,event);}
  await fireEvent.change(speedInput,{target:{value:'50'}});
  await waitFor(()=>expect(document.controls).toEqual({speed:.5,semitones:-3, tuning_cents: 0}));
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
  document.measure_map=structuredClone(original.map);document.controls={speed:.75,semitones:-3, tuning_cents: 0};document.loop_selection={start:1,end:3,fade_ms:5};document.loop_enabled=true;
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
  expect(document.controls).toEqual({speed:.75,semitones:-3, tuning_cents: 0});expect(document.loop_selection).toEqual({start:1,end:3,fade_ms:5});expect(mock.action).not.toHaveBeenCalled();
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
  document.measure_map=structuredClone(original.map);document.controls={speed:.75,semitones:-3, tuning_cents: 0};
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
  expect(document.controls).toEqual({speed:.75,semitones:-3, tuning_cents: 0});expect(document.measure_map).toEqual(original.map);
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
  document.measure_map=structuredClone(original.map);document.controls={speed:.75,semitones:-3, tuning_cents: 0};document.loop_selection={start:3,end:6,fade_ms:5};document.loop_enabled=true;
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  const empty={...original,map:{...original.map,revision:2,first_number:1,measures:[],boundaries:[]},view:{revision:2,measures:[]}};
  mock.previewMap.mockResolvedValue(empty);mock.mapView.mockResolvedValue({...original.view,revision:3});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));await fireEvent.click(screen.getByRole('button',{name:'Clear map'}));
  await waitFor(()=>expect(document.measure_map?.measures).toHaveLength(0));expect(mock.previewMap.mock.lastCall?.[2]).toEqual({type:'clear'});
  await waitFor(()=>expect(screen.getByRole('button',{name:'Undo'}).hasAttribute('disabled')).toBe(false));await fireEvent.click(screen.getByRole('button',{name:'Undo'}));
  await waitFor(()=>expect(document.measure_map?.measures).toEqual(original.map.measures));expect(document.measure_map?.boundaries).toEqual(original.map.boundaries);
  expect(document.controls).toEqual({speed:.75,semitones:-3, tuning_cents: 0});expect(document.loop_enabled).toBe(true);expect(document.loop_selection).toEqual({start:3,end:6,fade_ms:5});expect(document.sections[0].id).toBe('verse');
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
  // Practice always follows; the measure map keeps its own toggle.
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
  // In Practice a manual pan holds while playing until the playhead next leaves the page.
  await fireEvent.click(overview,{clientX:300});
  expect(parseFloat(viewport.style.left)).toBeGreaterThan(0);
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
  const click=within(screen.getByRole('group',{name:'Click track'}));
  expect(click.getByRole('button',{name:'On'}).getAttribute('aria-pressed')).toBe('true');
  await fireEvent.click(click.getByRole('button',{name:'On'}));
  for(let i=0;i<15;i++)await fireEvent.keyDown(click.getByRole('slider',{name:'Click gain'}),{key:'ArrowUp'});
  expect(click.getByRole('slider',{name:'Click gain'}).getAttribute('aria-valuenow')).toBe('65');
  await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  expect(screen.getByRole('button',{name:'Metronome off'}).getAttribute('aria-pressed')).toBe('false');
  expect((screen.getByLabelText('Metronome volume') as HTMLInputElement).value).toBe('65');
  await fireEvent.click(screen.getByRole('button',{name:'Metronome off'}));
  await fireEvent.click(screen.getByRole('button',{name:'Preview numbering'}));
  await waitFor(()=>expect(mock.metronome.mock.lastCall).toEqual(['test',true,.65,draft.map]));
  await fireEvent.click(screen.getByRole('button',{name:'Close measure map'}));
  await waitFor(()=>expect(mock.metronome.mock.lastCall).toEqual(['test',true,.65,original.map]));
  expect(within(screen.getByRole('group',{name:'Click track'})).getByRole('button',{name:'On'}).getAttribute('aria-pressed')).toBe('true');
  expect(document.measure_map).toEqual(original.map);
});

it('defaults to measure loops and snaps A/B shortcuts, with exact inputs and a free selection option',async()=>{
  const original=timingFixture(48000);document.measure_map=original.map;transport.position=5;
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();expect(screen.getByRole('button',{name:'Snap to measures'}).getAttribute('aria-pressed')).toBe('true');
  await fireEvent.click(screen.getByRole('button',{name:'Loop',pressed:false}));
  await waitFor(()=>expect(document.loop_selection).toEqual({start:4,end:8,fade_ms:5}));
  await fireEvent.keyDown(window,{key:'Enter'});await waitFor(()=>expect(transport.position).toBe(0));
  await fireEvent.keyDown(window,{key:'a'});await waitFor(()=>expect(document.loop_selection?.start).toBe(0));
  await fireEvent.keyDown(window,{key:'ArrowRight'});await shown(4);
  await fireEvent.keyDown(window,{key:'ArrowRight',shiftKey:true});await shown(4.1);
  await fireEvent.keyDown(window,{key:'b'});await waitFor(()=>expect(document.loop_selection?.end).toBe(4));
  await fireEvent.change(within(await openBounds()).getByLabelText('Loop end seconds'),{target:{value:'5.25'}});
  await waitFor(()=>expect(document.loop_selection?.end).toBe(5.25));
  await fireEvent.click(screen.getByRole('button',{name:'Snap to measures'}));
  expect(screen.getByRole('button',{name:'Snap to measures'}).getAttribute('aria-pressed')).toBe('false');
  await fireEvent.keyDown(window,{key:'b'});await waitFor(()=>expect(document.loop_selection?.end).toBe(4.1));
});


it('sets an initial mapped loop to one measure near a barline, preserving free-time A/B behavior',async()=>{
  const original=timingFixture(48000);document.measure_map=original.map;transport.position=3.99;
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();await fireEvent.keyDown(window,{key:'a'});
  await waitFor(()=>expect(document.loop_selection).toEqual({start:0,end:4,fade_ms:5}));
  await fireEvent.keyDown(window,{key:'x'});await waitFor(()=>expect(document.loop_selection).toBeNull());
  await fireEvent.keyDown(screen.getByRole('slider',{name:'Song position'}),{key:'End'});await waitFor(()=>expect(transport.position).toBe(12));
  await fireEvent.keyDown(window,{key:'b'});
  await waitFor(()=>expect(document.loop_selection).toEqual({start:0,end:12,fade_ms:5}));
});



it.each([false,true])('pages practice playback with mapped=%s, retaining zoom through boundaries, loop wraps and EOF',async(mapped)=>{
  if(mapped){
    const fixture=timingFixture(48000);document.measure_map=fixture.map;
    mock.initialize.mockResolvedValue({...response(),map_view:fixture.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  }
  await ready();
  expect(screen.queryByRole('button',{name:/Follow playhead/})).toBeNull();
  expect(screen.queryByRole('button',{name:'Zoom in'})).toBeNull();
  await fireEvent.keyDown(window,{key:'='}); // 0..6 s.
  const viewport=screen.getByRole('button',{name:'Move waveform view'}).querySelector('.viewport') as HTMLElement;
  expect(viewport.style.width).toBe('50%');expect(viewport.style.left).toBe('0%');
  transport.position=7;await shown(7);
  expect(viewport.style.left).toBe('0%'); // Paused playback does not follow.
  await fireEvent.click(screen.getByRole('button',{name:'Play'}));
  await waitFor(()=>expect(viewport.style.left).toBe('50%'));
  transport.position=5.999;await waitFor(()=>expect(viewport.style.left).toBe('0%')); // Backwards wrap.
  transport.position=6;await waitFor(()=>expect(viewport.style.left).toBe('50%')); // Exclusive page end.
  transport.position=12;await shown(12);
  expect(viewport.style.left).toBe('50%');expect(viewport.style.width).toBe('50%');
  expect(mock.action.mock.calls.map(c=>c[0].type)).toEqual(['toggle_play']); // Paging never seeks audio.
});

it('follows the playing playhead in Practice by pages, after panning, zooming and other screens',async()=>{
  await ready();await fireEvent.keyDown(window,{key:'='}); // 0..6 s.
  const overview=screen.getByRole('button',{name:'Move waveform view'}),field=screen.getByRole('region',{name:/^Waveform/});
  const box={left:0,width:1000,height:200,top:0,right:1000,bottom:200,x:0,y:0,toJSON(){}};
  for(const element of [overview,field])vi.spyOn(element,'getBoundingClientRect').mockReturnValue(box);
  field.setPointerCapture=vi.fn();field.hasPointerCapture=vi.fn(()=>false);
  const viewport=overview.querySelector('.viewport') as HTMLElement;
  await fireEvent.click(overview,{clientX:900});expect(viewport.style.left).toBe('50%');
  for(const [type,x] of [['pointerdown',500],['pointermove',700],['pointerup',700]] as const){
    const event=new MouseEvent(type,{bubbles:true,cancelable:true,clientX:x,button:1});Object.defineProperty(event,'pointerId',{value:1});await fireEvent(field,event);
  }
  await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  await fireEvent.click(screen.getByRole('button',{name:'Practice'}));
  // Starting playback off-page reveals the playhead, keeping the zoom.
  const width=viewport.style.width;
  await fireEvent.click(screen.getByRole('button',{name:'Play'}));
  await waitFor(()=>expect(viewport.style.left).toBe('0%'));expect(viewport.style.width).toBe(width);
  // Crossing the page's right edge turns the page.
  transport.position=6.5;await waitFor(()=>expect(parseFloat(viewport.style.left)).toBeCloseTo(50));
});

it('keeps a minimap move made during playback until the playhead next leaves the page',async()=>{
  await ready();await fireEvent.keyDown(window,{key:'='}); // 0..6 s.
  const overview=screen.getByRole('button',{name:'Move waveform view'});
  vi.spyOn(overview,'getBoundingClientRect').mockReturnValue({left:0,width:1000,height:34,top:0,right:1000,bottom:34,x:0,y:0,toJSON(){}});
  const viewport=overview.querySelector('.viewport') as HTMLElement;
  await fireEvent.click(screen.getByRole('button',{name:'Play'}));await waitFor(()=>expect(transport.playing).toBe(true));
  transport.position=1;await shown(1);
  await fireEvent.click(overview,{clientX:900});expect(viewport.style.left).toBe('50%');
  transport.position=2;await shown(2);transport.position=3;await shown(3);
  expect(viewport.style.left).toBe('50%'); // The pan sticks while playing.
  transport.position=.5;await waitFor(()=>expect(viewport.style.left).toBe('0%')); // A wrap reveals the playhead.
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
  await fireEvent.keyDown(screen.getByRole('slider',{name:'Measure-map song position'}),{key:'End'});
  await shown(12,'Measure-map song position');
  await fireEvent.keyDown(screen.getByRole('slider',{name:'Measure-map song position'}),{key:'ArrowLeft',shiftKey:true});
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

const libraryEntry=(key:string,title:string,artist:string,added:number,opened:number)=>({song_key:key,metadata:{title,artist,album:'Workshop'},file_location:`/music/${title}.wav`,date_added:added,last_opened:opened,duration_ms:12_000});
const songRows=()=>within(screen.getByRole('listbox',{name:'Songs'})).getAllByRole('option');
it('opens the searchable library, pauses playback, sorts and edits metadata only through the edit action',async()=>{
  const current=libraryEntry('test','Workshop take','Zed',1000,2000),other={...libraryEntry('other','Riff 2','Alpha',2000,1000),duration_ms:3_725_000};
  mock.readLibrary.mockResolvedValue({version:1,songs:[current,other]});mock.updateLibrarySong.mockImplementation(async(key,metadata)=>({version:1,songs:[{...current,metadata},other]}));
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Play'}));await fireEvent.click(screen.getByRole('button',{name:'Library'}));
  await screen.findByRole('button',{name:'Practise Riff 2'});expect(mock.action).toHaveBeenLastCalledWith({type:'pause'},'test');
  // Rows show title, artist and length, recently opened first; the loaded song is marked.
  expect(songRows().map(row=>row.textContent)).toEqual([expect.stringMatching(/Workshop take.*Current.*Zed.*0:12/),expect.stringMatching(/Riff 2.*Alpha.*1:02:05/)]);
  expect(songRows()[0].getAttribute('aria-current')).toBe('true');
  expect(screen.queryByText('/music/Riff 2.wav')).toBeNull();
  await fireEvent.change(screen.getByRole('combobox',{name:'Sort by'}),{target:{value:'artist'}});expect(songRows()[0].textContent).toContain('Riff 2');
  await fireEvent.click(screen.getByRole('button',{name:'Ascending order'}));expect(songRows()[0].textContent).toContain('Workshop take');
  await fireEvent.input(screen.getByRole('searchbox',{name:'Find a song'}),{target:{value:'Zed Workshop'}});expect(screen.queryByRole('button',{name:'Practise Riff 2'})).toBeNull();
  await fireEvent.input(screen.getByRole('searchbox',{name:'Find a song'}),{target:{value:'bossa nova'}});await screen.findByText('No songs match “bossa nova”.');
  await fireEvent.click(screen.getByRole('button',{name:'Clear search'}));expect(songRows()).toHaveLength(2);
  // A click only selects; it never shows the edit fields.
  await fireEvent.click(songRows()[0]);expect(songRows()[0].getAttribute('aria-selected')).toBe('true');expect(screen.queryByRole('textbox',{name:'Title'})).toBeNull();
  await fireEvent.click(screen.getByRole('button',{name:'Edit details of Workshop take'}));
  const editor=screen.getByRole('complementary',{name:'Edit song details'});
  expect(within(editor).getByText('0:12')).toBeTruthy();expect(within(editor).getByText('/music/Workshop take.wav')).toBeTruthy();
  expect(within(editor).queryByRole('button',{name:'Save details'})).toBeNull();
  await fireEvent.input(screen.getByRole('textbox',{name:'Title'}),{target:{value:'My guitar take'}});
  await fireEvent.click(screen.getByRole('button',{name:'Save details'}));await waitFor(()=>expect(mock.updateLibrarySong).toHaveBeenCalled());await backToPractice('My guitar take');await screen.findByRole('heading',{name:'My guitar take'});expect(mock.updateLibrarySong).toHaveBeenCalledExactlyOnceWith('test',{title:'My guitar take',artist:'Zed',album:'Workshop'});
  expect(screen.queryByRole('textbox',{name:'Title'})).toBeNull();expect(mock.update).not.toHaveBeenCalled();
});
it('opens a song by double-click or Enter, moves the selection with the arrows, and edits with ⌘I',async()=>{
  const uninstall=installFieldCommit(window.document);
  const other=libraryEntry('other','Another take','Player',1000,1);
  mock.readLibrary.mockResolvedValue({version:2,songs:[libraryEntry('test','Workshop take','Player',1,2),other]});
  mock.openLibrarySong.mockResolvedValue({...response(),song:{key:'other',reference_token:'other',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Another take',path:other.file_location,duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Library'}));await screen.findByRole('button',{name:'Practise Another take'});
  const list=screen.getByRole('listbox',{name:'Songs'});list.focus();
  await fireEvent.keyDown(list,{key:'ArrowDown'});expect(songRows()[0].getAttribute('aria-selected')).toBe('true');
  await fireEvent.keyDown(list,{key:'ArrowDown'});expect(songRows()[1].getAttribute('aria-selected')).toBe('true');
  expect(list.getAttribute('aria-activedescendant')).toBe(songRows()[1].id);
  await fireEvent.keyDown(list,{key:'i',metaKey:true});await fireEvent.keyDown(list,{key:'i',ctrlKey:true});
  expect((await screen.findByRole('textbox',{name:'Title'}) as HTMLInputElement).value).toBe('Another take');
  expect(window.document.activeElement).toBe(screen.getByRole('textbox',{name:'Title'}));
  // Esc unwinds: the field returns home, then the editor closes, then the Library.
  await fireEvent.keyDown(screen.getByRole('textbox',{name:'Title'}),{key:'Escape'});await fireEvent.keyDown(window.document.activeElement!,{key:'Escape'});
  await waitFor(()=>expect(screen.queryByRole('complementary',{name:'Edit song details'})).toBeNull());
  expect(screen.getByRole('main',{name:'Music library'})).toBeTruthy();
  await fireEvent.keyDown(screen.getByRole('listbox',{name:'Songs'}),{key:'Enter'});
  await waitFor(()=>expect(mock.openLibrarySong).toHaveBeenCalledExactlyOnceWith('other'));await screen.findByRole('heading',{name:'Another take'});
  mock.openLibrarySong.mockClear();await fireEvent.click(screen.getByRole('button',{name:'Library'}));
  await fireEvent.dblClick(await screen.findByRole('option',{name:/Workshop take/}));
  await waitFor(()=>expect(mock.openLibrarySong).toHaveBeenCalledExactlyOnceWith('test'));
  uninstall();
});
describe('the details editor never loses edits',()=>{
  const two=()=>[libraryEntry('test','Workshop take','Player',1,2),libraryEntry('other','Another take','Player',1000,1)];
  async function editFirst(){
    mock.readLibrary.mockResolvedValue({version:2,songs:two()});
    mock.updateLibrarySong.mockImplementation(async(key:string,metadata:{title:string;artist:string;album:string})=>({version:2,songs:two().map(entry=>entry.song_key===key?{...entry,metadata}:entry)}));
    await fireEvent.click(screen.getByRole('button',{name:'Library'}));
    await fireEvent.click(await screen.findByRole('button',{name:'Edit details of Workshop take'}));
    await fireEvent.input(screen.getByRole('textbox',{name:'Title'}),{target:{value:'Edited take'}});
    (window.document.activeElement as HTMLElement).blur();
  }
  it('follows the selection with ↓ without taking focus, saving the edits first, and Enter opens the song',async()=>{
    await ready();await editFirst();
    const list=screen.getByRole('listbox',{name:'Songs'});list.focus();
    await fireEvent.keyDown(list,{key:'ArrowDown'});
    await waitFor(()=>expect(mock.updateLibrarySong).toHaveBeenCalledExactlyOnceWith('test',{title:'Edited take',artist:'Player',album:'Workshop'}));
    await waitFor(()=>expect((screen.getByRole('textbox',{name:'Title'}) as HTMLInputElement).value).toBe('Another take'));
    expect(window.document.activeElement).toBe(list);
    await fireEvent.keyDown(list,{key:'Enter'});
    await waitFor(()=>expect(mock.openLibrarySong).toHaveBeenCalledExactlyOnceWith('other'));
  });
  it('keeps the selection and the edits when saving before a switch fails',async()=>{
    await ready();await editFirst();mock.updateLibrarySong.mockRejectedValue(new Error('Disk full'));
    await fireEvent.click(screen.getByRole('option',{name:/Another take/}));
    await screen.findByText(/Couldn’t save: Error: Disk full/);
    expect((screen.getByRole('textbox',{name:'Title'}) as HTMLInputElement).value).toBe('Edited take');
    expect(screen.getByRole('option',{name:/Another take/}).getAttribute('aria-selected')).toBe('false');
  });
  it('discards the edits when Esc closes the editor',async()=>{
    await ready();await editFirst();
    await fireEvent.keyDown(screen.getByRole('listbox',{name:'Songs'}),{key:'Escape'});
    await waitFor(()=>expect(screen.queryByRole('complementary',{name:'Edit song details'})).toBeNull());
    expect(mock.updateLibrarySong).not.toHaveBeenCalled();expect(screen.getByRole('main',{name:'Music library'})).toBeTruthy();
    await fireEvent.click(screen.getByRole('button',{name:'Edit details of Workshop take'}));
    expect((screen.getByRole('textbox',{name:'Title'}) as HTMLInputElement).value).toBe('Workshop take');
  });
  it('saves the edits before leaving for another view, and stays when that save fails',async()=>{
    mock.readKeymap.mockResolvedValue({path:'/k.json',configuration:{version:1,keys:{show_practice:['Mod+1']}},warning:null});
    await ready();await editFirst();
    mock.updateLibrarySong.mockRejectedValueOnce(new Error('Disk full'));
    await fireEvent.keyDown(window,{key:'1',ctrlKey:true});
    await screen.findByText(/Couldn’t save: Error: Disk full/);expect(screen.getByRole('main',{name:'Music library'})).toBeTruthy();
    await fireEvent.keyDown(window,{key:'1',ctrlKey:true});
    await screen.findByRole('main',{name:'Practice workspace'});
    expect(mock.updateLibrarySong).toHaveBeenLastCalledWith('test',{title:'Edited take',artist:'Player',album:'Workshop'});
    await screen.findByRole('heading',{name:'Edited take'});
  });
  it('saves the edits before quitting, and keeps the app open when that fails',async()=>{
    await ready();await editFirst();
    const onExit=mock.onExitRequested.mock.lastCall![0] as ()=>Promise<void>;
    mock.updateLibrarySong.mockRejectedValueOnce(new Error('Disk full'));
    await onExit();expect(mock.finishExit).not.toHaveBeenCalled();await screen.findByText(/Couldn’t save: Error: Disk full/);
    await onExit();expect(mock.updateLibrarySong).toHaveBeenLastCalledWith('test',{title:'Edited take',artist:'Player',album:'Workshop'});
    expect(mock.finishExit).toHaveBeenCalledOnce();
  });
});
it('returns to the current song as it was left with Esc, without reloading it',async()=>{
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Library'}));await screen.findByRole('button',{name:'Back to Workshop take'});
  await fireEvent.keyDown(screen.getByRole('listbox',{name:'Songs'}),{key:'Escape'});
  await screen.findByRole('main',{name:'Practice workspace'});expect(mock.openLibrarySong).not.toHaveBeenCalled();
});
it('focuses the search with ⌘F and keeps typed letters in it',async()=>{
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Library'}));const list=await screen.findByRole('listbox',{name:'Songs'});
  await fireEvent.keyDown(list,{key:'f',ctrlKey:true});await fireEvent.keyDown(list,{key:'f',metaKey:true});
  expect(window.document.activeElement).toBe(screen.getByRole('searchbox',{name:'Find a song'}));
});
it('opens Settings with ⌘, from any screen and from the Library’s Settings button',async()=>{
  await ready();expect(within(screen.getByRole('banner')).queryByRole('button',{name:'Settings'})).toBeNull();
  await fireEvent.keyDown(window,{key:',',ctrlKey:true});await fireEvent.keyDown(window,{key:',',metaKey:true});
  const dialog=await screen.findByRole('dialog',{name:'Library settings'});expect(within(dialog).getByText('/library')).toBeTruthy();
  await fireEvent.click(within(dialog).getByRole('button',{name:'Close library dialog'}));
  await fireEvent.click(screen.getByRole('button',{name:'Library'}));
  await fireEvent.click(within(screen.getByRole('banner')).getByRole('button',{name:'Settings'}));
  await screen.findByRole('dialog',{name:'Library settings'});
});
it('shows the empty Library with Import on first launch',async()=>{
  mock.initialize.mockResolvedValue({song:null,document:null,transport:{...transport,song_key:null,reference_token:null}});
  mock.libraryStatus.mockResolvedValue({selected:null,available:false,error:null,import_notice_seen:false});mock.readLibrary.mockResolvedValue({version:2,songs:[]});
  render(App);await screen.findByRole('heading',{name:'Bring in the songs you’re working on'});
  expect(screen.getAllByRole('button',{name:'Import song'})).toHaveLength(2);expect(screen.queryByRole('button',{name:/Try the demo/})).toBeNull();
});
it('imports one dropped file through the Import flow and refuses several',async()=>{
  let drop!:(event:import('./bridge').FileDrop)=>void;mock.onFileDrop.mockImplementation(async(handler)=>{drop=handler;return ()=>{};});
  mock.inspectImportSong.mockResolvedValue({path:'/music/Spain.flac',candidates:[]});
  await ready();
  drop({type:'enter',paths:['/a.wav','/b.wav']});drop({type:'drop',paths:['/a.wav','/b.wav']});
  await screen.findByText(/Drop one audio file at a time\. 2 files were dropped/);expect(mock.inspectImportSong).not.toHaveBeenCalled();
  drop({type:'enter',paths:['/music/Spain.flac']});await screen.findByText('Drop to import “Spain.flac”');
  drop({type:'drop',paths:['/music/Spain.flac']});
  await waitFor(()=>expect(mock.importSong).toHaveBeenCalledWith('/music/Spain.flac'));
  expect(mock.inspectImportSong).toHaveBeenCalledExactlyOnceWith('/music/Spain.flac');expect(mock.pickImportSong).not.toHaveBeenCalled();
  expect(screen.queryByText('Drop to import “Spain.flac”')).toBeNull();
});
it('reopens a song from the library and leaves existing practice intact on open failure',async()=>{
  const entry=libraryEntry('other','Another take','Player',1000,2000);mock.readLibrary.mockResolvedValue({version:1,songs:[libraryEntry('test','Workshop take','Player',1,2),entry]});mock.openLibrarySong.mockRejectedValueOnce(new Error('File missing; use Open song'));
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Library'}));await screen.findByRole('button',{name:'Practise Another take'});await fireEvent.click(screen.getByRole('button',{name:'Practise Another take'}));await screen.findByText(/File missing/);expect(mock.openLibrarySong).toHaveBeenCalledWith('other');
  await backToPractice();expect(screen.getByRole('heading',{name:'Workshop take'})).toBeTruthy();await fireEvent.click(screen.getByRole('button',{name:'Library'}));await screen.findByRole('button',{name:'Practise Another take'});
  mock.openLibrarySong.mockResolvedValue({...response(),song:{key:'other', reference_token:'other',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Another take',path:entry.file_location,duration:12,rate:48000,peaks:[[-.2,.2]]}});await fireEvent.click(screen.getByRole('button',{name:'Practise Another take'}));await screen.findByRole('heading',{name:'Another take'});expect(screen.getByRole('main',{name:'Practice workspace'})).toBeTruthy();
});
it('shows library load failures with retry and preserves typed metadata after a failed save',async()=>{
  mock.readLibrary.mockRejectedValueOnce(new Error('Invalid library; original file left intact'));
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Library'}));await screen.findByText(/Invalid library/);
  mock.readLibrary.mockResolvedValue({version:1,songs:[libraryEntry('test','Workshop take','Player',1,2)]});await fireEvent.click(screen.getByRole('button',{name:'Reload'}));await screen.findByRole('button',{name:'Edit details of Workshop take'});
  await fireEvent.click(screen.getByRole('button',{name:'Edit details of Workshop take'}));await fireEvent.input(screen.getByRole('textbox',{name:'Artist'}),{target:{value:'My artist'}});mock.updateLibrarySong.mockRejectedValue(new Error('Disk full'));await fireEvent.click(screen.getByRole('button',{name:'Save details'}));await screen.findByText(/Disk full.*Your changes are kept here/);expect((screen.getByRole('textbox',{name:'Artist'}) as HTMLInputElement).value).toBe('My artist');
});

it.each(['practice','mapping','library'] as const)('routes opening a song through the library from %s and retains the current song on cancel',async(context)=>{
  mock.openSong.mockResolvedValue(null);
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Play'}));
  if(context==='mapping')await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  if(context==='library')await fireEvent.click(screen.getByRole('button',{name:'Library'}));
  await fireEvent.keyDown(window,{key:'o',ctrlKey:true});
  await waitFor(()=>expect(mock.openSong).toHaveBeenCalledOnce());
  await screen.findByRole('main',{name:'Music library'});
  expect(screen.queryByRole('group',{name:'Song views'})).toBeNull();
  expect(within(screen.getByRole('banner')).queryByRole('button',{name:'Notes'})).toBeNull();
  expect(screen.getAllByRole('button',{name:'Import song'})).toHaveLength(1);
  expect(transport.playing).toBe(false);
  await backToPractice();
  expect(screen.getByRole('main',{name:'Practice workspace'})).toBeTruthy();expect(screen.getByRole('heading',{name:'Workshop take'})).toBeTruthy();
});

const notesButton=()=>within(screen.getByRole('banner')).getByRole('button',{name:'Notes'});
const notesField=()=>screen.getByRole('textbox',{name:'Notes for this song'}) as HTMLTextAreaElement;
it('saves plain-text notes before closing the drawer and keeps shortcuts out of the textarea',async()=>{
  await ready();await fireEvent.click(notesButton());
  const notes=notesField();mock.action.mockClear();expect(globalThis.document.activeElement).toBe(notes);const looping=document.loop_enabled;
  const text='  Guitar in drop D\n\nWork on measures 15–17. **Plain text**  ';
  await fireEvent.input(notes,{target:{value:text}});
  for(const key of ['Enter',' ','n','l','a','1'])await fireEvent.keyDown(notes,{key});
  expect(mock.action).not.toHaveBeenCalled();expect(document.loop_enabled).toBe(looping);expect(screen.queryByRole('dialog')).toBeNull();expect(notesField()).toBe(notes);
  await fireEvent.click(notesButton());await waitFor(()=>expect(screen.queryByRole('complementary',{name:'Notes'})).toBeNull());
  expect(document.notes).toBe(text);expect(document.sections).toHaveLength(2);
  await fireEvent.click(notesButton());expect(notesField().value).toBe(text);
  expect(screen.getByRole('button',{name:'Library'})).toBeTruthy();
});
it('opens the notes drawer beside the readout and waveform without touching the transport',async()=>{
  transport.playing=true;await ready();mock.action.mockClear();
  expect(screen.queryByRole('complementary',{name:'Notes'})).toBeNull();
  expect(within(screen.getByRole('group',{name:'Song views'})).queryByRole('button',{name:'Notes'})).toBeNull();
  expect(notesButton().getAttribute('aria-pressed')).toBe('false');expect(notesButton().title).toBe('Show or hide notes (N)');
  await fireEvent.keyDown(window,{key:'n',code:'KeyN'});
  const drawer=screen.getByRole('complementary',{name:'Notes'});
  expect(notesButton().getAttribute('aria-pressed')).toBe('true');expect(notesButton().getAttribute('aria-controls')).toBe(drawer.id);
  // The drawer sits beside both the readout band and the workspace; Practice stays mounted.
  const practice=screen.getByRole('region',{name:'Now playing'}).closest('.practice')!;
  expect(drawer.parentElement).toBe(practice);expect(screen.getByRole('main',{name:'Practice workspace'})).toBeTruthy();
  expect(within(drawer).queryByRole('button',{name:/close/i})).toBeNull();
  expect(notesField().placeholder).toBe('What to practise, where it goes wrong, reminders for next time…');
  expect(within(drawer).getByText('Typing · Esc returns to the waveform, so Space plays again')).toBeTruthy();
  // Esc from the text returns home, so Space plays again; the drawer stays open.
  await fireEvent.keyDown(notesField(),{key:'Escape'});
  expect(globalThis.document.activeElement?.hasAttribute('data-home')).toBe(true);expect(screen.getByRole('complementary',{name:'Notes'})).toBeTruthy();
  await fireEvent.blur(notesField());expect(within(drawer).getByText('Saved as you type')).toBeTruthy();
  await fireEvent.keyDown(window,{key:' ',code:'Space'});await waitFor(()=>expect(mock.action).toHaveBeenCalledWith({type:'toggle_play'},'test'));
  mock.action.mockClear();
  await fireEvent.keyDown(window,{key:'n',code:'KeyN'});await waitFor(()=>expect(screen.queryByRole('complementary',{name:'Notes'})).toBeNull());
  expect(mock.action).not.toHaveBeenCalled();
});
it('opens Practice with the drawer from another view and keeps the drawer open across views',async()=>{
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  expect(notesButton().getAttribute('aria-pressed')).toBe('false');
  await fireEvent.click(notesButton());
  expect(screen.getByRole('main',{name:'Practice workspace'})).toBeTruthy();expect(screen.getByRole('complementary',{name:'Notes'})).toBeTruthy();
  await fireEvent.click(screen.getByRole('button',{name:'Stems'}));await fireEvent.click(screen.getByRole('button',{name:'Practice'}));
  expect(screen.getByRole('complementary',{name:'Notes'})).toBeTruthy();
});
it('shows Saving… in the drawer only when a save is slow',async()=>{
  await ready();await fireEvent.click(notesButton());const drawer=screen.getByRole('complementary',{name:'Notes'});
  let done!:()=>void;mock.update.mockImplementationOnce(()=>new Promise(resolve=>{done=()=>resolve(response());}));
  await fireEvent.input(notesField(),{target:{value:'slow disk'}});
  await waitFor(()=>expect(within(screen.getByRole('banner')).getByRole('status').textContent).toBe('Saving…'));
  expect(within(drawer).queryByText('Saving…')).toBeNull();
  expect(await within(drawer).findByText('Saving…',{},{timeout:2000})).toBeTruthy();
  done();await waitFor(()=>expect(within(drawer).queryByText('Saving…')).toBeNull());
});
it('keeps the drawer open and retains text on failed save, then closes after retry',async()=>{
  await ready();await fireEvent.click(notesButton());const drawer=screen.getByRole('complementary',{name:'Notes'});
  mock.update.mockRejectedValueOnce(new Error('Disk full'));await fireEvent.input(notesField(),{target:{value:'Do not lose this'}});
  await fireEvent.click(notesButton());
  const alert=await within(drawer).findByRole('alert');expect(alert.textContent).toContain('Couldn’t save your notes: Disk full. Your text is kept here.');
  expect(notesField().value).toBe('Do not lose this');
  expect(within(screen.getByRole('banner')).getByRole('status').textContent).toBe('Notes not saved');
  // The drawer's alert explains the failure; no duplicate toast.
  expect(screen.queryByText(/Changes could not be saved/)).toBeNull();
  await fireEvent.click(screen.getByRole('button',{name:'Library'}));expect(screen.queryByRole('main',{name:'Music library'})).toBeNull();
  await fireEvent.click(within(alert).getByRole('button',{name:'Retry save'}));await waitFor(()=>expect(document.notes).toBe('Do not lose this'));
  await waitFor(()=>expect(within(drawer).queryByRole('alert')).toBeNull());
  expect(within(screen.getByRole('banner')).getByRole('status').textContent).toBe('Saved');
  await fireEvent.click(notesButton());await waitFor(()=>expect(screen.queryByRole('complementary',{name:'Notes'})).toBeNull());
});
it('keeps over-long notes in the drawer without saving them and blocks leaving the song',async()=>{
  await ready();await fireEvent.click(notesButton());const drawer=screen.getByRole('complementary',{name:'Notes'});mock.update.mockClear();
  await fireEvent.input(notesField(),{target:{value:'é'.repeat(524_289)}});
  expect((await within(drawer).findByRole('alert')).textContent).toContain('Notes are over 1 MiB and can’t be saved. The last saved version is kept');
  expect(within(drawer).queryByRole('button',{name:'Retry save'})).toBeNull();
  expect(within(screen.getByRole('banner')).getByRole('status').textContent).toBe('Notes not saved');
  await fireEvent.click(screen.getByRole('button',{name:'Library'}));await new Promise(resolve=>setTimeout(resolve,350));
  expect(screen.queryByRole('main',{name:'Music library'})).toBeNull();expect(mock.update).not.toHaveBeenCalled();expect(document.notes).toBe('');
  await fireEvent.input(notesField(),{target:{value:'Short again'}});await waitFor(()=>expect(document.notes).toBe('Short again'));
  expect(within(drawer).queryByRole('alert')).toBeNull();
});
it('makes the notes read-only while the app is busy, so nothing typed then is lost',async()=>{
  transport.audio_error='Stem reader failed';mock.stemsStatus.mockResolvedValue(attachedStems());
  mock.initialize.mockImplementation(async()=>({...response(),song:{key:'test',reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}}));
  await ready();await fireEvent.click(notesButton());expect(notesField().readOnly).toBe(false);
  let finish!:(value:Snapshot)=>void;mock.action.mockImplementationOnce(()=>new Promise(resolve=>finish=resolve));
  await fireEvent.click(screen.getByRole('button',{name:'Retry audio'}));await waitFor(()=>expect(mock.action).toHaveBeenCalledWith({type:'retry_audio'},'test'));
  expect(notesField().readOnly).toBe(true);
  transport.audio_error=null;finish(response());await waitFor(()=>expect(notesField().readOnly).toBe(false));
});
const otherSong={key:'other',reference_token:'other',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Another take',path:'/music/other.wav',duration:12,rate:48000,peaks:[[-.2,.2]] as [number,number][]};
async function openOther(notes:string){
  mock.readLibrary.mockResolvedValue({version:1,songs:[libraryEntry('test','Workshop take','Player',1,2),libraryEntry('other','Another take','Player',1000,2000)]});
  mock.openLibrarySong.mockImplementation(async()=>{document={...document,song_key:'other',reference_token:'other',notes};transport={...transport,song_key:'other',reference_token:'other'};return {...response(),song:otherSong};});
}
it('keeps each song’s notes when switching songs with the drawer open',async()=>{
  await openOther('B notes');await ready();await fireEvent.click(notesButton());
  await fireEvent.input(notesField(),{target:{value:'A notes'}});
  await fireEvent.click(screen.getByRole('button',{name:'Library'}));await screen.findByRole('main',{name:'Music library'});
  expect(mock.update.mock.lastCall![0]).toMatchObject({reference_token:'test',notes:'A notes'});
  await fireEvent.click(await screen.findByRole('button',{name:'Practise Another take'}));await screen.findByRole('heading',{name:'Another take'});
  expect(screen.getByRole('complementary',{name:'Notes'})).toBeTruthy();expect(notesField().value).toBe('B notes');
  mock.update.mockClear();await fireEvent.input(notesField(),{target:{value:'B notes, more'}});
  await waitFor(()=>expect(mock.update).toHaveBeenCalled());
  expect(mock.update.mock.lastCall![0]).toMatchObject({reference_token:'other',notes:'B notes, more'});
  expect(mock.update.mock.calls.every(([value])=>value.reference_token==='other')).toBe(true);
});
async function slowFirstSave(){
  let done!:()=>void;mock.update.mockImplementationOnce(async(value:PracticeDocument)=>{await new Promise<void>(resolve=>{done=resolve;});document=structuredClone(value);return response();});
  await fireEvent.input(notesField(),{target:{value:'first'}});await waitFor(()=>expect(mock.update).toHaveBeenCalledOnce());
  await fireEvent.input(notesField(),{target:{value:'first, then more'}});
  return ()=>done();
}
it('saves text typed during a slow save before closing the drawer',async()=>{
  await ready();await fireEvent.click(notesButton());mock.update.mockClear();const release=await slowFirstSave();
  await fireEvent.click(notesButton());expect(screen.getByRole('complementary',{name:'Notes'})).toBeTruthy();
  release();await waitFor(()=>expect(screen.queryByRole('complementary',{name:'Notes'})).toBeNull());
  expect(document.notes).toBe('first, then more');expect(mock.update.mock.lastCall![0].notes).toBe('first, then more');
});
it('saves text typed during a slow save before switching songs',async()=>{
  await openOther('B notes');await ready();await fireEvent.click(notesButton());mock.update.mockClear();const release=await slowFirstSave();
  await fireEvent.click(screen.getByRole('button',{name:'Library'}));expect(screen.queryByRole('main',{name:'Music library'})).toBeNull();
  release();await screen.findByRole('main',{name:'Music library'});expect(document.notes).toBe('first, then more');
  await fireEvent.click(await screen.findByRole('button',{name:'Practise Another take'}));await screen.findByRole('heading',{name:'Another take'});
  expect(notesField().value).toBe('B notes');expect(mock.update.mock.calls.filter(([value])=>value.reference_token==='test').map(([value])=>value.notes)).toEqual(['first','first, then more']);
});
it('saves notes of exactly 1 MiB',async()=>{
  await ready();await fireEvent.click(notesButton());const text='é'.repeat(524_288);
  await fireEvent.input(notesField(),{target:{value:text}});await waitFor(()=>expect(document.notes).toBe(text));
  expect(within(screen.getByRole('complementary',{name:'Notes'})).queryByRole('alert')).toBeNull();
});
it('focuses the notes text when Notes brings Practice forward with the drawer already open',async()=>{
  await ready();await fireEvent.click(notesButton());await fireEvent.keyDown(notesField(),{key:'Escape'});
  await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));await fireEvent.click(notesButton());
  await waitFor(()=>expect(globalThis.document.activeElement).toBe(notesField()));
});
it('flushes pending notes before a normal app exit and keeps the app open on save failure',async()=>{
  await ready();await fireEvent.click(notesButton());
  await fireEvent.input(notesField(),{target:{value:'Saved on quit'}});
  await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));expect(screen.queryByRole('main',{name:'Practice workspace'})).toBeNull();
  const onExit=mock.onExitRequested.mock.lastCall![0] as ()=>Promise<void>;mock.update.mockRejectedValueOnce(new Error('Disk full'));await onExit();expect(mock.finishExit).not.toHaveBeenCalled();
  // The blocked quit brings Practice forward with the drawer and its alert.
  await waitFor(()=>expect(screen.getByRole('main',{name:'Practice workspace'})).toBeTruthy());
  expect(within(screen.getByRole('complementary',{name:'Notes'})).getByRole('alert')).toBeTruthy();
  await onExit();expect(document.notes).toBe('Saved on quit');expect(mock.finishExit).toHaveBeenCalledOnce();
});
it('loads stored notes and flushes them when navigating to the library',async()=>{
  document.notes='Remember the bridge';mock.initialize.mockResolvedValue({...response(),song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();await fireEvent.click(notesButton());const field=notesField();expect(field.value).toBe('Remember the bridge');await fireEvent.input(field,{target:{value:'Remember the bridge\nRepeat slowly'}});
  await fireEvent.click(screen.getByRole('button',{name:'Library'}));await screen.findByRole('main',{name:'Music library'});expect(document.notes).toBe('Remember the bridge\nRepeat slowly');
});

it('requires an accepted measure map for count-in, count-in each repeat and the click, and says so',async()=>{
  await ready();
  const playback=screen.getByRole('region',{name:'Playback'});
  const modes=within(playback).getByRole('group',{name:'Practice modes'});
  const count=within(modes).getByRole('button',{name:'Count-in'});
  // Unavailable for want of a map, but focusable with its reason, and clicks do nothing.
  expect(count.getAttribute('aria-disabled')).toBe('true');expect(count.hasAttribute('disabled')).toBe(false);expect(count.getAttribute('aria-pressed')).toBe('false');
  expect(count.title).toBe('Count-in needs a measure map');
  const each=within(modes).getByRole('button',{name:'Count in before each repeat'});
  expect(each.getAttribute('aria-disabled')).toBe('true');expect(each.title).toBe('Count-in needs a measure map');
  expect(within(modes).getByRole('button',{name:'Loop',pressed:false}).hasAttribute('disabled')).toBe(false);
  const click=within(within(playback).getByRole('group',{name:'Click track'}));
  expect(click.getByRole('button',{name:'Off'}).getAttribute('aria-disabled')).toBe('true');
  count.focus();expect(window.document.activeElement).toBe(count);
  const calls=mock.countIn.mock.calls.length,clicks=mock.metronome.mock.calls.length;
  for(const button of [count,each,click.getByRole('button',{name:'Off'})])await fireEvent.click(button);
  await new Promise(resolve=>setTimeout(resolve,20));
  expect(mock.countIn.mock.calls.length).toBe(calls);expect(mock.metronome.mock.calls.length).toBe(clicks);
  expect(count.getAttribute('aria-pressed')).toBe('false');expect(each.getAttribute('aria-pressed')).toBe('false');
  expect(click.getByRole('button',{name:'Off'})).toBeTruthy();
  expect(click.getByRole('button',{name:'Off'}).title).toBe('The click needs a measure map');
  expect(click.getByRole('slider',{name:'Click gain'}).getAttribute('aria-disabled')).toBe('true');
  // The step buttons jump 5 s, and the readout offers to add a map.
  expect(within(playback).getByRole('button',{name:'Back 5 seconds'})).toBeTruthy();
  await fireEvent.click(screen.getByRole('button',{name:'Add a measure map ›'}));
  expect(screen.getByRole('button',{name:'Measure map'}).getAttribute('aria-current')).toBe('page');
  expect(mock.countIn.mock.lastCall).toEqual(['test',false,false,.5]);
});

it('keeps count-in before Play, Loop and count-in each repeat independent, in Practice only',async()=>{
  const original=timingFixture(48000);document.measure_map=original.map;
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();
  // The playback bar is rebuilt on returning to Practice, so query it afresh.
  const modes={getByRole:(...args:Parameters<typeof screen.getByRole>)=>within(screen.getByRole('group',{name:'Practice modes'})).getByRole(...args)};
  const each=()=>modes.getByRole('button',{name:'Count in before each repeat'});
  await fireEvent.keyDown(window,{key:'c'});
  expect(modes.getByRole('button',{name:'Count-in'}).getAttribute('aria-pressed')).toBe('true');
  expect(each().getAttribute('aria-disabled')).toBe('true'); // Needs looping on.
  await fireEvent.click(modes.getByRole('button',{name:'Loop',pressed:false}));
  await waitFor(()=>expect(each().getAttribute('aria-disabled')).toBe('false'));
  expect(each().getAttribute('aria-pressed')).toBe('false');
  expect(modes.getByRole('button',{name:'Count-in'}).getAttribute('aria-pressed')).toBe('true');
  await fireEvent.click(each());
  expect(each().getAttribute('aria-pressed')).toBe('true');
  expect(screen.getByText(/count-in each repeat/)).toBeTruthy();
  const level=within(screen.getByRole('group',{name:'Click track'})).getByRole('slider',{name:'Click gain'});
  for(let i=0;i<20;i++)await fireEvent.keyDown(level,{key:'ArrowUp'});
  await waitFor(()=>expect(mock.countIn.mock.lastCall).toEqual(['test',true,true,.7]));
  // Turning the click off leaves count-in alone; turning count-in off leaves each repeat alone.
  await fireEvent.click(within(screen.getByRole('group',{name:'Click track'})).getByRole('button',{name:'On'}));
  expect(modes.getByRole('button',{name:'Count-in'}).getAttribute('aria-pressed')).toBe('true');
  await fireEvent.keyDown(window,{key:'C',shiftKey:true});
  expect(modes.getByRole('button',{name:'Count-in'}).getAttribute('aria-pressed')).toBe('false');
  expect(each().getAttribute('aria-pressed')).toBe('true');
  await waitFor(()=>expect(mock.countIn.mock.lastCall).toEqual(['test',false,true,.7]));
  await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));
  expect(screen.queryByRole('group',{name:'Practice modes'})).toBeNull();
  await waitFor(()=>expect(mock.countIn.mock.lastCall).toEqual(['test',false,false,.7]));
  await fireEvent.click(screen.getByRole('button',{name:'Close measure map'}));
  await waitFor(()=>expect(mock.countIn.mock.lastCall).toEqual(['test',false,true,.7]));
  // Loop off keeps the each-repeat choice, but disables it.
  await fireEvent.keyDown(window,{key:'l'});
  await waitFor(()=>expect(document.loop_enabled).toBe(false));
  await waitFor(()=>expect(each().getAttribute('aria-disabled')).toBe('true'));
  expect(each().getAttribute('aria-pressed')).toBe('true');
  expect(document.measure_map).toEqual(original.map);
});

it('waits for native count-in configuration before Play and reports counting without moving the playhead',async()=>{
  const original=timingFixture(48000);document.measure_map=original.map;
  mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
  await ready();await waitFor(()=>expect(mock.countIn).toHaveBeenCalled());
  let finish!:(value:object)=>void;mock.countIn.mockReturnValueOnce(new Promise(resolve=>{finish=resolve;}));
  await fireEvent.click(screen.getByRole('button',{name:'Count-in',pressed:false}));
  await waitFor(()=>expect(mock.countIn.mock.lastCall).toEqual(['test',true,false,.5]));
  await fireEvent.click(screen.getByRole('button',{name:'Play'}));expect(mock.action).not.toHaveBeenCalled();
  transport.counting_in=true;finish({});await screen.findByRole('button',{name:'Pause'});
  expect(await screen.findByText('Counting in…')).toBeTruthy();
  expect(screen.getByRole('button',{name:'Pause'})).toBeTruthy();
  expect(screen.getByRole('slider',{name:'Song position'}).getAttribute('aria-valuenow')).toBe('0');
  await fireEvent.click(screen.getByRole('button',{name:'Pause'}));await screen.findByRole('button',{name:'Play'});
});

async function openRecordingDetails(title='Workshop take') {
  mock.readLibrary.mockResolvedValue({version:2,songs:[libraryEntry('test',title,'Player',1,2)]});
  expect(screen.queryByRole('button',{name:'Replace recording…'})).toBeNull();
  await fireEvent.click(screen.getByRole('button',{name:'Library'}));
  await screen.findByRole('button',{name:`Edit details of ${title}`});
  // Opening never shows the edit fields: replacing needs the explicit edit action.
  expect(screen.queryByRole('button',{name:'Replace recording…'})).toBeNull();
  await fireEvent.click(screen.getByRole('button',{name:`Edit details of ${title}`}));
  const details=screen.getByRole('complementary',{name:'Edit song details'});
  const replace=within(details).getByRole('button',{name:'Replace recording…'});
  expect(details.textContent).toContain(`/music/${title}.wav`);
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
  await ready();mock.pickImportSong.mockResolvedValue({path:'/duplicate.wav',candidates:[libraryEntry('test','Workshop take','Player',1,2)]});mock.importSong.mockResolvedValue(null);
  await fireEvent.click(screen.getByRole('button',{name:'Library'}));
  await fireEvent.click(screen.getByRole('button',{name:'Import song'}));
  const separate=await screen.findByRole('button',{name:/^Import as a separate song/});await waitFor(()=>expect(separate.hasAttribute('disabled')).toBe(false));await fireEvent.click(separate);
  await waitFor(()=>expect(mock.importSong).toHaveBeenCalledWith('/duplicate.wav'));
  await backToPractice();
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
  // Without a song the app shows the Library; the song views don't exist.
  await screen.findByRole('main',{name:'Music library'});
  expect(screen.queryByRole('group',{name:'Song views'})).toBeNull();
  await screen.findByRole('button',{name:'Practise Missing recording'});
  expect(screen.queryByRole('button',{name:'Replace recording…'})).toBeNull();
  await fireEvent.click(screen.getByRole('button',{name:'Edit details of Missing recording'}));
  await fireEvent.click(within(screen.getByRole('complementary',{name:'Edit song details'})).getByRole('button',{name:'Replace recording…'}));
  await screen.findByRole('dialog',{name:'Replace reference recording'});
  await waitFor(()=>expect(mock.libraryReference).toHaveBeenCalledExactlyOnceWith(entry.song_key));
  await fireEvent.click(screen.getByRole('button',{name:'Choose recording with changed timing'}));
  await waitFor(()=>expect(mock.replaceLibraryReference).toHaveBeenCalledExactlyOnceWith(entry.song_key,'persisted-reference-before-picker',false));
  await screen.findByRole('heading',{name:'Missing recording'});
  expect(mock.openLibrarySong).not.toHaveBeenCalled();
});

it('sets up the library on first import and explains copied originals before choosing audio',async()=>{
  mock.libraryStatus.mockResolvedValue({selected:null,available:false,error:null,import_notice_seen:false});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Library'}));await fireEvent.click(screen.getByRole('button',{name:'Import song'}));
  const setup=await screen.findByRole('dialog',{name:'Where should your songs live?'});
  expect(within(setup).getByText(/original files stay where they are/)).toBeTruthy();
  expect(mock.pickImportSong).not.toHaveBeenCalled();
  const close=within(setup).getByRole('button',{name:'Close library dialog'});expect(close.parentElement?.firstElementChild).toBe(close);
  await fireEvent.click(close);expect(mock.chooseLibrary).not.toHaveBeenCalled();
});
it('keeps the selected song when switching cannot save pending settings',async()=>{
  await ready();mock.update.mockRejectedValue(new Error('Cannot save'));
  await fireEvent.change(speedField(),{target:{value:'80'}});
  await fireEvent.click(screen.getByRole('button',{name:'Library'}));await fireEvent.click(screen.getByRole('button',{name:'Settings'}));
  await fireEvent.click(screen.getByRole('button',{name:/^Open an existing library/}));
  await within(screen.getByRole('dialog',{name:'Library settings'})).findByText(/Cannot save/);expect(mock.chooseLibrary).not.toHaveBeenCalled();expect(screen.getByText('Current')).toBeTruthy();
});
it('offers unavailable-root actions without silently creating a library',async()=>{
  mock.libraryStatus.mockResolvedValue({selected:{id:'lib',root:'/missing'},available:false,error:'Drive disconnected',import_notice_seen:true});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Library'}));await screen.findByText('Drive disconnected');
  expect(screen.getByRole('button',{name:'Retry'})).toBeTruthy();expect(screen.getByRole('button',{name:'Locate library…'})).toBeTruthy();expect(screen.getByRole('button',{name:'Open another library…'})).toBeTruthy();
  expect(screen.getByText('/missing')).toBeTruthy();expect(screen.queryByRole('listbox',{name:'Songs'})).toBeNull();expect(mock.chooseLibrary).not.toHaveBeenCalled();
});
it('deletes only through Library edit confirmation and unloads the active song',async()=>{
  const entry=libraryEntry('test','Workshop take','Player',1,2);mock.readLibrary.mockResolvedValue({version:1,songs:[entry]});
  await ready();expect(screen.queryByRole('button',{name:'Delete song…'})).toBeNull();
  await fireEvent.click(screen.getByRole('button',{name:'Library'}));await screen.findByRole('button',{name:'Edit details of Workshop take'});
  expect(screen.queryByRole('button',{name:'Delete song…'})).toBeNull();await fireEvent.click(screen.getByRole('button',{name:'Edit details of Workshop take'}));
  await fireEvent.click(screen.getByRole('button',{name:'Delete song…'}));expect(mock.deleteLibrarySong).not.toHaveBeenCalled();
  const deletion=screen.getByRole('dialog',{name:'Delete “Workshop take”?'});expect(within(deletion).getByText(/can’t be undone/)).toBeTruthy();
  expect(window.document.activeElement).toBe(within(deletion).getByRole('button',{name:'Keep song'}));
  mock.deleteLibrarySong.mockResolvedValue({song:null,document:null,transport:{...transport,song_key:null,reference_token:null}});mock.readLibrary.mockResolvedValue({version:1,songs:[]});
  await fireEvent.click(within(deletion).getByRole('button',{name:'Delete permanently'}));await waitFor(()=>expect(mock.deleteLibrarySong).toHaveBeenCalledWith('test'));
  await waitFor(()=>expect(screen.queryByRole('heading',{name:'Workshop take'})).toBeNull());expect(screen.getByRole('main',{name:'Music library'})).toBeTruthy();
});
it('shows copy progress without cancellation or a verification phase',async()=>{
  let progress!:(value:{copied:number,total:number})=>void;mock.onImportProgress.mockImplementation(async handler=>{progress=handler;return()=>{};});
  let finish!:(value:null)=>void;mock.importSong.mockImplementation(()=>new Promise(resolve=>{finish=resolve;}));
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Library'}));await fireEvent.click(screen.getByRole('button',{name:'Import song'}));
  await waitFor(()=>expect(mock.importSong).toHaveBeenCalled());progress({copied:50,total:100});await screen.findByText('Importing song… 50%');
  expect(screen.queryByRole('button',{name:'Cancel'})).toBeNull();expect(screen.queryByText(/Verifying recording/)).toBeNull();finish(null);
});

function attachedStems(){return {reference_token:'test',set_id:'set',available:true,warning:null,attenuation:.5,model:'roformer-sw',tracks:[{role:'bass' as const,gain:1,mute:false,solo:false},{role:'vocals' as const,gain:1,mute:false,solo:false}]};}
it('keeps stems navigation transport neutral and isolates role shortcuts from notes and mapping',async()=>{
 mock.stemsStatus.mockResolvedValue(attachedStems());mock.setStemMix.mockResolvedValue(attachedStems());transport.playing=true;
 await ready();expect(screen.getByRole('slider',{name:'Bass gain'})).toBeTruthy();mock.action.mockClear();
 await fireEvent.keyDown(window,{key:'1'});await waitFor(()=>expect(mock.setStemMix).toHaveBeenCalled());expect(mock.setStemMix.mock.lastCall![2][0].mute).toBe(true);
 await fireEvent.click(screen.getByRole('button',{name:'Stems'}));expect(screen.getByRole('main',{name:'Stems'})).toBeTruthy();expect(mock.action).not.toHaveBeenCalled();
 await fireEvent.click(screen.getByRole('button',{name:'Close stems'}));await fireEvent.click(screen.getByRole('button',{name:'Notes'}));mock.setStemMix.mockClear();
 await fireEvent.keyDown(screen.getByRole('textbox',{name:'Notes for this song'}),{key:'1'});expect(mock.setStemMix).not.toHaveBeenCalled();
 await fireEvent.click(screen.getByRole('button',{name:'Notes'}));await fireEvent.click(screen.getByRole('button',{name:'Measure map'}));await fireEvent.keyDown(window,{key:'1'});expect(mock.setStemMix).not.toHaveBeenCalled();
});
it('keeps failed replacement usable, and requires explicit removal confirmation while retaining position',async()=>{
 mock.stemsStatus.mockResolvedValue(attachedStems());mock.setStemMix.mockResolvedValue(attachedStems());mock.pickStemsDirectory.mockResolvedValue('/new-stems');mock.attachStems.mockRejectedValue(new Error('Invalid provenance'));
 transport.position=4;transport.playing=true;mock.initialize.mockImplementation(async()=>({...response(),song:{key:'test',reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}}));await ready();await fireEvent.click(screen.getByRole('button',{name:'Stems'}));await fireEvent.click(screen.getByRole('button',{name:'Replace stems…'}));expect(mock.attachStems).not.toHaveBeenCalled();
 await fireEvent.click(screen.getByRole('button',{name:'Choose replacement…'}));await screen.findByText('Error: Invalid provenance');expect(transport.playing).toBe(false);expect(transport.position).toBe(4);expect(screen.getByText('roformer-sw')).toBeTruthy();
 mock.removeStems.mockResolvedValue({...attachedStems(),set_id:null,tracks:[],available:false});await fireEvent.click(screen.getByRole('button',{name:'Remove stems…'}));expect(mock.removeStems).not.toHaveBeenCalled();await fireEvent.click(screen.getByRole('button',{name:'Remove stems'}));await waitFor(()=>expect(mock.removeStems).toHaveBeenCalledWith('test','set'));expect(transport.position).toBe(4);expect(document.sections).toHaveLength(2);
});
it('offers visible original fallback with disabled saved track controls',async()=>{
 mock.stemsStatus.mockResolvedValue({...attachedStems(),available:false,warning:'Vocals file missing'});await ready();expect(screen.getByText(/Vocals file missing/)).toBeTruthy();expect(screen.getByRole('slider',{name:'Bass gain'}).getAttribute('aria-disabled')).toBe('true');await fireEvent.keyDown(window,{key:'1'});expect(mock.setStemMix).not.toHaveBeenCalled();expect(screen.getByRole('button',{name:'Play'}).hasAttribute('disabled')).toBe(false);
});
it('toggles looping on and off with L and the Loop button, never changing count-in',async()=>{
 const original=timingFixture(48000);document.measure_map=original.map;mock.initialize.mockResolvedValue({...response(),map_view:original.view,song:{key:'test',reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}});
 await ready();await fireEvent.keyDown(window,{key:'l'});await screen.findByRole('button',{name:'Loop',pressed:true});
 expect(screen.getByText('Looping')).toBeTruthy();
 await fireEvent.keyDown(window,{key:'l'});await screen.findByRole('button',{name:'Loop',pressed:false});
 await fireEvent.click(screen.getByRole('button',{name:'Loop',pressed:false}));await screen.findByRole('button',{name:'Loop',pressed:true});
 expect(screen.getByRole('button',{name:'Count-in'}).getAttribute('aria-pressed')).toBe('false');
 expect(screen.getByRole('button',{name:'Count in before each repeat'}).getAttribute('aria-pressed')).toBe('false');
 expect(mock.countIn.mock.lastCall).toEqual(['test',false,false,.5]);
});

it.each([true,false])('distinguishes cleanup warnings from unavailable stems in Practice and Stems (available=%s)',async available=>{
 mock.stemsStatus.mockResolvedValue({...attachedStems(),available,warning:available?'Old set cleanup failed':'Vocals file missing'});mock.setStemMix.mockResolvedValue(attachedStems());await ready();
 const warning=screen.getByText(available?/Old set cleanup failed/:/Vocals file missing/);
 expect(warning.textContent?.includes('Using Original playback')).toBe(!available);
 expect(screen.getByRole('slider',{name:'Bass gain'}).getAttribute('aria-disabled')).toBe(String(!available));
 await fireEvent.keyDown(window,{key:'1'});
 if(available)await waitFor(()=>expect(mock.setStemMix).toHaveBeenCalled());else expect(mock.setStemMix).not.toHaveBeenCalled();
 await fireEvent.click(screen.getByRole('button',{name:'Stems'}));
 const details=screen.getByRole('main',{name:'Stems'});
 expect(within(details).getByText(available?/Old set cleanup failed/:/Vocals file missing/).textContent?.includes('track controls are disabled')).toBe(!available);
 expect(within(details).getAllByText(available?'Ready':'Original fallback')[0]).toBeTruthy();
});

it('re-prepares attached stems during audio retry and refreshes availability without changing the practice view',async()=>{
 transport.audio_error='Stem reader failed';transport.position=4;
 mock.initialize.mockImplementation(async()=>({...response(),song:{key:'test',reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session',title:'Workshop take',path:'/test/song.wav',duration:12,rate:48000,peaks:[[-.2,.2]]}}));
 mock.stemsStatus.mockResolvedValue({...attachedStems(),available:false,warning:'Vocals file missing'});
 await ready();await fireEvent.keyDown(window,{key:'='});const viewport=screen.getByRole('button',{name:'Move waveform view'}).querySelector('.viewport') as HTMLElement;const zoomBefore=viewport.style.width;expect(zoomBefore).not.toBe('100%');
 let finish!:(value:Snapshot)=>void;mock.action.mockImplementationOnce(()=>new Promise(resolve=>finish=resolve));
 await fireEvent.click(screen.getByRole('button',{name:'Retry audio'}));await waitFor(()=>expect(mock.action).toHaveBeenCalledWith({type:'retry_audio'},'test'));
 expect(screen.getByRole('button',{name:'Library'}).hasAttribute('disabled')).toBe(true);expect(screen.getByRole('button',{name:'Retry audio'}).hasAttribute('disabled')).toBe(true);
 const report=mock.onStemProgress.mock.lastCall![0] as (value:StemProgress)=>void;
 report({reference_token:'test',phase:'Scanning headroom',completed:1,total:2,workflow:{completed:.5,total:1,steps:[{id:'levels',label:'Measure shared playback headroom',state:'active',completed:1,total:2}]}});await screen.findByText('Measure shared playback headroom…');
 const restored={...attachedStems(),attenuation:.4,tracks:[{role:'bass' as const,gain:.5,mute:true,solo:false}]};mock.stemsStatus.mockResolvedValue(restored);transport.audio_error=null;finish(response());
 await waitFor(()=>expect(screen.getByRole('slider',{name:'Bass gain'}).getAttribute('aria-disabled')).toBe('false'));expect(screen.getByRole('slider',{name:'Bass gain'}).getAttribute('aria-valuenow')).toBe('50');expect(screen.queryByText(/Vocals file missing/)).toBeNull();
 expect(viewport.style.width).toBe(zoomBefore);expect(document.sections).toHaveLength(2);expect(transport.position).toBe(4);
 await fireEvent.click(screen.getByRole('button',{name:'Stems'}));expect(screen.getByText('-7.96 dB')).toBeTruthy();
});

it('shows attachment work without filenames and retains the completed checklist',async()=>{
 await ready();await fireEvent.click(screen.getByRole('button',{name:'Stems'}));
 mock.pickStemsDirectory.mockResolvedValue('/prototype');let finish!:(value:ReturnType<typeof attachedStems>)=>void;
 mock.attachStems.mockImplementationOnce(()=>new Promise(resolve=>finish=resolve));
 await fireEvent.click(screen.getByRole('button',{name:'Attach stems…'}));await waitFor(()=>expect(mock.attachStems).toHaveBeenCalledWith('test','/prototype',expect.any(String)));
 const report=mock.onStemProgress.mock.lastCall![0] as (value:StemProgress)=>void;
 const steps:NonNullable<StemProgress['workflow']>['steps']=[{id:'check',label:'Check bass audio',state:'active',completed:1,total:2},{id:'save',label:'Save attached stems',state:'pending',completed:0,total:null}];
 report({reference_token:'test',operation_id:mock.attachStems.mock.lastCall![2],phase:'Hash /private/audio/bass.wav',completed:1,total:2,workflow:{completed:.5,total:2,steps}});
 await screen.findByRole('progressbar',{name:'Overall stem processing'});
 report({reference_token:'test',operation_id:'earlier-attachment',phase:'Processing complete',completed:1,total:1,workflow:{completed:2,total:2,steps:steps.map(step=>({...step,state:'complete',completed:1,total:1}))}});
 await Promise.resolve();expect(screen.getByRole('progressbar',{name:'Overall stem processing'}).getAttribute('value')).toBe('0.25');
 expect(screen.getByRole('list',{name:'Processing steps'}).textContent).toContain('Check bass audio');expect(screen.queryByText(/private\/audio/)).toBeNull();
 // The attach reply can beat the final progress event; publication still completes the list.
 finish(attachedStems());
 await screen.findByText('Stems ready');expect(screen.getByRole('list',{name:'Processing steps'})).toBeTruthy();expect(screen.getByRole('progressbar',{name:'Overall stem processing'}).getAttribute('value')).toBe('1');
});


describe('integrated separation workflow',()=>{
 it('downloads explicitly, waits at Ready, freezes playback during separation and keeps note edits available',async()=>{
  let value=separationStatus();mock.separationStatus.mockImplementation(async()=>structuredClone(value));
  mock.installSeparationModel.mockImplementation(async()=>{value.install={...value.install,id:'install',running:true,phase:'Downloading model',cancellable:true};return structuredClone(value);});
  mock.startSeparation.mockImplementation(async()=>{transport.playing=false;value.job={...value.job,id:'job',running:true,phase:'Separating instruments',reference_token:'test',cancellable:true};return structuredClone(value);});
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Stems'}));
  expect(mock.installSeparationModel).not.toHaveBeenCalled();await fireEvent.click(screen.getByRole('button',{name:'Set up stem separation'}));await waitFor(()=>expect(mock.installSeparationModel).toHaveBeenCalledOnce());
  value.model_ready=true;value.install={...value.install,running:false,phase:'Complete',cancellable:false};const separate=await screen.findByRole('button',{name:'Separate song'});await waitFor(()=>expect((separate as HTMLButtonElement).disabled).toBe(false));expect(mock.startSeparation).not.toHaveBeenCalled();
  await fireEvent.click(separate);await waitFor(()=>expect(mock.startSeparation).toHaveBeenCalledWith('test'));await fireEvent.click(screen.getByRole('button',{name:'Close stems'}));
  expect((screen.getByRole('button',{name:'Play'}) as HTMLButtonElement).disabled).toBe(true);expect(speedField().disabled).toBe(true);
  for(const name of ['Back 5 seconds','Forward 5 seconds'])expect((screen.getByRole('button',{name}) as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByRole('button',{name:'Play'}).title).toBe('Playback is paused while the song is split into stems');
  const toast=screen.getByText(/Playback is paused until it finishes/).closest('[role=status]') as HTMLElement;expect(toast.textContent).toContain('Splitting song into stems');
  expect(within(await openPitch()).getByRole('button',{name:'Transpose up'}).hasAttribute('disabled')).toBe(true);expect((screen.getByRole('button',{name:'Library'}) as HTMLButtonElement).disabled).toBe(true);expect((screen.getByRole('button',{name:/Chorus 0:03/}) as HTMLButtonElement).disabled).toBe(true);const before=structuredClone(document);await fireEvent.click(screen.getByRole('button',{name:/Chorus 0:03/}));expect(document.loop_selection).toEqual(before.loop_selection);
  await fireEvent.click(screen.getByRole('button',{name:'Notes'}));await fireEvent.input(screen.getByRole('textbox',{name:'Notes for this song'}),{target:{value:'notes during separation'}});await waitFor(()=>expect(document.notes).toBe('notes during separation'));expect(mock.cancelSeparation).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button',{name:'Notes'}));await waitFor(()=>expect(screen.queryByRole('complementary',{name:'Notes'})).toBeNull());await fireEvent.click(screen.getByRole('button',{name:'Open Stems'}));await screen.findByRole('button',{name:'Cancel'});
  mock.cancelSeparation.mockImplementation(async()=>{value.job={...value.job,phase:'Stopping',cancellable:false};return structuredClone(value);});expect((screen.getByRole('button',{name:'Cancel'}) as HTMLButtonElement).disabled).toBe(false);await fireEvent.click(screen.getByRole('button',{name:'Cancel'}));await waitFor(()=>expect(mock.cancelSeparation).toHaveBeenCalledWith('job'));expect((await screen.findByRole('button',{name:'Stopping…'}) as HTMLButtonElement).disabled).toBe(true);
  value.job={...value.job,running:false,phase:'Cancelled'};await waitFor(()=>expect((screen.getByRole('button',{name:'Retry separation'}) as HTMLButtonElement).disabled).toBe(false));
 });
 it('keeps playback and screen navigation available during tool setup and restores current progress',async()=>{
  const value=separationStatus();value.runtime.ready=false;value.runtime.supported=true;value.install={...value.install,id:'setup',order:1,running:true,phase:'Installing separation tools',cancellable:true,progress:{stage:'runtime.dependencies',completed:0,total:null,workflow:{completed:0,total:0,steps:[{id:'runtime.dependencies',label:'Install separation tools',state:'active',completed:0,total:null}]}}};mock.separationStatus.mockResolvedValue(value);
  await ready();expect(mock.installSeparationModel).not.toHaveBeenCalled();expect((screen.getByRole('button',{name:'Play'}) as HTMLButtonElement).disabled).toBe(false);
  await fireEvent.click(screen.getByRole('button',{name:'Stems'}));await screen.findByText('Install separation tools');
  for(const name of ['Reinstall separation tools','Remove separation tools…','Reinstall model'])expect((screen.getByRole('button',{name}) as HTMLButtonElement).disabled).toBe(true);
  await fireEvent.click(screen.getByRole('button',{name:'Close stems'}));await fireEvent.click(screen.getByRole('button',{name:'Play'}));await waitFor(()=>expect(transport.playing).toBe(true));
  await fireEvent.click(screen.getByRole('button',{name:'Open Stems'}));expect(screen.getByText('Install separation tools')).toBeTruthy();expect(mock.cancelSeparation).not.toHaveBeenCalled();expect(mock.startSeparation).not.toHaveBeenCalled();
 });
 it('asks about active-work quit and only cancels through confirmed native exit',async()=>{
  const value=separationStatus();value.install={...value.install,id:'download',running:true,phase:'Downloading model',cancellable:true};mock.separationStatus.mockResolvedValue(value);let exit!:()=>void;mock.onExitRequested.mockImplementation(async(handler:()=>void)=>{exit=handler;return ()=>{};});
  await ready();exit();await screen.findByRole('dialog',{name:'Quit and cancel processing'});expect(mock.finishExit).not.toHaveBeenCalled();await fireEvent.click(screen.getByRole('button',{name:'Keep app open'}));expect(mock.cancelSeparation).not.toHaveBeenCalled();exit();await fireEvent.click(await screen.findByRole('button',{name:'Quit and cancel'}));await waitFor(()=>expect(mock.finishExit).toHaveBeenCalledOnce());
 });
 it('uses native active-work quit state before the first frontend job poll',async()=>{
  let exit!:(activeProcessing?:boolean)=>void;mock.onExitRequested.mockImplementation(async(handler:(activeProcessing?:boolean)=>void)=>{exit=handler;return ()=>{};});await ready();exit(true);await screen.findByRole('dialog',{name:'Quit and cancel processing'});expect(mock.finishExit).not.toHaveBeenCalled();await fireEvent.click(screen.getByRole('button',{name:'Keep app open'}));expect(mock.finishExit).not.toHaveBeenCalled();
 });

 it('confirms quit while download admission is pending even when native work was still inactive',async()=>{
  let exit!:(activeProcessing?:boolean)=>void;mock.onExitRequested.mockImplementation(async(handler:(activeProcessing?:boolean)=>void)=>{exit=handler;return ()=>{};});
  let resolve!:(status:SeparationStatus)=>void;mock.installSeparationModel.mockReturnValue(new Promise<SeparationStatus>(r=>resolve=r));
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Stems'}));await fireEvent.click(screen.getByRole('button',{name:'Set up stem separation'}));await waitFor(()=>expect(mock.installSeparationModel).toHaveBeenCalledOnce());
  exit(false);await screen.findByRole('dialog',{name:'Quit and cancel processing'});expect(mock.finishExit).not.toHaveBeenCalled();await fireEvent.click(screen.getByRole('button',{name:'Keep app open'}));
  const admitted=separationStatus();admitted.install={...admitted.install,id:'download',running:true,phase:'Downloading model',cancellable:true};resolve(admitted);await screen.findByRole('button',{name:'Cancel'});expect(mock.finishExit).not.toHaveBeenCalled();
 });
 it('does not ask about a pending runtime readiness check',async()=>{
  const missing=separationStatus();missing.runtime.ready=false;mock.separationStatus.mockResolvedValue(missing);let exit!:(activeProcessing?:boolean)=>void;mock.onExitRequested.mockImplementation(async(handler:(activeProcessing?:boolean)=>void)=>{exit=handler;return ()=>{};});let resolve!:(status:SeparationStatus)=>void;mock.refreshSeparationRuntime.mockReturnValue(new Promise<SeparationStatus>(r=>resolve=r));
  await ready();await fireEvent.click(screen.getByRole('button',{name:'Stems'}));await fireEvent.click(screen.getByRole('button',{name:'Check tools again'}));await waitFor(()=>expect(mock.refreshSeparationRuntime).toHaveBeenCalledOnce());exit(false);await waitFor(()=>expect(mock.finishExit).toHaveBeenCalledOnce());expect(screen.queryByRole('dialog',{name:'Quit and cancel processing'})).toBeNull();resolve(separationStatus());
 });
 it('keeps one exit handshake while confirmed native cleanup is still pending',async()=>{
  const active=separationStatus();active.install={...active.install,id:'download',running:true,phase:'Downloading model',cancellable:true};mock.separationStatus.mockResolvedValue(active);let exit!:(activeProcessing?:boolean)=>void;mock.onExitRequested.mockImplementation(async(handler:(activeProcessing?:boolean)=>void)=>{exit=handler;return ()=>{};});let complete!:()=>void;mock.finishExit.mockReturnValue(new Promise<void>(resolve=>complete=resolve));
  await ready();exit(true);await fireEvent.click(await screen.findByRole('button',{name:'Quit and cancel'}));await waitFor(()=>expect(mock.finishExit).toHaveBeenCalledOnce());exit(true);exit(false);await new Promise(resolve=>setTimeout(resolve,20));expect(mock.finishExit).toHaveBeenCalledOnce();expect(screen.queryByRole('dialog',{name:'Quit and cancel processing'})).toBeNull();complete();
 });

});

it.each([
  {context:'practice',focused:false}, {context:'practice',focused:true},
  {context:'mapping',focused:false}, {context:'mapping',focused:true}
])('uses measure starts for navigation in $context with focused timeline=$focused', async ({context,focused}) => {
  const original = timingFixture(48000);
  original.map.boundaries[0].frame = 48000;
  original.view.measures[0].start = 48000;
  original.map.measures[0].kind = original.view.measures[0].kind = 'pickup';
  // Separate the second measure from the first with uncounted audio.
  original.map.boundaries.push({ id: 'gap-end', frame: 264000 });
  original.map.measures[1].start = original.view.measures[1].start_id = 'gap-end';
  original.view.measures[1].start = 264000;
  document.measure_map = original.map;
  document.controls = { speed: .75, semitones: -3, tuning_cents: 0 };
  document.loop_selection = { start: 6, end: 7, fade_ms: 5 };
  document.loop_enabled = true;
  transport.position = 6.5;
  transport.playing = true;
  mock.initialize.mockResolvedValue({ ...response(), map_view: original.view,
    song: { key: 'test', reference_token:'test', asset_id:'asset', timeline_id:'timeline', reference_revision:1, session_id:'session', title:'Workshop take', path:'/test/song.wav', duration:12, rate:48000, peaks:[[-.2,.2]] }
  });
  await ready();
  if (context === 'mapping') await fireEvent.click(screen.getByRole('button', { name:'Measure map' }));
  const target = focused ? screen.getByRole('slider', {name:context === 'mapping' ? 'Measure-map song position' : 'Song position'}) : window;
  for (const [key, seconds] of [['ArrowLeft', 5.5], ['ArrowLeft', 1], ['ArrowRight', 5.5], ['Enter', 1]] as const) {
    // A focused slider owns its keys; Enter is a global shortcut only.
    await fireEvent.keyDown(key === 'Enter' ? window : target, { key });
    await shown(seconds, context === 'mapping' ? 'Measure-map song position' : 'Song position');
    expect(mock.action).toHaveBeenLastCalledWith({ type:'seek', seconds }, 'test');
  }
  expect(transport.playing).toBe(true);
  expect(document.controls).toEqual({ speed:.75, semitones:-3, tuning_cents: 0 });
  expect(document.loop_selection).toEqual({ start:6, end:7, fade_ms:5 });
  expect(document.loop_enabled).toBe(false);
  // No preceding/following measure means no additional seek at the map edges.
  mock.action.mockClear();
  await fireEvent.keyDown(target, { key:'ArrowLeft' });
  expect(mock.action).not.toHaveBeenCalled();
  await fireEvent.keyDown(target, { key:'ArrowRight' });
  await shown(5.5, context === 'mapping' ? 'Measure-map song position' : 'Song position');
  mock.action.mockClear();
  await fireEvent.keyDown(target, { key:'ArrowRight' });
  expect(mock.action).not.toHaveBeenCalled();
  await fireEvent.keyDown(target, { key:'ArrowRight', shiftKey:true });
  await waitFor(() => expect(transport.position).toBe(5.6));
});

it('retains time-based navigation for an empty measure map', async () => {
  const original = timingFixture(48000);
  original.map.measures = [];
  original.view.measures = [];
  document.measure_map = original.map;
  mock.initialize.mockResolvedValue({ ...(await mock.initialize()), map_view:original.view, document:structuredClone(document) });
  await ready();
  for (const [key, seconds] of [['ArrowRight', 5], ['ArrowRight', 10], ['ArrowLeft', 5], ['Enter', 0]] as const) {
    await fireEvent.keyDown(window, { key });
    await shown(seconds);
  }
});

describe('key routing: owners keep their keys, everything else reaches the dispatcher', () => {
  const modality = (value: 'keyboard' | 'pointer') => { window.document.documentElement.dataset.modality = value; };
  it('plays with Space after clicking a button (button-Space regression)', async () => {
    await ready(); modality('pointer');
    const loop = screen.getByRole('button', { name: 'Loop', pressed: false }); loop.focus();
    await fireEvent.keyDown(loop, { key: ' ', code: 'Space' });
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'toggle_play' }, 'test'));
  });
  it('lets a Tab-focused button activate with Space instead of playing', async () => {
    await ready(); modality('keyboard');
    const space = new KeyboardEvent('keydown', { key: ' ', code: 'Space', bubbles: true, cancelable: true });
    await fireEvent(screen.getByRole('button', { name: 'Loop', pressed: false }), space);
    expect(space.defaultPrevented).toBe(false); expect(mock.action).not.toHaveBeenCalled();
    // Letters still reach the shortcuts from a Tab-focused button.
    await fireEvent.keyDown(screen.getByRole('button', { name: 'Loop', pressed: false }), { key: 'l' });
    await waitFor(() => expect(document.loop_enabled).toBe(true));
  });
  it('fires letter shortcuts and Space from a focused waveform slider (slider-letter regression)', async () => {
    document.loop_selection = { start: 3, end: 6, fade_ms: 5 };
    mock.initialize.mockResolvedValue({ ...(await mock.initialize()), document: structuredClone(document) });
    await ready(); modality('keyboard');
    const slider = screen.getByRole('slider', { name: 'Song position' });
    await fireEvent.keyDown(slider, { key: 'l' });
    await waitFor(() => expect(document.loop_enabled).toBe(true));
    await fireEvent.keyDown(slider, { key: ' ', code: 'Space' });
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'toggle_play' }, 'test'));
    // The slider keeps its documented keys: End seeks to the end, not via a shortcut.
    await fireEvent.keyDown(slider, { key: 'End' });
    await waitFor(() => expect(mock.action).toHaveBeenLastCalledWith({ type: 'seek', seconds: 12 }, 'test'));
  });
  it('fires shortcuts from a stem knob but keeps its arrow keys', async () => {
    mock.stemsStatus.mockResolvedValue(attachedStems());
    await ready(); mock.action.mockClear();
    const knob = screen.getByRole('slider', { name: 'Bass gain' });
    await fireEvent.keyDown(knob, { key: 'ArrowLeft' });
    expect(mock.action).not.toHaveBeenCalledWith(expect.objectContaining({ type: 'seek' }), 'test');
    await fireEvent.keyDown(knob, { key: ' ', code: 'Space' });
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'toggle_play' }, 'test'));
  });
  it('fires shortcuts from the click knob but keeps its arrow keys, and none from number fields', async () => {
    const fixture = timingFixture(48000); document.measure_map = fixture.map;
    mock.initialize.mockResolvedValue({ ...response(), map_view: fixture.view, song: { key: 'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session', title: 'Workshop take', path: '/test/song.wav', duration: 12, rate: 48000, peaks: [[-.2, .2]] } });
    await ready(); modality('keyboard');
    const knob = within(screen.getByRole('group', { name: 'Click track' })).getByRole('slider', { name: 'Click gain' });
    for (const key of ['ArrowLeft', 'ArrowDown', 'Home']) await fireEvent.keyDown(knob, { key });
    expect(knob.getAttribute('aria-valuenow')).toBe('0');
    expect(mock.action).not.toHaveBeenCalled(); expect(transport.position).toBe(0);
    await fireEvent.keyDown(knob, { key: ' ', code: 'Space' });
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'toggle_play' }, 'test'));
    await fireEvent.keyDown(knob, { key: 'l' });
    await waitFor(() => expect(document.loop_enabled).toBe(true));
    mock.action.mockClear();
    await fireEvent.keyDown(speedField(), { key: ' ', code: 'Space' });
    expect(mock.action).not.toHaveBeenCalled();
  });
  it('lets an open popover own the keys pressed inside it; Esc closes it first and returns focus', async () => {
    document.loop_selection = { start: 3, end: 6, fade_ms: 5 }; document.loop_enabled = true;
    mock.initialize.mockResolvedValue({ ...(await mock.initialize()), document: structuredClone(document) });
    await ready();
    // Keyboard: focus moves into the popover; its keys stay there.
    modality('keyboard');
    const tile = screen.getByRole('button', { name: /^Pitch / }); tile.focus();
    await fireEvent.click(tile);
    const pitch = screen.getByRole('dialog', { name: 'Pitch' });
    await waitFor(() => expect(pitch.contains(window.document.activeElement)).toBe(true));
    for (const key of [' ', 'l', 'x', ',']) await fireEvent.keyDown(window.document.activeElement!, { key });
    expect(mock.action).not.toHaveBeenCalled(); expect(document.loop_enabled).toBe(true); expect(document.loop_selection).not.toBeNull();
    await fireEvent.keyDown(window.document.activeElement!, { key: 'Escape' });
    expect(screen.queryByRole('dialog', { name: 'Pitch' })).toBeNull();
    expect(window.document.activeElement).toBe(tile);
    expect(document.loop_enabled).toBe(true);
    // Pointer: focus stays home, so shortcuts keep working with the popover open,
    // and Esc unwinds the popover before it turns looping off.
    modality('pointer');
    screen.getByRole('region', { name: /^Waveform/ }).focus();
    await fireEvent.click(screen.getByRole('button', { name: /^Pitch / }));
    expect(screen.getByRole('dialog', { name: 'Pitch' })).toBeTruthy();
    await fireEvent.keyDown(window.document.activeElement!, { key: ' ', code: 'Space' });
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'toggle_play' }, 'test'));
    await fireEvent.keyDown(window.document.activeElement!, { key: 'Escape' });
    expect(screen.queryByRole('dialog', { name: 'Pitch' })).toBeNull(); expect(document.loop_enabled).toBe(true);
    await fireEvent.keyDown(window.document.activeElement!, { key: 'Escape' });
    await waitFor(() => expect(document.loop_enabled).toBe(false));
  });
  it('closes a popover on a click outside it', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /^Pitch / }));
    expect(screen.getByRole('dialog', { name: 'Pitch' })).toBeTruthy();
    await fireEvent.pointerDown(screen.getByRole('region', { name: /^Waveform/ }));
    expect(screen.queryByRole('dialog', { name: 'Pitch' })).toBeNull();
  });
  it('unwinds Esc to home focus first, then runs the Esc shortcut', async () => {
    document.loop_selection = { start: 3, end: 6, fade_ms: 5 }; document.loop_enabled = true;
    mock.initialize.mockResolvedValue({ ...(await mock.initialize()), document: structuredClone(document) });
    await ready(); modality('keyboard');
    const slider = screen.getByRole('slider', { name: 'Song position' }); slider.focus();
    await fireEvent.keyDown(slider, { key: 'Escape' });
    expect(window.document.activeElement).toBe(screen.getByRole('region', { name: /^Waveform/ }));
    expect(document.loop_enabled).toBe(true);
    await fireEvent.keyDown(window.document.activeElement!, { key: 'Escape' });
    await waitFor(() => expect(document.loop_enabled).toBe(false));
  });
});

describe('fields hand focus back and commit before buttons act', () => {
  const cleanups: (() => void)[] = [];
  beforeEach(() => {
    cleanups.push(installNativeChrome(window.document), installFieldCommit(window.document));
    // jsdom does not fire change on blur; browsers do. Emulate it.
    const focused = new WeakMap<EventTarget, string>();
    const focusin = (event: FocusEvent) => { if (event.target instanceof HTMLInputElement) focused.set(event.target, event.target.value); };
    const focusout = (event: FocusEvent) => { const field = event.target; if (field instanceof HTMLInputElement && focused.get(field) !== field.value) field.dispatchEvent(new Event('change', { bubbles: true })); };
    window.document.addEventListener('focusin', focusin); window.document.addEventListener('focusout', focusout);
    cleanups.push(() => { window.document.removeEventListener('focusin', focusin); window.document.removeEventListener('focusout', focusout); });
  });
  afterEach(() => cleanups.splice(0).forEach(cleanup => cleanup()));
  const speedField = () => window.document.getElementById('speed-percent') as HTMLInputElement;
  const type = async (field: HTMLInputElement, value: string) => { field.focus(); field.value = value; await fireEvent.input(field); };
  const press = async (button: HTMLElement) => { await fireEvent.mouseDown(button, { button: 0 }); await fireEvent.click(button); };
  const home = () => screen.getByRole('region', { name: /^Waveform/ });

  it('commits an edited field before a clicked button acts, then Space plays', async () => {
    await ready();
    await type(speedField(), '80');
    await press(screen.getByRole('button', { name: 'Faster by 5 percent' }));
    await waitFor(() => expect(document.controls.speed).toBe(.85));
    expect(window.document.activeElement).toBe(home());
    await fireEvent.keyDown(window.document.activeElement!, { key: ' ', code: 'Space' });
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'toggle_play' }, 'test'));
  });
  it('commits on Enter in a practice field and returns home focus', async () => {
    await ready();
    const hz = within(await openPitch()).getByLabelText('Recording tuned to A (Hz)') as HTMLInputElement;
    await type(hz, '432');
    await fireEvent.keyDown(hz, { key: 'Enter' });
    expect(window.document.activeElement).toBe(home());
    await waitFor(() => expect(document.controls.tuning_cents).toBeCloseTo(31.77, 2));
    expect(transport.position).toBe(0);
  });
  it('reverts the Hz field on Esc, returns home and closes the Pitch popover without committing', async () => {
    await ready();
    const hz = within(await openPitch()).getByLabelText('Recording tuned to A (Hz)') as HTMLInputElement;
    await type(hz, '432');
    await fireEvent.keyDown(hz, { key: 'Escape' });
    expect(hz.value).toBe('440');
    expect(window.document.activeElement).toBe(home());
    await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Pitch' })).toBeNull());
    await new Promise(resolve => setTimeout(resolve, 20));
    expect(document.controls.tuning_cents).toBe(0); expect(mock.update).not.toHaveBeenCalled();
  });
  it('reverts the speed field on Esc and returns home without committing', async () => {
    await ready();
    await type(speedField(), '50');
    await fireEvent.keyDown(speedField(), { key: 'Escape' });
    expect(speedField().value).toBe('100');
    expect(window.document.activeElement).toBe(home());
    await new Promise(resolve => setTimeout(resolve, 20));
    expect(document.controls.speed).toBe(1); expect(mock.update).not.toHaveBeenCalled();
  });
});

describe('undo and the macOS menu bar', () => {
  const platform = Object.getOwnPropertyDescriptor(Navigator.prototype, 'platform')!;
  beforeEach(() => { menu.items.length = 0; Object.defineProperty(navigator, 'platform', { value: 'MacIntel', configurable: true }); });
  afterEach(() => { delete (navigator as unknown as Record<string, unknown>).platform; Object.defineProperty(Navigator.prototype, 'platform', platform); });
  const command = async (id: string) => { await waitFor(() => expect(menu.items.some(item => item.id === id)).toBe(true)); menu.items.filter(item => item.id === id).at(-1)!.action!(); };

  it('ignores menu commands while a dialog blocks commands', async () => {
    await ready();
    await command('show_shortcuts');
    await command('play_pause');
    await new Promise(resolve => setTimeout(resolve, 20));
    expect(mock.action).not.toHaveBeenCalledWith({ type: 'toggle_play' }, 'test');
    await fireEvent.click(screen.getByRole('button', { name: 'Close shortcuts' }));
    await command('play_pause');
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'toggle_play' }, 'test'));
  });
  it('sends menu Undo to the app outside text fields and ⌘Z restores a removed section', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: 'Delete Chorus' }));
    await waitFor(() => expect(document.sections.find(s => s.id === 'chorus')).toBeUndefined());
    await command('edit_undo');
    await waitFor(() => expect(document.sections.find(s => s.id === 'chorus')).toBeTruthy());
    await fireEvent.click(screen.getByRole('button', { name: 'Delete Chorus' }));
    await waitFor(() => expect(document.sections.find(s => s.id === 'chorus')).toBeUndefined());
    await fireEvent.keyDown(window, { key: 'z', metaKey: true });
    await waitFor(() => expect(document.sections.find(s => s.id === 'chorus')).toBeTruthy());
  });
  it('always consumes ⌘Z outside text fields, even with nothing to undo, but not inside them', async () => {
    await ready();
    const undo = new KeyboardEvent('keydown', { key: 'z', metaKey: true, bubbles: true, cancelable: true });
    window.document.body.dispatchEvent(undo);
    expect(undo.defaultPrevented).toBe(true);
    const redo = new KeyboardEvent('keydown', { key: 'z', metaKey: true, shiftKey: true, bubbles: true, cancelable: true });
    window.document.body.dispatchEvent(redo);
    expect(redo.defaultPrevented).toBe(true);
    const inField = new KeyboardEvent('keydown', { key: 'z', metaKey: true, bubbles: true, cancelable: true });
    speedField().dispatchEvent(inField);
    expect(inField.defaultPrevented).toBe(false);
  });
});

describe('Stage practice screen', () => {
  const mapped = () => {
    const fixture = timingFixture(48000); document.measure_map = fixture.map;
    mock.initialize.mockResolvedValue({ ...response(), map_view: fixture.view, song: { key: 'test', reference_token:'test',asset_id:'asset',timeline_id:'timeline',reference_revision:1,session_id:'session', title: 'Workshop take', path: '/test/song.wav', duration: 12, rate: 48000, peaks: [[-.2, .2]] } });
  };

  it('switches song views with the segmented control and keeps the header across them', async () => {
    await ready();
    const views = screen.getByRole('group', { name: 'Song views' });
    expect(within(views).getByRole('button', { name: 'Practice' }).getAttribute('aria-current')).toBe('page');
    await fireEvent.click(within(views).getByRole('button', { name: 'Stems' }));
    expect(screen.getByRole('main', { name: 'Stems' })).toBeTruthy();
    expect(within(views).getByRole('button', { name: 'Stems' }).getAttribute('aria-current')).toBe('page');
    await fireEvent.click(within(views).getByRole('button', { name: 'Measure map' }));
    expect(screen.queryByRole('main', { name: 'Stems' })).toBeNull();
    await fireEvent.click(within(views).getByRole('button', { name: 'Practice' }));
    expect(screen.getByRole('main', { name: 'Practice workspace' })).toBeTruthy();
    // Notes is not a view: the header's Notes button opens and closes the drawer.
    expect(within(views).queryByRole('button', { name: 'Notes' })).toBeNull();
    const notes = within(screen.getByRole('banner')).getByRole('button', { name: 'Notes' });
    await fireEvent.click(notes); expect(screen.getByRole('complementary', { name: 'Notes' })).toBeTruthy();
    expect(screen.getByRole('main', { name: 'Practice workspace' })).toBeTruthy();
    expect(within(screen.getByRole('banner')).getByRole('button', { name: 'Notes' }).getAttribute('aria-pressed')).toBe('true');
    await fireEvent.click(within(screen.getByRole('banner')).getByRole('button', { name: 'Notes' }));
    await waitFor(() => expect(screen.queryByRole('complementary', { name: 'Notes' })).toBeNull());
    expect(transport.playing).toBe(false); expect(mock.action).not.toHaveBeenCalled();
  });

  it('reads measure, beat and passage at a distance, with bounds as measure and beat', async () => {
    mapped(); transport.position = 5.5; document.loop_selection = { start: 0, end: 8, fade_ms: 5 }; document.loop_enabled = true;
    document.selected_section = 'verse';
    mock.status.mockImplementation(async () => ({ ...transport, musical_position: { measure: 'map-5', label: '2', beat: 2.5 } }));
    mock.initialize.mockResolvedValue({ ...(await mock.initialize()), document: structuredClone(document) });
    await ready();
    const readout = within(screen.getByRole('region', { name: 'Now playing' }));
    await readout.findByText('Measure');
    expect(readout.getByRole('button', { name: /^Measure 2/ })).toBeTruthy();
    expect(readout.getByLabelText('Beat 2 of 4')).toBeTruthy();
    expect(readout.getByText('beat 2 · 0:05.5 / 0:12')).toBeTruthy();
    expect(readout.getByText('Looping')).toBeTruthy();
    expect(readout.getByText('m. 1 – 2')).toBeTruthy();
    expect(readout.getByText('2 bars · Verse')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Jump to A' }).textContent).toContain('m. 1 · 1');
    expect(screen.getByRole('button', { name: 'Jump to B' }).textContent).toBe('B0:08.0'); // Free time after the map: a time.
    // Exact seconds are entered only from the passage card's popover.
    expect(screen.queryByLabelText('Loop end seconds')).toBeNull();
    const bounds = within(await openBounds());
    expect((bounds.getByLabelText('Loop end seconds') as HTMLInputElement).value).toBe('8.00');
    await fireEvent.change(bounds.getByLabelText('Loop start seconds'), { target: { value: '0.5' } });
    await waitFor(() => expect(document.loop_selection).toEqual({ start: .5, end: 8, fade_ms: 5 }));
    // Sections list as measures, the nested Chorus indented.
    const sections = within(screen.getByRole('complementary', { name: 'Sections' }));
    expect(sections.getByRole('button', { name: /Verse m\. 1–2/ })).toBeTruthy();
    expect(sections.getByRole('button', { name: /^Chorus/ }).closest('.row')?.classList.contains('nested')).toBe(true);
  });

  it('goes to a measure number from the measure readout', async () => {
    mapped(); mock.status.mockImplementation(async () => ({ ...transport, musical_position: { measure: 'map-3', label: '1', beat: 1 } }));
    await ready();
    await fireEvent.click(await within(screen.getByRole('region', { name: 'Now playing' })).findByRole('button', { name: /^Measure 1/ }));
    const popover = screen.getByRole('dialog', { name: 'Go to measure' });
    await fireEvent.input(within(popover).getByLabelText('Go to measure number'), { target: { value: '2' } });
    await fireEvent.click(within(popover).getByRole('button', { name: 'Go' }));
    await waitFor(() => expect(mock.action).toHaveBeenCalledWith({ type: 'seek', seconds: 4 }, 'test'));
    expect(screen.queryByRole('dialog', { name: 'Go to measure' })).toBeNull();
  });

  it('collapses the sections list without losing the lane or the section shortcuts', async () => {
    await ready();
    const toggle = screen.getByRole('button', { name: 'Sections' });
    expect(toggle.getAttribute('aria-expanded')).toBe('true');
    await fireEvent.click(toggle);
    expect(screen.queryByRole('complementary', { name: 'Sections' })).toBeNull();
    expect(screen.getByRole('button', { name: 'Sections' }).getAttribute('aria-expanded')).toBe('false');
    expect(within(screen.getByLabelText('Sections on timeline')).getByRole('button', { name: 'Chorus' })).toBeTruthy();
    await fireEvent.keyDown(window, { key: ']' });
    await waitFor(() => expect(document.selected_section).toBe('verse'));
  });

  it('zooms with = and -, and shows the whole song with 0, without buttons on screen', async () => {
    await ready();
    const viewport = screen.getByRole('button', { name: 'Move waveform view' }).querySelector('.viewport') as HTMLElement;
    await fireEvent.keyDown(window, { key: '=' }); expect(viewport.style.width).toBe('50%');
    await fireEvent.keyDown(window, { key: '=' }); expect(viewport.style.width).toBe('25%');
    await fireEvent.keyDown(window, { key: '-' }); expect(viewport.style.width).toBe('50%');
    await fireEvent.keyDown(window, { key: '0' }); expect(viewport.style.width).toBe('100%');
    expect(mock.action).not.toHaveBeenCalled();
  });
  it('points to the Stems view from the mixer when the song has no stems, and offers no zoom buttons', async () => {
    await ready();
    const mix = within(screen.getByRole('group', { name: 'Mix' }));
    expect(mix.getByRole('slider', { name: 'Click gain' })).toBeTruthy();
    expect(mix.queryByRole('slider', { name: /Bass/ })).toBeNull();
    for (const name of ['Zoom in', 'Zoom out', 'Show whole song']) expect(screen.queryByRole('button', { name })).toBeNull();
    await fireEvent.click(mix.getByRole('button', { name: 'Split into stems ›' }));
    expect(screen.getByRole('main', { name: 'Stems' })).toBeTruthy();
  });

  it('shows six stems after the click and mutes from the keyboard', async () => {
    mock.stemsStatus.mockResolvedValue({ ...attachedStems(), tracks: (['bass', 'drums', 'guitar', 'piano', 'vocals', 'other'] as const).map(role => ({ role, gain: 1, mute: false, solo: false })) });
    await ready();
    const mix = within(screen.getByRole('group', { name: 'Mix' }));
    await waitFor(() => expect(mix.getAllByRole('slider').map(knob => knob.getAttribute('aria-label'))).toEqual(['Click gain', 'Bass gain', 'Drums gain', 'Guitar gain', 'Piano gain', 'Vocals gain', 'Other gain']));
    await fireEvent.keyDown(window, { key: '3' });
    await waitFor(() => expect(mix.getByRole('button', { name: 'Mute Guitar' }).getAttribute('aria-pressed')).toBe('true'));
  });
});
