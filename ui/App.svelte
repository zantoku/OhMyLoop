<script lang="ts">
  import { onMount, tick, untrack } from 'svelte';
  import StemsScreen from './StemsScreen.svelte';
  import {SeparationSession} from './separation-session';
  import {finishedAttempt,separationFrozen,separationToast,type ProcessingEstimate,type SeparationState} from './separation';
  import type {Attachment} from './stems-screen';
  import {StemsSession,type StemsState} from './stems-session';
  import {completedProcessing,processingLabel,type StemRole,type StemProgress} from './stems';
  import SettingsDialog from './SettingsDialog.svelte';
  import { settingsBlockedReason, settingsDownload, type SettingsSection, type SettingsSummary } from './settings';
  import { BeatModelSession, type BeatModelState } from './beat-model-session';
  import { readThemePreference, saveThemePreference, type ThemePreference } from './theme';
  import { readShortcutsButton, saveShortcutsButton } from './shortcuts-button';
  import NotesDrawer from './NotesDrawer.svelte';
  import { readNotesFont, saveNotesFont, type NotesFont } from './notes-font';
  import RecordingChooser from './RecordingChooser.svelte';
  import {RecordingReplacement} from './recording-replacement';
  import {NotesSession,notesUnsaved,type NotesState} from './notes-session';
  import LibraryScreen from './LibraryScreen.svelte';
  import ManagedLibraryDialog from './ManagedLibraryDialog.svelte';
  import DropOverlay from './DropOverlay.svelte';
  import { FileDropTarget, type DropPreview } from './file-drop';
  import { fileName } from './library';
  import {ManagedLibrarySession,type ManagedState} from './managed-library-session';
  import { CountInSession, type CountInState } from './count-in-session';
  import { MetronomeSession, type MetronomeState } from './metronome-session';
  import { snapLoopPoint } from './practice-timeline';
  import Icon from './Icon.svelte';
  import AppHeader from './AppHeader.svelte';
  import Toasts from './Toasts.svelte';
  import PlaybackBar from './PlaybackBar.svelte';
  import PracticeScreen from './PracticeScreen.svelte';
  import TransportButtons from './TransportButtons.svelte';
  import PracticeModes from './PracticeModes.svelte';
  import PracticeMixer from './PracticeMixer.svelte';
  import { notices } from './notices';
  import { closeTopTransient } from './transient';
  import MeasureMapScreen, { type MapNotice } from './MeasureMapScreen.svelte';
  import { adjacentMeasure, measureAtFrame, type MapView, type MapPreview, type MeasureMap } from './timing';
  import { bridge, native } from './bridge';
  import { Keymap, loadKeymap, type KeymapSource } from './keymap';
  import { ActionDispatcher, type ActionPayload } from './action-dispatcher';
  import { MAPPING_ACTIONS, type ActionId, type ButtonId, type MappingActionId } from './actions';
  import { clamp, ordered, range, zoom } from './model';
  import { followPage, pageForPlayhead } from './timeline';
  import type { Action, Controls, Loop, PracticeDocument, Section, Snapshot, Song, Transport, View } from './types';
  import { normalized, ORIGINAL, stepSpeed, transpose } from './playback-controls';
  import type { ViewSegment } from './ViewSwitcher.svelte';
  import { ownsKey } from './key-routing';
  import { currentModality } from './input-modality';
  import { atHome, focusHome } from './home-focus';
  import { AppMenu } from './app-menu';
  import { appEditAction, editCommand, routeEditCommand, type EditCommand } from './undo-routing';

  let song = $state<Song | null>(null);
  let project = $state<PracticeDocument | null>(null);
  let transport = $state<Transport>({ song_key: null, reference_token: null, position: 0, playing: false, finished: false, audio_error: null, save_error: null, device: null });
  let busy = $state('');
  let error = $state('');
  let startupNotice=$state('');
  let saved = $state(true);
  const mac = navigator.platform.includes('Mac');
  let keymap = $state(new Keymap(mac));
  let keymapPath = $state<string | null>(null);
  let keymapWarning = $state<string | null>(null);
  let keymapLoading = $state(false);
  let keymapSource = $state<KeymapSource>('builtin');
  let screen = $state<'practice' | 'mapping' | 'library' | 'stems'>('practice');
  let separationState=$state<SeparationState>({status:null,pending:false,error:''});
  let quitConfirmation=$state(false);
  let exiting=$state(false);
  let quitDialog:HTMLDialogElement;
  $effect(()=>{if(quitConfirmation)quitDialog?.showModal();else quitDialog?.close();});
  let separationAdmission=$state(false);
  let separationEstimate=$state<ProcessingEstimate|null>(null);
  const separationActive=$derived(separationAdmission||(separationState.status?.job.running??false));
  const separationSession=new SeparationSession(bridge,state=>{separationState=state;},async status=>{if(song?.reference_token===status.job.reference_token&&status.job.result){stemsSession.reset(status.job.result);const value=await bridge.status();if(value.reference_token===song.reference_token)transport=value;}});
  async function startSeparation(){if(separationAdmission)return;separationAdmission=true;try{await transportActions;flushControls();await saves;await stemsSession.flush();await countInSession.flush();await clickSession.flush();if(song){await separationSession.separate(song.reference_token);const status=await bridge.status();if(status.reference_token===song.reference_token)transport=status;}}finally{separationAdmission=false;}}
  async function finishExit(){
    if(exiting)return;exiting=true;quitConfirmation=false;
    try{
      flushControls();
      if(!await notesSession.flush()){showNotesFailure();return;}
      // Unsaved song details block quitting too; the Library shows why.
      if(libraryScreen&&!await libraryScreen.flush()){screen='library';return;}
      await stemsSession.flush();await saves;await bridge.finishExit();
    }
    catch(e){error=String(e);}finally{exiting=false;}
  }
  let stemsState=$state<StemsState>({snapshot:null,pending:false,error:''});
  let stemProgress=$state<StemProgress|null>(null),stemsLoading=$state(false);
  // A manual attachment started on the Stems screen; its progress events carry its id.
  let attachment=$state<Attachment|null>(null);
  let stemEpoch=0;
  const stemsSession=new StemsSession(bridge.setStemMix,state=>{stemsState=state;});
  // Free space changes between visits, so Stems asks again whenever it opens.
  function refreshEstimate(reference:string,epoch=stemEpoch){void bridge.separationEstimate(reference).then(value=>{if(epoch===stemEpoch)separationEstimate=value;}).catch(()=>{});}
  async function reloadStems(reference:string){const epoch=++stemEpoch;separationEstimate=null;refreshEstimate(reference,epoch);stemsLoading=true;stemsSession.reset(null);try{const value=await bridge.stemsStatus(reference);if(mounted&&epoch===stemEpoch&&song?.reference_token===reference)stemsSession.reset(value);}catch(e){if(mounted&&epoch===stemEpoch)error=String(e);}finally{if(epoch===stemEpoch)stemsLoading=false;}}
  function soloRole(role:StemRole){const track=stemsState.snapshot?.tracks.find(track=>track.role===role);if(track)stemsSession.change(role,{solo:!track.solo});}
  function muteRole(role:StemRole){const track=stemsState.snapshot?.tracks.find(track=>track.role===role);if(track)stemsSession.change(role,{mute:!track.mute});}
  async function stemsWork(remove=false){
    await transportActions;
    if(!song||busy)return;
    const reference=song.reference_token,setId=stemsState.snapshot?.set_id;
    busy=remove?'Removing stems…':'Choosing stems…';error='';stemProgress=null;if(!remove)attachment=null;
    try{
      if(!await prepareChange())return;
      const path=remove?null:await bridge.pickStemsDirectory();if(!remove&&!path)return;
      if(song?.reference_token!==reference)return;
      if(transport.playing)apply(await bridge.action({type:'pause'},reference));
      busy=remove?'Removing stems…':'Attaching stems…';stemProgress=null;if(!remove)attachment={id:crypto.randomUUID(),progress:null,running:true,error:''};
      const value=remove?await bridge.removeStems(reference,setId!):await bridge.attachStems(reference,path!,attachment!.id);
      if(mounted&&song?.reference_token===reference){if(!remove&&attachment)attachment={...attachment,running:false,progress:completedProcessing(attachment.progress)};stemsSession.reset(value);const status=await bridge.status();if(status.reference_token===reference)transport=status;}
    }catch(e){if(mounted){if(attachment?.running)attachment={...attachment,running:false,error:String(e)};else error=String(e);}}finally{if(mounted){busy='';if(attachment?.running)attachment={...attachment,running:false};}}
  }
  let libraryScreen=$state<LibraryScreen>();
  let stemsScreen=$state<StemsScreen>();
  let libraryEpoch=$state(0);
  let managedState=$state<ManagedState>({status:{selected:null,available:false,error:null,import_notice_seen:false},dialog:null,selection:null,pendingImport:false,deleteKey:null,deleteTitle:''});
  const managed=new ManagedLibrarySession(value=>{managedState=value;});
  let notesState=$state<NotesState>({text:'',dirty:false,pending:false,slow:false,tooLong:false,error:''});
  // The notes drawer belongs to Practice and stays open across views and songs.
  let notesOpen=$state(false);
  let notesPanel=$state<NotesDrawer>();
  let notesFont=$state<NotesFont>(readNotesFont());
  const notesSession=new NotesSession(async(key,text)=>{
    if(song?.reference_token!==key)throw new Error('The song changed before notes could be saved.');
    await update({notes:text},true);
  },value=>{notesState=value;});
  const shortcuts = $derived(keymap.shortcuts(screen));
  let snapToMeasures = $state(true);
  let countInEnabled = $state(false), repeatCountIn = $state(false);
  let countInState = $state<CountInState>({pending:false,error:''});
  const countInSession = new CountInSession(value=>{countInState=value;});
  const hasMeasureMap = $derived(!!project?.measure_map?.measures.length);
  const countInActive = $derived(countInEnabled && hasMeasureMap);
  // Count-in before Play and before each loop repeat are independent; Practice only.
  $effect(()=>{
    const live=screen!=='mapping'&&hasMeasureMap&&!transport.audio_error;
    if(native&&song&&!separationActive)countInSession.set({key:song.reference_token,enabled:live&&countInEnabled,repeat:live&&repeatCountIn,volume:clickLevel/100});
  });
  let clickEnabled = $state(true), clickLevel = $state(50);
  let clickState = $state<MetronomeState>({pending:false,error:''});
  let reviewMap = $state<{key:string;map:MeasureMap|null}|null>(null);
  const clickSession = new MetronomeSession(value=>{clickState=value;});
  const audibleMap = $derived(screen==='mapping'&&reviewMap?.key===song?.reference_token ? reviewMap!.map : project?.measure_map??null);
  // Practice uses the shared click preference; the Measure map always clicks
  // the displayed map or draft at full level (measure-map.md, decision 7).
  $effect(()=>{
    const mapping=screen==='mapping';
    if(native&&song&&!separationActive)clickSession.set({key:song.reference_token,enabled:(mapping||clickEnabled)&&!!audibleMap?.measures.length&&!transport.audio_error,volume:mapping?1:clickLevel/100,map:audibleMap?$state.snapshot(audibleMap):null});
  });
  let mapNotice = $state<MapNotice|null>(null);
  let mapView = $state<MapView | null>(null);
  let measureMap = $state<MeasureMapScreen>();
  let measureNumber = $state(1);
  let view = $state<View>({ start: 0, end: 12 });
  // Practice always follows the playing playhead: it pages when playback
  // starts off-page, when the playhead crosses the page's right edge, and when
  // it wraps back (loop or seek). A manual pan holds until one of those happens.
  let followed={position:0,playing:false};
  $effect(()=>{
    const {position,playing}=transport,previous=followed;
    followed={position,playing};
    if(screen!=='practice'||!song)return;
    const duration=song.duration;
    untrack(()=>{view=followPage(view,duration,previous,{position,playing});});
  });
  let practiceScreen = $state<PracticeScreen>();
  let sectionDialog: HTMLDialogElement;
  let helpDialog: HTMLDialogElement;
  // Reactive open state: set when shown, cleared by the dialogs' close events.
  let sectionOpen = $state(false), helpOpen = $state(false);
  let replacementDialog: RecordingChooser;
  const replacement=new RecordingReplacement();
  async function replaceLibrary(songKey?:string){
    if(!songKey||busy||!await notesSession.flush())return;
    busy='Checking reference recording…';error='';
    try{flushControls();await saves.catch(()=>{});if(song?.key===songKey){replacement.active(song.reference_token);}else{await replacement.library(songKey);}replacementDialog.showModal();}
    catch(e){error=String(e);}finally{busy='';}
  }
  function showReplacement(){
    if(!song||busy)return;replacement.active(song.reference_token);replacementDialog.showModal();
  }
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
  const playbackControls = $derived<Controls>({ ...ORIGINAL, ...project?.controls });
  const speed = $derived(playbackControls.speed);

  function apply(snapshot: Snapshot, documentRevision = revision) {
    if(documentRevision!==revision)return;
    if(!snapshot.song && song && snapshot.transport.reference_token!==song.reference_token)return;
    if(snapshot.library_status)managed.accept(snapshot.library_status);
    if(snapshot.startup_notice)startupNotice=snapshot.startup_notice;
    if(snapshot.library_error)error=snapshot.library_error;
    if(!snapshot.song&&!song&&screen!=='library')screen='library';
    if (snapshot.song) { if(song?.reference_token!==snapshot.song.reference_token){stemProgress=null;attachment=null;void reloadStems(snapshot.song.reference_token);countInEnabled=false;repeatCountIn=false;resetNotes(snapshot.song.reference_token,snapshot.document?.notes??'');} song = snapshot.song; view = { start: 0, end: snapshot.song.duration }; }
    if (documentRevision === revision) { project = snapshot.document; if (snapshot.map_view !== undefined) mapView = snapshot.map_view; }
    if (documentRevision === revision) transport = snapshot.transport;
  }
  async function recordingChange(timingUnchanged:boolean) {
    if(busy||!await notesSession.flush())return;
    replacementDialog?.close();busy='Opening recording…';error='';
    try {if(!await prepareChange())return;const snapshot=await replacement.replace(timingUnchanged);
      if(snapshot){revision++;apply(snapshot);saved=true;undo=[];screen='practice';}
    }catch(e){error=String(e);}finally{busy='';}
  }
  function flushControls() {
    if (controlTimer && project) {
      clearTimeout(controlTimer); controlTimer = undefined;
      void update({ controls: project.controls });
    }
  }
  function loaded(snapshot:Snapshot|null){if(!mounted||!snapshot)return;revision++;apply(snapshot);saved=true;undo=[];screen='practice';}
  function unloaded(snapshot:Snapshot){
    stemEpoch++;stemsLoading=false;stemsSession.reset(null);revision++;song=null;project=null;mapView=null;notesSession.reset('', '');reviewMap=null;undo=[];apply(snapshot);saved=true;screen='library';libraryEpoch++;
  }
  async function prepareChange(){
    await stemsSession.flush();
    if(!await notesSession.flush()){showNotesFailure();throw new Error('Notes aren’t saved yet. Review them in the notes drawer.');}
    if(libraryScreen&&!await libraryScreen.flush())throw new Error('Song details could not be saved. Close this dialog to review and retry.');
    flushControls();await saves;
    if(!saved){await update({},true).catch(e=>{throw new Error(`Changes could not be saved: ${String(e)}`);});if(!saved)return false;}return true;
  }
  async function managedWork(label:string,job:()=>Promise<void>){
    if(busy)return;stemProgress=null;busy=label;error='';try{await job();}catch(e){if(mounted)error=String(e);}finally{if(mounted){busy='';stemProgress=null;}}
  }
  async function chooseLibrary(create:boolean,locate=false){
    await managedWork('Opening library…',async()=>{
      if(!await prepareChange())return;
      const continueImport=managed.needsImport;
      const snapshot=await managed.choose(create,locate);if(!mounted||!snapshot)return;
      unloaded(snapshot);
      if(continueImport)loaded(await managed.continueImport());
    });
  }
  /** Imports a dropped file, or asks for a recording without one. */
  async function importSong(path?:string){await managedWork('Importing song…',async()=>{if(await prepareChange())loaded(await managed.beginImport(path??null));});}
  async function openLibrarySong(key:string){
    await managedWork('Opening song…',async()=>{if(await prepareChange())loaded(await bridge.openLibrarySong(key));});
  }
  // App-wide Settings (settings.md): a modal dialog over the current screen.
  // Preferences are per computer; tools and the library are app-wide.
  let settingsOpen=$state(false),settingsSection=$state<SettingsSection>('library');
  let theme=$state<ThemePreference>(readThemePreference()),shortcutsButton=$state(readShortcutsButton()),preferenceSaveFailed=$state(false);
  let librarySongCount=$state<number|null>(null),settingsInfo=$state('');
  let beatState=$state<BeatModelState>({status:null,pending:false,error:''});
  const beatModel=new BeatModelSession({analysisStatus:bridge.analysisStatus,installModel:bridge.installModel,removeModel:bridge.removeModel,cancelAnalysis:bridge.cancelAnalysis},value=>{beatState=value;});
  // Decision 6: while background work runs, Settings can't open and says why.
  // A download started inside Settings is the exception: closing Settings
  // mustn't strand its progress and Cancel, so Settings reopens on it.
  let ownedDownload=$state<'stems'|'beats'|null>(null);
  const runningDownload=$derived(settingsDownload(separationState.status,beatState.status));
  $effect(()=>{if(ownedDownload&&runningDownload!==ownedDownload&&!separationState.pending&&!beatState.pending)ownedDownload=null;});
  /** Remembers a download started from the open Settings. */
  function ownDownload(section:'stems'|'beats'){if(settingsOpen)ownedDownload=section;}
  const settingsBlocked=$derived(settingsBlockedReason({separating:separationActive,
    stemSetup:(!!separationState.status?.install.running||!!separationState.pendingInstall)&&ownedDownload!=='stems',
    beatDownload:!!beatState.status?.running&&beatState.status.kind==='install'&&ownedDownload!=='beats',
    analysing:(!!beatState.status?.running&&beatState.status.kind!=='install')||!!separationState.status?.heavy_work,busy}));
  $effect(()=>{if(!settingsBlocked)settingsInfo='';});
  const settingsSummary=$derived<SettingsSummary>({library:managedState.status,songCount:librarySongCount,theme,notesFont,device:transport.device,audioError:transport.audio_error,native,
    shortcuts:keymapSource,separation:separationState.status,analysis:beatState.status});
  async function countSongs(){
    if(!managedState.status.available){librarySongCount=null;return;}
    try{const catalogue=await bridge.readLibrary();if(mounted)librarySongCount=catalogue.songs.length;}catch{if(mounted)librarySongCount=null;}
  }
  $effect(()=>{if(settingsOpen){void libraryEpoch;void managedState.status.selected?.root;void managedState.status.available;untrack(()=>{void countSongs();});}});
  function showSettings({settingsSection:section}:ActionPayload){
    if(section)settingsSection=section;else if(!settingsOpen&&ownedDownload&&runningDownload===ownedDownload)settingsSection=ownedDownload;
    if(helpOpen)helpDialog.close();
    if(!settingsOpen){error='';settingsOpen=true;if(native)void beatModel.refresh();}
  }
  function setTheme(value:ThemePreference){theme=value;preferenceSaveFailed=!saveThemePreference(value);}
  function setNotesFont(value:NotesFont){notesFont=value;preferenceSaveFailed=!saveNotesFont(value);}
  function setShortcutsButton(shown:boolean){shortcutsButton=shown;preferenceSaveFailed=!saveShortcutsButton(shown);}
  async function reveal(location:'library'|'shortcut_file'){await bridge.revealLocation(location);}
  const revealLabel=navigator.userAgent.includes('Windows')?'Show in Explorer':mac?'Show in Finder':'Show in file manager';
  // Files dropped onto any view import through the same flow as Import song.
  let dropPreview=$state<DropPreview|null>(null);
  const fileDrop=new FileDropTarget(()=>!commandsBlocked()&&actionAvailable('open_song'),preview=>{dropPreview=preview;},
    path=>{void runAction('open_song',{path});},reason=>{error=reason;});
  function update(change: Partial<PracticeDocument>,throwOnError=false): Promise<void> {
    if (!project) return Promise.resolve();
    project = { ...project, ...change }; const document = $state.snapshot(project);
    const thisRevision = ++revision; saved = false;
    saves = saves.catch(() => {}).then(async () => {
      try {
        const snapshot = await bridge.update(document); apply(snapshot, thisRevision);
        if (thisRevision === revision) saved = true;
      } catch (e) { if(throwOnError)throw e;error = `Changes could not be saved: ${String(e)}`; }
    });
    return saves;
  }
  let transportActions=Promise.resolve();
  function action(value:Action):Promise<void>{const reference=song?.reference_token;transportActions=transportActions.catch(()=>{}).then(async()=>{if(song?.reference_token===reference)await performAction(value);});return transportActions;}
  async function performAction(action: Action) {
    if (!song || busy || separationActive) return;
    const referenceToken=song.reference_token;
    const preparingStems=action.type==='retry_audio'&&!!stemsState.snapshot?.set_id;
    if(preparingStems){busy='Preparing stem playback…';stemProgress=null;}
    try {
      flushControls(); await saves.catch(()=>{}); const actionRevision = revision;
      if(preparingStems)await stemsSession.flush();
      if(action.type==='toggle_play'&&!transport.playing){await stemsSession.flush();await countInSession.flush();}
      const snapshot=await bridge.action(action,referenceToken);apply(snapshot,actionRevision);
      if(preparingStems&&snapshot.transport.reference_token===song?.reference_token)await reloadStems(song.reference_token);
      // Explicit navigation reveals the resulting native position even when
      // paused or continuous following is off, preserving each screen's zoom.
      if(action.type==='seek'&&snapshot.transport.reference_token===song?.reference_token){
        if(screen==='mapping')measureMap?.revealPlayhead(snapshot.transport.position);
        else view=pageForPlayhead(view,song.duration,snapshot.transport.position);
      }
    }
    catch (e) { error = String(e); }
    finally{if(preparingStems){busy='';stemProgress=null;}}
  }
  function controls(change: Partial<Controls>) {
    const next = normalized({ ...playbackControls, ...change });
    if (separationActive || !project || !next) return;
    project = { ...project, controls: next };
    revision++;
    clearTimeout(controlTimer);
    saved = false;
    controlTimer = setTimeout(() => { controlTimer = undefined; void update({ controls: project!.controls }); }, 90);
  }
  async function chooseLoop(loop: Loop) { if(separationActive)return;await update({ loop_selection: loop, loop_enabled: true }); }
  function clearLoop() { return update({ loop_selection: null, loop_enabled: false }); }
  // Loop is a plain on/off toggle; count-in settings are independent of it.
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
    if(separationActive)return;
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
    dialogError = ''; sectionDialog.showModal(); sectionOpen = true; requestAnimationFrame(() => nameInput?.focus());
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
  async function setBoundary(edge: 'start' | 'end') {
    await transportActions;
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
      if (mounted) { keymap = loaded.model; keymapPath = loaded.path; keymapWarning = loaded.warning; keymapSource = loaded.source; void installMenu(loaded.model); }
    } finally { keymapLoading = false; }
  }
  // macOS only: Windows and Linux have no menu bar (see the native-ui skill).
  let appMenu = $state.raw<AppMenu | null>(null);
  let menuGeneration = 0;
  function menuCommand(id: ActionId) { if (!commandsBlocked()) void runAction(id); }
  /** Says why Settings can't open when ⌘, is pressed during background work. */
  function explainBlocked(id: ActionId) { if (id === 'show_settings' && !settingsOpen && settingsBlocked) settingsInfo = settingsBlocked; }
  function appEdit(command: EditCommand) {
    const id = appEditAction(screen, command);
    if (id && !commandsBlocked()) void runAction(id);
  }
  async function installMenu(model: Keymap) {
    if (!native || !mac) return;
    const generation = ++menuGeneration;
    try {
      const menu = await AppMenu.build(model, { run: menuCommand, edit: command => routeEditCommand(command, document, appEdit) });
      // Only the latest build becomes the app menu; superseded builds are released.
      if (generation !== menuGeneration || !mounted) { void menu.close(); return; }
      await menu.activate();
      const previous = appMenu; appMenu = menu; void previous?.close();
    } catch (e) { if (mounted) error = `Could not build the menu bar: ${String(e)}`; }
  }
  // A string, so the effect below runs only when enabled state actually changes.
  const menuEnabled = $derived(appMenu ? appMenu.actions.filter(id => !commandsBlocked() && actionAvailable(id)).join(' ') : '');
  $effect(() => { appMenu?.sync(new Set(menuEnabled.split(' ').filter(Boolean) as ActionId[])); });
  async function commitMap(value: MapPreview, songKey:string) {
    if(busy||song?.reference_token!==songKey)throw new Error('The song changed. Please reopen the measure map.');
    flushControls(); await saves.catch(()=>{});
    if(busy||song?.reference_token!==songKey||!project)throw new Error('The song changed. Please try again.');
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
    const target=adjacentMeasure(measures,frame,direction);
    if(target){if(screen==='mapping')measureMap?.selectMeasure(target.id);return action({type:'seek',seconds:target.start/song.rate});}
  }
  const mappingHandlers = Object.fromEntries(Object.keys(MAPPING_ACTIONS).map(id=>[id,()=>measureMap?.command(id as MappingActionId)])) as Record<MappingActionId,()=>Promise<void>>;
  // Notes that can't be saved block leaving the song. The drawer opens to show
  // why and offer Retry, and a blocked Library or quit brings Practice forward.
  function showNotesFailure(){if(song&&screen!=='library')screen='practice';}
  // Every song change flushes notes first (prepareChange, recordingChange) and
  // the text is read-only while busy, so nothing should be pending here. The
  // engine has already switched songs and rejects stale saves, so pending text
  // can't be saved any more; say so instead of dropping it silently.
  function resetNotes(key:string,text:string){
    if(notesState.dirty)error='Notes typed while the song was changing could not be saved.';
    notesSession.reset(key,text);
  }
  // Closing saves first; a failed save keeps the drawer open with its alert.
  async function closeNotes(){const reference=song?.reference_token;if(await notesSession.flush()&&song?.reference_token===reference)notesOpen=false;}
  // From another view, Notes opens Practice with the drawer.
  async function toggleNotes(){
    if(!await leaveLibrary())return;
    if(screen!=='practice'){screen='practice';if(notesOpen){await tick();notesPanel?.focusText();}else notesOpen=true;}
    else if(notesOpen)await closeNotes();else notesOpen=true;
  }
  /** Every way out of the Library saves the song details first; a failed save stays. */
  async function leaveLibrary(){return screen!=='library'||!libraryScreen||await libraryScreen.flush();}
  async function showView(next:'practice'|'mapping'|'stems'){if(!await leaveLibrary())return;screen=next;if(next==='stems'&&song&&native)refreshEstimate(song.reference_token);}
  async function showLibrary(){
    await transportActions;
    if(!await notesSession.flush()){showNotesFailure();return;}
    if(transport.playing)await action({type:'pause'});
    screen='library';
  }
  const dispatcher = new ActionDispatcher({
    separation_install:()=>{ownDownload('stems');attachment=null;return separationSession.install();},separation_start:()=>{attachment=null;return startSeparation();},
    separation_cancel:()=>{const s=separationState.status;return separationSession.cancel(s?.job.running?s.job.id:s?.install.id??'');},
    separation_reinstall_model:()=>{ownDownload('stems');return separationSession.reinstallModel();},separation_reinstall_runtime:()=>{ownDownload('stems');return separationSession.reinstallRuntime();},separation_remove_runtime:()=>separationSession.removeRuntime(),
    separation_remove_model:()=>separationSession.remove(),separation_refresh_runtime:()=>separationSession.runtime(),
    separation_open:()=>showView('stems'),
    retry_stem_mix:()=>stemsSession.retry(),
    show_stems:()=>showView('stems'),close_stems:()=>{screen=song?'practice':'library';},attach_stems:()=>stemsWork(),remove_stems:()=>stemsWork(true),
    stem_gain:({role,gain})=>{if(role&&Number.isFinite(gain))stemsSession.change(role,{gain});},
    solo_bass:()=>soloRole('bass'),
    solo_drums:()=>soloRole('drums'),
    solo_guitar:()=>soloRole('guitar'),
    solo_piano:()=>soloRole('piano'),
    solo_vocals:()=>soloRole('vocals'),
    solo_other:()=>soloRole('other'),
    mute_bass:()=>muteRole('bass'),mute_drums:()=>muteRole('drums'),mute_guitar:()=>muteRole('guitar'),mute_piano:()=>muteRole('piano'),mute_vocals:()=>muteRole('vocals'),mute_other:()=>muteRole('other'),
    import_separate:()=>importSong(),replace_recording:showReplacement,replacement_same_timing:()=>recordingChange(true),replacement_new_timeline:()=>recordingChange(false),replacement_cancel:()=>{replacementDialog.close();},
    show_settings:showSettings,close_settings:()=>{settingsOpen=false;},managed_close:()=>{managed.close();},
    reveal_library:()=>reveal('library'),reveal_keymap:()=>reveal('shortcut_file'),open_keymap:()=>bridge.openShortcutFile(),
    // Outside Settings (the Measure map), a refused or failed start shows as a toast.
    map_install_model:async()=>{ownDownload('beats');await beatModel.install();if(!settingsOpen&&beatState.error)error=beatState.error;},map_cancel_model:()=>beatModel.cancel(),map_remove_model:()=>beatModel.remove(),
    managed_create:()=>chooseLibrary(true),managed_open:()=>chooseLibrary(false),managed_locate:()=>chooseLibrary(false,true),
    managed_retry:()=>managedWork('Retrying library…',async()=>{if(!await prepareChange())return;const snapshot=await bridge.retryLibrary();if(mounted)unloaded(snapshot);}),
    managed_continue:()=>managedWork('Importing song…',async()=>{loaded(await managed.continueImport());}),
    managed_duplicate_import:()=>managedWork('Importing song…',async()=>{loaded(await managed.importSelected());}),
    managed_duplicate_open:({songKey})=>managedWork('Opening song…',async()=>{if(songKey)loaded(await managed.openDuplicate(songKey));}),
    managed_delete:({songKey})=>{if(songKey)managed.delete(songKey,libraryScreen?.titleFor(songKey)??'this song');},
    managed_confirm_delete:()=>managedWork('Deleting song…',async()=>{if(!await prepareChange())return;const snapshot=await managed.confirmDelete();if(snapshot&&mounted){if(!snapshot.transport.song_key)unloaded(snapshot);else{apply(snapshot);libraryEpoch++;screen='library';}}}),
    show_notes:toggleNotes,close_notes:closeNotes,retry_notes:async()=>{await notesSession.flush();},
    library_replace:({songKey})=>replaceLibrary(songKey),
    show_library:showLibrary,library_close:()=>showView('practice'),library_open:({songKey})=>{if(songKey)return openLibrarySong(songKey);},
    library_edit:()=>libraryScreen?.command('library_edit'),library_save:()=>libraryScreen?.command('library_save'),
    library_find:()=>libraryScreen?.command('library_find'),library_clear_search:()=>libraryScreen?.command('library_clear_search'),
    library_cancel:()=>libraryScreen?.command('library_cancel'),library_retry:()=>libraryScreen?.command('library_retry'),
    library_sort:payload=>libraryScreen?.command('library_sort',payload),library_reverse:()=>libraryScreen?.command('library_reverse'),
    ...mappingHandlers,
    show_measure_map:()=>showView('mapping'), show_practice:()=>showView('practice'),
    previous_measure:()=>nextMeasure(-1), next_measure:()=>nextMeasure(1),
    goto_measure:()=>{const measure=mapView?.measures.find(m=>m.number===measureNumber);if(measure&&song){if(screen==='mapping')measureMap?.selectMeasure(measure.id);return action({type:'seek',seconds:measure.start/song.rate});}},
    practice_count_in:()=>{countInEnabled=!countInActive;}, repeat_count_in:()=>{repeatCountIn=!repeatCountIn;},
    toggle_sections:()=>practiceScreen?.command('toggle_sections'), show_pitch:()=>practiceScreen?.command('show_pitch'),
    show_goto_measure:()=>practiceScreen?.command('show_goto_measure'), show_passage_bounds:()=>practiceScreen?.command('show_passage_bounds'),
    practice_metronome:()=>{clickEnabled=!clickEnabled;}, toggle_loop_snap:()=>{snapToMeasures=!snapToMeasures;},
    play_pause: () => action({ type: 'toggle_play' }), toggle_loop: toggleLoop,
    set_loop_start: () => setBoundary('start'), set_loop_end: () => setBoundary('end'),
    jump_loop_start: () => action({ type: 'seek', seconds: selection!.start }),
    jump_loop_end: () => action({ type: 'seek', seconds: selection!.end }),
    jump_song_start: () => action({ type: 'seek', seconds: song && mapView?.measures.length ? mapView.measures[0].start / song.rate : 0 }), clear_loop: clearLoop,
    seek_backward: () => mapView?.measures.length ? nextMeasure(-1) : seekBy(-5),
    seek_forward: () => mapView?.measures.length ? nextMeasure(1) : seekBy(5),
    seek_backward_fine: () => seekBy(-.1), seek_forward_fine: () => seekBy(.1),
    previous_section: () => nextSection(-1), next_section: () => nextSection(1),
    add_section: () => startSection(), open_song: async ({path}) => { await showLibrary(); if(screen==='library')await importSong(path); },
    disable_loop: () => update({ loop_enabled: false }), show_shortcuts: () => { helpDialog.showModal(); helpOpen = true; },
    reload_keymap: reloadKeymap, fit_loop: fitLoop,
    zoom_in: () => { view = zoom(view, song!.duration, .5, selection ? (selection.start + selection.end) / 2 : transport.position); },
    zoom_out: () => { view = zoom(view, song!.duration, 2, (view.start + view.end) / 2); },
    show_whole_song: () => { view = { start: 0, end: song!.duration }; },
    reset_speed: () => controls({ speed: 1 }), reset_pitch: () => controls({ semitones: 0 }), reset_tuning: () => controls({ tuning_cents: 0 }),
    speed_down_10: () => controls({ speed: stepSpeed(speed, -10) }), speed_down_5: () => controls({ speed: stepSpeed(speed, -5) }),
    speed_up_5: () => controls({ speed: stepSpeed(speed, 5) }), speed_up_10: () => controls({ speed: stepSpeed(speed, 10) }),
    transpose_down: () => controls({ semitones: transpose(playbackControls.semitones, -1) }), transpose_up: () => controls({ semitones: transpose(playbackControls.semitones, 1) }),
    retry_audio: () => action({ type: 'retry_audio' }), retry_save: () => update({}),
    undo_delete: undoDelete, dismiss_error: () => { error = ''; },
    select_section: ({ section }) => selectSection(section!), edit_section: ({ section }) => startSection(section),
    delete_section: ({ section }) => removeSection(section!.id),
    save_section: saveSection, close_editor: () => sectionDialog.close(), close_shortcuts: () => helpDialog.close()
  }, () => ({ context: screen, busy: !!busy||stemsLoading, song: !!song, range: !!selection, sections: !!sections.length,
    audio: !!song && native && !transport.audio_error, loop_on: !!project?.loop_enabled,
    undo: !!undo.length, editor: sectionOpen, help: helpOpen,
    zoom_out: !!song && view.end - view.start < song.duration }));

  async function runAction(id: ActionId, payload: ActionPayload = {}) {
    try { if (actionAvailable(id)) await dispatcher.run(id, payload); } catch (e) { error = String(e); }
  }
  function runButton(id: ButtonId, payload: ActionPayload = {}) { return runAction(keymap.actionFor(id), payload); }
  // Moving between screens and the global dialogs work on every screen.
  const NAVIGATION_ACTIONS:ActionId[]=['show_settings','close_settings','show_library','show_practice','show_measure_map','show_notes','close_notes','retry_notes','show_stems','show_shortcuts','close_shortcuts','reload_keymap','dismiss_error'];
  function actionAvailable(id:ActionId) {
    if(!dispatcher.available(id))return false;
    if(separationActive&&separationFrozen(id))return false;
    if(id==='show_settings')return settingsOpen||!settingsBlocked;
    if(id==='close_settings')return settingsOpen;
    if(id==='reveal_library')return native&&settingsOpen&&!!managedState.status.selected&&managedState.status.available;
    if(id==='open_keymap')return native&&!!keymapPath&&keymapSource!=='builtin';
    if(id==='reveal_keymap')return native&&!!keymapPath;
    if(id==='map_install_model')return native&&!!beatState.status&&!beatState.status.running&&!beatState.pending;
    if(id==='map_cancel_model')return !!beatState.status?.running&&beatState.status.kind==='install'&&!beatState.pending;
    if(id==='map_remove_model')return native&&!!beatState.status?.model_ready&&!beatState.status.running&&!beatState.pending;
    if(id.startsWith('separation_')){const s=separationState.status;const occupied=!!s?.install.running||!!s?.job.running;if(id==='separation_cancel')return !separationState.pending&&!!(s?.job.running?s.job.cancellable:s?.install.cancellable);if(id==='separation_install'||id==='separation_reinstall_runtime'||id==='separation_reinstall_model')return native&&!!s&&s.runtime.supported!==false&&!occupied&&!separationState.pending;if(id==='separation_remove_runtime')return native&&!!s?.runtime.path&&!occupied&&!separationState.pending;if(id==='separation_start')return native&&!!song?.path&&!!s?.model_ready&&!!s.runtime.ready&&!occupied&&!s.heavy_work&&!separationState.pending;if(id==='separation_remove_model')return !!s?.model_ready&&!occupied&&!separationState.pending;if(id==='separation_refresh_runtime')return native&&!occupied&&!separationState.pending;if(id==='separation_open')return native&&(occupied||!!song);return !!song;}
    if(id==='retry_stem_mix')return !!stemsState.error&&!!stemsState.snapshot?.set_id;
    if(id==='stem_gain')return !!stemsState.snapshot?.available&&!transport.audio_error;
    if(id.startsWith('mute_')||id.startsWith('solo_'))return !!stemsState.snapshot?.available&&!transport.audio_error&&stemsState.snapshot.tracks.some(track=>track.role===id.slice(5));
    if(id==='show_stems'||id==='attach_stems')return native&&!!song?.path;
    if(id==='remove_stems')return native&&!!stemsState.snapshot?.set_id;
    if(screen==='stems'&&!['retry_stem_mix','close_stems','attach_stems','remove_stems',...NAVIGATION_ACTIONS].includes(id))return false;
    if(id==='show_notes')return screen!=='library';
    if(id==='close_notes')return notesOpen&&screen==='practice';
    if(id==='retry_notes')return !!notesState.error&&!notesState.pending;
    if(id.startsWith('managed_')){
      if(id==='managed_close')return !!managedState.dialog;
      if(id==='managed_confirm_delete')return managedState.dialog==='delete';
      if(id==='managed_continue')return managedState.dialog==='notice';
      if(id==='managed_duplicate_import'||id==='managed_duplicate_open')return managedState.dialog==='duplicates';
      if(id==='managed_delete')return screen==='library'&&(libraryScreen?.canRun(id)??false);
      if(id==='managed_create')return managedState.dialog==='setup'||settingsOpen;
      if(id==='managed_open')return managedState.dialog==='setup'||settingsOpen||(screen==='library'&&!managedState.status.available);
      return (screen==='library'||settingsOpen)&&!!managedState.status.selected&&!managedState.status.available;
    }
    if(id.startsWith('library_')&&id!=='library_close')return libraryScreen?.canRun(id)??false;
    // Settings → Audio offers Retry audio over the Library too.
    if(screen==='library'&&!(settingsOpen&&id==='retry_audio')&&!['library_close','open_song','import_separate','replacement_same_timing','replacement_new_timeline','replacement_cancel',...NAVIGATION_ACTIONS].includes(id))return false;
    if(id==='practice_count_in')return hasMeasureMap&&native&&(!transport.audio_error||countInActive);
    if(id==='repeat_count_in')return hasMeasureMap&&native&&!transport.audio_error;
    if(id==='practice_metronome')return native&&!!audibleMap?.measures.length&&(clickEnabled||!transport.audio_error);
    if(id==='toggle_loop_snap')return !!mapView?.measures.length;
    if(id in MAPPING_ACTIONS)return !!measureMap?.canRun(id as MappingActionId);
    if(['previous_measure','next_measure','goto_measure','show_goto_measure'].includes(id))return !!mapView?.measures.length;
    return true;
  }
  function buttonAvailable(id: ButtonId) { return actionAvailable(keymap.actionFor(id)); }
  // Modal states own the keyboard and the menu bar's commands.
  function commandsBlocked() { return quitConfirmation || !!stemsScreen?.confirming() || !!measureMap?.modal() || !!busy || sectionOpen || helpOpen || settingsOpen || !!replacementDialog?.isOpen() || !!managedState.dialog; }
  // Single routing point: key owners keep their own keys, everything else goes
  // through the keymap to the dispatcher. Esc first unwinds focus to home.
  function keyboard(event: KeyboardEvent) {
    if (event.defaultPrevented || ownsKey(event.target, event.key, currentModality())) return;
    // ⌘, explains a block even while busy work blocks every other command.
    if (keymap.match(event, screen) === 'show_settings' && !actionAvailable('show_settings')) { event.preventDefault(); explainBlocked('show_settings'); return; }
    if (commandsBlocked()) return;
    if (event.key === 'Escape' && closeTopTransient()) { event.preventDefault(); return; }
    if (event.key === 'Escape' && !atHome()) { event.preventDefault(); focusHome(); return; }
    const id = keymap.match(event,screen);
    // Undo/Redo outside text entries never fall through to WebKit's document
    // undo stack, which could silently undo an unfocused field.
    if (!id || !actionAvailable(id)) { if (editCommand(event, mac)) event.preventDefault(); return; }
    event.preventDefault(); void runAction(id);
  }
  // The Library header has no song views (library.md); the current song's row returns to Practice.
  const viewSegments=$derived<ViewSegment[]>(song&&screen!=='library'?[
    {button:'show_practice',label:'Practice',current:screen==='practice'},
    {button:'show_measure_map',label:'Measure map',current:screen==='mapping'},
    {button:'show_stems',label:'Stems',current:screen==='stems'}
  ]:[]);
  // The persistent separation toast on the other views. Opening Stems marks
  // the latest outcome seen, so its done or failed toast goes away.
  let seenOutcome=$state('');
  $effect(()=>{if(screen==='stems'){const finished=finishedAttempt(separationState.status,song?.reference_token??'');if(finished)seenOutcome=finished.attempt.id;}});
  const processing=$derived(screen==='stems'?null:separationToast(separationState.status,song?.reference_token??'',separationActive,seenOutcome));
  const busyText=$derived(screen==='stems'&&attachment?.running?'':busy&&stemProgress?`${processingLabel(stemProgress)}…`:busy||(stemsLoading?'Preparing stem playback…':''));
  const noticeList=$derived(notices({
    error:managedState.dialog||settingsOpen?'':error,info:settingsInfo,startupNotice,busy:busyText,preview:!native,undoCount:undo.length,
    audioError:song?transport.audio_error:null,audioRetry:buttonAvailable('retry_audio'),saveError:transport.save_error,keymapWarning,keymapLoading,
    stemMixError:stemsState.error,stemMixRetry:buttonAvailable('retry_stem_mix'),stemWarning:screen==='stems'?null:stemsState.snapshot?.warning??null,stemsUnavailable:!stemsState.snapshot?.available,
    countInError:countInState.error,clickError:clickState.error,processing,mapNotice:screen==='mapping'?mapNotice:null
  }));
  onMount(() => {
    mounted = true; busy = 'Restoring practice…';
    if(native)separationSession.start();

    const restore = bridge.initialize().then(snapshot => { if (mounted) apply(snapshot); }).catch(e => { if (mounted) error = String(e); });
    Promise.all([restore, reloadKeymap()]).finally(() => { if (mounted) busy = ''; });
    let unlistenStems:(()=>void)|undefined;
    void bridge.onStemProgress(progress=>{if(!mounted)return;if(progress.operation_id){if(attachment?.running&&progress.operation_id===attachment.id)attachment={...attachment,progress};return;}if(!!busy&&busy!=='Choosing stems…'&&busy!=='Removing stems…'&&(progress.reference_token===song?.reference_token||busy.startsWith('Opening')||busy.startsWith('Restoring')||busy.startsWith('Import'))){stemProgress=progress;}}).then(unlisten=>{if(mounted)unlistenStems=unlisten;else unlisten();}).catch(e=>{if(mounted)error=String(e);});
    let unlistenProgress:(()=>void)|undefined;
    void bridge.onImportProgress(progress=>{if(mounted&&busy.startsWith('Import'))busy=progress.total&&progress.copied>=progress.total?'Loading recording…':progress.total?`Importing song… ${Math.round(progress.copied/progress.total*100)}%`:'Importing song…';}).then(unlisten=>{if(mounted)unlistenProgress=unlisten;else unlisten();}).catch(e=>{if(mounted)error=String(e);});
    void managed.refresh().catch(e=>{if(mounted)error=String(e);});
    let unlistenDrop:(()=>void)|undefined;
    void bridge.onFileDrop(event=>{if(mounted)fileDrop.handle(event);}).then(unlisten=>{if(mounted)unlistenDrop=unlisten;else unlisten();}).catch(e=>{if(mounted)error=String(e);});
    let unlistenExit:(()=>void)|undefined;
    void bridge.onExitRequested(async(nativeProcessing=false)=>{
      if(exiting||quitConfirmation)return;
      if(nativeProcessing||separationState.pendingInstall||separationState.status?.install.running||separationActive){quitConfirmation=true;return;}
      await finishExit();
    }).then(unlisten=>{if(mounted)unlistenExit=unlisten;else unlisten();}).catch(e=>{error=String(e);});
    // The beat model's state: always while Settings or the Measure map is open
    // or while it runs, and every few seconds otherwise.
    if(native)void beatModel.refresh();let beatTicks=0;
    const beatTimer=native?setInterval(()=>{beatTicks++;if(settingsOpen||screen==='mapping'||beatState.status?.running||beatTicks%6===0)void beatModel.refresh();},500):undefined;
    let polling = false;
    const timer = setInterval(async () => {
      if (!song || busy || polling) return;
      polling = true;
      try { const statusRevision=revision;const status = await bridge.status(); if (mounted && statusRevision===revision && status.reference_token === song?.reference_token) transport = status; }
      catch (e) { if (mounted) error = String(e); } finally { polling = false; }
    }, 50);
    return () => { mounted = false; clearInterval(timer); clearInterval(beatTimer); beatModel.dispose(); clearTimeout(controlTimer); unlistenExit?.();unlistenDrop?.();unlistenProgress?.();unlistenStems?.();stemsSession.dispose();separationSession.dispose();managed.dispose();notesSession.dispose();clickSession.dispose();countInSession.dispose(); };
  });
