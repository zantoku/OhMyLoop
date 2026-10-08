<script lang="ts">
  import { onMount } from 'svelte';
  import type { Loop, Section, View } from './types';
  import { clamp, range, sectionColor, time, zoom } from './model';

  let { peaks, duration, sections, selection, position, view, onseek, onselection, onsection, onview }:
    { peaks: [number, number][]; duration: number; sections: Section[]; selection: Loop | null;
      position: number; view: View; onseek: (value: number) => void; onselection: (value: Loop) => void;
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
  let width = $state(800);
  let height = $state(200);
  const current = $derived(preview ?? selection);
  const span = $derived(view.end - view.start);
  const percent = (seconds: number) => (seconds - view.start) / span * 100;
  const ticks = $derived(Array.from({ length: 7 }, (_, i) => view.start + span * i / 6));

  function draw(target: HTMLCanvasElement | undefined, start: number, end: number) {
    if (!target || !peaks.length) return;
    const ctx = target.getContext('2d'); if (!ctx) return;
    const box = target.getBoundingClientRect(), dpr = window.devicePixelRatio || 1;
    target.width = Math.round(box.width * dpr); target.height = Math.round(box.height * dpr);
    ctx.scale(dpr, dpr); const w = box.width, h = box.height;
    ctx.clearRect(0, 0, w, h);
    const max = Math.max(.01, ...peaks.filter((_, i) => i % 16 === 0).map(p => Math.max(-p[0], p[1])));
    const scale = h * .39 / max;
    ctx.strokeStyle = 'rgba(209,221,245,.12)'; ctx.lineWidth = 1;
    ctx.beginPath(); ctx.moveTo(0, h / 2); ctx.lineTo(w, h / 2); ctx.stroke();
    ctx.fillStyle = target === overview ? '#adbcdb' : '#bccceb';
    for (let x = 0; x < w; x += 2) {
      const first = Math.floor((start + (end - start) * x / w) / duration * peaks.length);
      const last = Math.min(peaks.length, Math.max(first + 1, Math.ceil((start + (end - start) * (x + 2) / w) / duration * peaks.length)));
      let low = 0, high = 0;
      for (let i = first; i < last; i++) { const p = peaks[i]; if (p) { low = Math.min(low, p[0]); high = Math.max(high, p[1]); } }
      ctx.fillRect(x, h / 2 - high * scale, 1.4, Math.max(1, (high - low) * scale));
    }
  }
  $effect(() => { void width; void height; void peaks; draw(canvas, view.start, view.end); draw(overview, 0, duration); });
  onMount(() => {
    const observer = new ResizeObserver(() => { width = field.clientWidth; height = field.clientHeight; });
    observer.observe(field);
    // Cancel page scrolling even on WebViews that make wheel listeners passive by default.
    field.addEventListener('wheel', wheel, { passive: false });
    return () => { observer.disconnect(); field.removeEventListener('wheel', wheel); };
  });
  function point(event: MouseEvent): number {
    const box = field.getBoundingClientRect(); return clamp(view.start + (event.clientX - box.left) / box.width * span, 0, duration);
  }
  function wheel(event: WheelEvent) {
    if (!event.deltaY) return;
    event.preventDefault();
    if (drag) return;
    const box = field.getBoundingClientRect();
    if (!box.width) return;
    const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? box.height : 1;
    const factor = Math.exp(clamp(event.deltaY * unit, -240, 240) * .0025);
    const anchor = clamp((event.clientX - box.left) / box.width, 0, 1);
    onview(zoom(view, duration, factor, point(event), anchor));
  }
  function down(event: PointerEvent) {
    if (drag || (event.button !== 0 && event.button !== 1)) return;
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
      const originalSpan = drag.view.end - drag.view.start;
      const start = clamp(drag.view.start - (event.clientX - drag.x) / drag.width * originalSpan, 0, duration - originalSpan);
      onview({ start, end: start + originalSpan });
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
    const center = (event.clientX - box.left) / box.width * duration;
    const start = clamp(center - span / 2, 0, duration - span); onview({ start, end: start + span });
  }
</script>

<div class="overview-wrap">
  <button class="overview" onclick={panOverview} aria-label="Move waveform view" title="Click to move the detailed view">
    <canvas bind:this={overview}></canvas>
    <span class="viewport" style:left={`${view.start / duration * 100}%`} style:width={`${span / duration * 100}%`}></span>
    <span class="overview-playhead" style:left={`${position / duration * 100}%`}></span>
  </button>
  <span class="overview-label">Whole song</span><span class="overview-duration">{time(duration, 0)}</span>
</div>

<div class="section-lane" aria-label="Sections on timeline">
  {#each sections.filter(s => s.end > view.start && s.start < view.end) as section (section.id)}
    <button title={`${section.name}: ${time(section.start)} – ${time(section.end)}`} onclick={() => onsection(section)}
      style:left={`${Math.max(0, percent(section.start))}%`} style:width={`${Math.min(100, percent(section.end)) - Math.max(0, percent(section.start))}%`}
      style:background={sectionColor(section.id)}>{section.name}</button>
  {/each}
</div>

<div class="wave-field" class:panning={drag?.mode === 'pan'} bind:this={field} role="region" tabindex="-1"
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
  .wave-field.panning,.wave-field.panning .loop-handle{cursor:grabbing}
  .seek-surface{position:absolute;inset:0;outline-offset:-3px}.wave-canvas,.time-ruler{pointer-events:none}
  .overview-wrap{position:relative;padding-bottom:22px;margin-bottom:8px;flex-shrink:0}.overview{display:block;width:100%;height:42px;border:0;padding:5px 0;background:#e8edf6;position:relative;overflow:hidden;border-radius:5px;cursor:pointer}.overview canvas{width:100%;height:100%;display:block}.viewport{position:absolute;inset-block:0;border:1px solid #6d82bb;background:#526fc013;pointer-events:none;min-width:3px}.overview-playhead{position:absolute;inset-block:0;border-left:2px solid #526fc0}.overview-label,.overview-duration{position:absolute;bottom:3px;color:#758098;font-size:11px}.overview-duration{right:0;font-variant-numeric:tabular-nums}.section-lane{position:relative;height:25px;flex-shrink:0;background:#eef1f7;border-radius:4px 4px 0 0;overflow:hidden}.section-lane button{position:absolute;height:100%;padding:0 9px;border:0;border-right:2px solid #f3f5fa;text-align:left;font:500 11px inherit;color:#27334c;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;cursor:pointer}.wave-field{position:relative;height:auto;flex:1;min-height:120px;background:#233552;border-radius:0 0 6px 6px;overflow:hidden;touch-action:none;cursor:crosshair;user-select:none}.wave-field:focus-visible{outline:3px solid #7e92cd;outline-offset:3px}.wave-canvas{position:absolute;left:0;top:31px;width:100%;height:calc(100% - 46px)}.time-ruler{height:30px;position:absolute;inset:0 0 auto;font-size:11px;color:#bbcae2;font-variant-numeric:tabular-nums}.time-ruler span{position:absolute;top:8px;transform:translateX(-50%)}.time-ruler span:first-child{transform:none;padding-left:8px}.time-ruler span:last-child{transform:translateX(-100%);padding-right:8px}.wave-grid{position:absolute;inset:30px 0 0;pointer-events:none}.wave-grid i{position:absolute;height:100%;border-left:1px solid #bbccea0b}.loop-shade{position:absolute;inset-block:30px 0;background:#dda54e29;pointer-events:none}.loop-handle{position:absolute;top:29px;bottom:0;width:20px;padding:0;border:0;border-left:2px solid #e6b65e;background:transparent;cursor:ew-resize;touch-action:none;border-radius:0;color:#233552}.loop-handle span{position:absolute;top:1px;left:-2px;background:#e6b65e;border-radius:0 0 3px 0;font-size:10px;font-weight:700;padding:3px 5px}.loop-handle.end{transform:translateX(-18px);border-left:0;border-right:2px solid #e6b65e}.loop-handle.end span{left:auto;right:-2px;border-radius:0 0 0 3px}.playhead{position:absolute;inset-block:26px 0;border-left:2px solid #f9fcff;pointer-events:none}.playhead span{position:absolute;top:-1px;left:-5px;width:8px;height:8px;background:#fff;transform:rotate(45deg)}
  @media(max-height:740px){.overview-wrap{display:none}.wave-field{min-height:90px}}
</style>
