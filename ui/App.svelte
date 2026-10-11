<script lang="ts">
  import { onMount } from 'svelte';
  import ThemeControl from './ThemeControl.svelte';
  import NotesScreen from './NotesScreen.svelte';
  import {NotesSession,type NotesState} from './notes-session';
  import LibraryScreen from './LibraryScreen.svelte';
  import MetronomeControls from './MetronomeControls.svelte';
  import { MetronomeSession, type MetronomeState } from './metronome-session';
  import { snapLoopPoint } from './practice-timeline';
  import Waveform from './Waveform.svelte';
  import Icon from './Icon.svelte';
  import PlaybackSettings from './PlaybackSettings.svelte';
  import MeasureEditor from './MeasureEditor.svelte';
  import MapPlayback from './MapPlayback.svelte';
  import { measureAtFrame, type MapView, type MapPreview, type MeasureMap } from './timing';
  import { bridge, native } from './bridge';
  import { Keymap, loadKeymap } from './keymap';
  import { ActionDispatcher, type ActionPayload } from './action-dispatcher';
  import { MAPPING_ACTIONS, type ActionId, type ButtonId, type MappingActionId } from './actions';
  import { clamp, editing, ordered, range, sectionColor, time, zoom } from './model';
  import { pageForPlayhead } from './timeline';
  import type { Action, Loop, PracticeDocument, Section, Snapshot, Song, Transport, View } from './types';

  let song = $state<Song | null>(null);
  let project = $state<PracticeDocument | null>(null);
  let transport = $state<Transport>({ song_key: null, position: 0, playing: false, finished: false, audio_error: null, save_error: null, device: null });
  let busy = $state('');
  let error = $state('');
  let saved = $state(true);
  const mac = navigator.platform.includes('Mac');
  let keymap = $state(new Keymap(mac));
  let keymapPath = $state<string | null>(null);
  let keymapWarning = $state<string | null>(null);
  let keymapLoading = $state(false);
  let screen = $state<'practice' | 'mapping' | 'library' | 'notes'>('practice');
  let libraryScreen=$state<LibraryScreen>();
  const navigationButton=$derived(screen==='library'?'library_close':'show_library');
  let notesState=$state<NotesState>({text:'',dirty:false,pending:false,error:''});
  const notesSession=new NotesSession(async(key,text)=>{
    if(song?.key!==key)throw new Error('The song changed before notes could be saved.');
    await update({notes:text},true);
  },value=>{notesState=value;});
  const shortcuts = $derived(keymap.shortcuts(screen));
  let snapToMeasures = $state(true);
  let practiceFollowing = $state(true);
  let clickEnabled = $state(true), clickLevel = $state(50);
  let clickState = $state<MetronomeState>({pending:false,error:''});
  let reviewMap = $state<{key:string;map:MeasureMap|null}|null>(null);
  const clickSession = new MetronomeSession(value=>{clickState=value;});
  const audibleMap = $derived(screen==='mapping'&&reviewMap?.key===song?.key ? reviewMap!.map : project?.measure_map??null);
  $effect(()=>{
    if(native&&song)clickSession.set({key:song.key,enabled:clickEnabled&&!!audibleMap?.measures.length&&!transport.audio_error,volume:clickLevel/100,map:audibleMap?$state.snapshot(audibleMap):null});
  });
  let mapView = $state<MapView | null>(null);
  let measureEditor = $state<MeasureEditor>();
  let measureNumber = $state(1);
  let view = $state<View>({ start: 0, end: 12 });
  $effect(()=>{
    if(screen==='practice'&&practiceFollowing&&transport.playing&&song)
      view=pageForPlayhead(view,song.duration,transport.position);
  });
  let sectionDialog: HTMLDialogElement;
  let helpDialog: HTMLDialogElement;
  let nameInput: HTMLInputElement;
  let sectionId = $state<string | null>(null);
  let sectionName = $state('');
  let sectionStart = $state(0);
  let sectionEnd = $state(0);
  let dialogError = $state('');
  let revision = 0;
  let saves: Promise<void> = Promise.resolve();
  let controlTimer: ReturnType<typeof setTimeout> | undefined;
  let undo = $state<{ section: Section; selected: boolean }[]>([]);
  let mounted = false;
  const sections = $derived(ordered(project?.sections ?? []));
  const selection = $derived(project?.loop_selection ?? null);
  const speed = $derived(project?.controls.speed ?? 1);
  const pitch = $derived(project?.controls.semitones ?? 0);

  function apply(snapshot: Snapshot, documentRevision = revision) {
    if(snapshot.library_error)error=snapshot.library_error;
    if (snapshot.song) { if(song?.key!==snapshot.song.key){practiceFollowing=true;notesSession.reset(snapshot.song.key,snapshot.document?.notes??'');} song = snapshot.song; view = { start: 0, end: snapshot.song.duration }; }
    if (documentRevision === revision) { project = snapshot.document; if (snapshot.map_view !== undefined) mapView = snapshot.map_view; }
    if (documentRevision === revision) transport = snapshot.transport;
  }
  function flushControls() {
    if (controlTimer && project) {
      clearTimeout(controlTimer); controlTimer = undefined;
      void update({ controls: project.controls });
    }
  }
  async function open(demo = false, libraryKey?:string) {
    if (busy||!await notesSession.flush()) return;
    busy = demo ? 'Loading demo…' : 'Opening song…'; error = '';
    try {
      flushControls(); await saves.catch(()=>{});
      const snapshot = libraryKey?await bridge.openLibrarySong(libraryKey):demo ? await bridge.loadDemo() : await bridge.openSong();
      if (snapshot) { revision++; apply(snapshot); saved = true; undo = []; screen = 'practice'; }
    } catch (e) { error = String(e); }
    finally { busy = ''; }
  }
  function update(change: Partial<PracticeDocument>,throwOnError=false): Promise<void> {
    if (!project) return Promise.resolve();
    project = { ...project, ...change }; const document = $state.snapshot(project);
    const thisRevision = ++revision; saved = false;
    saves = saves.catch(() => {}).then(async () => {
      try {
        const snapshot = await bridge.update(document); apply(snapshot, thisRevision);
        if (thisRevision === revision) saved = true;
      } catch (e) { error = `Changes could not be saved: ${String(e)}`;if(throwOnError)throw e; }
    });
    return saves;
  }
  async function action(action: Action) {
    if (!song || busy) return;
    try {
      flushControls(); await saves.catch(()=>{}); const actionRevision = revision;
      const snapshot=await bridge.action(action);apply(snapshot,actionRevision);
      // Explicit navigation reveals the resulting native position even when
      // paused or continuous following is off, preserving each screen's zoom.
      if(action.type==='seek'&&snapshot.transport.song_key===song?.key){
        if(screen==='mapping')measureEditor?.revealPlayhead(snapshot.transport.position);
        else view=pageForPlayhead(view,song.duration,snapshot.transport.position);
      }
    }
    catch (e) { error = String(e); }
  }
  function controls(nextSpeed: number, nextPitch: number) {
    if (!project || !Number.isFinite(nextSpeed) || !Number.isFinite(nextPitch)) return;
    project = { ...project, controls: { speed: clamp(nextSpeed, .25, 2), semitones: clamp(nextPitch, -12, 12) } };
    revision++;
    clearTimeout(controlTimer);
    saved = false;
    controlTimer = setTimeout(() => { controlTimer = undefined; void update({ controls: project!.controls }); }, 90);
  }
  async function chooseLoop(loop: Loop) { await update({ loop_selection: loop, loop_enabled: true }); }
  function clearLoop() { return update({ loop_selection: null, loop_enabled: false }); }
  async function toggleLoop() {
    if (!project || !song) return;
    if (!selection) {
      const start = Math.min(transport.position, Math.max(0, song.duration - .1));
      const measure = snapToMeasures ? measureAtFrame(mapView?.measures??[],Math.round(start*song.rate)) : null;
      const loop = measure ? {start:measure.start/song.rate,end:measure.end/song.rate,fade_ms:5} : {start,end:Math.min(song.duration,start+4),fade_ms:5};
      await update({loop_selection:loop,loop_enabled:true});
    } else await update({ loop_enabled: !project.loop_enabled });
  }
  async function selectSection(section: Section) {
    const loop = { start: section.start, end: section.end, fade_ms: 5 };
    await update({ selected_section: section.id, loop_selection: loop });
    await action({ type: 'seek', seconds: section.start });
  }
  function nextSection(direction: number) {
    if (!sections.length) return;
    const index = sections.findIndex(s => s.id === project?.selected_section);
    const selected = index < 0 ? (direction > 0 ? 0 : sections.length - 1) : clamp(index + direction, 0, sections.length - 1);
    return selectSection(sections[selected]);
  }
  function startSection(section?: Section) {
    if (!song) return;
    sectionId = section?.id ?? null; sectionName = section?.name ?? '';
    sectionStart = section?.start ?? selection?.start ?? Math.min(transport.position, song.duration - .1);
    sectionEnd = section?.end ?? selection?.end ?? song.duration;
    dialogError = ''; sectionDialog.showModal(); requestAnimationFrame(() => nameInput?.focus());
  }
  async function saveSection() {
    if (!song || !project) return;
    const selected = range(sectionStart, sectionEnd, song.duration);
    if (!sectionName.trim() || sectionName.length > 128) { dialogError = 'Give this section a name (up to 128 characters).'; return; }
    if (!selected || sectionStart >= sectionEnd || sectionStart < 0 || sectionEnd > song.duration) { dialogError = 'Choose a start and end inside the song, at least 0.1 seconds apart.'; return; }
    const section: Section = { id: sectionId ?? crypto.randomUUID(), name: sectionName.trim(), start: sectionStart, end: sectionEnd };
    const next = sectionId ? project.sections.map(s => s.id === sectionId ? section : s) : [...project.sections, section];
    await update({ sections: next, selected_section: section.id }); sectionDialog.close();
  }
  async function removeSection(id: string) {
    if (!project) return;
    const section = project.sections.find(s => s.id === id);
    if (section) undo = [...undo, { section, selected: project.selected_section === id }];
    await update({ sections: project.sections.filter(s => s.id !== id), selected_section: project.selected_section === id ? null : project.selected_section });
  }
  async function undoDelete() {
    const last = undo.at(-1); if (!last || !project) return;
    undo = undo.slice(0, -1);
    await update({ sections: [...project.sections, last.section], selected_section: last.selected && !project.selected_section ? last.section.id : project.selected_section });
  }
  function fitLoop() { if (selection && song) { const pad = (selection.end - selection.start) * .15; view = { start: Math.max(0, selection.start - pad), end: Math.min(song.duration, selection.end + pad) }; } }
  function setBoundary(edge: 'start' | 'end') {
    if (!song) return;
    const at = snapToMeasures ? snapLoopPoint(transport.position,mapView?.measures??[],song.rate) : transport.position;
    if(!selection&&snapToMeasures){
      const measure=measureAtFrame(mapView?.measures??[],Math.round(transport.position*song.rate));
      if(measure){const loop=range(measure.start/song.rate,measure.end/song.rate,song.duration);if(loop)void chooseLoop(loop);return;}
    }
    const start = edge === 'start' ? at : selection?.start ?? 0;
    const end = edge === 'end' ? at : selection?.end ?? Math.min(song.duration, transport.position + 4);
    const loop = range(start, end, song.duration); if (loop) void chooseLoop(loop);
  }
  function seekBy(seconds: number) { if (song) return action({ type: 'seek', seconds: clamp(transport.position + seconds, 0, song.duration) }); }
  async function reloadKeymap() {
    if (keymapLoading) return;
    keymapLoading = true;
    try {
      const loaded = await loadKeymap(bridge.readKeymap, mac);
      if (mounted) { keymap = loaded.model; keymapPath = loaded.path; keymapWarning = loaded.warning; }
    } finally { keymapLoading = false; }
  }
  async function commitMap(value: MapPreview, songKey:string) {
    if(busy||song?.key!==songKey)throw new Error('The song changed. Please reopen the measure map.');
    flushControls(); await saves.catch(()=>{});
    if(busy||song?.key!==songKey||!project)throw new Error('The song changed. Please try again.');
    busy='Saving measure map…';const document={...$state.snapshot(project),measure_map:value.map};
    const thisRevision=++revision;
    try {
      saves=saves.catch(()=>{}).then(async()=>{apply(await bridge.update(document),thisRevision);});
      await saves;mapView=value.view;saved=true;
    } finally {busy='';}
  }
  function nextMeasure(direction:number) {
    if(!song||!mapView?.measures.length)return;
    const frame=Math.round(transport.position*song.rate), measures=mapView.measures;
    const target=direction>0?measures.find(m=>m.start>frame):[...measures].reverse().find(m=>m.start<frame);
    if(target){if(screen==='mapping')measureEditor?.selectMeasure(target.id);return action({type:'seek',seconds:target.start/song.rate});}
  }
  const mappingHandlers = Object.fromEntries(Object.keys(MAPPING_ACTIONS).map(id=>[id,()=>measureEditor?.command(id as MappingActionId)])) as Record<MappingActionId,()=>Promise<void>>;
  async function closeNotes(){if(await notesSession.flush())screen='practice';}
  async function showLibrary(){
    if(!await notesSession.flush())return;
    if(transport.playing)await action({type:'pause'});
    screen='library';
  }
  const dispatcher = new ActionDispatcher({
    show_notes:()=>{screen='notes';},close_notes:closeNotes,retry_notes:async()=>{await notesSession.flush();},
    show_library:showLibrary,library_close:()=>{screen='practice';},library_open:({songKey})=>open(false,songKey),
    library_edit:payload=>libraryScreen?.command('library_edit',payload),library_save:()=>libraryScreen?.command('library_save'),
    library_cancel:()=>libraryScreen?.command('library_cancel'),library_retry:()=>libraryScreen?.command('library_retry'),
    library_sort:payload=>libraryScreen?.command('library_sort',payload),library_reverse:()=>libraryScreen?.command('library_reverse'),
    ...mappingHandlers,
    show_measure_map:()=>{screen='mapping';}, show_practice:()=>{screen='practice';},
    previous_measure:()=>nextMeasure(-1), next_measure:()=>nextMeasure(1),
    goto_measure:()=>{const measure=mapView?.measures.find(m=>m.number===measureNumber);if(measure&&song){if(screen==='mapping')measureEditor?.selectMeasure(measure.id);return action({type:'seek',seconds:measure.start/song.rate});}},
    practice_follow:()=>{practiceFollowing=!practiceFollowing;},
    practice_metronome:()=>{clickEnabled=!clickEnabled;}, toggle_loop_snap:()=>{snapToMeasures=!snapToMeasures;},
    play_pause: () => action({ type: 'toggle_play' }), toggle_loop: toggleLoop,
    set_loop_start: () => setBoundary('start'), set_loop_end: () => setBoundary('end'),
    jump_loop_start: () => action({ type: 'seek', seconds: selection!.start }),
    jump_loop_end: () => action({ type: 'seek', seconds: selection!.end }),
    jump_song_start: () => action({ type: 'seek', seconds: 0 }), clear_loop: clearLoop,
    seek_backward: () => seekBy(-5), seek_forward: () => seekBy(5),
    seek_backward_fine: () => seekBy(-.1), seek_forward_fine: () => seekBy(.1),
    previous_section: () => nextSection(-1), next_section: () => nextSection(1),
    add_section: () => startSection(), open_song: async () => { await showLibrary(); if(screen==='library')await open(); },
    disable_loop: () => update({ loop_enabled: false }), show_shortcuts: () => helpDialog.showModal(),
    reload_keymap: reloadKeymap, load_demo: () => open(true), fit_loop: fitLoop,
    zoom_in: () => { view = zoom(view, song!.duration, .5, selection ? (selection.start + selection.end) / 2 : transport.position); },
    zoom_out: () => { view = zoom(view, song!.duration, 2, (view.start + view.end) / 2); },
    show_whole_song: () => { view = { start: 0, end: song!.duration }; },
    reset_speed: () => controls(1, pitch), reset_pitch: () => controls(speed, 0),
    retry_audio: () => action({ type: 'retry_audio' }), retry_save: () => update({}),
    undo_delete: undoDelete, dismiss_error: () => { error = ''; },
    select_section: ({ section }) => selectSection(section!), edit_section: ({ section }) => startSection(section),
    delete_section: ({ section }) => removeSection(section!.id), speed_preset: ({ speed }) => controls(speed!, pitch),
    save_section: saveSection, close_editor: () => sectionDialog.close(), close_shortcuts: () => helpDialog.close()
  }, () => ({ context: screen, busy: !!busy, song: !!song, range: !!selection, sections: !!sections.length,
    audio: !!song && native && !transport.audio_error, loop_on: !!project?.loop_enabled,
    undo: !!undo.length, editor: !!sectionDialog?.open, help: !!helpDialog?.open,
    zoom_out: !!song && view.end - view.start < song.duration }));

  async function runAction(id: ActionId, payload: ActionPayload = {}) {
    try { if (actionAvailable(id)) await dispatcher.run(id, payload); } catch (e) { error = String(e); }
  }
  function runButton(id: ButtonId, payload: ActionPayload = {}) { return runAction(keymap.actionFor(id), payload); }
  function actionAvailable(id:ActionId) {
    if(!dispatcher.available(id))return false;
    if(screen==='notes'&&!['close_notes','retry_notes','show_library','open_song','show_shortcuts','close_shortcuts','reload_keymap','dismiss_error'].includes(id))return false;
    if(id==='retry_notes')return notesState.dirty&&!notesState.pending;
    if(id.startsWith('library_')&&id!=='library_close')return libraryScreen?.canRun(id)??false;
    if(screen==='library'&&!['show_library','library_close','open_song','load_demo','show_shortcuts','close_shortcuts','reload_keymap','dismiss_error'].includes(id))return false;
    if(id==='practice_metronome')return native&&(clickEnabled||(!transport.audio_error&&!!audibleMap?.measures.length));
    if(id==='toggle_loop_snap')return !!mapView?.measures.length;
    if(id in MAPPING_ACTIONS)return !!measureEditor?.canRun(id as MappingActionId);
    if(['previous_measure','next_measure','goto_measure'].includes(id))return !!mapView?.measures.length;
    return true;
  }
  function buttonAvailable(id: ButtonId) { return actionAvailable(keymap.actionFor(id)); }
  function keyboard(event: KeyboardEvent) {
    if (['Enter',' '].includes(event.key) && event.target instanceof Element && event.target.closest('button, [role="button"]')) return;
    if (editing(event.target) || busy || sectionDialog?.open || helpDialog?.open) return;
    const id = keymap.match(event,screen);
    if (!id || !actionAvailable(id)) return;
    event.preventDefault(); void runAction(id);
  }
  onMount(() => {
    mounted = true; busy = 'Restoring practice…';
    const restore = bridge.initialize().then(snapshot => { if (mounted) apply(snapshot); }).catch(e => { if (mounted) error = String(e); });
    Promise.all([restore, reloadKeymap()]).finally(() => { if (mounted) busy = ''; });
    let unlistenExit:(()=>void)|undefined,exiting=false;
    void bridge.onExitRequested(async()=>{
      if(exiting)return;exiting=true;
      try{
        flushControls();
        if(await notesSession.flush()){await saves;await bridge.finishExit();}
      }catch(e){error=String(e);}finally{exiting=false;}
    }).then(unlisten=>{if(mounted)unlistenExit=unlisten;else unlisten();}).catch(e=>{error=String(e);});
    let polling = false;
    const timer = setInterval(async () => {
      if (!song || busy || polling) return;
      polling = true;
      try { const statusRevision=revision;const status = await bridge.status(); if (mounted && statusRevision===revision && status.song_key === song?.key) transport = status; }
      catch (e) { if (mounted) error = String(e); } finally { polling = false; }
    }, 50);
    return () => { mounted = false; clearInterval(timer); clearTimeout(controlTimer); unlistenExit?.();notesSession.dispose();clickSession.dispose(); };
  });
