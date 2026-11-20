<script lang="ts">
  // Whole-song map settings (decision 6, D1–D3): measure numbering and beat
  // grouping, each previewed in green on the waveform before Apply, and
  // clearing the map, with Undo. A modal dialog that owns its keys: Esc
  // unwinds the clear confirmation, then closes; the leading X closes too.
  // Focus returns to where it was when it closes.
  import { tick } from 'svelte';
  import Icon from './Icon.svelte';
  import InlineConfirm from './InlineConfirm.svelte';
  import { focusHome } from './home-focus';
  import { readGrouping } from './meter-form';
  import { TransientStack, provideTransientStack } from './transient';
  import type { MapEdit, MeasureMap } from './timing';
  import type { ButtonId, MappingActionId } from './actions';
  import type { Keymap } from './keymap';
  let { open, songKey, map, error, keymap, available, run, onclose, onedit }:
    { open: boolean; songKey: string; map: MeasureMap | null; error: string; keymap: Keymap; available: (id: ButtonId) => boolean;
      run: (id: ButtonId) => void; onclose: () => void; onedit: (edit: MapEdit) => Promise<void> } = $props();
  let dialog: HTMLDialogElement;
  const levels = new TransientStack();
  provideTransientStack(levels);
  let origin = $state(1), signature = $state(''), grouping = $state(''), confirming = $state(false), identity = '', previousSong = '';
  const signatures = $derived([...new Map((map?.measures ?? []).map(m => [`${m.meter.numerator}/${m.meter.denominator}`, m.meter])).entries()]);
  const meter = $derived(signatures.find(([key]) => key === signature)?.[1]);
  const groupingOf = (key: string) => map?.measures.find(m => `${m.meter.numerator}/${m.meter.denominator}` === key)?.meter.grouping.join('+') ?? '';
  // A new map revision refreshes the fields; the chosen signature stays while it exists.
  $effect(() => {
    const next = JSON.stringify([songKey, map?.revision]);
    if (next === identity) return;
    identity = next; origin = map?.first_number ?? 1;
    if (songKey !== previousSong || !signatures.some(([key]) => key === signature)) signature = signatures[0]?.[0] ?? '';
    previousSong = songKey; grouping = groupingOf(signature);
  });
  const counts = $derived.by(() => {
    let values: number[] | null = null;
    try { values = readGrouping(grouping); } catch { /* Reported when previewing. */ }
    const matches = map?.measures.filter(m => m.meter.numerator === meter?.numerator && m.meter.denominator === meter?.denominator) ?? [];
    return { changed: matches.filter(m => !m.protected && (!values || m.meter.grouping.join('+') !== values.join('+'))).length, locked: matches.filter(m => m.protected).length };
  });
  const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? '' : 's'}`;

  export function canRun(id: MappingActionId) { return id !== 'map_grouping' || (!!meter && counts.changed > 0); }
  export async function command(id: MappingActionId) {
    if (id === 'map_origin') await onedit({ type: 'set_origin', first_number: origin });
    if (id === 'map_grouping' && meter) await onedit({ type: 'set_grouping_for_meter', numerator: meter.numerator, denominator: meter.denominator, grouping: readGrouping(grouping) });
  }

  let previous: Element | null = null;
  $effect(() => {
    if (open && !dialog.open) {
      previous = document.activeElement; confirming = false;
      dialog.showModal();
      void tick().then(() => dialog.querySelector<HTMLElement>('input')?.focus({ preventScroll: true }));
    } else if (!open && dialog.open) {
      dialog.close();
      const back = previous; previous = null;
      if (back instanceof HTMLElement && back.isConnected && back !== document.body && !back.closest('[hidden], [inert]')) back.focus({ preventScroll: true });
      else focusHome();
    }
  });
  function escape(event: KeyboardEvent) {
    if (event.key !== 'Escape' || event.isComposing) return;
    event.preventDefault();
    if (!levels.closeTop()) onclose();
  }
</script>

<dialog bind:this={dialog} class="song-map" aria-labelledby="song-map-title" onkeydown={escape} oncancel={event => { event.preventDefault(); onclose(); }}>
  {#if open}
    <div class="heading">
      <button class="icon-button" aria-label="Close song map settings" title="Close (Esc)" onclick={onclose}><Icon name="close" size={16}/></button>
      <h2 id="song-map-title">Song map settings</h2>
    </div>
    <section aria-labelledby="song-map-numbering">
      <h3 id="song-map-numbering">Measure numbering</h3>
      <form class="row" onsubmit={event => { event.preventDefault(); run('map_origin'); }}>
        <label for="song-map-origin" class="grow">First full measure is number</label>
        <input id="song-map-origin" class="number" type="number" min="1" max="1000000" step="1" bind:value={origin}/>
        <button class="compact-button" disabled={!available('map_origin')} title={keymap.title('map_origin')}>{keymap.buttonLabel('map_origin', 'Preview')}</button>
      </form>
      <span class="hint">Match your sheet music. A pickup before it stays unnumbered.</span>
    </section>
    <section aria-labelledby="song-map-grouping">
      <h3 id="song-map-grouping">Beat grouping</h3>
      <form class="row" onsubmit={event => { event.preventDefault(); run('map_grouping'); }}>
        <span>In every</span>
        <select aria-label="Time signature" bind:value={signature} onchange={() => { grouping = groupingOf(signature); }} disabled={!signatures.length}>{#each signatures as [key]}<option value={key}>{key}</option>{/each}</select>
        <span>measure, group as</span>
        <input class="grouping" aria-label="Beat grouping for every measure in this time signature" bind:value={grouping} placeholder="2+2+3" disabled={!meter}/>
        <span class="grow"></span>
        <button class="compact-button" disabled={!available('map_grouping')} title={keymap.title('map_grouping')}>{keymap.buttonLabel('map_grouping', 'Preview')}</button>
      </form>
      <span class="hint" role="status">{meter ? `${plural(counts.changed, 'measure')} change · ${plural(counts.locked, 'locked measure')} skipped. ` : ''}Grouping counts {meter ? `${meter.denominator}th` : 'denominator'} notes and must add up to the numerator; leave it blank to clear it.</span>
    </section>
    <section aria-labelledby="song-map-clear">
      <h3 id="song-map-clear">Start over</h3>
      {#if confirming}
        <InlineConfirm label="Clear the measure map" text="Every measure, anchor and lock is removed. Undo brings the map back; detected beats stay for a new draft." confirmLabel="Clear map"
          disabled={!available('map_clear')} onconfirm={() => { confirming = false; run('map_clear'); }} oncancel={() => { confirming = false; }}/>
      {:else}
        <div class="row"><span class="hint grow">Removes every measure. Undo brings the map back.</span>
          <button class="compact-button danger" disabled={!available('map_clear')} title={keymap.title('map_clear')} onclick={() => { confirming = true; }}>Clear map…</button></div>
      {/if}
    </section>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <p class="hint footer">Previews show in green on the waveform. Apply or discard them there.</p>
  {/if}
</dialog>

<style>
  dialog[open]{width:480px;padding:0;border-radius:12px}
  .heading{display:flex;align-items:center;gap:8px;padding:12px 16px}
  .heading h2{margin:0;font-size:16px}
  section{display:flex;flex-direction:column;gap:8px;padding:14px 20px;border-top:1px solid var(--border)}
  h3{font-size:13px}
  .row{display:flex;align-items:center;gap:8px;font-size:12px;color:var(--text-secondary)}
  dialog label{margin:0;font-size:12px;font-weight:400}
  .grow{flex:1}
  input,select{height:30px;padding:0 8px;border:1px solid var(--border-strong);border-radius:6px;background:var(--input-bg);color:var(--text);font-size:13px;font-variant-numeric:tabular-nums}
  dialog input.number{width:64px;padding:0 8px;text-align:right}
  dialog input.grouping{width:76px;padding:0 8px}
  .hint{margin:0;font-size:11px;line-height:1.45;color:var(--text-secondary)}
  .footer{padding:12px 20px 16px;border-top:1px solid var(--border)}
  .error{margin:0 20px 12px;padding:7px 9px;border:1px solid var(--error-border);border-radius:6px;background:var(--error-bg);color:var(--error-text);font-size:12px;line-height:1.45}
  .danger{border-color:var(--error-border)}
</style>
