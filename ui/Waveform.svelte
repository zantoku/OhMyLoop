<script lang="ts">
  import { onMount } from 'svelte';
  import MeasureRuler from './MeasureRuler.svelte';
  import type { MapView } from './timing';
  import type { Loop, Section, View } from './types';
  import { clamp, range, sectionColor, time } from './model';

  import { drawWaveform, sourcePoint, panView, overviewView, wheelView } from './timeline';

  let { peaks, duration, sections, selection, position, view, mapView = null, rate = 48000, onseek, onselection, onsection, onview }:
    { peaks: [number, number][]; duration: number; sections: Section[]; selection: Loop | null;
      position: number; view: View; mapView?: MapView | null; rate?: number; onseek: (value: number) => void; onselection: (value: Loop) => void;
      onsection: (section: Section) => void; onview: (view: View) => void } = $props();
  let canvas: HTMLCanvasElement;
  let overview: HTMLCanvasElement;
  let field: HTMLDivElement;
  let preview = $state<Loop | null>(null);
  type Drag = { x: number; pointer: number } & (
    { mode: 'pan'; view: View; width: number } |
    { mode: 'create' | 'start' | 'end'; at: number }
  );
  let drag = $state<Drag | null>(null);
  let overviewPointer = $state<number | null>(null);
  let keyboardFocus = $state(false);
  let width = $state(800);
  let height = $state(200);
  const current = $derived(preview ?? selection);
  const span = $derived(view.end - view.start);
  const percent = (seconds: number) => (seconds - view.start) / span * 100;
  const ticks = $derived(Array.from({ length: 7 }, (_, i) => view.start + span * i / 6));

  $effect(() => { void width; void height; void peaks; drawWaveform(canvas, peaks, duration, view.start, view.end); drawWaveform(overview, peaks, duration, 0, duration, true); });
  onMount(() => {
    const observer = new ResizeObserver(() => { width = field.clientWidth; height = field.clientHeight; });
    observer.observe(field);
    // Scripted focus during dragging can retain WebKit's keyboard focus ring.
    const pointerInput = () => { keyboardFocus = false; };
    const keyboardInput = () => { keyboardFocus = true; };
    window.addEventListener('pointerdown', pointerInput, true);
    window.addEventListener('keydown', keyboardInput, true);
    // Cancel page scrolling even on WebViews that make wheel listeners passive by default.
    field.addEventListener('wheel', wheel, { passive: false });
    return () => {
      observer.disconnect(); field.removeEventListener('wheel', wheel);
      window.removeEventListener('pointerdown', pointerInput, true);
      window.removeEventListener('keydown', keyboardInput, true);
    };
  });
  function point(event: MouseEvent): number {
    return sourcePoint(view, duration, field.getBoundingClientRect(), event.clientX);
  }
  function wheel(event: WheelEvent) {
    if (!event.deltaY) return;
    event.preventDefault();
    if (drag || overviewPointer !== null) return;
    const box = field.getBoundingClientRect();
    if (!box.width) return;
    onview(wheelView(view, duration, box, event));
  }
  function down(event: PointerEvent) {
    if (drag || overviewPointer !== null || (event.button !== 0 && event.button !== 1)) return;
    if (event.button === 1) {
      const box = field.getBoundingClientRect();
      if (!box.width) return;
      drag = { mode: 'pan', x: event.clientX, pointer: event.pointerId, view: { ...view }, width: box.width };
    } else {
      const handle = (event.target as HTMLElement).closest<HTMLElement>('[data-handle]')?.dataset.handle;
      drag = { mode: handle === 'start' || handle === 'end' ? handle : 'create', at: point(event), x: event.clientX, pointer: event.pointerId };
    }
    field.setPointerCapture(event.pointerId);
    field.focus(); event.preventDefault();
  }
  function move(event: PointerEvent) {
    if (!drag || event.pointerId !== drag.pointer) return;
    if (drag.mode === 'pan') {
      onview(panView(drag.view, duration, event.clientX - drag.x, drag.width));
      return;
    }
    const p = point(event);
    if (drag.mode === 'create') preview = range(drag.at, p, duration);
    else if (current) {
      if (drag.mode === 'start') preview = { ...current, start: clamp(p, 0, current.end - .1) };
      else preview = { ...current, end: clamp(p, current.start + .1, duration) };
    }
  }
  function up(event: PointerEvent) {
    if (!drag || drag.pointer !== event.pointerId) return;
    if (drag.mode !== 'pan') {
      const wasDrag = drag.mode !== 'create' || Math.abs(event.clientX - drag.x) > 4;
      if (wasDrag && preview) onselection(preview); else if (!wasDrag) onseek(point(event));
    }
    cancel(event);
  }
  function cancel(event: PointerEvent) {
    if (!drag || drag.pointer !== event.pointerId) return;
    preview = null; drag = null;
    if (field.hasPointerCapture(event.pointerId)) field.releasePointerCapture(event.pointerId);
  }
  function shiftBoundary(event: KeyboardEvent, edge: 'start' | 'end') {
    if (!selection || !['ArrowLeft', 'ArrowRight'].includes(event.key)) return;
    event.preventDefault(); event.stopPropagation();
    const step = (event.shiftKey ? .01 : .1) * (event.key === 'ArrowRight' ? 1 : -1);
    onselection({ ...selection, [edge]: edge === 'start' ? clamp(selection.start + step, 0, selection.end - .1) : clamp(selection.end + step, selection.start + .1, duration) });
  }
  function panOverview(event: MouseEvent) {
    const box = event.currentTarget instanceof HTMLElement ? event.currentTarget.getBoundingClientRect() : overview.getBoundingClientRect();
    if (!box.width) return;
    onview(overviewView(view, duration, box, event.clientX));
  }
  function overviewDown(event: PointerEvent) {
    if (event.button !== 0 || drag || overviewPointer !== null) return;
    const target = event.currentTarget as HTMLButtonElement;
    overviewPointer = event.pointerId;
    target.setPointerCapture(event.pointerId);
    target.focus(); event.preventDefault();
    panOverview(event);
  }
  function overviewMove(event: PointerEvent) {
    if (event.pointerId === overviewPointer) panOverview(event);
  }
  function overviewUp(event: PointerEvent) {
    if (event.pointerId !== overviewPointer) return;
    panOverview(event);
    overviewCancel(event);
  }
  function overviewCancel(event: PointerEvent) {
    if (event.pointerId !== overviewPointer) return;
    overviewPointer = null;
    const target = event.currentTarget as HTMLButtonElement;
    if (target.hasPointerCapture(event.pointerId)) target.releasePointerCapture(event.pointerId);
  }
