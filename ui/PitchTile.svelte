<script lang="ts">
  // Pitch at a glance: transposition and the recording's reference tuning,
  // each highlighted only when changed. Opens a popover to adjust both.
  import Icon from './Icon.svelte';
  import Popover from './Popover.svelte';
  import { centsForReference, hzText, REFERENCE_HZ, referenceHz, semitoneText } from './playback-controls';
  import { entered } from './field-value';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  let { semitones, cents, disabled, keymap, available, run, ontuning }:
    { semitones: number; cents: number; disabled: boolean; keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void; ontuning: (cents: number) => void } = $props();
  let open = $state(false), tile = $state<HTMLButtonElement>();
  export function toggle() { open = !open; }
  const transposed = $derived(semitones !== 0);
  const retuned = $derived(Math.abs(referenceHz(cents) - 440) > .05);
  const hz = $derived(hzText(cents));
  const note = $derived(retuned ? `Plays ${cents > 0 ? '+' : '−'}${Math.abs(cents).toFixed(1)} ¢ to match A = 440 Hz. Saved with this song.` : 'Standard tuning. Saved with this song.');
</script>

<div class="pitch">
  <button class="tile" class:open bind:this={tile} aria-haspopup="dialog" aria-expanded={open} aria-label={`Pitch ${semitoneText(semitones)}, recording tuned to A = ${hz} Hz`}
    disabled={!available('pitch')} title={keymap.title('pitch')} onclick={() => run('pitch')}>
    <span class="caption">Pitch</span>
    <span class="semitones" class:changed={transposed}>{semitoneText(semitones)}</span>
    <span class="hz" class:changed={retuned}>A {hz} Hz</span>
  </button>
  <Popover {open} label="Pitch" anchor={tile} onclose={() => { open = false; }}>
    <div class="row">
      <span class="label">Transpose</span>
      <button class="stepper" aria-label="Transpose down" {disabled} title={keymap.title('transpose_down')} onclick={() => run('transpose_down')}>−</button>
      <strong>{semitoneText(semitones)}</strong>
      <button class="stepper" aria-label="Transpose up" {disabled} title={keymap.title('transpose_up')} onclick={() => run('transpose_up')}>+</button>
      <button class="reset" aria-label="Reset transpose" {disabled} title={keymap.title('reset_pitch')} onclick={() => run('reset_pitch')}><Icon name="reset" size={15}/></button>
    </div>
    <hr/>
    <div class="row">
      <label class="label" for="reference-hz">Recording tuned to</label>
      <span>A =</span>
      <input id="reference-hz" aria-label="Recording tuned to A (Hz)" type="number" min={REFERENCE_HZ.min.toFixed(1)} max={REFERENCE_HZ.max.toFixed(1)} step="any" {disabled}
        value={hz} onchange={event => entered(event, hz, value => ontuning(centsForReference(value)))}/>
      <span>Hz</span>
      <button class="reset" aria-label="Reset tuning" {disabled} title={keymap.title('reset_tuning')} onclick={() => run('reset_tuning')}><Icon name="reset" size={15}/></button>
    </div>
    <span class="note">{note}</span>
  </Popover>
</div>

<style>
  .pitch{position:relative;align-self:stretch;display:flex}
  .tile{width:112px;display:flex;flex-direction:column;align-items:flex-start;justify-content:center;gap:2px;padding:0 14px;border:1px solid var(--border);border-radius:8px;background:var(--surface);color:var(--text-secondary);text-align:left}
  .tile:hover:not(:disabled){background:var(--hover)}
  .tile.open{border-color:var(--blue)}
  .caption{font-size:11px;font-weight:700;letter-spacing:.06em;text-transform:uppercase}
  .semitones{font-size:26px;line-height:1.1;font-weight:700;font-variant-numeric:tabular-nums}
  .hz{font-size:12px;font-variant-numeric:tabular-nums}
  .changed{color:var(--blue)}.hz.changed{font-weight:700}
  .row{display:flex;align-items:center;gap:6px;font-size:13px}
  .label{flex:1;font-size:12px;font-weight:600}
  .row strong{min-width:48px;text-align:center;font-size:16px;font-variant-numeric:tabular-nums}
  .stepper{width:30px;height:30px;padding:0;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text-secondary);font-size:15px}
  .reset{display:grid;place-items:center;width:28px;height:30px;padding:0;border:0;border-radius:5px;background:transparent;color:var(--text-secondary)}
  .stepper:hover:not(:disabled),.reset:hover:not(:disabled){background:var(--hover);color:var(--text)}
  hr{width:100%;height:1px;margin:0;border:0;background:var(--border)}
  input{width:58px;height:26px;padding:0;border:0;border-bottom:1px solid var(--border-strong);background:transparent;color:var(--text);font-size:16px;font-weight:700;text-align:right;font-variant-numeric:tabular-nums;appearance:textfield}
  input::-webkit-inner-spin-button,input::-webkit-outer-spin-button{appearance:none;margin:0}
  .note{font-size:11px;color:var(--text-secondary)}
</style>