</script>

<svelte:window onkeydown={keyboard}/>
<div class="app-shell">
  <header class="app-header">
    <div class="brand" aria-label="OhMyLoop">
      <svg viewBox="0 0 34 34" width="34" height="34" aria-hidden="true"><path d="M9 8a12 12 0 1 1-4 13" fill="none" stroke="var(--blue)" stroke-width="3" stroke-linecap="round"/><path d="m5 6 4 2-1 5" fill="none" stroke="var(--blue)" stroke-width="3" stroke-linecap="round"/><path d="M11 17h2l2-5 3 10 3-6h2" fill="none" stroke="var(--amber)" stroke-width="2" stroke-linecap="round"/></svg>
      <span>OhMyLoop</span>
    </div>
    <div class="song-heading"><h1>{song?.title ?? 'Make room for practice'}</h1><span>{song ? `${time(song.duration, 0)} • ${song.path ? 'Local song' : 'Built-in demo'}` : 'One song. As many tries as it takes.'}</span></div>
    <button class="button library-button" disabled={!buttonAvailable(navigationButton)} title={keymap.title(navigationButton)} onclick={()=>runButton(navigationButton)}>{keymap.buttonLabel(navigationButton,screen==='library'?'Practice':'Library')}</button>
  </header>

  {#if error}<div class="notice error" role="alert"><span>{error}</span><button class="text-button" title={keymap.title('dismiss_error')} onclick={() => runButton('dismiss_error')}>Dismiss</button></div>{/if}
  {#if clickState.error}<div class="notice error" role="alert">{clickState.error}</div>{/if}
  {#if keymapWarning}<div class="notice" role="alert"><span>{keymapWarning}</span><button class="text-button" title={keymap.title('reload_keymap')} disabled={keymapLoading} onclick={() => runButton('reload_keymap')}>Reload configuration</button></div>{/if}
  {#if transport.audio_error && song}<div class="notice audio-notice" role="status"><span>Audio output is unavailable. Reconnect your output device, then retry.</span><button class="text-button" title={keymap.title('retry_audio')} onclick={() => runButton('retry_audio')}>Retry audio</button></div>{/if}
  {#if transport.save_error}<div class="notice error" role="alert"><span>{transport.save_error}</span><button class="text-button" title={keymap.title('retry_save')} onclick={() => runButton('retry_save')}>Retry save</button></div>{/if}
  {#if !native}<div class="preview-note">Interface preview · Audio playback and file opening are available in the desktop app.</div>{/if}

  <LibraryScreen bind:this={libraryScreen} active={screen==='library'} currentKey={song?.key??null} {keymap} available={buttonAvailable} run={(id,payload)=>{void runButton(id,payload);}} onchanged={entry=>{if(song?.key===entry.song_key)song={...song,title:entry.metadata.title};}}/>
  {#if song}<NotesScreen active={screen==='notes'} state={notesState} {keymap} available={buttonAvailable} onchange={text=>notesSession.change(text)} run={id=>{void runButton(id);}}/>{/if}
  <main class="practice-layout" hidden={screen!=='practice'} aria-busy={!!busy}>
    <aside class="sections-panel">
      <div class="panel-title"><h2>Sections</h2><span>{sections.length || ''}</span></div>
      <p class="panel-intro">Find your place. Work on what needs it.</p>
      {#if sections.length}
        <div class="section-list">
          {#each sections as section (section.id)}
            <div class:selected={project?.selected_section === section.id} class="section-row" style:--section-color={sectionColor(section.id)}>
              <button class="section-select" title={keymap.title('select_section')} onclick={() => runButton('select_section', { section })} aria-pressed={project?.selected_section === section.id}>
                <span class="section-dot"></span><span class="section-text"><strong>{section.name}</strong><span>{time(section.start)} – {time(section.end)}</span></span><span class="section-duration">{time(section.end - section.start, 0)}</span>
              </button>
              <div class="section-actions"><button class="icon-button" aria-label={`Edit ${section.name}`} title={keymap.title('edit_section')} onclick={() => runButton('edit_section', { section })}><Icon name="edit" size={15}/></button><button class="icon-button" aria-label={`Delete ${section.name}`} title={keymap.title('delete_section')} onclick={() => runButton('delete_section', { section })}><Icon name="trash" size={15}/></button></div>
            </div>
          {/each}
        </div>
      {:else}<div class="sections-empty">{song ? 'Save a passage as a section to jump back to it.' : 'Your song’s sections will live here.'}</div>{/if}
      <button class="button add-section" disabled={!buttonAvailable('add_section')} title={keymap.title('add_section')} onclick={() => runButton('add_section')}><Icon name="plus" size={17}/>Add section</button>
      <div class="sidebar-tip"><span class="keycap">{keymap.shortcut('previous_section')}</span><span class="keycap">{keymap.shortcut('next_section')}</span><span>Move between sections</span></div>
    </aside>

    <section class="workbench" aria-label="Practice workspace">
      {#if song && project}
        <div class="workspace-heading"><div class="workspace-tools"><button class="button map-screen-button" disabled={!buttonAvailable('show_notes')} title={keymap.title('show_notes')} onclick={()=>runButton('show_notes')}>{keymap.buttonLabel('show_notes','Notes')}</button><button class="button map-screen-button" disabled={!buttonAvailable('show_measure_map')} title={keymap.title('show_measure_map')} onclick={()=>runButton('show_measure_map')}>{keymap.buttonLabel('show_measure_map','Measure map')}</button></div><div class="zoom-controls"><button class="icon-button" aria-label={keymap.buttonLabel('zoom_out', 'Zoom out')} disabled={!buttonAvailable('zoom_out')} title={keymap.title('zoom_out')} onclick={() => runButton('zoom_out')}><Icon name="minus" size={17}/></button><span>{Math.round(song.duration / (view.end - view.start))}×</span><button class="icon-button" aria-label={keymap.buttonLabel('zoom_in', 'Zoom in')} title={keymap.title('zoom_in')} onclick={() => runButton('zoom_in')}><Icon name="plus" size={17}/></button><button class="icon-button" aria-label={keymap.buttonLabel('show_whole_song', 'Show whole song')} title={keymap.title('show_whole_song')} onclick={() => runButton('show_whole_song')}><Icon name="fit" size={17}/></button></div></div>
        <Waveform {mapView} {snapToMeasures} rate={song.rate} peaks={song.peaks} duration={song.duration} {sections} {selection} position={transport.position} {view} onseek={seconds => action({ type: 'seek', seconds })} onselection={chooseLoop} onsection={section => runButton('select_section', { section })} onview={next => { view = next; }}/>
        <div class="practice-listening">
          {#if mapView?.measures.length}<label class="snap-control" title={keymap.title('loop_snap')}><input type="checkbox" checked={snapToMeasures} disabled={!buttonAvailable('loop_snap')} onchange={()=>runButton('loop_snap')}/>{keymap.buttonLabel('loop_snap','Snap to measures')}</label>{/if}
          {#if screen==='practice'}<button class="text-button practice-follow" aria-pressed={practiceFollowing} disabled={!buttonAvailable('practice_follow')} title={keymap.title('practice_follow')} onclick={()=>runButton('practice_follow')}>{keymap.buttonLabel('practice_follow',practiceFollowing?'Follow playhead on':'Follow playhead off')}</button><MetronomeControls enabled={clickEnabled} bind:level={clickLevel} {keymap} button="practice_metronome" available={buttonAvailable('practice_metronome')} run={()=>{void runButton('practice_metronome');}} {native}/>{/if}
        </div>
        <div class="selection-bar">
          {#if selection}
            <div class="loop-times"><button class="endpoint endpoint-button" aria-label={keymap.description(keymap.actionFor('jump_loop_start'))} title={keymap.title('jump_loop_start')} onclick={() => runButton('jump_loop_start')}>A</button><input aria-label="Loop start seconds" type="number" min="0" max={selection.end - .1} step="any" value={selection.start.toFixed(2)} onchange={e => { const r = range(Number(e.currentTarget.value), selection.end, song!.duration); if (r) void chooseLoop(r); }}/><button class="endpoint endpoint-button" aria-label={keymap.description(keymap.actionFor('jump_loop_end'))} title={keymap.title('jump_loop_end')} onclick={() => runButton('jump_loop_end')}>B</button><input aria-label="Loop end seconds" type="number" min={selection.start + .1} max={song.duration} step="any" value={selection.end.toFixed(2)} onchange={e => { const r = range(selection.start, Number(e.currentTarget.value), song!.duration); if (r) void chooseLoop(r); }}/><span class="unit">seconds</span><span class="loop-length">{(selection.end - selection.start).toFixed(2)}s passage</span></div>
            <div class="selection-actions"><button class="text-button" title={keymap.title('fit_loop')} onclick={() => runButton('fit_loop')}>Fit passage</button><button class="text-button" title={keymap.title('save_selection')} onclick={() => runButton('save_selection')}>Save as section</button><button class="icon-button" aria-label={keymap.buttonLabel('clear_loop', 'Clear loop selection')} title={keymap.title('clear_loop')} onclick={() => runButton('clear_loop')}><Icon name="close" size={16}/></button></div>
          {:else}<span class="selection-hint">Choose any passage to loop, or save it as a named section.</span><span class="selection-shortcut"><span class="keycap">{keymap.shortcut('set_loop_start')}</span><span class="keycap">{keymap.shortcut('set_loop_end')}</span>Set boundaries at the playhead</span>{/if}
        </div>

        <div class="transport-row">
          <div class="transport-buttons" role="group" aria-label="Playback controls"><button class="icon-button" aria-label={keymap.buttonLabel('previous_section', 'Previous section')} disabled={!buttonAvailable('previous_section')} title={keymap.title('previous_section')} onclick={() => runButton('previous_section')}><Icon name="back"/></button><button class="play-button" aria-label={keymap.buttonLabel('play', transport.playing ? 'Pause' : 'Play')} title={keymap.title('play')} disabled={!buttonAvailable('play')} onclick={() => runButton('play')}><Icon name={transport.playing ? 'pause' : 'play'} size={27}/></button><button class="icon-button" aria-label={keymap.buttonLabel('next_section', 'Next section')} disabled={!buttonAvailable('next_section')} title={keymap.title('next_section')} onclick={() => runButton('next_section')}><Icon name="next"/></button></div>
          <div class="transport-time"><strong>{time(transport.position)}</strong><span>/ {time(song.duration)}</span><small>{transport.playing ? 'Playing' : transport.finished ? 'End of song' : 'Paused'}</small></div>
          <button class:active={project.loop_enabled} class="loop-toggle" onclick={() => runButton('loop')} aria-pressed={project.loop_enabled} title={keymap.title('loop')}><Icon name="loop"/>{keymap.buttonLabel('loop', project.loop_enabled ? 'Loop on' : 'Loop off')}</button>
        </div>

        <PlaybackSettings disabled={!!busy} {speed} {pitch} {keymap} {controls} {runButton}/>

      {:else}
        <div class="empty-workbench"><svg viewBox="0 0 200 90" aria-hidden="true"><path d="M10 45h12l5-11 8 25 8-40 9 56 8-48 8 35 8-21h12m17 4h12l5-11 8 25 8-40 9 56 8-48 8 35 8-21h12" fill="none" stroke="#9caed1" stroke-width="3" stroke-linecap="round"/></svg><h2>A good place to get it right.</h2><p>Browse your library, find a song, and play it at your pace.</p><div><button class="button primary" title={keymap.title('show_library')} onclick={() => runButton('show_library')} disabled={!buttonAvailable('show_library')}>Browse library</button><button class="text-button" title={keymap.title('demo')} onclick={() => runButton('demo')} disabled={!buttonAvailable('demo')}>Try the demo</button></div><span>WAV, MP3, FLAC, M4A and Ogg</span></div>
      {/if}
    </section>
  </main>
  {#if song}
    {#key song.key}
      {#snippet mapPlayback()}<MapPlayback {transport} duration={song?.duration??0} {speed} loopEnabled={project?.loop_enabled??false} {keymap} available={buttonAvailable} run={id=>{void runButton(id);}} onspeed={value=>controls(value,pitch)} disabled={!!busy}/>{/snippet}
      <MeasureEditor bind:this={measureEditor} {song} map={project?.measure_map??null} {mapView} {transport} {keymap} bind:clickEnabled bind:clickLevel onreview={map=>{reviewMap={key:song!.key,map};}} playback={mapPlayback} active={screen==='mapping'} oncommit={commitMap} onseek={seconds=>{void action({type:'seek',seconds});}} run={id=>{void runButton(id);}} available={buttonAvailable}/>
    {/key}
    <div class="musical-navigation" hidden={screen==='library'||screen==='notes'||!mapView?.measures.length}>
      <span>{transport.musical_position?`Measure ${transport.musical_position.label}, beat ${transport.musical_position.beat.toFixed(2)}`:'Unmapped position'}</span>
      <button class="text-button" disabled={!buttonAvailable('previous_measure')} title={keymap.title('previous_measure')} onclick={()=>runButton('previous_measure')}>{keymap.buttonLabel('previous_measure','Previous measure')}</button>
      <button class="text-button" disabled={!buttonAvailable('next_measure')} title={keymap.title('next_measure')} onclick={()=>runButton('next_measure')}>{keymap.buttonLabel('next_measure','Next measure')}</button>
      <label>Measure <input aria-label="Go to measure number" type="number" min="1" step="1" bind:value={measureNumber}/></label>
      <button class="text-button" disabled={!buttonAvailable('goto_measure')} title={keymap.title('goto_measure')} onclick={()=>runButton('goto_measure')}>{keymap.buttonLabel('goto_measure','Go')}</button>
    </div>

  {/if}
  <footer class="statusbar"><span>{#if undo.length && !busy}Section removed.<button class="text-button" title={keymap.title('undo_delete')} onclick={() => runButton('undo_delete')}>Undo</button>{:else}{busy || (song ? (saved ? 'Practice settings saved' : 'Saving practice settings…') : 'Ready when you are')}{#if song && saved}<Icon name="check" size={13}/>{/if}{/if}</span><span>{song ? (transport.device ?? (native ? 'No audio output' : 'Browser preview')) : 'Local songs • Independent speed and pitch'}</span><div class="statusbar-tools"><ThemeControl/><button class="shortcuts-button" title={keymap.title('shortcuts')} onclick={() => runButton('shortcuts')}>Shortcuts {#if screen==='practice'||screen==='mapping'}<span class="keycap">{keymap.shortcut(keymap.actionFor('play'))}</span>{keymap.description(keymap.actionFor('play'))}{/if}</button></div></footer>
</div>

<dialog class="section-dialog" bind:this={sectionDialog} onclose={() => { dialogError = ''; }}>
  <form onsubmit={event => { event.preventDefault(); void runButton('save_section'); }}><div class="dialog-heading"><h2>{sectionId ? 'Edit section' : 'Save a section'}</h2><button type="button" class="icon-button" aria-label="Close section editor" title={keymap.title('close_editor')} onclick={() => runButton('close_editor')}><Icon name="close"/></button></div><p>Keep a passage you’ll want to come back to.</p><label>Section name<input bind:this={nameInput} bind:value={sectionName} maxlength="128" placeholder="Verse, solo, a tricky phrase…" required/></label><div class="dialog-times"><label>Start (seconds)<input type="number" min="0" max={song?.duration ?? 0} step="any" bind:value={sectionStart} required/></label><label>End (seconds)<input type="number" min="0" max={song?.duration ?? 0} step="any" bind:value={sectionEnd} required/></label></div>{#if dialogError}<p class="form-error" role="alert">{dialogError}</p>{/if}<div class="dialog-actions"><button type="button" class="button" title={keymap.title('cancel_editor')} onclick={() => runButton('cancel_editor')}>Cancel</button><button class="button primary" type="submit" title={keymap.title('save_section')}>Save section</button></div></form>
</dialog>
<dialog class="help-dialog" bind:this={helpDialog}>
  <div class="dialog-heading"><h2>Keep your hands on the guitar</h2><button class="icon-button" aria-label="Close shortcuts" title={keymap.title('close_shortcuts')} onclick={() => runButton('close_shortcuts')}><Icon name="close"/></button></div>
  <p>Shortcuts work when you’re not editing a field.</p>
  <dl>{#each shortcuts as shortcut (shortcut.action)}<div><dt>{shortcut.keys}</dt><dd>{shortcut.description}</dd></div>{/each}</dl>
  <details class="keymap-settings"><summary>Customize shortcuts</summary>
    {#if keymapPath}<p>Edit your configuration file, then reload it.</p><code class="keymap-path">{keymapPath}</code>
    <button class="text-button" title={keymap.title('reload_keymap')} disabled={keymapLoading} onclick={() => runButton('reload_keymap')}>{keymapLoading ? 'Reloading…' : 'Reload configuration'}</button>
    {:else}<p>Configuration files are available in the desktop app.</p>{/if}
  </details>
</dialog>
