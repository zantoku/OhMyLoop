<script lang="ts">
  // The floating toolbar for the selected measure (concept MB): just below the
  // ruler, centred on the measure, with a notch pointing up at it. Against an
  // edge it stays inside the waveform and the notch slides; once the measure
  // has scrolled away it collapses to an edge tab ("m. 37") that brings it
  // back (map-toolbar.ts). It fades while a line is dragged.
  // It is one Tab stop; when it was reached by keyboard, ←/→, Home and End
  // move between its buttons (data-nav-keys, key-routing.ts). Otherwise the
  // arrows keep seeking, as after a click. Panels open below it as popovers.
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import Popover from './Popover.svelte';
  import { currentModality } from './input-modality';
  import { ownsKey, trackKeyboardFocus } from './key-routing';
  import { placeToolbar } from './map-toolbar';
  import { measureName, propertiesText, type Check } from './measure-tools';
  import { time } from './model';
  import type { FieldGeometry } from './MapWaveform.svelte';
  import type { UnmappedRange } from './map-ranges';
  import type { MeasureView } from './timing';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  export type Panel = 'properties' | 'more' | null;
  let { measure, freeRange, rate, geometry, fading, panel, split, join, shift, keymap, available, run, onpanel, onsplithover, properties, more }:
    { measure: MeasureView | null; freeRange: UnmappedRange | null; rate: number; geometry: FieldGeometry; fading: boolean; panel: Panel;
      split: Check; join: Check; shift: Check; keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void;
      onpanel: (panel: Panel) => void; onsplithover: (hovering: boolean) => void; properties: Snippet; more: Snippet } = $props();

  let bar = $state<HTMLDivElement>(), width = $state(420);
  let propertiesButton = $state<HTMLButtonElement>(), moreButton = $state<HTMLButtonElement>();
  const range = $derived(measure ?? freeRange);
  const placement = $derived(range ? placeToolbar({ start: geometry.x(range.start), end: geometry.x(range.end), field: geometry.width, width }) : null);
  const name = $derived(measure ? measureName(measure) : 'Free time');
  const freeText = $derived(freeRange ? `${time(freeRange.start / rate)} – ${time(freeRange.end / rate)}` : '');
  // A check that fails keeps the button focusable with its reason (aria-disabled).
  const tip = (id: ButtonId, check: Check) => check.ok ? keymap.title(id) : check.reason;
  const guarded = (id: ButtonId) => () => { if (available(id)) run(id); };
  const off = (id: ButtonId, check: Check = { ok: true, detail: '' }) => !check.ok || !available(id);

  function navigate(event: KeyboardEvent) {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key) || !ownsKey(event.target, event.key, currentModality())) return;
    const tools = [...bar!.querySelectorAll<HTMLButtonElement>(':scope > [data-tool], :scope > .shift > [data-tool]')];
    const index = tools.indexOf(document.activeElement as HTMLButtonElement);
    if (index < 0) return;
    event.preventDefault(); event.stopPropagation();
    const next = { ArrowLeft: index - 1, ArrowRight: index + 1, Home: 0, End: tools.length - 1 }[event.key]!;
    tools[(next + tools.length) % tools.length].focus();
  }
  // Roving focus: one button takes the Tab stop, and it's the first one.
  $effect(() => {
    void range; void panel;
    if (!bar || bar.contains(document.activeElement)) return;
    bar.querySelectorAll<HTMLButtonElement>('[data-tool]').forEach((tool, i) => { tool.tabIndex = i === 0 ? 0 : -1; });
  });
  // A panel never stays open for a measure that has scrolled away.
  $effect(() => { if (placement?.kind === 'tab' && panel) onpanel(null); });
  function rove(event: FocusEvent) {
    if (bar) trackKeyboardFocus(bar, event, currentModality());
    const tool = (event.target as HTMLElement).closest<HTMLButtonElement>('[data-tool]');
    if (!tool || !bar) return;
    for (const other of bar.querySelectorAll<HTMLButtonElement>('[data-tool]')) other.tabIndex = other === tool ? 0 : -1;
  }
</script>

