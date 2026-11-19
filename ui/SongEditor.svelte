<script lang="ts">
  // The song's details beside the list, opened only by an explicit edit (⋯ or
  // ⌘I). Title, artist and album are editable; length, dates and file are
  // read-only facts. Replace recording and Delete song live only here.
  import Icon from './Icon.svelte';
  import { libraryDate, songLength, type LibraryEntry, type SongMetadata } from './library';
  import type { ButtonId } from './actions';
  import type { ActionPayload } from './action-dispatcher';
  import type { Keymap } from './keymap';
  let { entry, metadata = $bindable(), saveError, keymap, available, run }:
    { entry: LibraryEntry; metadata: SongMetadata; saveError: string; keymap: Keymap;
      available: (id: ButtonId) => boolean; run: (id: ButtonId, payload?: ActionPayload) => void } = $props();
  let titleField: HTMLInputElement;
  export function focusTitle() { titleField?.focus(); titleField?.select(); }
  const dirty = $derived(metadata.title !== entry.metadata.title || metadata.artist !== entry.metadata.artist || metadata.album !== entry.metadata.album);
</script>

<aside class="editor" aria-label="Edit song details">
  <div class="head">
    <button class="icon-button" aria-label="Close editor" disabled={!available('library_cancel')} title={keymap.title('library_cancel')} onclick={() => run('library_cancel')}><Icon name="close" size={16}/></button>
    <span class="caption">Edit details</span>
  </div>
  <form onsubmit={event => { event.preventDefault(); run('library_save'); }}>
    <label>Title<input bind:this={titleField} bind:value={metadata.title} required maxlength="1024"/></label>
    <label>Artist<input bind:value={metadata.artist} maxlength="1024"/></label>
    <label>Album<input bind:value={metadata.album} maxlength="1024"/></label>
    {#if saveError}<div class="save-error" role="alert"><Icon name="warning" size={16}/><span>Couldn’t save: {saveError}. Your changes are kept here.</span></div>{/if}
    {#if dirty || saveError}
      <div class="actions">
        <button class="button primary" disabled={!available('library_save')} title={keymap.title('library_save')}>{saveError ? 'Retry save' : keymap.buttonLabel('library_save', 'Save details')}</button>
        <button type="button" class="button" disabled={!available('library_cancel')} title={keymap.title('library_cancel')} onclick={() => run('library_cancel')}>Cancel</button>
      </div>
    {/if}
  </form>
  <dl>
    <dt>Length</dt><dd class="number">{songLength(entry.duration_ms)}</dd>
    <dt>Added</dt><dd>{libraryDate(entry.date_added)}</dd>
    <dt>Last opened</dt><dd>{libraryDate(entry.last_opened)}</dd>
    <dt>File</dt><dd class="file copyable" title="The library’s own copy of the recording">{entry.file_location}</dd>
  </dl>
  <div class="spacer"></div>
  <section class="recording" aria-label="Recording">
    <div class="row"><span class="label">Recording</span>
      <button class="button" disabled={!available('library_replace')} title={keymap.title('library_replace')} onclick={() => run('library_replace', { songKey: entry.song_key })}>{keymap.buttonLabel('library_replace', 'Replace recording…')}</button></div>
    <span class="hint">Use a better take or a file that changed. Sections and the map stay when the timing is identical.</span>
  </section>
  <div class="row"><span class="hint">Deleting removes the library copy, notes, map and stems. The original file stays.</span>
    <button class="button delete" disabled={!available('managed_delete')} title={keymap.title('managed_delete')} onclick={() => run('managed_delete', { songKey: entry.song_key })}>Delete song…</button></div>
</aside>

<style>
  .editor{width:340px;flex:none;display:flex;flex-direction:column;gap:14px;padding:18px 20px;border-left:1px solid var(--border);background:var(--sidebar);overflow:auto;overscroll-behavior:contain}
  .head{display:flex;align-items:center;gap:8px}
  .head .icon-button{margin-left:-6px}
  .caption{font-size:11px;font-weight:700;letter-spacing:.06em;text-transform:uppercase;color:var(--text-secondary)}
  form{display:flex;flex-direction:column;gap:10px}
  label{display:flex;flex-direction:column;gap:4px;font-size:12px;font-weight:600;color:var(--text-secondary)}
  input{height:32px;padding:0 8px;border:1px solid var(--border-strong);border-radius:6px;background:var(--surface);color:var(--text);font:inherit;font-size:13px;font-weight:400}
  .save-error{display:flex;gap:8px;padding:8px 10px;border:1px solid var(--error-border);border-radius:8px;background:var(--error-bg);color:var(--error-text);font-size:12px;line-height:1.4}
  .save-error :global(svg){flex:none}
  .actions{display:flex;gap:8px}
  .button{height:30px;padding:0 12px;font-size:12px;font-weight:600}
  dl{margin:0;display:grid;grid-template-columns:auto minmax(0,1fr);gap:7px 16px;font-size:12px}
  dt{color:var(--text-secondary)}
  dd{margin:0}
  .number{font-variant-numeric:tabular-nums}
  .file{overflow-wrap:anywhere;color:var(--text-secondary)}
  .spacer{flex:1}
  .recording{display:flex;flex-direction:column;gap:4px;padding-top:14px;border-top:1px solid var(--border)}
  .row{display:flex;align-items:center;gap:8px}
  .label{flex:1;font-size:12px;font-weight:600}
  .hint{flex:1;font-size:11px;line-height:1.45;color:var(--text-secondary)}
  .delete{color:var(--error-text)}
</style>
