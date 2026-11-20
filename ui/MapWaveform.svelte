<script lang="ts" module>
  /** Pixel geometry of the detailed waveform, for overlays such as the floating toolbar. */
  export type FieldGeometry = { width: number; x: (frame: number) => number };
</script>

<script lang="ts">
  // The Measure map's waveform (concept MB): a whole-song minimap, then the
  // detailed field with a measure-number ruler, barlines, dotted beat lines
  // across the full height, detected beats as amber marks along the bottom,
  // a draft in dashed green, hatched free time and the playhead. On the
  // selected measure, grips on its barlines and diamonds on its beats are the
  // drag handles, and only they are Tab stops (other barlines still drag).
  // A drag is one edit; arrow keys nudge a focused handle
  // (Shift for fine steps) and Delete removes a focused beat's anchor.
  // Overlays (the toolbar) render above the field without being clipped.
  import { onMount, type Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import { markersForView } from './marker-density';
  import { clamp, time } from './model';
  import { drawWaveform, sourcePoint, panView, overviewView, wheelView } from './timeline';
  import { unmappedRanges, type UnmappedRange } from './map-ranges';
  import { adjacentMeasure, measureAtFrame, type GridPoint, type MapEdit, type MapView, type MeasureView } from './timing';
  import type { View } from './types';
  type Line = Extract<MapEdit, { type: 'move_boundary' | 'set_beat' }>;
  let { peaks, duration, rate, position, view, mapView, draft = false, selected, freeRange = null, selectedBeat = null, candidates = [], splitAt = null,
    disabled = false, onview, onseek, onselect, ongap, onbeat, onedit, ondragging, overlay }:
    { peaks: [number, number][]; duration: number; rate: number; position: number; view: View; mapView: MapView | null; draft?: boolean;
      selected: string | null; freeRange?: UnmappedRange | null; selectedBeat?: number | null; candidates?: { frame: number; downbeat: boolean }[]; splitAt?: number | null;
      disabled?: boolean; onview: (view: View) => void; onseek: (seconds: number) => void; onselect: (id: string) => void; ongap: (frame: number) => void;
      onbeat: (quarter: number) => void; onedit: (edit: MapEdit) => void; ondragging: (dragging: boolean) => void; overlay?: Snippet<[FieldGeometry]> } = $props();

  const RULER = 24, ACCENT = '#8ea8ed';
  let canvas: HTMLCanvasElement, overview: HTMLCanvasElement, field: HTMLDivElement;
  let size = $state(0), width = $state(800), overviewPointer = $state<number | null>(null);
  type Drag = { pointer: number; x: number; width: number; view: View; edit: Line | null; pan: boolean };
  let drag = $state<Drag | null>(null), draggedFrame = $state<number | null>(null);

  const measures = $derived(mapView?.measures ?? []);
  const span = $derived(view.end - view.start);
  const pct = (frame: number) => (frame / rate - view.start) / span * 100;
  const px = (frame: number) => (frame / rate - view.start) / span * width;
  const shown = (start: number, end: number) => end / rate > view.start && start / rate < view.end;
  const visible = $derived(measures.filter(m => shown(m.start, m.end)));
  const measure = $derived(measures.find(m => m.id === selected) ?? null);
  // Without any measure there is no free time to show, only the song.
  const gaps = $derived(measures.length ? unmappedRanges(measures, Math.round(duration * rate)) : []);
  const playheadShown = $derived(position >= view.start && position <= view.end);
  // Labels thin with density like Practice's ruler; every barline stays drawn until they'd touch.
  const labels = $derived(markersForView(measures.map(m => ({ frame: m.start, measure: m })), view.start * rate, view.end * rate, width, 34).markers);
  const labelled = $derived(new Set(labels.map(l => l.measure.id)));
  const starts = $derived(new Set(measures.map(m => m.start_id)));
  const barlines = $derived(markersForView([...new Set(measures.flatMap(m => [m.start, m.end]))].sort((a, b) => a - b).map(frame => ({ frame })), view.start * rate, view.end * rate, width, 10));
  const beats = $derived(markersForView(measures.flatMap(m => m.grid.filter(p => p.quarter > 0)), view.start * rate, view.end * rate, width, 12));
  const detected = $derived(markersForView(candidates, view.start * rate, view.end * rate, width, 5).markers);
  const handles = $derived(visible.filter(m => m.id === selected || (m.end - m.start) / rate / span * width >= 16));
  // Handles need room: zoom in (Fit measure) to drag inside a narrow measure.
  const roomy = $derived(!!measure && (measure.end - measure.start) / rate / span * width >= 28);
  const diamonds = $derived(measure && roomy ? measure.grid.filter(p => p.quarter > 0) : []);
  // Times label free time only, away from measure numbers and barlines.
  const ticks = $derived(Array.from({ length: 7 }, (_, i) => view.start + span * i / 6).filter(t => !measureAtFrame(measures, Math.round(t * rate)) &&
    !measures.some(m => [m.start, m.end].some(f => Math.abs(f / rate - t) / span * width < 55))));
  const mapped = $derived(measures.length ? { start: measures[0].start / rate / duration * 100, end: measures.at(-1)!.end / rate / duration * 100 } : null);
  const called = (m: MeasureView) => m.number === null ? 'the pickup' : `measure ${m.label}`;
  const locked = (m: MeasureView) => disabled || m.protected;
  // The selected measure's own barlines carry grips; dragging its end keeps it selected.
  const edges = $derived(new Set(measure ? [measure.start_id, measure.end_id] : []));
  const owner = (m: MeasureView, boundary: string) => measure && edges.has(boundary) ? measure.id : m.id;

  /** Where a dragged line is shown while the pointer moves. */
  function at(frame: number, matches: (edit: Line) => boolean) { return drag?.edit && draggedFrame !== null && matches(drag.edit) ? draggedFrame : frame; }
  const boundaryAt = (id: string, frame: number) => at(frame, edit => edit.type === 'move_boundary' && edit.id === id);
  const beatAt = (m: MeasureView, p: GridPoint) => at(p.frame, edit => edit.type === 'set_beat' && edit.measure === m.id && edit.offset.num / edit.offset.den === p.quarter);
  const offset = (quarter: number) => ({ num: Math.round(quarter * 960), den: 960 });

  const frameAt = (event: MouseEvent) => Math.round(sourcePoint(view, duration, field.getBoundingClientRect(), event.clientX) * rate);
  function seekAt(seconds: number) {
    const frame = Math.round(seconds * rate), target = measureAtFrame(measures, frame);
    if (target) onselect(target.id); else if (gaps.some(g => g.start <= frame && frame < g.end)) ongap(frame);
    onseek(seconds);
  }
  function seekKey(event: KeyboardEvent) {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
    event.preventDefault(); event.stopPropagation();
    if (event.key === 'Home' || event.key === 'End') { seekAt(event.key === 'Home' ? 0 : duration); return; }
    const direction = event.key === 'ArrowRight' ? 1 : -1;
    if (!event.shiftKey && measures.length) {
      const target = adjacentMeasure(measures, Math.round(position * rate), direction);
      if (target) seekAt(target.start / rate);
    } else seekAt(clamp(position + direction * (event.shiftKey ? .1 : 5), 0, duration));
  }

  $effect(() => { void size; void peaks; drawWaveform(canvas, peaks, duration, view.start, view.end); drawWaveform(overview, peaks, duration, 0, duration, true); });
  onMount(() => {
    const observer = new ResizeObserver(() => { size++; width = Math.max(1, field.clientWidth); });
    observer.observe(field);
    const wheel = (event: WheelEvent) => { event.preventDefault(); if (!drag && overviewPointer === null) onview(wheelView(view, duration, field.getBoundingClientRect(), event)); };
    field.addEventListener('wheel', wheel, { passive: false });
    return () => { observer.disconnect(); field.removeEventListener('wheel', wheel); };
  });

  function down(event: PointerEvent) {
    if (drag || overviewPointer !== null || ![0, 1].includes(event.button)) return;
    const target = (event.target as HTMLElement).closest<HTMLElement>('[data-boundary],[data-quarter]');
    const id = target?.dataset.measure ?? null;
    let edit: Line | null = null;
    if (target && id && event.button === 0) {
      if (target.getAttribute('aria-disabled') === 'true') return;
      onselect(id);
      if (target.dataset.boundary) edit = { type: 'move_boundary', id: target.dataset.boundary, frame: frameAt(event) };
      else { const quarter = Number(target.dataset.quarter); onbeat(quarter); edit = { type: 'set_beat', measure: id, id: target.dataset.beat || null, frame: frameAt(event), offset: offset(quarter) }; }
    }
    drag = { pointer: event.pointerId, x: event.clientX, width: field.getBoundingClientRect().width, view: { ...view }, edit, pan: event.button === 1 };
    draggedFrame = edit ? frameAt(event) : null;
    if (edit) ondragging(true);
    field.setPointerCapture(event.pointerId);
    if (edit && target) target.focus(); else field.focus();
    event.preventDefault();
  }
  function move(event: PointerEvent) {
    if (!drag || event.pointerId !== drag.pointer) return;
    if (drag.pan) onview(panView(drag.view, duration, event.clientX - drag.x, drag.width));
    else if (drag.edit) draggedFrame = frameAt(event);
  }
  function up(event: PointerEvent) {
    if (!drag || event.pointerId !== drag.pointer) return;
    if (!drag.pan) {
      if (drag.edit && Math.abs(event.clientX - drag.x) > 2) onedit({ ...drag.edit, frame: frameAt(event) });
      else if (!drag.edit) seekAt(frameAt(event) / rate);
    }
    cancel(event);
  }
  function cancel(event: PointerEvent) {
    if (!drag || event.pointerId !== drag.pointer) return;
    if (drag.edit) ondragging(false);
    drag = null; draggedFrame = null;
    if (field.hasPointerCapture(event.pointerId)) field.releasePointerCapture(event.pointerId);
  }
  function nudge(event: KeyboardEvent, m: MeasureView, frame: number, edit: (frame: number) => MapEdit, remove?: MapEdit) {
    if ((event.key === 'Delete' || event.key === 'Backspace') && remove) { event.preventDefault(); event.stopPropagation(); if (!locked(m)) onedit(remove); return; }
    if (!['ArrowLeft', 'ArrowRight'].includes(event.key)) return;
    event.preventDefault(); event.stopPropagation();
    if (locked(m)) return;
    const next = clamp(frame + Math.round(rate * (event.shiftKey ? .001 : .01)) * (event.key === 'ArrowRight' ? 1 : -1), 0, Math.round(duration * rate));
    const change = edit(next);
    onselect(change.type === 'move_boundary' && measure && edges.has(change.id) ? measure.id : m.id); onedit(change);
  }
  function overviewPan(event: MouseEvent) { onview(overviewView(view, duration, overview.getBoundingClientRect(), event.clientX)); }
  function overviewDown(event: PointerEvent) { if (event.button !== 0 || drag) return; overviewPointer = event.pointerId; (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId); event.preventDefault(); overviewPan(event); }
  function overviewEnd(event: PointerEvent) { if (overviewPointer !== event.pointerId) return; overviewPointer = null; const button = event.currentTarget as HTMLElement; if (button.hasPointerCapture(event.pointerId)) button.releasePointerCapture(event.pointerId); }
  const geometry = $derived<FieldGeometry>({ width, x: px });
</script>

<div class="map-wave" style:--accent={ACCENT}>
  <button class="overview" class:panning={overviewPointer !== null} aria-label="Move measure-map view" title="Click or drag to move the detailed view" onclick={overviewPan}
    onpointerdown={overviewDown} onpointermove={e => { if (e.pointerId === overviewPointer) overviewPan(e); }} onpointerup={e => { if (e.pointerId === overviewPointer) overviewPan(e); overviewEnd(e); }}
    onpointercancel={overviewEnd} onlostpointercapture={overviewEnd}>
    <canvas bind:this={overview}></canvas>
    {#if mapped}<span class="mapped" class:draft style:left={`${mapped.start}%`} style:width={`${mapped.end - mapped.start}%`}></span>{/if}
    <span class="map-viewport" style:left={`${view.start / duration * 100}%`} style:width={`${span / duration * 100}%`}></span>
    <i style:left={`${position / duration * 100}%`}></i>
  </button>

  <div class="stage">
    <!-- Before the field, so Tab reaches the toolbar first; drawn above it (z-index). -->
    <div class="overlay">{@render overlay?.(geometry)}</div>
    <div class="field" class:panning={drag?.pan} class:draft bind:this={field} role="region" tabindex="-1" data-home
      aria-label="Measure-map waveform. Click to seek and select a measure; drag barlines and beat diamonds to correct timing. Scroll to zoom, middle-drag to pan."
      onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={cancel} onlostpointercapture={cancel} onauxclick={e => { if (e.button === 1) e.preventDefault(); }}>
      <div class="seek" class:offscreen={!playheadShown} role="slider" tabindex="0" data-owns-keys="ArrowLeft ArrowRight Home End" aria-label="Measure-map song position"
        aria-valuemin={0} aria-valuemax={duration} aria-valuenow={position} aria-valuetext={time(position)} onkeydown={seekKey}></div>
      <div class="ruler" aria-hidden="true"></div>
      <canvas class="wave" bind:this={canvas}></canvas>
      {#each gaps.filter(g => shown(g.start, g.end)) as gap (gap.start)}
        {@const chosen = freeRange?.start === gap.start}
        <div class="free" class:chosen style:left={`${Math.max(0, pct(gap.start))}%`} style:width={`${Math.min(100, pct(gap.end)) - Math.max(0, pct(gap.start))}%`}>{#if px(Math.min(gap.end, view.end * rate)) - px(Math.max(gap.start, view.start * rate)) >= 64}<span>free time</span>{/if}</div>
      {/each}
      {#if measure && shown(measure.start, measure.end)}<div class="shade" style:left={`${Math.max(0, pct(measure.start))}%`} style:width={`${Math.min(100, pct(measure.end)) - Math.max(0, pct(measure.start))}%`}></div>{/if}
      {#each beats.markers as p, i (i)}<i class="beat-line" style:left={`${pct(p.frame)}%`}></i>{/each}
      {#each barlines.markers as line (line.frame)}<i class="barline" style:left={`${pct(line.frame)}%`}></i>{/each}
      {#if measure}{#each [boundaryAt(measure.start_id, measure.start), boundaryAt(measure.end_id, measure.end)] as edge}<i class="barline edge" style:left={`${pct(edge)}%`}></i>{/each}{/if}
      {#each labels as { measure: m } (m.id)}
        <span class="label" class:current={m.id === selected} style:left={`${Math.max(0, pct(m.start))}%`}>{m.number === null && px(m.end) - px(m.start) < 56 ? '' : m.label}{#if m.protected}<span class="lock" aria-label={`${m.label} locked`}><Icon name="lock" size={12}/></span>{/if}</span>
      {/each}
      {#each ticks as tick, i}<span class="label time" class:first={i === 0 && tick === view.start} class:last={tick === view.end} style:left={`${(tick - view.start) / span * 100}%`}>{time(tick)}</span>{/each}
      {#each detected as d, i (i)}<i class="detected" class:downbeat={d.downbeat} style:left={`${pct(d.frame)}%`}></i>{/each}
      {#if splitAt !== null}<i class="split-preview" style:left={`${pct(splitAt)}%`}></i>{/if}
      {#each handles as m (m.id)}
        {#if m.start / rate >= view.start}
          <button class="grip" class:current={roomy && edges.has(m.start_id)} tabindex={edges.has(m.start_id) ? 0 : -1} data-owns-keys="ArrowLeft ArrowRight" data-boundary={m.start_id} data-measure={owner(m, m.start_id)} style:left={`${pct(boundaryAt(m.start_id, m.start))}%`}
            aria-label={`Barline before ${called(m)}`} aria-disabled={locked(m)} title="Drag to the downbeat; arrow keys nudge, Shift for fine steps"
            onclick={() => onselect(owner(m, m.start_id))} onkeydown={e => nudge(e, m, m.start, frame => ({ type: 'move_boundary', id: m.start_id, frame }))}></button>
        {/if}
        {#if !starts.has(m.end_id) && m.end / rate <= view.end}
          <button class="grip" class:current={roomy && edges.has(m.end_id)} tabindex={edges.has(m.end_id) ? 0 : -1} data-owns-keys="ArrowLeft ArrowRight" data-boundary={m.end_id} data-measure={m.id} style:left={`${pct(boundaryAt(m.end_id, m.end))}%`}
            aria-label={`Barline after ${called(m)}`} aria-disabled={locked(m)} title="Drag to the barline; arrow keys nudge, Shift for fine steps"
            onclick={() => onselect(m.id)} onkeydown={e => nudge(e, m, m.end, frame => ({ type: 'move_boundary', id: m.end_id, frame }))}></button>
        {/if}
      {/each}
      {#if measure}
        {#each diamonds as p (p.quarter)}
          <button class="diamond" class:anchor={!!p.id} class:chosen={selectedBeat === p.quarter} data-owns-keys="ArrowLeft ArrowRight Delete Backspace" data-quarter={p.quarter} data-beat={p.id ?? ''} data-measure={measure.id}
            style:left={`${pct(beatAt(measure, p))}%`} aria-label={`${measure.number === null ? 'Pickup' : `Measure ${measure.label}`} beat ${p.label}${p.id ? ', anchored' : ''}`} aria-disabled={locked(measure)}
            title={`${p.id ? 'Anchored beat' : 'Beat'}: drag to correct its timing; arrow keys nudge${p.id ? ', Delete removes the anchor' : ''}`}
            onclick={() => onbeat(p.quarter)} onfocus={() => onbeat(p.quarter)}
            onkeydown={e => nudge(e, measure!, p.frame, frame => ({ type: 'set_beat', measure: measure!.id, id: p.id, frame, offset: offset(p.quarter) }), p.id ? { type: 'remove_beat', measure: measure!.id, id: p.id } : undefined)}></button>
        {/each}
      {/if}
      {#if playheadShown}<div class="playhead" style:left={`${pct(position * rate)}%`} aria-hidden="true"></div>{/if}
      {#if barlines.stride > 1}<span class="detail" aria-hidden="true">Every {barlines.stride} barlines · zoom in for detail</span>
      {:else if beats.stride > 1}<span class="detail" aria-hidden="true">Every {beats.stride} beats · zoom in for detail</span>{/if}
    </div>
  </div>
</div>

<style>
  .map-wave{flex:1;min-height:0;display:flex;flex-direction:column;gap:6px}
  .overview{position:relative;flex:none;display:block;width:100%;height:30px;padding:3px 0;border:0;border-radius:6px;background:var(--overview-bg);overflow:hidden;touch-action:none;cursor:grab}
  .overview.panning{cursor:grabbing}
  .overview canvas{width:100%;height:100%;display:block}
  .mapped{position:absolute;bottom:0;height:4px;background:var(--text-secondary);opacity:.7}.mapped.draft{background:#7fd1a6;opacity:1}
  .map-viewport{position:absolute;inset-block:0;border:1.5px solid var(--blue);border-radius:4px;background:var(--blue-wash);min-width:3px;pointer-events:none}
  .overview i{position:absolute;inset-block:0;border-left:2px solid var(--text)}
  .stage{position:relative;flex:1;min-height:0;display:flex}
  .field{position:relative;flex:1;min-height:150px;background:var(--wave-bg);border-radius:8px;overflow:hidden;touch-action:none;cursor:crosshair}
  .field.panning{cursor:grabbing}
  .seek{position:absolute;inset:0}
  .ruler{position:absolute;inset:0 0 auto;height:24px;background:#ffffff0a;border-bottom:1px solid #ffffff14;pointer-events:none}
  .wave{position:absolute;left:0;top:30px;width:100%;height:calc(100% - 48px);pointer-events:none}
  .free{position:absolute;inset-block:24px 0;background:repeating-linear-gradient(135deg,#ffffff12 0 6px,transparent 6px 12px);pointer-events:none}
  .free.chosen{outline:2px solid var(--accent);outline-offset:-2px;background-color:#8ea8ed1f}
  .free span{position:absolute;top:6px;left:6px;font-size:11px;font-style:italic;color:#aab6cc;white-space:nowrap}
  .shade{position:absolute;inset-block:0;background:#8ea8ed29;pointer-events:none}
  .beat-line{position:absolute;top:24px;bottom:0;width:1px;background-image:repeating-linear-gradient(180deg,#e1e8f380 0 2px,transparent 2px 6px);pointer-events:none}
  .draft .beat-line{background-image:repeating-linear-gradient(180deg,#7fd1a6bf 0 2px,transparent 2px 6px)}
  .barline{position:absolute;inset-block:0;width:1px;background:#e1e8f39e;pointer-events:none}
  .barline.edge{width:2px;margin-left:-1px;background:var(--accent)}
  .draft .barline,.draft .barline.edge{width:2px;background:repeating-linear-gradient(180deg,#7fd1a6 0 6px,transparent 6px 10px)}
  .label{position:absolute;top:4px;display:flex;align-items:center;gap:4px;padding-left:6px;font-size:12px;font-weight:600;color:#c9d4e8;white-space:nowrap;pointer-events:none}
  .label.current{font-weight:800;color:#fff}
  .draft .label{color:#7fd1a6}
  .label.time{font-weight:500;color:#8b98b0;transform:translateX(-50%)}
  .label.time.first{transform:none}.label.time.last{transform:translateX(-100%);padding-right:8px}
  .lock{display:inline-flex;color:#c9d4e8}
  .detected{position:absolute;bottom:5px;width:0;height:0;margin-left:-3px;border-inline:3.5px solid transparent;border-bottom:7px solid #e6c18acc;pointer-events:none}
  .detected.downbeat{margin-left:-4px;border-inline-width:4.5px;border-bottom:9px solid #e6c18a}
  .split-preview{position:absolute;top:24px;bottom:0;width:2px;margin-left:-1px;background-image:repeating-linear-gradient(180deg,#ffd27a 0 5px,transparent 5px 9px);pointer-events:none}
  .grip{position:absolute;top:24px;bottom:0;width:12px;margin-left:-6px;padding:0;border:0;background:transparent;cursor:ew-resize;touch-action:none}
  .grip.current::after{content:'';position:absolute;left:0;top:calc(50% - 26px);width:12px;height:28px;border-radius:4px;background:var(--accent);box-shadow:0 0 0 2px var(--wave-bg)}
  .grip[aria-disabled=true]{cursor:default}
  .diamond{position:absolute;bottom:22px;width:12px;height:12px;margin-left:-6px;padding:0;border:2px solid var(--accent);border-radius:3px;background:var(--wave-bg);transform:rotate(45deg);cursor:ew-resize;touch-action:none}
  .diamond.anchor{background:var(--accent)}
  .diamond.chosen{box-shadow:0 0 0 3px #8ea8ed66}
  .diamond[aria-disabled=true]{opacity:.55;cursor:default}
  .playhead{position:absolute;inset-block:0;width:2px;margin-left:-1px;background:#fff;pointer-events:none}
  .playhead::before{content:'';position:absolute;top:0;left:-5px;border-inline:6px solid transparent;border-top:8px solid #fff}
  .detail{position:absolute;right:10px;top:30px;padding:3px 5px;border-radius:3px;background:#233552d9;color:#afbdd4;font-size:10px;pointer-events:none}
  .overlay{position:absolute;inset:0;z-index:3;pointer-events:none}
  /* Focus shows on the drawn element: the playhead, a grip or a diamond. */
  :global([data-modality=keyboard]) .seek:focus-visible{outline:none}
  :global([data-modality=keyboard]) .seek.offscreen:focus-visible{outline:2px solid var(--focus);outline-offset:-2px}
  :global([data-modality=keyboard]) .seek:focus-visible ~ .playhead{outline:2px solid var(--focus);outline-offset:1px}
  :global([data-modality=keyboard]) .grip:focus-visible{outline:none}
  :global([data-modality=keyboard]) .grip:focus-visible::after{content:'';position:absolute;left:0;top:calc(50% - 26px);width:12px;height:28px;border-radius:4px;background:var(--accent);outline:2px solid var(--focus);outline-offset:1px}
  :global([data-modality=keyboard]) .diamond:focus-visible{outline:2px solid var(--focus);outline-offset:1px}
</style>
