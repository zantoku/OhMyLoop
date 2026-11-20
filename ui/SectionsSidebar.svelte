<script lang="ts">
  // Saved sections in song order, nested ones indented. Selecting one jumps
  // to it; with looping on it becomes the loop.
  import Icon from './Icon.svelte';
  import { sectionColor } from './model';
  import { sectionRange } from './practice-readout';
  import { nestedSections } from './section-layout';
  import type { ButtonId } from './actions';
  import type { ActionPayload } from './action-dispatcher';
  import type { Keymap } from './keymap';
  import type { MeasureView } from './timing';
  import type { Section } from './types';
  let { id, sections, selected, measures, rate, keymap, available, run }:
    { id: string; sections: Section[]; selected: string | null; measures: readonly MeasureView[]; rate: number; keymap: Keymap;
      available: (id: ButtonId) => boolean; run: (id: ButtonId, payload?: ActionPayload) => void } = $props();
  const nested = $derived(nestedSections(sections));
</script>

<aside {id} class="sections" aria-label="Sections">
  <div class="heading"><h2>Sections</h2><span>{sections.length || ''}</span></div>
  {#if sections.length}
    <div class="list">
      {#each sections as section (section.id)}
        <div class="row" class:selected={selected === section.id} class:nested={nested.has(section.id)} style:--section-color={sectionColor(section.id)}>
          <button class="select" aria-pressed={selected === section.id} disabled={!available('select_section')} title={keymap.title('select_section')} onclick={() => run('select_section', { section })}>
            <span class="dot"></span><span class="text"><strong>{section.name}</strong> <span>{sectionRange(section, measures, rate)}</span></span>
          </button>
          <div class="actions">
            <button aria-label={`Edit ${section.name}`} title={keymap.title('edit_section')} onclick={() => run('edit_section', { section })}><Icon name="edit" size={14}/></button>
            <button aria-label={`Delete ${section.name}`} title={keymap.title('delete_section')} onclick={() => run('delete_section', { section })}><Icon name="trash" size={14}/></button>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <p class="empty">Choose a passage, then save it as a section ({keymap.shortcut('add_section')}) to come back to it.</p>
  {/if}
  <div class="footer">
    <button class="keycap" aria-label={keymap.buttonLabel('previous_section', 'Previous section')} disabled={!available('previous_section')} title={keymap.title('previous_section')} onclick={() => run('previous_section')}>{keymap.shortcut(keymap.actionFor('previous_section')) || '‹'}</button>
    <button class="keycap" aria-label={keymap.buttonLabel('next_section', 'Next section')} disabled={!available('next_section')} title={keymap.title('next_section')} onclick={() => run('next_section')}>{keymap.shortcut(keymap.actionFor('next_section')) || '›'}</button>
    <span>previous / next</span>
  </div>
</aside>

<style>
  .sections{width:210px;flex:none;display:flex;flex-direction:column;min-height:0;margin:4px 0 10px 24px;border:1px solid var(--border);border-radius:8px;background:var(--sidebar)}
  .heading{display:flex;align-items:center;gap:8px;padding:10px 6px 6px 14px}
  h2{font-size:13px;font-weight:700}
  .heading span{font-size:12px;color:var(--text-secondary)}
  .list{flex:1;min-height:0;overflow:auto;overscroll-behavior:contain;display:flex;flex-direction:column;gap:2px;padding:0 6px}
  .row{position:relative;flex:none;border:1px solid transparent;border-radius:6px}
  .row.selected{border-color:var(--border-strong);background:var(--selected)}
  .select{width:100%;display:flex;align-items:center;gap:9px;padding:6px 8px;border:0;border-radius:6px;background:transparent;text-align:left}
  .nested .select{padding-left:26px}
  .select:hover:not(:disabled){background:var(--hover)}
  .dot{width:9px;height:9px;flex:none;border-radius:50%;background:var(--section-color)}
  .text{flex:1;min-width:0;display:flex;flex-direction:column}
  .text strong{font-size:13px;font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .text span{font-size:11px;color:var(--text-secondary);font-variant-numeric:tabular-nums;white-space:nowrap}
  .actions{display:none;position:absolute;right:4px;top:50%;transform:translateY(-50%);gap:2px;padding:2px;border-radius:5px;background:var(--hover)}
  .row:hover .actions,.row:focus-within .actions{display:flex}
  .actions button{display:grid;place-items:center;width:24px;height:22px;padding:0;border:0;border-radius:4px;background:transparent;color:var(--text-secondary)}
  .actions button:hover{background:var(--surface);color:var(--text)}
  .empty{padding:4px 14px;font-size:12px;line-height:1.6;color:var(--text-secondary)}
  .footer{display:flex;align-items:center;gap:6px;margin-top:auto;padding:8px 12px 10px;font-size:11px;color:var(--text-secondary)}
  .keycap{min-width:20px;height:20px;padding:0 5px;border:1px solid var(--border-strong);border-radius:3px;background:transparent;color:inherit;font-size:11px}
  .keycap:hover:not(:disabled){background:var(--hover);color:var(--text)}
</style>
