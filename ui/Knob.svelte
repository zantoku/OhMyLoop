<script lang="ts">
  // A rotary gain knob from 0 to 100 %. Drag vertically, scroll or use the
  // arrow keys; double-click resets to 100 %. It owns only its arrow keys and
  // Home/End, so letters and Space still reach the shortcuts.
  import type { Snippet } from 'svelte';
  import { clamp } from './model';
  let { label, value, disabled = false, dimmed = false, title = 'Drag vertically, scroll or use arrow keys. Double-click resets to 100%.', onchange, children }:
    { label: string; value: number; disabled?: boolean; dimmed?: boolean; title?: string; onchange: (value: number) => void; children?: Snippet } = $props();
  const KEYS = ['ArrowUp', 'ArrowRight', 'ArrowDown', 'ArrowLeft', 'Home', 'End'];
  let drag: { y: number; value: number; pointer: number } | null = null;
  const percent = $derived(Math.round(value * 100));
  // The arc sweeps 270° from 7:30 (minimum) clockwise to 4:30 (maximum).
  const point = (degrees: number, radius: number) => { const r = (degrees - 90) * Math.PI / 180; return [22 + radius * Math.cos(r), 22 + radius * Math.sin(r)]; };
  function arc(to: number) {
    const [x0, y0] = point(-135, 18), [x1, y1] = point(to, 18);
    return `M${x0.toFixed(2)} ${y0.toFixed(2)}A18 18 0 ${to + 135 > 180 ? 1 : 0} 1 ${x1.toFixed(2)} ${y1.toFixed(2)}`;
  }
  const angle = $derived(-135 + 270 * clamp(value, 0, 1));
  const tip = $derived(point(angle, 9));
  function set(next: number) { if (!disabled) onchange(Math.round(clamp(next, 0, 1) * 100) / 100); }
  function key(event: KeyboardEvent) {
    if (!KEYS.includes(event.key)) return;
    event.preventDefault(); event.stopPropagation();
    set(event.key === 'Home' ? 0 : event.key === 'End' ? 1 : value + (['ArrowUp', 'ArrowRight'].includes(event.key) ? .01 : -.01));
  }
  function down(event: PointerEvent) {
    if (disabled || event.button !== 0) return;
    event.preventDefault();
    (event.currentTarget as HTMLElement).focus();
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    drag = { y: event.clientY, value, pointer: event.pointerId };
  }
  function move(event: PointerEvent) { if (drag?.pointer === event.pointerId) set(drag.value + (drag.y - event.clientY) / 100); }
  function end(event: PointerEvent) { if (drag?.pointer === event.pointerId) drag = null; }
</script>

<div class="channel" class:dimmed role="group" aria-label={`${label} track`}>
  <span class="name">{label}</span>
  <div class="knob" role="slider" tabindex={disabled ? -1 : 0} data-owns-keys={KEYS.join(' ')} aria-disabled={disabled}
    aria-label={`${label} gain`} aria-valuemin={0} aria-valuemax={100} aria-valuenow={percent} aria-valuetext={`${percent}%`} {title}
    onpointerdown={down} onpointermove={move} onpointerup={end} onpointercancel={end} onkeydown={key} ondblclick={() => set(1)}
    onwheel={event => { if (disabled) return; event.preventDefault(); set(value + (event.deltaY < 0 ? .02 : -.02)); }}>
    <svg viewBox="0 0 44 44" width="42" height="42" aria-hidden="true">
      <path class="track" d={arc(135)}/>
      {#if value > 0}<path class="level" d={arc(angle)}/>{/if}
      <circle cx="22" cy="22" r="11"/>
      <line x1="22" y1="22" x2={tip[0].toFixed(2)} y2={tip[1].toFixed(2)}/>
    </svg>
  </div>
  <span class="value">{percent}%</span>
  {@render children?.()}
</div>

<style>
  .channel{width:66px;display:flex;flex-direction:column;align-items:center;gap:3px}
  .name{font-size:11px;font-weight:600;color:var(--text-secondary)}
  .knob{width:42px;height:42px;border-radius:50%;cursor:ns-resize;touch-action:none}
  .knob[aria-disabled=true]{opacity:.45;cursor:default}
  svg{display:block}
  path{fill:none;stroke-width:4;stroke-linecap:round}
  .track{stroke:var(--border-strong)}
  .level{stroke:var(--blue)}
  .dimmed .level{stroke:var(--border-strong)}
  circle{fill:var(--surface);stroke:var(--border-strong)}
  line{stroke:var(--text);stroke-width:2;stroke-linecap:round}
  :global([data-modality=keyboard]) .knob:focus-visible{outline-offset:0}
  .value{font-size:11px;color:var(--text-secondary);font-variant-numeric:tabular-nums}
</style>
