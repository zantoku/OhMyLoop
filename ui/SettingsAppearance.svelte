<script lang="ts">
  // Settings → Appearance (E1, E2): the theme and the notes font, each applied
  // at once and saved for this computer.
  import ChoiceGroup from './ChoiceGroup.svelte';
  import type { ThemePreference } from './theme';
  import type { NotesFont } from './notes-font';
  let { theme, notesFont, saveFailed, ontheme, onnotesfont }:
    { theme: ThemePreference; notesFont: NotesFont; saveFailed: boolean; ontheme: (theme: ThemePreference) => void; onnotesfont: (font: NotesFont) => void } = $props();
  const sample = 'm. 33  Am7    D7     Gmaj7\ne|-----5-7-8-|-7-5------|\nB|---5-------|-----8-5--|\nSlow down the bend in m. 37.';
</script>

<section class="settings-section" aria-labelledby="settings-appearance">
  <h3 id="settings-appearance">Appearance</h3>
  <div class="settings-row">
    <span class="label"><strong>Theme</strong><span class="settings-hint">Auto follows your system.</span></span>
    <ChoiceGroup label="Theme" value={theme} onchange={ontheme} options={[{ value: 'auto', label: 'Auto' }, { value: 'light', label: 'Light' }, { value: 'dark', label: 'Dark' }]}/>
  </div>
  <div class="settings-row">
    <span class="label"><strong>Notes font</strong><span class="settings-hint">Monospaced lines up chord charts and tab.</span></span>
    <ChoiceGroup label="Notes font" value={notesFont} onchange={onnotesfont} options={[{ value: 'proportional', label: 'Proportional' }, { value: 'monospace', label: 'Monospaced' }]}/>
  </div>
  <pre class="preview" class:mono={notesFont === 'monospace'} aria-label="Notes font preview">{sample}</pre>
  {#if saveFailed}<span class="settings-hint tone-error" role="status">This choice couldn’t be saved. It applies until you quit.</span>{/if}
</section>

<style>
  .preview{margin:0;padding:10px 12px;border:1px solid var(--border);border-radius:6px;background:var(--page);color:var(--text);font-family:inherit;font-size:13px;line-height:1.55;white-space:pre-wrap}
  .preview.mono{font-family:ui-monospace,"SF Mono",Menlo,Consolas,"DejaVu Sans Mono",monospace;font-size:12px}
</style>
