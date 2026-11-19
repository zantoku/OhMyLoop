<script lang="ts">
  // The notes drawer on Practice's right edge (Notes concept NC). The header's
  // Notes button and N open and close it; it has no close X. The text area owns
  // every key while focused, and Esc returns focus to the waveform so Space and
  // the letter shortcuts work again. Saving is shown here only when it is slow
  // or fails; the header's save state covers the normal case.
  import { onMount } from 'svelte';
  import Icon from './Icon.svelte';
  import { focusHome } from './home-focus';
  import { notesUnsaved, saveFailureReason, type NotesState } from './notes-session';
  import type { NotesFont } from './notes-font';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  let { notes, readonly = false, font, keymap, available, onchange, run }:
    { notes: NotesState; readonly?: boolean; font: NotesFont; keymap: Keymap; available: (id: ButtonId) => boolean;
      onchange: (text: string) => void; run: (id: ButtonId) => void } = $props();
  let field = $state<HTMLTextAreaElement>();
  let typing = $state(false);
  const playKey = $derived(keymap.shortcut(keymap.actionFor('play')));

  // Opening the drawer is for writing, so the text takes focus. Closing it
  // returns focus to where it was before, or home.
  onMount(() => {
    const previous = document.activeElement;
    field?.focus({ preventScroll: true });
    return () => {
      const active = document.activeElement;
      if (active && active !== document.body && active.isConnected && !field?.contains(active)) return;
      if (previous instanceof HTMLElement && previous.isConnected && previous !== document.body && !previous.closest('[hidden], [inert]')) previous.focus({ preventScroll: true });
      if (document.activeElement !== previous || previous === document.body) focusHome();
    };
  });

  /** Focuses the text, as opening the drawer does. */
  export function focusText() { field?.focus({ preventScroll: true }); }

  function key(event: KeyboardEvent) {
    if (event.key !== 'Escape' || event.isComposing || event.defaultPrevented) return;
    event.preventDefault();
    focusHome();
  }
</script>

<aside id="notes" class="notes" aria-label="Notes">
  <div class="heading"><h2>Notes</h2>{#if notes.slow}<span role="status">Saving…</span>{/if}</div>
  {#if notes.error}
    <div class="alert" role="alert">
      <span class="message"><Icon name="warning" size={15}/><span>Couldn’t save your notes: {saveFailureReason(notes.error)}. Your text is kept here.</span></span>
      <button class="retry" disabled={!available('retry_notes')} title={keymap.title('retry_notes')} onclick={() => run('retry_notes')}>{keymap.buttonLabel('retry_notes', 'Retry save')}</button>
    </div>
  {:else if notes.tooLong}
    <div class="alert" role="alert">
      <span class="message"><Icon name="warning" size={15}/><span>Notes are over 1 MiB and can’t be saved. The last saved version is kept; shorten them to save again.</span></span>
    </div>
  {/if}
  <textarea bind:this={field} class:mono={font === 'monospace'} aria-label="Notes for this song" placeholder="What to practise, where it goes wrong, reminders for next time…"
    spellcheck="true" {readonly} value={notes.text} oninput={event => onchange(event.currentTarget.value)} onkeydown={key}
    onfocus={() => { typing = true; }} onblur={() => { typing = false; }}></textarea>
  <p class="hint">{#if typing}Typing · Esc returns to the waveform{#if playKey}, so {playKey} plays again{/if}{:else if !notesUnsaved(notes)}Saved as you type{/if}</p>
</aside>

<style>
  .notes{width:320px;flex:none;min-height:0;display:flex;flex-direction:column;gap:8px;padding:14px 16px;border-left:1px solid var(--border);background:var(--sidebar)}
  .heading{display:flex;align-items:center;gap:8px;min-height:22px}
  h2{flex:1;font-size:13px;font-weight:700}
  .heading span,.hint{font-size:11px;color:var(--text-secondary)}
  .alert{display:flex;flex-direction:column;gap:6px;padding:8px 10px;border:1px solid var(--error-border);border-radius:8px;background:var(--error-bg);color:var(--error-text);font-size:12px;line-height:1.4}
  .message{display:flex;gap:6px;overflow-wrap:anywhere}
  .retry{align-self:flex-start;height:26px;padding:0 10px;border:1px solid var(--error-border);border-radius:5px;background:var(--surface);color:var(--error-text);font-size:12px;font-weight:600}
  .retry:hover:not(:disabled){background:var(--hover)}
  textarea{flex:1;min-height:0;width:100%;resize:none;overscroll-behavior:contain;border:1px solid var(--border-strong);border-radius:6px;padding:10px 12px;background:var(--surface);color:var(--text);font-size:13px;line-height:1.6;white-space:pre-wrap}
  textarea:focus{border-color:var(--blue)}
  textarea.mono{font-family:ui-monospace,"SF Mono",Menlo,Consolas,"DejaVu Sans Mono",monospace;font-size:12px}
</style>
