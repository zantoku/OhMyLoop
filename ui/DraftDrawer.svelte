<script lang="ts">
  // The drafting drawer (measure-map.md, 2026-11-19): on an empty map the
  // drafting steps sit in a full-height drawer on the right, from below the
  // header to the playback bar, like Notes on Practice. Redraft reopens it
  // once a map exists. From detected beats: find the beats (with the one-time
  // model download), read them (meter, beat note, first full measure), then
  // preview and apply. By hand: mark a span, count its measures, preview and
  // apply. Previews show in green and the click follows them; nothing is
  // saved until Apply. Inputs are session state (DraftInputs).
  import Icon from './Icon.svelte';
  import SettingsProgress from './SettingsProgress.svelte';
  import { draftError, findState, stepMarks, type DraftInputs, type StepMark } from './draft-steps';
  import { readMeter } from './meter-form';
  import { validEpsilon } from './measure-tools';
  import { parseClock, time } from './model';
  import { BEAT_MODEL_BYTES, beatModel, megabytes } from './settings';
  import type { AnalysisStatus, Candidates, MapEdit, MapPreview } from './timing';
  import type { ButtonId, MappingActionId } from './actions';
  import type { ActionPayload } from './action-dispatcher';
  import type { Keymap } from './keymap';
  let { inputs = $bindable(), previewed = $bindable(''), epsilonMs = $bindable(30), duration, rate, position, status, candidates, analysisError, error, stale, preview, pending, separating, mapped,
    keymap, available, run, onpreview }:
    { inputs: DraftInputs; previewed?: string; epsilonMs?: number; duration: number; rate: number; position: number; status: AnalysisStatus | null; candidates: Candidates | null;
      analysisError: string; error: string; stale: boolean; preview: MapPreview | null; pending: boolean; separating: boolean; mapped: boolean;
      keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId, payload?: ActionPayload) => void; onpreview: (edit: MapEdit) => Promise<void> } = $props();

  const model = $derived(beatModel(status));
  const find = $derived(findState({ model, analysing: !!status?.running && status.kind !== 'install', found: !!candidates, separating }));
  const marks = $derived(stepMarks(inputs.source, find, !!preview));
  const progress = $derived(status?.progress?.total ? Math.min(1, status.progress.completed / status.progress.total) : null);
  const action = $derived(inputs.source === 'detected' ? 'map_use_analysis' : 'map_build');
  const signature = () => JSON.stringify([inputs, epsilonMs]);
  const changed = $derived(!!preview && previewed !== signature());

  /** True when the inputs changed since the shown draft was previewed. */
  export function isStale() { return changed; }
  export function canRun(id: MappingActionId) {
    if (pending) return false;
    if (id === 'map_use_analysis' || id === 'map_first_downbeat') return !!candidates && !status?.running;
    return true;
  }
  const frame = (text: string, what: string) => {
    const seconds = parseClock(text);
    if (!Number.isFinite(seconds) || seconds > duration) throw new Error(`Enter ${what} as a time in the song, such as 0:03.21.`);
    return Math.round(seconds * rate);
  };
  export async function command(id: MappingActionId) {
    if (!canRun(id)) return;
    const playhead = time(position, 3);
    if (id === 'map_span_start') { inputs.start = playhead; return; }
    if (id === 'map_span_end') { inputs.end = playhead; return; }
    if (id === 'map_first_downbeat') { inputs.first = playhead; return; }
    const meter = readMeter({ numerator: inputs.numerator, denominator: inputs.denominator, pulse: inputs.pulse, grouping: '' });
    previewed = signature();
    if (id === 'map_use_analysis') {
      if (!validEpsilon(epsilonMs)) throw new Error('Beat timing tolerance must be between 0 and 1000 ms.');
      await onpreview({ type: 'build_from_pulses', pulses: candidates!.beats.map(b => b.frame), downbeats: candidates!.beats.filter(b => b.downbeat).map(b => b.frame),
        first_downbeat: frame(inputs.first, 'the first full measure'), epsilon_ms: epsilonMs,
        // A redraft replaces only its span, so locked corrections outside it stay.
        start: mapped ? frame(inputs.start, 'the start') : 0, end: mapped ? frame(inputs.end, 'the end') : Math.round(duration * rate), meter });
    } else if (id === 'map_build') {
      await onpreview({ type: 'build', start: frame(inputs.start, 'the start'), end: frame(inputs.end, 'the end'), count: inputs.count, meter });
    }
  }
  const markText: Record<StepMark, string> = { current: '', done: '✓', upcoming: '' };