</script>

<ManagedLibraryDialog {error} busy={!!busy} kind={managedState.dialog} {keymap} candidates={managedState.selection?.candidates??[]} importName={managedState.selection?fileName(managedState.selection.path):''} songTitle={managedState.deleteTitle} run={(id,songKey)=>{void runButton(id,{songKey});}}/>
<dialog bind:this={quitDialog} class="quit-confirmation" aria-label="Quit and cancel processing" oncancel={event=>{event.preventDefault();quitConfirmation=false;}}><div class="screen-title"><button class="icon-button" aria-label="Close quit confirmation" onclick={()=>quitConfirmation=false}><Icon name="close"/></button><h2>Processing is still running</h2></div><p>Quit and cancel waits for processing to stop and cleans unfinished outputs. A save already in progress will finish.</p><div class="actions"><button class="button" onclick={()=>quitConfirmation=false}>Keep app open</button><button class="button" disabled={exiting} onclick={finishExit}>Quit and cancel</button></div></dialog>
<SettingsDialog open={settingsOpen} section={settingsSection} summary={settingsSummary} error={settingsOpen?error:''} working={busy} download={settingsDownload(separationState.status,beatState.status)}
  separation={separationState} beat={beatState} shortcuts={{showButton:shortcutsButton,path:keymapPath,warning:keymapWarning,loading:keymapLoading}} {revealLabel} saveFailed={preferenceSaveFailed}
  onsection={section=>{settingsSection=section;}} ontheme={setTheme} onnotesfont={setNotesFont} onshowbutton={setShortcutsButton}
  {keymap} available={buttonAvailable} run={id=>{void runButton(id);}}/>
