<script lang="ts">
  // The Measure map screen (concept MB, docs/design/measure-map.md): a compact
  // readout, the waveform with a floating toolbar at the selected measure, the
  // view bar below it, the drafting drawer on the right while the map is empty
  // (or while redrafting), and the playback bar. The metronome always clicks
  // the displayed map or draft at full level (App owns it). This component
  // owns the editing selection, the view and the panels; MappingSession runs
  // edits and history, AnalysisSession the beat analysis, and Rust validates
  // every edit. Commands arrive through App's dispatcher (canRun/command).
  import { untrack, onDestroy } from 'svelte';
  import { AnalysisSession, type AnalysisState } from './analysis-session';
  import { MappingSession, type MappingState } from './mapping-session';
  import { draftInputs, type DraftInputs } from './draft-steps';
  import { joinCheck, measureName, shiftCheck, splitCheck, splitTarget, validEpsilon, validShiftStep } from './measure-tools';
  import { unmappedRanges } from './map-ranges';
  import { fitRange, followPage, pageForPlayhead, revealRange } from './timeline';
  import { time } from './model';
  import { measureAtFrame, type MapEdit, type MapPreview, type MapView, type MeasureMap, type Meter } from './timing';
  import Icon from './Icon.svelte';
  import MapReadout from './MapReadout.svelte';
  import MapWaveform from './MapWaveform.svelte';
  import MapToolbar, { type Panel } from './MapToolbar.svelte';
  import MeasureProperties from './MeasureProperties.svelte';
  import MeasureExpert from './MeasureExpert.svelte';
  import FreeTimeCount from './FreeTimeCount.svelte';
  import MapViewBar from './MapViewBar.svelte';
  import DraftDrawer from './DraftDrawer.svelte';
  import SongMapDialog from './SongMapDialog.svelte';
  import PlaybackBar from './PlaybackBar.svelte';
  import TransportButtons from './TransportButtons.svelte';
  import SpeedControl from './SpeedControl.svelte';
  import type { Song, Transport, View } from './types';
  import type { Keymap } from './keymap';
  import type { ButtonId, MappingActionId } from './actions';
  import type { ActionPayload } from './action-dispatcher';
  export type MapNotice = { id: number; text: string };
  let { song, map, mapView, transport, speed, loopEnabled, keymap, active, frozen = false, separating = false, modelDownloading = false, busy = false,
    measureNumber = $bindable(1), onreview, oncommit, onseek, onspeed, onnotice, run, available }:
    { song: Song; map: MeasureMap | null; mapView: MapView | null; transport: Transport; speed: number; loopEnabled: boolean; keymap: Keymap; active: boolean;
      frozen?: boolean; separating?: boolean; modelDownloading?: boolean; busy?: boolean; measureNumber?: number;
      onreview: (map: MeasureMap | null) => void; oncommit: (value: MapPreview, songKey: string) => Promise<void>; onseek: (seconds: number) => void;
      onspeed: (speed: number) => void; onnotice: (notice: MapNotice | null) => void;
      run: (id: ButtonId, payload?: ActionPayload) => void; available: (id: ButtonId) => boolean } = $props();

  // Sessions: edits and their history, and beat analysis.
  let editState = $state<MappingState>({ pending: false, error: '', preview: null, undo: 0, redo: 0 });
  let analysis = $state<AnalysisState>({ status: null, candidates: null, requesting: false, error: '' });
  const session = new MappingSession(() => ({ key: song.reference_token, map: map ? $state.snapshot(map) : null }), value => oncommit(value, song.reference_token), next => { editState = next; });
  const analyzer = new AnalysisSession(() => ({ key: song.reference_token, revision: map?.revision ?? 0 }), value => { analysis = value; });
  onDestroy(() => { analyzer.dispose(); session.dispose(); });
  $effect(() => {
    if (!active) return;
    void analyzer.refresh();
    const timer = setInterval(() => { if (analysis.status?.running || analysis.requesting || modelDownloading) void analyzer.refresh(); }, 400);
    return () => clearInterval(timer);
  });
  // A model download (owned by App) starting or ending changes what drafting offers.
  $effect(() => { void modelDownloading; if (active) void untrack(() => analyzer.refresh()); });

  const candidates = $derived(analysis.candidates?.reference_token === song.reference_token ? analysis.candidates : null);
  const preview = $derived(editState.preview);
  const displayed = $derived(preview?.view ?? mapView);
  const measures = $derived(displayed?.measures ?? []);
  const hasMap = $derived(!!map?.measures.length);
  const audibleMap = $derived(preview?.map ?? map);
  $effect(() => { if (active) onreview(audibleMap ? $state.snapshot(audibleMap) : null); });

  // Session settings that survive closing panels and switching screens.
  let inputs = $state<DraftInputs>(draftInputs(untrack(() => song.duration)));
  // The inputs the shown draft was previewed with, so Apply survives leaving the screen.
  let previewed = $state('');
  // Who made the shown preview: the drafting drawer, or Song settings (numbering, grouping).
  let previewOrigin = $state<'draft' | 'song'>('draft');
  let epsilonMs = $state(30), step = $state(1), resnap = $state(true);
  let detectedVisibility = $state<boolean | null>(null);
  const showDetected = $derived(detectedVisibility ?? !measures.length);
  // New suggestions switch drafting to them and start at their first downbeat.
  let seenResult: number | null = null;
  $effect(() => {
    const result = analysis.status?.result?.reference_token === song.reference_token ? analysis.status.id : 0;
    if (!candidates || result === seenResult) return;
    seenResult = result;
    untrack(() => { inputs.source = 'detected'; const first = candidates.beats.find(b => b.downbeat) ?? candidates.beats[0]; if (first) inputs.first = time(first.frame / song.rate, 3); });
  });

  // The view: always follows the playhead while playing (decision 4).
  let view = $state<View>(untrack(() => ({ start: 0, end: song.duration })));
  let followed = { position: 0, playing: false };
  $effect(() => {
    const { position, playing } = transport, previous = followed;
    followed = { position, playing };
    if (active) untrack(() => { view = followPage(view, song.duration, previous, { position, playing }); });
  });
  /** Explicit seeks reveal the playhead even while paused. */
  export function revealPlayhead(position: number) { view = pageForPlayhead(view, song.duration, position); }

  // Selection: a measure, or a free-time range (by a frame inside it).
  let selected = $state<string | null>(untrack(() => (measureAtFrame(mapView?.measures ?? [], Math.round(transport.position * song.rate)) ?? mapView?.measures[0])?.id ?? null));
  let freeFrame = $state<number | null>(null);
  const measure = $derived(measures.find(m => m.id === selected) ?? null);
  const freeRange = $derived(freeFrame === null || measure ? null : unmappedRanges(measures, Math.round(song.duration * song.rate)).find(r => r.start <= freeFrame! && freeFrame! < r.end) ?? null);
  let lastStart = 0;
  $effect(() => { if (measure) { const start = measure.start; untrack(() => { lastStart = start; }); } });
  // An edit that removes the selected measure (shift, join, split) keeps the selection where it was.
  $effect(() => {
    const current = measures;
    untrack(() => { if (selected !== null && !current.some(m => m.id === selected)) selected = (measureAtFrame(current, lastStart) ?? current[0])?.id ?? null; });
  });
  // Playback selects the measure it plays, or the free time it plays through.
  $effect(() => {
    if (!active || !transport.playing) return;
    const frame = Math.round(transport.position * song.rate), playing = measureAtFrame(measures, frame);
    untrack(() => { if (playing) { selected = playing.id; freeFrame = null; } else { selected = null; freeFrame = frame; } });
  });
  function select(id: string | null) { selected = id; freeFrame = null; formError = ''; }
  export function selectMeasure(id: string) {
    select(id);
    const m = measures.find(m => m.id === id);
    if (m) view = revealRange(view, song.duration, m.start / song.rate, m.end / song.rate);
  }

  // Panels, the drawer and the dialog.
  let panel = $state<Panel>(null), selectedBeat = $state<number | null>(null);
  let redrafting = $state(false), dialogOpen = $state(false), dragging = $state(false), splitHover = $state(false);
  $effect(() => { void selected; void freeRange?.start; untrack(() => { panel = null; selectedBeat = null; }); });
  const showDrawer = $derived(!hasMap || redrafting);
  /** A modal dialog of this screen is open, so App blocks shortcuts. */
  export function modal() { return dialogOpen; }

  let propertiesForm = $state<MeasureProperties>(), expertForm = $state<MeasureExpert>(), counter = $state<FreeTimeCount>();
  let drawer = $state<DraftDrawer>(), songDialog = $state<SongMapDialog>();
  let formError = $state(''), errorAt = $state<'panel' | 'dialog' | 'drawer' | 'wave'>('wave');
  const error = $derived(formError || editState.error);
  const frame = $derived(Math.round(transport.position * song.rate));
  const split = $derived(measure ? splitCheck(measure, frame) : { ok: false as const, reason: '' });
  const join = $derived(measure ? joinCheck(displayed, measure) : { ok: false as const, reason: '' });
  const shift = $derived(measure ? shiftCheck(displayed, measure) : { ok: false as const, reason: '' });
  const splitAt = $derived(splitHover && measure && split.ok ? splitTarget(measure, frame)?.frame ?? null : null);
  const useResnap = $derived(resnap && !!candidates);
  const stale = $derived(!!candidates && hasMap && candidates.map_revision !== (map?.revision ?? 0));
  let removedMeter: Meter | null = null;
  let notices = 0;

  const TOOLBAR: readonly MappingActionId[] = ['map_split', 'map_merge', 'map_shift_left', 'map_shift_right', 'map_protect', 'map_properties', 'map_more', 'map_notation', 'map_boundary', 'map_beat', 'map_remove_beat', 'map_anchor_playhead', 'map_remove', 'map_reclaim'];
  const DRAFT: readonly MappingActionId[] = ['map_build', 'map_use_analysis', 'map_span_start', 'map_span_end', 'map_first_downbeat'];
  const anchorTarget = () => {
    if (!measure || frame < measure.start || frame >= measure.end) return null;
    const beats = measure.grid.filter(p => p.quarter > 0);
    return beats.find(p => p.quarter === selectedBeat) ?? beats.reduce<typeof beats[number] | null>((best, p) => !best || Math.abs(p.frame - frame) < Math.abs(best.frame - frame) ? p : best, null);
  };
  const chosenBeat = () => measure?.grid.find(p => p.quarter > 0 && p.quarter === selectedBeat) ?? null;

  export function canRun(id: MappingActionId): boolean {
    if (id === 'map_show_detected') return !!candidates;
    if (id === 'map_whole') return true;
    if (id === 'map_fit') return !!measure || !!freeRange;
    if (id === 'map_respace') return false;
    if (editState.pending) return false;
    if (id === 'map_undo') return editState.undo > 0;
    if (id === 'map_redo') return editState.redo > 0;
    if (id === 'map_apply') return !!preview && !(previewOrigin === 'draft' && (drawer?.isStale() ?? false));
    if (id === 'map_discard') return !!preview;
    if (id === 'map_clear') return hasMap || !!preview;
    if (id === 'map_redraft') return hasMap && !preview;
    if (id === 'map_song_settings') return hasMap && !preview;
    if (id === 'map_origin') return hasMap && !preview && dialogOpen;
    if (id === 'map_grouping') return hasMap && !preview && dialogOpen && (songDialog?.canRun(id) ?? false);
    if (id === 'map_analyze') return !!analysis.status?.model_ready && !analysis.status.running && !analysis.requesting && !preview;
    if (id === 'map_cancel_analysis') return !!analysis.status?.running && analysis.status.kind !== 'install' && !analysis.requesting;
    if (DRAFT.includes(id)) return showDrawer && (drawer?.canRun(id) ?? false);
    if (id === 'map_shift_resnap') return !!candidates;
    if (!TOOLBAR.includes(id) || preview) return false;
    if (id === 'map_properties') return !!measure || !!freeRange;
    if (id === 'map_reclaim') return !!freeRange && panel === 'properties';
    if (!measure) return false;
    switch (id) {
      case 'map_split': return split.ok;
      case 'map_merge': return join.ok;
      case 'map_shift_left': case 'map_shift_right': return shift.ok && validShiftStep(step) && (!useResnap || validEpsilon(epsilonMs));
      case 'map_notation': return panel === 'properties' && !measure.protected;
      case 'map_boundary': return panel === 'more';
      case 'map_beat': return panel === 'more' && !measure.protected && (expertForm?.canBeat() ?? false);
      case 'map_remove_beat': return !measure.protected && !!chosenBeat()?.id;
      case 'map_anchor_playhead': return !measure.protected && !!anchorTarget();
      case 'map_remove': return !measure.protected;
      default: return true; // map_protect, map_more
    }
  }

  async function edit(value: MapEdit, previewing = false) {
    // A pending split toast's Undo must never undo a later edit.
    onnotice(null);
    formError = '';
    await session.edit(value, previewing);
  }
  const failed = () => !!editState.error;
  export async function command(id: MappingActionId) {
    if (!canRun(id)) return;
    const fromDrawer = DRAFT.includes(id) || (showDrawer && (id === 'map_analyze' || (previewOrigin === 'draft' && (id === 'map_apply' || id === 'map_discard'))));
    errorAt = fromDrawer ? 'drawer' : panel ? 'panel' : dialogOpen && id !== 'map_clear' ? 'dialog' : 'wave';
    formError = '';
    try {
      if (DRAFT.includes(id)) { await drawer?.command(id); return; }
      if (id === 'map_origin' || id === 'map_grouping') { await songDialog?.command(id); if (!failed()) dialogOpen = false; return; }
      const target = measure;
      // With the toolbar collapsed to its edge tab, a toolbar key first shows the measure.
      const range = target ?? freeRange;
      if (TOOLBAR.includes(id) && range && (range.end / song.rate <= view.start || range.start / song.rate >= view.end)) {
        view = revealRange(view, song.duration, range.start / song.rate, range.end / song.rate);
        return;
      }
      switch (id) {
        case 'map_show_detected': detectedVisibility = !showDetected; return;
        case 'map_fit': { const range = target ?? freeRange!; view = fitRange(song.duration, range.start / song.rate, range.end / song.rate); return; }
        case 'map_whole': view = { start: 0, end: song.duration }; return;
        case 'map_undo': onnotice(null); await session.history('undo'); return;
        case 'map_redo': await session.history('redo'); return;
        case 'map_apply': await session.apply(); if (!failed()) redrafting = false; return;
        case 'map_discard': session.discard(); return;
        case 'map_clear': dialogOpen = false; if (hasMap) await edit({ type: 'clear' }); else session.discard(); return;
        case 'map_redraft': redrafting = !redrafting; return;
        case 'map_shift_resnap': resnap = !resnap; return;
        case 'map_song_settings': dialogOpen = true; return;
        case 'map_analyze': await analyzer.start(); return;
        case 'map_cancel_analysis': await analyzer.cancel(); return;
        case 'map_properties': panel = panel === 'properties' ? null : 'properties'; return;
        case 'map_more': panel = panel === 'more' ? null : 'more'; return;
        case 'map_protect': await edit({ type: 'set_protected', id: target!.id, protected: !target!.protected }); return;
        case 'map_notation': await edit(propertiesForm!.edit()); if (!failed()) panel = null; return;
        case 'map_boundary': await edit(expertForm!.boundaryEdit()); return;
        case 'map_beat': { const value = expertForm!.beatEdit(); if (value) await edit(value); return; }
        case 'map_remove_beat': { const beat = chosenBeat(); if (beat?.id) await edit({ type: 'remove_beat', measure: target!.id, id: beat.id }); return; }
        case 'map_anchor_playhead': { const beat = anchorTarget()!; selectedBeat = beat.quarter; await edit({ type: 'set_beat', measure: target!.id, id: beat.id, frame, offset: { num: Math.round(beat.quarter * 960), den: 960 } }); return; }
        case 'map_merge': await edit({ type: 'merge', id: target!.id }); return;
        case 'map_shift_left': case 'map_shift_right':
          await edit({ type: 'shift', id: target!.id, pulses: step, direction: id === 'map_shift_left' ? 'left' : 'right', ...(useResnap ? { resnap: { pulses: candidates!.beats.map(b => b.frame), epsilon_ms: epsilonMs } } : {}) });
          return;
        case 'map_split': {
          const beat = splitTarget(target!, frame)!.beat, index = measures.indexOf(target!), next = measures[index + 1];
          await edit({ type: 'split_at_beat', id: target!.id, beat });
          if (failed()) return;
          const renumbered = !!next && measures.find(m => m.id === next.id)?.label !== next.label;
          onnotice({ id: ++notices, text: `Split ${measureName(target!)} before beat ${beat}.${renumbered ? ' Later measures renumbered.' : ''}` });
          return;
        }
        case 'map_remove':
          removedMeter = target!.meter;
          await edit({ type: 'remove', id: target!.id });
          if (!failed()) { selected = null; freeFrame = target!.start; }
          return;
        case 'map_reclaim': {
          const range = freeRange!;
          await edit({ type: 'reclaim', start: range.start, end: range.end, meter: counter!.chosenMeter() });
          if (failed()) return;
          panel = null; selected = measures.find(m => m.start === range.start)?.id ?? null; freeFrame = selected ? null : freeFrame;
          return;
        }
      }
    } catch (e) { formError = String(e); }
  }
  // The meter a free-time range is counted with: the measure that was there, else its neighbour's.
  const countMeter = $derived(removedMeter ?? (freeRange ? ([...measures].reverse().find(m => m.end <= freeRange.start) ?? measures.find(m => m.start >= freeRange.end))?.meter ?? null : null));
  // Analysis problems stay visible once the drawer has stepped aside (E3, E4).
  const analysisNote = $derived(!hasMap ? null : separating ? 'Analysis waits while the song is split into stems'
    : analysis.error || analysis.status?.error ? 'Beat analysis failed' : stale ? 'Analysis out of date' : null);
  const clickText = $derived(transport.audio_error ? 'Click off: no audio output' : !measures.length ? 'The click starts once there’s a map' : preview ? 'Click follows the draft' : 'Click follows the map');
  const shownError = (at: typeof errorAt) => error && errorAt === at ? error : '';
