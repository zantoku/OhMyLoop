<script lang="ts">
  // Playback speed: a large exact percentage, reset, and 5/10-point steps.
  import Icon from './Icon.svelte';
  import { SPEED_PERCENT, SPEED_STEPS, speedPercent } from './playback-controls';
  import { entered } from './field-value';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  let { speed, disabled, keymap, run, onspeed }:
    { speed: number; disabled: boolean; keymap: Keymap; run: (id: ButtonId) => void; onspeed: (speed: number) => void } = $props();
  const STEP_BUTTONS: Record<(typeof SPEED_STEPS)[number], { id: ButtonId; name: string }> = {
    [-10]: { id: 'speed_down_10', name: 'Slower by 10 percent' }, [-5]: { id: 'speed_down_5', name: 'Slower by 5 percent' },
    5: { id: 'speed_up_5', name: 'Faster by 5 percent' }, 10: { id: 'speed_up_10', name: 'Faster by 10 percent' }
  };
</script>

<div class="speed" aria-label="Speed" role="group">
  <div class="value">
    <label class="caption" for="speed-percent">Speed</label>
    <input id="speed-percent" aria-label="Speed percent" type="number" min={SPEED_PERCENT.min} max={SPEED_PERCENT.max} step="any" {disabled}
      value={speedPercent(speed)} onchange={event => entered(event, speedPercent(speed), value => onspeed(value / 100))}/>
    <span class="unit">%</span>
    <button class="reset" aria-label="Reset speed" {disabled} title={keymap.title('reset_speed')} onclick={() => run('reset_speed')}><Icon name="reset" size={16}/></button>
  </div>
  <div class="steps">
    {#each SPEED_STEPS as step (step)}
      <button {disabled} aria-label={STEP_BUTTONS[step].name} title={keymap.title(STEP_BUTTONS[step].id)} onclick={() => run(STEP_BUTTONS[step].id)}>{step > 0 ? `+${step}` : `−${-step}`}</button>
    {/each}
  </div>
</div>

<style>
  .speed{display:flex;flex-direction:column;align-items:flex-end;gap:6px}
  .value{display:flex;align-items:baseline;gap:4px}
  .caption{margin-right:8px;font-size:11px;font-weight:700;letter-spacing:.06em;text-transform:uppercase;color:var(--text-secondary)}
  input{width:96px;padding:0;border:0;border-bottom:1px solid var(--border-strong);background:transparent;color:var(--text);font-size:44px;line-height:1;font-weight:700;text-align:right;font-variant-numeric:tabular-nums;appearance:textfield}
  input::-webkit-inner-spin-button,input::-webkit-outer-spin-button{appearance:none;margin:0}
  .unit{font-size:22px;font-weight:700}
  .reset{align-self:center;display:grid;place-items:center;width:30px;height:30px;margin-left:2px;padding:0;border:0;border-radius:5px;background:transparent;color:var(--text-secondary)}
  .reset:hover:not(:disabled),.steps button:hover:not(:disabled){background:var(--hover);color:var(--text)}
  .steps{display:flex;gap:4px}
  .steps button{min-width:40px;height:30px;padding:0 6px;border:1px solid var(--border);border-radius:5px;background:var(--surface);color:var(--text-secondary);font-size:12px;font-variant-numeric:tabular-nums}
</style>