<svelte:window onkeydown={keyboard}/>
<div class="app-shell">
  <AppHeader title={song?.title ?? null} inLibrary={screen==='library'} views={viewSegments} {keymap} available={buttonAvailable} run={id=>{void runButton(id);}}>
    {#if song}<span class="save-state" class:failed={!!transport.save_error||notesUnsaved(notesState)} role="status">{#if transport.save_error}Not saved{:else if notesUnsaved(notesState)}<Icon name="warning" size={13}/>Notes not saved{:else if saved}<Icon name="check" size={13}/>Saved{:else}Saving…{/if}</span>{/if}
    {#if screen==='library'}<button class="header-button" disabled={!buttonAvailable('settings')} title={settingsBlocked??keymap.title('settings')} onclick={()=>runButton('settings')}><Icon name="settings" size={15}/>{keymap.buttonLabel('settings','Settings')}</button>{/if}
    {#if song&&screen!=='library'}<button class="header-button" aria-controls={notesOpen&&screen==='practice'?'notes':undefined} aria-pressed={notesOpen&&screen==='practice'} disabled={!buttonAvailable('show_notes')} title={keymap.title('show_notes')} onclick={()=>runButton('show_notes')}><Icon name="notes" size={15}/>{keymap.buttonLabel('show_notes','Notes')}</button>{/if}
    {#if shortcutsButton||screen!=='practice'}<button class="header-button icon" aria-label={keymap.buttonLabel('shortcuts','Keyboard shortcuts')} title={keymap.title('shortcuts')} onclick={()=>runButton('shortcuts')}><Icon name="keyboard" size={16}/></button>{/if}
  </AppHeader>
  <Toasts notices={noticeList} {keymap} run={id=>{void runButton(id);}}/>

  <LibraryScreen bind:this={libraryScreen} active={screen==='library'} currentKey={song?.key??null} status={managedState.status} epoch={libraryEpoch} {keymap} available={buttonAvailable} run={(id,payload)=>{void runButton(id,payload);}} onchanged={entry=>{if(song?.key===entry.song_key)song={...song,title:entry.metadata.title};}}/>
  <div class="screen-frame" hidden={screen!=='stems'}>
    <StemsScreen bind:this={stemsScreen} active={screen==='stems'} reference={song?.reference_token??''} state={stemsState} separation={separationState} {attachment} space={separationEstimate} {keymap} available={buttonAvailable} run={(id,payload)=>{void runButton(id,payload);}}/>
  </div>
  {#if song}
    {#key song.reference_token}
      <MeasureMapScreen bind:this={measureMap} bind:measureNumber active={screen==='mapping'} {song} map={project?.measure_map??null} {mapView} {transport} {speed} loopEnabled={project?.loop_enabled??false}
        frozen={separationActive} separating={separationActive} modelDownloading={!!beatState.status?.running&&beatState.status.kind==='install'} busy={!!busy} {keymap}
        onreview={map=>{reviewMap={key:song!.reference_token,map};}} oncommit={commitMap} onseek={seconds=>{void action({type:'seek',seconds});}} onspeed={value=>controls({speed:value})}
        onnotice={notice=>{mapNotice=notice;}} run={(id,payload)=>{void runButton(id,payload);}} available={buttonAvailable}/>
    {/key}
  {/if}
  {#if dropPreview}<DropOverlay preview={dropPreview}/>{/if}
  <RecordingChooser bind:this={replacementDialog} {keymap} run={id=>{void runButton(id);}}/>

  {#if song && project}
    {#snippet notesDrawer()}<NotesDrawer bind:this={notesPanel} notes={notesState} readonly={!!busy} font={notesFont} {keymap} available={buttonAvailable} onchange={text=>notesSession.change(text)} run={id=>{void runButton(id);}}/>{/snippet}
    <PracticeScreen bind:this={practiceScreen} bind:measureNumber active={screen==='practice'} {song} controls={playbackControls} {sections} selectedSection={project.selected_section}
      {selection} looping={project.loop_enabled} repeatCountIn={repeatCountIn&&hasMeasureMap} {transport} {mapView} {view} {snapToMeasures} frozen={separationActive} busy={!!busy}
      {keymap} available={buttonAvailable} run={(id,payload)=>{void runButton(id,payload);}} onseek={seconds=>action({type:'seek',seconds})} onselection={chooseLoop}
      onview={next=>{view=next;}} oncontrols={controls} drawer={notesOpen?notesDrawer:undefined}/>
    {#if screen==='practice'}
      <PlaybackBar>
        <TransportButtons playing={transport.playing} mapped={!!mapView?.measures.length} frozen={separationActive} {keymap} available={buttonAvailable} run={id=>{void runButton(id);}}/>
        <PracticeModes countIn={countInActive} loop={project.loop_enabled} repeatCountIn={repeatCountIn&&hasMeasureMap} mapped={hasMeasureMap} {keymap} available={buttonAvailable} run={id=>{void runButton(id);}}/>
        <span class="bar-spacer"></span>
        <PracticeMixer {clickEnabled} {clickLevel} clickAvailable={buttonAvailable('practice_metronome')} hasMap={hasMeasureMap} stems={stemsState.snapshot}
          stemsDisabled={separationActive||!!busy||stemsLoading||!stemsState.snapshot?.available||!!transport.audio_error} frozen={separationActive}
          {keymap} available={buttonAvailable} run={(id,payload)=>{void runButton(id,payload);}} onclicklevel={level=>{clickLevel=level;}}/>
      </PlaybackBar>
    {/if}
  {/if}
</div>

<dialog class="section-dialog" bind:this={sectionDialog} onclose={() => { dialogError = ''; sectionOpen = false; }}>
  <form onsubmit={event => { event.preventDefault(); void runButton('save_section'); }}><div class="dialog-heading"><h2>{sectionId ? 'Edit section' : 'Save a section'}</h2><button type="button" class="icon-button" aria-label="Close section editor" title={keymap.title('close_editor')} onclick={() => runButton('close_editor')}><Icon name="close"/></button></div><p>Keep a passage you’ll want to come back to.</p><label>Section name<input bind:this={nameInput} bind:value={sectionName} maxlength="128" placeholder="Verse, solo, a tricky phrase…" required/></label><div class="dialog-times"><label>Start (seconds)<input type="number" min="0" max={song?.duration ?? 0} step="any" bind:value={sectionStart} required/></label><label>End (seconds)<input type="number" min="0" max={song?.duration ?? 0} step="any" bind:value={sectionEnd} required/></label></div>{#if dialogError}<p class="form-error" role="alert">{dialogError}</p>{/if}<div class="dialog-actions"><button type="button" class="button" title={keymap.title('cancel_editor')} onclick={() => runButton('cancel_editor')}>Cancel</button><button class="button primary" type="submit" title={keymap.title('save_section')}>Save section</button></div></form>
</dialog>
<dialog class="help-dialog" bind:this={helpDialog} onclose={() => { helpOpen = false; }}>
  <div class="dialog-heading"><h2>Keep your hands on the guitar</h2><button class="icon-button" aria-label="Close shortcuts" title={keymap.title('close_shortcuts')} onclick={() => runButton('close_shortcuts')}><Icon name="close"/></button></div>
  <p>Shortcuts work when you’re not editing a field.</p>
  <dl>{#each shortcuts as shortcut (shortcut.action)}<div><dt>{shortcut.keys}</dt><dd>{shortcut.description}</dd></div>{/each}</dl>
  <button class="text-button settings-link" disabled={!buttonAvailable('settings')} title={settingsBlocked??keymap.title('settings')} onclick={() => runButton('settings', { settingsSection: 'shortcuts' })}>Change shortcuts: Settings › Shortcuts</button>
</dialog>

<style>
.app-shell{position:relative;height:100vh;display:flex;flex-direction:column;min-height:0}
.screen-frame{flex:1;min-height:0;display:flex;flex-direction:column;padding:16px 24px 20px}
.screen-frame[hidden]{display:none}
.bar-spacer{flex:1}
.save-state{display:flex;align-items:center;gap:4px;color:var(--success-text);white-space:nowrap}
.save-state.failed{color:var(--error-text)}
.header-button{height:32px;display:flex;align-items:center;gap:7px;padding:0 10px;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text-secondary);font-size:12px}
.header-button.icon{width:32px;justify-content:center;padding:0}
.header-button[aria-pressed=true]{border-color:var(--border-strong);background:var(--selected)}
.header-button:hover:not(:disabled){background:var(--hover);color:var(--text)}
.quit-confirmation{background:var(--surface);border:1px solid var(--border);border-radius:9px;max-width:520px;padding:24px}.quit-confirmation[open]{display:grid;gap:20px}.quit-confirmation::backdrop{background:#0008}.quit-confirmation p{line-height:1.5}.quit-confirmation .actions{display:flex;gap:12px}
</style>