</script>

<div class="map-screen" hidden={!active}>
  <div class="body">
    <div class="workspace" aria-busy={busy}>
      <MapReadout position={transport.position} duration={song.duration} rate={song.rate} mapView={displayed} mapped={hasMap} drafting={showDrawer && hasMap} {keymap} {available} {run}/>
      <main class="stage" aria-label="Measure mapping workspace">
        <MapWaveform peaks={song.peaks} duration={song.duration} rate={song.rate} position={transport.position} {view} mapView={displayed} draft={!!preview}
          {selected} {freeRange} {selectedBeat} candidates={showDetected ? candidates?.beats ?? [] : []} {splitAt} disabled={editState.pending || !!preview}
          onview={next => { view = next; }} {onseek} onselect={select} ongap={value => { selected = null; freeFrame = value; }} onbeat={quarter => { selectedBeat = quarter; }}
          onedit={value => { errorAt = 'wave'; void edit(value); }} ondragging={value => { dragging = value; }}>
          {#snippet overlay(geometry)}
            {#if !preview && (measure || freeRange)}
              <MapToolbar {measure} {freeRange} rate={song.rate} {geometry} fading={dragging} {panel} {split} {join} {shift} {keymap} {available} {run}
                onpanel={next => { panel = next; }} onsplithover={value => { splitHover = value; }}>
                {#snippet properties()}
                  {#if measure}<MeasureProperties bind:this={propertiesForm} {measure} error={shownError('panel')} {keymap} {available} {run} oncancel={() => { panel = null; }}/>
                  {:else if freeRange}<FreeTimeCount bind:this={counter} range={freeRange} rate={song.rate} meter={countMeter} error={shownError('panel')} {keymap} {available} {run} oncancel={() => { panel = null; }}/>{/if}
                {/snippet}
                {#snippet more()}
                  {#if measure}<MeasureExpert bind:this={expertForm} {measure} rate={song.rate} bind:beat={selectedBeat} bind:step bind:resnap bind:epsilonMs detected={!!candidates} error={shownError('panel')} {keymap} {available} {run}/>{/if}
                {/snippet}
              </MapToolbar>
            {/if}
            {#if shownError('wave')}
              <div class="rejected" role="alert"><Icon name="warning" size={15}/><span>Not changed: {error.replace(/^Error: /, '')}</span>
                <button class="icon-button" aria-label="Dismiss" title={keymap.title('dismiss_error')} onclick={() => { formError = ''; session.clearError(); }}><Icon name="close" size={14}/></button></div>
            {/if}
          {/snippet}
        </MapWaveform>
        <MapViewBar detected={showDetected} hasDetected={!!candidates} preview={preview && (previewOrigin === 'song' || !showDrawer) ? { warnings: preview.warnings ?? [] } : null}
          note={showDrawer ? null : analysisNote} {keymap} {available} {run}/>
      </main>
    </div>
    {#if showDrawer && active}
      <DraftDrawer bind:this={drawer} bind:inputs bind:epsilonMs duration={song.duration} rate={song.rate} position={transport.position} status={analysis.status} {candidates}
        analysisError={analysis.error || analysis.status?.error || ''} error={shownError('drawer')} {stale} preview={previewOrigin === 'draft' ? preview : null} bind:previewed pending={editState.pending} {separating} mapped={hasMap}
        {keymap} {available} {run} onpreview={value => { previewOrigin = 'draft'; return edit(value, true); }}/>
    {/if}
  </div>
  <PlaybackBar>
    <TransportButtons playing={transport.playing} mapped={!!mapView?.measures.length} {frozen} {keymap} {available} run={id => run(id)}/>
    <label class="goto">Go to m.<input aria-label="Go to measure number" type="number" min="1" step="1" disabled={!available('goto_measure')} bind:value={measureNumber} onchange={() => run('goto_measure')}/></label>
    <SpeedControl compact {speed} disabled={frozen || busy} {keymap} run={id => run(id)} {onspeed}/>
    {#if loopEnabled}<button class="compact-button" disabled={!available('mapping_loop_off')} title={keymap.title('mapping_loop_off')} onclick={() => run('mapping_loop_off')}>{keymap.buttonLabel('mapping_loop_off', 'Turn looping off')}</button>{/if}
    <span class="spacer"></span>
    <span class="click" title="On this screen the click always plays, at full level"><Icon name="click" size={15}/>{clickText}</span>
  </PlaybackBar>
</div>
<SongMapDialog bind:this={songDialog} open={dialogOpen && active} songKey={song.key} {map} error={shownError('dialog')} {keymap} {available} {run}
  onclose={() => { dialogOpen = false; }} onedit={value => { previewOrigin = 'song'; return edit(value, true); }}/>

<style>
  .map-screen{flex:1;min-height:0;display:flex;flex-direction:column}
  .map-screen[hidden]{display:none}
  .body{flex:1;min-height:0;display:flex}
  .workspace{flex:1;min-width:0;min-height:0;display:flex;flex-direction:column}
  .workspace[aria-busy=true]{opacity:.65}
  .stage{flex:1;min-height:0;display:flex;flex-direction:column;gap:8px;padding:0 24px 10px}
  .rejected{position:absolute;left:50%;bottom:44px;transform:translateX(-50%);z-index:4;pointer-events:auto;display:flex;align-items:center;gap:8px;max-width:calc(100% - 32px);padding:6px 6px 6px 12px;border:1px solid var(--error-border);border-radius:8px;background:var(--error-bg);color:var(--error-text);font-size:12px;line-height:1.45;box-shadow:0 6px 18px #0000004d}
  .rejected .icon-button{width:24px;height:24px;color:inherit}
  .goto{display:flex;align-items:center;gap:8px;font-size:12px;font-weight:600;color:var(--text-secondary)}
  .goto input{width:56px;height:30px;padding:0 8px;border:1px solid var(--border-strong);border-radius:6px;background:var(--input-bg);color:var(--text);font-size:13px;text-align:right}
  .spacer{flex:1}
  .click{display:flex;align-items:center;gap:6px;font-size:12px;color:var(--text-secondary)}
</style>
