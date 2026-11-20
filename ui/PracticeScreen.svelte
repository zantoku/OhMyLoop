<script lang="ts">
  // Practice ("Stage"): a readout band with position, passage, speed and
  // pitch above the waveform, a collapsible sections list beside it, and the
  // passage bar below. A drawer (notes) may sit on the right edge, beside both
  // the readout band and the waveform. The playback bar belongs to the shell.
  import type { Snippet } from 'svelte';
  import LoopCard from './LoopCard.svelte';
  import PassageBar from './PassageBar.svelte';
  import PitchTile from './PitchTile.svelte';
  import PositionReadout from './PositionReadout.svelte';
  import SectionsSidebar from './SectionsSidebar.svelte';
  import SpeedControl from './SpeedControl.svelte';
  import Waveform from './Waveform.svelte';
  import type { ButtonId } from './actions';
  import type { ActionPayload } from './action-dispatcher';
  import type { Keymap } from './keymap';
  import type { MapView } from './timing';
  import type { Controls, Loop, Section, Song, Transport, View } from './types';
  let { active, song, controls, sections, selectedSection, selection, looping, repeatCountIn, transport, mapView, view, snapToMeasures, frozen, busy,
    measureNumber = $bindable(1), keymap, available, run, onseek, onselection, onview, oncontrols, drawer }:
    { active: boolean; song: Song; controls: Controls; sections: Section[]; selectedSection: string | null; selection: Loop | null; looping: boolean;
      repeatCountIn: boolean; transport: Transport; mapView: MapView | null; view: View; snapToMeasures: boolean; frozen: boolean; busy: boolean;
      measureNumber?: number; keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId, payload?: ActionPayload) => void;
      onseek: (seconds: number) => void; onselection: (loop: Loop) => void; onview: (view: View) => void; oncontrols: (change: Partial<Controls>) => void;
      drawer?: Snippet } = $props();

  let sectionsOpen = $state(true);
  let pitch = $state<PitchTile>(), position = $state<PositionReadout>(), loopCard = $state<LoopCard>();
  const measures = $derived(mapView?.measures ?? []);
  const sectionName = $derived.by(() => {
    const section = sections.find(s => s.id === selectedSection);
    return section && selection && section.start === selection.start && section.end === selection.end ? section.name : null;
  });

  /** Commands for state that only this screen shows. */
  export function command(id: 'toggle_sections' | 'show_pitch' | 'show_goto_measure' | 'show_passage_bounds') {
    if (id === 'toggle_sections') sectionsOpen = !sectionsOpen;
    else if (id === 'show_pitch') pitch?.toggle();
    else if (id === 'show_passage_bounds') loopCard?.toggle();
    else position?.toggleGoto();
  }
</script>

<div class="practice" hidden={!active}>
 <div class="workspace" aria-busy={busy}>
  <section class="readout" class:narrowed={!!drawer} aria-label="Now playing">
    <PositionReadout bind:this={position} bind:measureNumber position={transport.position} duration={song.duration} rate={song.rate} {mapView}
      musical={transport.musical_position} countingIn={!!transport.counting_in} {keymap} {available} {run}/>
    <LoopCard bind:this={loopCard} {selection} {looping} {repeatCountIn} {sectionName} {measures} rate={song.rate} duration={song.duration} {frozen}
      {keymap} {available} {run} onrange={onselection}/>
    <span class="spacer"></span>
    <SpeedControl speed={controls.speed} disabled={frozen || busy} {keymap} {run} onspeed={speed => oncontrols({ speed })}/>
    <PitchTile bind:this={pitch} semitones={controls.semitones} cents={controls.tuning_cents} disabled={frozen || busy} {keymap} {available} {run}
      ontuning={tuning_cents => oncontrols({ tuning_cents })}/>
  </section>
  <div class="body">
    {#if sectionsOpen}
      <SectionsSidebar id="practice-sections" {sections} selected={selectedSection} {measures} rate={song.rate} {keymap} {available} {run}/>
    {/if}
    <main class="stage" class:with-sections={sectionsOpen} aria-label="Practice workspace">
      <Waveform {mapView} {snapToMeasures} rate={song.rate} peaks={song.peaks} duration={song.duration} {sections} {selectedSection} {selection}
        position={transport.position} {view} {onseek} {onselection} onsection={section => run('select_section', { section })} {onview}/>
      <PassageBar sectionsId="practice-sections" {sectionsOpen} {selection} {snapToMeasures} {measures} rate={song.rate} {keymap} {available} {run}/>
    </main>
  </div>
 </div>
 {@render drawer?.()}
</div>

<style>
  .practice{flex:1;min-height:0;display:flex}
  .practice[hidden]{display:none}
  .workspace{flex:1;min-width:0;min-height:0;display:flex;flex-direction:column}
  .workspace[aria-busy=true]{pointer-events:none;opacity:.65}
  .readout{flex:none;display:flex;align-items:center;gap:34px;padding:14px 24px 10px}
  .readout.narrowed{gap:18px}
  .spacer{flex:1}
  .body{flex:1;min-height:0;display:flex}
  .stage{flex:1;min-width:0;display:flex;flex-direction:column;gap:6px;padding:4px 24px 10px}
  .stage.with-sections{padding-left:16px}
</style>