</script>

{#snippet number(n: number, mark: StepMark)}<span class="number {mark}" aria-hidden="true">{mark === 'done' ? markText.done : n}</span>{/snippet}
{#snippet meterFields()}
  <div class="row"><span class="grow">Time signature</span>
    <span class="meter" role="group" aria-label="Draft meter">
      <input aria-label="Draft meter numerator" type="number" min="1" max="99" step="1" bind:value={inputs.numerator}/><span aria-hidden="true">/</span>
      <input aria-label="Draft meter denominator" type="number" min="1" max="64" step="1" bind:value={inputs.denominator}/>
    </span>
  </div>
  <label class="row"><span class="grow">One beat is a</span>
    <select aria-label="Draft beat note" bind:value={inputs.pulse}><option value="1">Quarter note</option><option value="1/2">Eighth note</option><option value="3/2">Dotted quarter</option><option value="2">Half note</option><option value="1/4">Sixteenth note</option></select>
  </label>
{/snippet}
{#snippet playheadField(label: string, key: 'first' | 'start' | 'end', id: ButtonId)}
  <label class="row"><span class="grow">{label}</span><input class="time" aria-label={label} bind:value={inputs[key]}/>
    <button class="compact-button" disabled={!available(id)} title={keymap.title(id)} onclick={() => run(id)}>Playhead</button></label>
{/snippet}

<aside id="map-draft" class="drawer" aria-label="Draft a measure map">
  <div class="intro">
    <h2>{mapped ? 'Draft the map again' : 'Map this song'}</h2>
    <span class="hint">{mapped ? 'A new draft replaces the measures in its span; unlock locked measures first. Undo brings the map back.' : 'Get a first draft, then fix it by listening. You can change everything later.'}</span>
    <span class="segments" role="group" aria-label="Draft from">
      <button aria-pressed={inputs.source === 'detected'} onclick={() => { inputs.source = 'detected'; }}>From detected beats</button>
      <button aria-pressed={inputs.source === 'manual'} onclick={() => { inputs.source = 'manual'; }}>By hand</button>
    </span>
  </div>
  <ol>
    {#if inputs.source === 'detected'}
      <li class={marks[0]}>{@render number(1, marks[0])}<div class="step"><h3>Find the beats</h3>
        {#if find === 'checking'}<span class="hint" role="status">Checking the analysis model…</span>
        {:else if find === 'install'}
          <span class="hint">Beat analysis needs a one-time download of the analysis model (about {megabytes(BEAT_MODEL_BYTES)}). It works offline afterwards. Drafting by hand needs no download.</span>
          {#if model.kind === 'failed'}<p class="alert" role="alert">The last download failed{status?.error ? `: ${status.error}` : ''}.</p>{/if}
          <div class="actions"><button class="compact-button primary" disabled={!available('map_install_model')} title={keymap.title('map_install_model')} onclick={() => run('map_install_model')}>{keymap.buttonLabel('map_install_model', 'Download analysis model')}</button>
            <button class="compact-button ghost" disabled={!available('settings')} title={keymap.title('settings')} onclick={() => run('settings', { settingsSection: 'beats' })}>Beat analysis settings</button></div>
        {:else if find === 'downloading'}
          <span class="hint" role="status">{model.kind === 'downloading' ? model.phase : 'Downloading'}… Playback and drafting by hand keep working.</span>
          <SettingsProgress label="Analysis model download progress" progress={model.kind === 'downloading' ? model.progress : null} hint="">
            <button class="compact-button" disabled={!available('map_cancel_model')} title={keymap.title('map_cancel_model')} onclick={() => run('map_cancel_model')}>Cancel</button>
          </SettingsProgress>
        {:else if find === 'waiting'}
          <p class="note" role="status">Analysis waits while the song is split into stems: they share one processing slot. Drafting by hand works meanwhile.</p>
        {:else if find === 'analyze'}
          <span class="hint">Finds beats and downbeats. Takes about a minute; keep listening meanwhile.</span>
          <div class="actions"><button class="compact-button primary" disabled={!available('map_analyze')} title={keymap.title('map_analyze')} onclick={() => run('map_analyze')}>{keymap.buttonLabel('map_analyze', 'Analyze song')}</button></div>
        {:else if find === 'running'}
          <span class="hint" role="status">{status?.phase ?? 'Analyzing'}… {Math.floor((status?.elapsed_seconds ?? 0) / 60)}:{String(Math.floor((status?.elapsed_seconds ?? 0) % 60)).padStart(2, '0')} elapsed</span>
          <SettingsProgress label="Song analysis progress" {progress} hint="">
            <button class="compact-button" disabled={!available('map_cancel_analysis')} title={keymap.title('map_cancel_analysis')} onclick={() => run('map_cancel_analysis')}>{keymap.buttonLabel('map_cancel_analysis', 'Cancel')}</button>
          </SettingsProgress>
        {:else}
          <div class="found"><Icon name="check" size={14}/>{candidates!.beats.length} beats found
            <button class="compact-button ghost" disabled={!available('map_analyze')} title={separating ? 'Analysis waits while the song is split into stems' : keymap.title('map_analyze')} onclick={() => run('map_analyze')}>Analyze again</button></div>
          {#if separating}<p class="note" role="status">Analyzing again waits while the song is split into stems: they share one processing slot.</p>{/if}
          {#if stale}<p class="note" role="status">The map has changed since this analysis. Review the draft before applying, or analyze again.</p>{/if}
        {/if}
        {#if analysisError}<p class="alert" role="alert">{analysisError}</p>{/if}
      </div></li>
      <li class={marks[1]}>{@render number(2, marks[1])}<div class="step"><h3>Read the beats</h3>
        <span class="hint">How the detected beats become measures. They show as amber marks under the waveform.</span>
        {@render meterFields()}
        {@render playheadField('First full measure at', 'first', 'map_first_downbeat')}
        {#if mapped}
          <span class="hint">Replace only this span; locked measures outside it stay as they are.</span>
          {@render playheadField('Replace from', 'start', 'map_span_start')}
          {@render playheadField('Replace until', 'end', 'map_span_end')}
        {/if}
      </div></li>
      <li class={marks[2]}>{@render number(3, marks[2])}<div class="step"><h3>Preview and apply</h3>
        {#if preview?.draft}
          <div class="summary" role="status"><strong>{preview.draft.measure_count} measures</strong>
            <span>{preview.draft.full_measures} full{preview.draft.partial_pulses ? ' + 1 partial at the end' : ''} · {preview.draft.anchor_count} timing {preview.draft.anchor_count === 1 ? 'correction' : 'corrections'} · ε {preview.draft.epsilon_ms} ms</span>
            <span>Listen through: the click follows the draft.</span></div>
        {/if}
        {@render apply()}
        <details><summary>Tolerance</summary>
          <label class="row"><span class="grow">Follow detected beats within</span><input class="ms" aria-label="Beat timing tolerance (ms)" type="number" min="0" max="1000" step="any" bind:value={epsilonMs}/>ms</label>
          <span class="hint">Beats stay evenly spaced unless one would be further than this from a detected beat. 0 ms follows them exactly.</span>
        </details>
      </div></li>
    {:else}
      <li class={marks[0]}>{@render number(1, marks[0])}<div class="step"><h3>Mark the span</h3>
        <span class="hint">Where the measures start and end. Use the playhead while listening, or type exact times.</span>
        {@render playheadField('Starts at', 'start', 'map_span_start')}
        {@render playheadField('Ends at', 'end', 'map_span_end')}
      </div></li>
      <li class={marks[1]}>{@render number(2, marks[1])}<div class="step"><h3>Count the measures</h3>
        {@render meterFields()}
        <label class="row"><span class="grow">Number of measures</span><input class="ms" aria-label="Number of measures" type="number" min="1" max="4096" step="1" bind:value={inputs.count}/></label>
      </div></li>
      <li class={marks[2]}>{@render number(3, marks[2])}<div class="step"><h3>Preview and apply</h3>
        {#if preview && !preview.draft}<div class="summary" role="status"><strong>{preview.view.measures.length} measures</strong><span>Evenly spaced. Listen through: the click follows the draft.</span></div>{/if}
        {@render apply()}
      </div></li>
    {/if}
  </ol>
</aside>

{#snippet apply()}
  {#if error}<p class="alert" role="alert">{draftError(error)}</p>{/if}
  {#if changed}<p class="note" role="status">Settings changed. Preview again before applying.</p>{/if}
  <div class="actions end">
    {#if preview}<button class="compact-button" disabled={!available('map_discard')} title={keymap.title('map_discard')} onclick={() => run('map_discard')}>{keymap.buttonLabel('map_discard', 'Discard')}</button>{/if}
    <button class="compact-button" class:primary={!preview || changed} disabled={!available(action)} title={keymap.title(action)} onclick={() => run(action)}>{preview ? 'Preview again' : 'Preview draft'}</button>
    {#if preview}<button class="compact-button" class:primary={!changed} disabled={!available('map_apply')} title={keymap.title('map_apply')} onclick={() => run('map_apply')}>{keymap.buttonLabel('map_apply', 'Apply map')}</button>{/if}
  </div>
{/snippet}

<style>
  .drawer{width:340px;flex:none;min-height:0;overflow:auto;overscroll-behavior:contain;border-left:1px solid var(--border);background:var(--sidebar)}
  .intro{display:flex;flex-direction:column;gap:10px;padding:16px}
  h2{font-size:18px}
  h3{margin-top:3px;font-size:13px}
  .hint{font-size:11px;line-height:1.45;color:var(--text-secondary)}
  .intro .hint{font-size:12px}
  .segments{display:inline-flex;align-self:flex-start;gap:2px;padding:2px;border:1px solid var(--border);border-radius:7px;background:var(--page)}
  .segments button{height:24px;padding:0 9px;border:0;border-radius:5px;background:transparent;color:var(--text-secondary);font-size:12px;font-weight:600}
  .segments button[aria-pressed=true]{background:var(--surface);color:var(--text);box-shadow:0 1px 2px #0000002e}
  ol{margin:0;padding:0;list-style:none}
  li{display:flex;gap:12px;padding:14px 16px;border-top:1px solid var(--border)}
  li.upcoming{opacity:.5}
  .number{flex:none;width:24px;height:24px;display:grid;place-items:center;border-radius:7px;background:var(--border);color:var(--text-secondary);font-size:12px;font-weight:700}
  .number.current{background:var(--blue);color:var(--on-accent)}
  .number.done{background:var(--success-bg);color:var(--success-text)}
  .step{flex:1;min-width:0;display:flex;flex-direction:column;gap:8px}
  .row{display:flex;align-items:center;gap:8px;font-size:12px;color:var(--text-secondary)}
  .grow{flex:1}
  input,select{height:30px;border:1px solid var(--border-strong);border-radius:6px;background:var(--input-bg);color:var(--text);font-size:13px;padding:0 8px;font-variant-numeric:tabular-nums}
  .meter{display:inline-flex;align-items:center;gap:4px;font-size:16px}
  .meter input{width:42px;text-align:center;font-weight:700;appearance:textfield}
  .meter input::-webkit-inner-spin-button{appearance:none}
  .time{width:82px;text-align:right}
  .ms{width:60px;text-align:right}
  .actions{display:flex;flex-wrap:wrap;gap:6px}
  .actions.end{justify-content:flex-end}
  .found{display:flex;align-items:center;gap:8px;font-size:12px;color:var(--success-text)}
  .found .compact-button{margin-left:auto;height:22px;padding:0 4px}
  .summary{display:flex;flex-direction:column;gap:4px;padding:10px 12px;border:1px solid var(--success-border);border-radius:8px;background:var(--success-bg);color:var(--success-text);font-size:12px}
  .summary strong{font-size:16px;color:var(--text)}
  .note{padding:7px 9px;border:1px solid var(--amber-border);border-radius:6px;background:var(--amber-bg);color:var(--amber-text);font-size:12px;line-height:1.45}
  .alert{padding:7px 9px;border:1px solid var(--error-border);border-radius:6px;background:var(--error-bg);color:var(--error-text);font-size:12px;line-height:1.45}
  details summary{font-size:11px;color:var(--text-secondary);padding:2px 0}
  details .row{margin-top:6px}
</style>