{#if placement?.kind === 'bar'}
  <div class="toolbar" class:fading bind:this={bar} bind:offsetWidth={width} role="toolbar" tabindex="-1" aria-label={`${measure ? (measure.number === null ? 'Pickup' : `Measure ${measure.label}`) : 'Free time'} tools`}
    data-nav-keys="ArrowLeft ArrowRight Home End" style:left={`${placement.left}px`} onkeydown={navigate} onfocusin={rove} onfocusout={event => { if (bar) trackKeyboardFocus(bar, event, currentModality()); }}>
    <span class="notch" style:left={`${placement.notch}px`} aria-hidden="true"></span>
    <strong class="name">{name}</strong>
    {#if measure}
      <span class="sep"></span>
      <button data-tool bind:this={propertiesButton} class="tool" class:open={panel === 'properties'} aria-haspopup="dialog" aria-expanded={panel === 'properties'}
        disabled={!available('map_properties')} title={keymap.title('map_properties')} onclick={() => run('map_properties')}>{propertiesText(measure)}<span class="caret" aria-hidden="true">▾</span></button>
      <span class="sep"></span>
      <button data-tool class="tool" aria-disabled={off('map_split', split)} title={tip('map_split', split)} onclick={guarded('map_split')}
        onpointerenter={() => onsplithover(true)} onpointerleave={() => onsplithover(false)} onfocus={() => onsplithover(true)} onblur={() => onsplithover(false)}>
        <Icon name="split" size={16}/>{split.ok ? split.detail : 'Split'}</button>
      <button data-tool class="tool" aria-disabled={off('map_merge', join)} title={join.ok ? `${join.detail}. ${keymap.title('map_merge')}` : join.reason} onclick={guarded('map_merge')}><Icon name="join" size={16}/>Join</button>
      <span class="sep"></span>
      <span class="shift" role="group" aria-label="Shift this and the following barlines">
        <button data-tool class="tool icon" aria-label="Shift earlier" aria-disabled={off('map_shift_left', shift)} title={tip('map_shift_left', shift)} onclick={guarded('map_shift_left')}><Icon name="left" size={16}/></button>
        <span class="caption" aria-hidden="true">Shift</span>
        <button data-tool class="tool icon" aria-label="Shift later" aria-disabled={off('map_shift_right', shift)} title={tip('map_shift_right', shift)} onclick={guarded('map_shift_right')}><Icon name="right" size={16}/></button>
      </span>
      <span class="sep"></span>
      <button data-tool class="tool icon lock" class:locked={measure.protected} aria-pressed={measure.protected} aria-label={`${measure.protected ? 'Unlock' : 'Lock'} ${measureName(measure)}`}
        disabled={!available('map_protect')} title={keymap.title('map_protect')} onclick={() => run('map_protect')}><Icon name={measure.protected ? 'lock' : 'unlock'} size={15}/></button>
      <button data-tool bind:this={moreButton} class="tool icon" class:open={panel === 'more'} aria-label="More measure tools" aria-haspopup="dialog" aria-expanded={panel === 'more'}
        disabled={!available('map_more')} title={keymap.title('map_more')} onclick={() => run('map_more')}><Icon name="more" size={16}/></button>
      <Popover open={panel === 'properties'} label={`${measureName(measure)} properties`} anchor={propertiesButton} align="start" width={340} onclose={() => onpanel(null)}>{@render properties()}</Popover>
      <Popover open={panel === 'more'} label={`${measureName(measure)}: more`} anchor={moreButton} align="end" width={340} onclose={() => onpanel(null)}>{@render more()}</Popover>
    {:else if freeRange}
      <span class="range">{freeText}</span>
      <span class="sep"></span>
      <button data-tool bind:this={propertiesButton} class="tool" class:open={panel === 'properties'} aria-haspopup="dialog" aria-expanded={panel === 'properties'}
        disabled={!available('map_properties')} title={`Count this free-time range as one measure${keymap.shortcut('map_properties') ? ` (${keymap.shortcut('map_properties')})` : ''}`} onclick={() => run('map_properties')}>Count as a measure<span class="caret" aria-hidden="true">▾</span></button>
      <Popover open={panel === 'properties'} label="Count free time as a measure" anchor={propertiesButton} align="start" width={300} onclose={() => onpanel(null)}>{@render properties()}</Popover>
    {/if}
  </div>
{:else if placement?.kind === 'tab'}
  <button class="edge-tab {placement.side}" class:fading aria-label={`${measure ? measureName(measure) : 'Free time'} is selected, ${placement.side === 'left' ? 'earlier' : 'later'} in the song. Show it`}
    title={keymap.title('map_fit')} disabled={!available('map_fit')} onclick={() => run('map_fit')}>{measure ? measureName(measure) : 'Free time'}<span class="notch" aria-hidden="true"></span></button>
{/if}

<style>
  .toolbar,.edge-tab{position:absolute;top:32px;z-index:3;pointer-events:auto;border:1px solid var(--border-strong);border-radius:9px;background:var(--surface);color:var(--text);box-shadow:0 6px 18px #0000004d;transition:opacity .12s}
  .toolbar{display:flex;align-items:center;gap:2px;padding:4px;white-space:nowrap}
  .fading{opacity:.15;pointer-events:none}
  .notch{position:absolute;top:-6px;width:10px;height:10px;margin-left:-5px;transform:rotate(45deg);background:var(--surface);border:0 solid var(--border-strong);border-left-width:1px;border-top-width:1px}
  .name{padding:0 8px 0 6px;font-size:13px;font-variant-numeric:tabular-nums}
  .range{padding-right:6px;font-size:12px;color:var(--text-secondary);font-variant-numeric:tabular-nums}
  .sep{width:1px;align-self:stretch;margin:4px 2px;background:var(--border)}
  .tool{height:30px;display:inline-flex;align-items:center;gap:6px;padding:0 9px;border:0;border-radius:6px;background:transparent;color:var(--text);font-size:12px;font-weight:600}
  .tool.icon{padding:0 7px}
  .tool:hover:not(:disabled,[aria-disabled=true]),.tool.open{background:var(--selected)}
  .tool[aria-disabled=true]{opacity:.45}
  .caret{color:var(--text-secondary)}
  .shift{display:inline-flex;align-items:center}
  .caption{font-size:11px;font-weight:600;color:var(--text-secondary)}
  .lock{color:var(--text-secondary)}
  .lock.locked{background:var(--amber-bg);color:var(--amber-text)}
  .edge-tab{height:34px;display:inline-flex;align-items:center;padding:0 12px;font-size:12px;font-weight:700;font-variant-numeric:tabular-nums}
  .edge-tab.left{left:6px}.edge-tab.right{right:6px}
  .edge-tab .notch{top:50%;margin:-5px 0 0}
  .edge-tab.left .notch{left:-6px;border-width:0 0 1px 1px}
  .edge-tab.right .notch{right:-6px;border-width:1px 1px 0 0}
  .edge-tab:hover:not(:disabled){background:var(--hover)}
</style>