</script>

<div class="overview-wrap">
  <button class="overview" class:keyboard-focus={keyboardFocus} class:panning={overviewPointer !== null} onclick={panOverview}
    onpointerdown={overviewDown} onpointermove={overviewMove} onpointerup={overviewUp}
    onpointercancel={overviewCancel} onlostpointercapture={overviewCancel}
    aria-label="Move waveform view" title="Click or drag to move the detailed view">
    <canvas bind:this={overview}></canvas>
    <span class="viewport" style:left={`${view.start / duration * 100}%`} style:width={`${span / duration * 100}%`}></span>
    <span class="overview-playhead" style:left={`${position / duration * 100}%`}></span>
  </button>
  <span class="overview-label">Whole song</span><span class="overview-duration">{time(duration, 0)}</span>
</div>

{#if mapView?.measures.length}<MeasureRuler {mapView} {rate} {view} {onseek}/>{/if}

<div class="section-lane" aria-label="Sections on timeline">
  {#each sections.filter(s => s.end > view.start && s.start < view.end) as section (section.id)}
    <button title={`${section.name}: ${time(section.start)} – ${time(section.end)}`} onclick={() => onsection(section)}
      style:left={`${Math.max(0, percent(section.start))}%`} style:width={`${Math.min(100, percent(section.end)) - Math.max(0, percent(section.start))}%`}
      style:background={sectionColor(section.id)}>{section.name}</button>
  {/each}
</div>

<div class="wave-field" class:keyboard-focus={keyboardFocus} class:panning={drag?.mode === 'pan'} bind:this={field} role="region" tabindex="-1"
  aria-label="Waveform. Click to seek, drag to choose a loop. Scroll to zoom, middle-drag to pan."
  onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={cancel} onlostpointercapture={cancel}
  onauxclick={e => { if (e.button === 1) e.preventDefault(); }}>
  <div class="seek-surface" role="slider" tabindex="0" aria-label="Song position" aria-valuemin="0" aria-valuemax={duration} aria-valuenow={position} aria-valuetext={time(position)}
    onkeydown={e => { if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') { e.preventDefault(); e.stopPropagation(); onseek(clamp(position + (e.key === 'ArrowRight' ? 1 : -1) * (e.shiftKey ? .1 : 5), 0, duration)); } else if (e.key === 'Home' || e.key === 'End') { e.preventDefault(); e.stopPropagation(); onseek(e.key === 'Home' ? 0 : duration); } }}></div>
  <div class="time-ruler" aria-hidden="true">{#each ticks as tick, i}<span style:left={`${i / 6 * 100}%`}>{time(tick)}</span>{/each}</div>
  <canvas class="wave-canvas" bind:this={canvas}></canvas>
  <div class="wave-grid" aria-hidden="true">{#each ticks as tick, i}<i style:left={`${i / 6 * 100}%`}></i>{/each}</div>
  {#if current && current.end > view.start && current.start < view.end}
    <div class="loop-shade" style:left={`${Math.max(0, percent(current.start))}%`} style:width={`${Math.min(100, percent(current.end)) - Math.max(0, percent(current.start))}%`}></div>
    {#if current.start >= view.start && current.start <= view.end}
      <button class="loop-handle start" data-handle="start" style:left={`${percent(current.start)}%`} aria-label="Loop start" title="Drag loop start. Arrow keys adjust; Shift gives fine adjustment." onkeydown={(e) => shiftBoundary(e, 'start')}><span>A</span></button>
    {/if}
    {#if current.end >= view.start && current.end <= view.end}
      <button class="loop-handle end" data-handle="end" style:left={`${percent(current.end)}%`} aria-label="Loop end" title="Drag loop end. Arrow keys adjust; Shift gives fine adjustment." onkeydown={(e) => shiftBoundary(e, 'end')}><span>B</span></button>
    {/if}
  {/if}
  {#if position >= view.start && position <= view.end}
    <div class="playhead" style:left={`${percent(position)}%`} aria-hidden="true"><span></span></div>
  {/if}
</div>

<style>
  .overview:not(.keyboard-focus):focus,.wave-field:not(.keyboard-focus):focus,.wave-field:not(.keyboard-focus) :focus{outline:none}
  .seek-surface:focus-visible{outline:3px solid #8297d2}
  .wave-field.panning,.wave-field.panning .loop-handle{cursor:grabbing}
  .seek-surface{position:absolute;inset:0;outline-offset:-3px}.wave-canvas,.time-ruler{pointer-events:none}
  .overview-wrap{position:relative;padding-bottom:22px;margin-bottom:8px;flex-shrink:0}.overview{display:block;width:100%;height:42px;border:0;padding:5px 0;background:#e8edf6;position:relative;overflow:hidden;border-radius:5px;cursor:pointer}.overview canvas{width:100%;height:100%;display:block}.viewport{position:absolute;inset-block:0;border:1px solid #6d82bb;background:#526fc013;pointer-events:none;min-width:3px}.overview-playhead{position:absolute;inset-block:0;border-left:2px solid #526fc0}.overview-label,.overview-duration{position:absolute;bottom:3px;color:#758098;font-size:11px}.overview-duration{right:0;font-variant-numeric:tabular-nums}.section-lane{position:relative;height:25px;flex-shrink:0;background:#eef1f7;border-radius:4px 4px 0 0;overflow:hidden}.section-lane button{position:absolute;height:100%;padding:0 9px;border:0;border-right:2px solid #f3f5fa;text-align:left;font:500 11px inherit;color:#27334c;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;cursor:pointer}.wave-field{position:relative;height:auto;flex:1;min-height:120px;background:#233552;border-radius:0 0 6px 6px;overflow:hidden;touch-action:none;cursor:crosshair;user-select:none}.wave-field:focus-visible{outline:3px solid #7e92cd;outline-offset:3px}.wave-canvas{position:absolute;left:0;top:31px;width:100%;height:calc(100% - 46px)}.time-ruler{height:30px;position:absolute;inset:0 0 auto;font-size:11px;color:#bbcae2;font-variant-numeric:tabular-nums}.time-ruler span{position:absolute;top:8px;transform:translateX(-50%)}.time-ruler span:first-child{transform:none;padding-left:8px}.time-ruler span:last-child{transform:translateX(-100%);padding-right:8px}.wave-grid{position:absolute;inset:30px 0 0;pointer-events:none}.wave-grid i{position:absolute;height:100%;border-left:1px solid #bbccea0b}.loop-shade{position:absolute;inset-block:30px 0;background:#dda54e29;pointer-events:none}.loop-handle{position:absolute;top:29px;bottom:0;width:20px;padding:0;border:0;border-left:2px solid #e6b65e;background:transparent;cursor:ew-resize;touch-action:none;border-radius:0;color:#233552}.loop-handle span{position:absolute;top:1px;left:-2px;background:#e6b65e;border-radius:0 0 3px 0;font-size:10px;font-weight:700;padding:3px 5px}.loop-handle.end{transform:translateX(-18px);border-left:0;border-right:2px solid #e6b65e}.loop-handle.end span{left:auto;right:-2px;border-radius:0 0 0 3px}.playhead{position:absolute;inset-block:26px 0;border-left:2px solid #f9fcff;pointer-events:none}.playhead span{position:absolute;top:-1px;left:-5px;width:8px;height:8px;background:#fff;transform:rotate(45deg)}
  .overview{touch-action:none;user-select:none;cursor:grab}.overview.panning{cursor:grabbing}
  @media(max-height:740px){.overview-wrap{display:none}.wave-field{min-height:90px}}
</style>
