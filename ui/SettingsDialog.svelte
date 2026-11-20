<script lang="ts">
  // App-wide Settings (concept GA, docs/design/settings.md): a modal dialog
  // with a leading X, the six sections in a sidebar with a one-line status
  // each, and one section shown at a time. Changes apply at once.
  // The dialog owns every key pressed inside it (key-routing.ts): Esc unwinds
  // an inline confirmation, then closes; ↑/↓, Home and End move between
  // sections from the sidebar. Closing returns focus to where it was.
  import { tick } from 'svelte';
  import Icon from './Icon.svelte';
  import SettingsLibrary from './SettingsLibrary.svelte';
  import SettingsAppearance from './SettingsAppearance.svelte';
  import SettingsAudio from './SettingsAudio.svelte';
  import SettingsShortcuts from './SettingsShortcuts.svelte';
  import SettingsStems from './SettingsStems.svelte';
  import SettingsBeats from './SettingsBeats.svelte';
  import { SECTION_NAMES, SETTINGS_SECTIONS, sectionStatus, type SettingsSection, type SettingsSummary } from './settings';
  import { TransientStack, provideTransientStack } from './transient';
  import { focusHome } from './home-focus';
  import type { SeparationState } from './separation';
  import type { BeatModelState } from './beat-model-session';
  import type { ThemePreference } from './theme';
  import type { NotesFont } from './notes-font';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';

  export type ShortcutFile = { showButton: boolean; path: string | null; warning: string | null; loading: boolean };
  let { open, section, summary, error, working, download, separation, beat, shortcuts, revealLabel, saveFailed,
    onsection, ontheme, onnotesfont, onshowbutton, keymap, available, run }:
    { open: boolean; section: SettingsSection; summary: SettingsSummary; error: string; working: string;
      download: 'stems' | 'beats' | null; separation: SeparationState; beat: BeatModelState; shortcuts: ShortcutFile;
      revealLabel: string; saveFailed: boolean;
      onsection: (section: SettingsSection) => void; ontheme: (theme: ThemePreference) => void;
      onnotesfont: (font: NotesFont) => void; onshowbutton: (shown: boolean) => void;
      keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void } = $props();

  let dialog: HTMLDialogElement;
  const tabs: Record<string, HTMLButtonElement> = {};
  const levels = new TransientStack();
  provideTransientStack(levels);
  // While a download started here runs, the other sections' actions wait (decision 6).
  const locked = (own: SettingsSection) => download !== null && download !== own;

  let previous: Element | null = null;
  $effect(() => {
    if (open && !dialog.open) {
      previous = document.activeElement;
      dialog.showModal();
      void tick().then(() => tabs[section]?.focus({ preventScroll: true }));
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
    if (!levels.closeTop()) run('close_settings');
  }

  function navigate(event: KeyboardEvent) {
    const index = SETTINGS_SECTIONS.indexOf(section);
    const next = { ArrowDown: index + 1, ArrowUp: index - 1, Home: 0, End: SETTINGS_SECTIONS.length - 1 }[event.key];
    if (next === undefined) return;
    event.preventDefault();
    const target = SETTINGS_SECTIONS[(next + SETTINGS_SECTIONS.length) % SETTINGS_SECTIONS.length];
    onsection(target);
    tabs[target]?.focus({ preventScroll: true });
  }
</script>

<dialog bind:this={dialog} class="settings" aria-labelledby="settings-title" onkeydown={escape}
  oncancel={event => { event.preventDefault(); run('close_settings'); }}>
  {#if open}
    <div class="heading">
      <button class="icon-button" aria-label={keymap.buttonLabel('close_settings', 'Close Settings')} title={`${keymap.title('close_settings')} (Esc)`} onclick={() => run('close_settings')}><Icon name="close" size={16}/></button>
      <h2 id="settings-title">Settings</h2>
    </div>
    <div class="body">
      <div class="sidebar" role="tablist" aria-label="Settings sections" aria-orientation="vertical" tabindex="-1" onkeydown={navigate}>
        {#each SETTINGS_SECTIONS as id (id)}
          {@const status = sectionStatus(id, summary)}
          <button bind:this={tabs[id]} id="settings-tab-{id}" role="tab" aria-selected={id === section} aria-controls="settings-panel" tabindex={id === section ? 0 : -1}
            onclick={() => onsection(id)}>
            <span class="name">{SECTION_NAMES[id]}</span>
            <span class="status tone-{status.tone}">{status.text}</span>
          </button>
        {/each}
      </div>
      <div id="settings-panel" class="panel" role="tabpanel" aria-labelledby="settings-tab-{section}" tabindex="-1">
        {#if error}<p class="settings-alert panel-error" role="alert"><Icon name="warning" size={15}/><span>{error}</span></p>{/if}
        {#if section === 'library'}
          <SettingsLibrary status={summary.library} songCount={summary.songCount} {revealLabel} {working} locked={locked('library')} {keymap} {available} {run}/>
        {:else if section === 'appearance'}
          <SettingsAppearance theme={summary.theme} notesFont={summary.notesFont} {saveFailed} {ontheme} {onnotesfont}/>
        {:else if section === 'audio'}
          <SettingsAudio device={summary.device} audioError={summary.audioError} native={summary.native} {keymap} {available} {run}/>
        {:else if section === 'shortcuts'}
          <SettingsShortcuts showButton={shortcuts.showButton} source={summary.shortcuts} path={shortcuts.path} warning={shortcuts.warning} loading={shortcuts.loading}
            {revealLabel} {saveFailed} locked={locked('shortcuts')} {onshowbutton} {keymap} {available} {run}/>
        {:else if section === 'stems'}
          <SettingsStems {separation} locked={locked('stems')} {keymap} {available} {run}/>
        {:else}
          <SettingsBeats model={beat} locked={locked('beats')} {keymap} {available} {run}/>
        {/if}
      </div>
    </div>
  {/if}
</dialog>

<style>
  .settings[open]{width:min(820px,calc(100vw - 48px));height:min(620px,calc(100vh - 48px));max-width:none;padding:0;display:flex;flex-direction:column;border-radius:12px;overflow:hidden}
  .heading{display:flex;align-items:center;gap:10px;padding:10px 16px;border-bottom:1px solid var(--border)}
  .heading .icon-button{margin-left:-6px}
  .heading h2{font-size:16px}
  .body{flex:1;min-height:0;display:flex}
  .sidebar{width:200px;flex:none;display:flex;flex-direction:column;gap:2px;padding:10px;border-right:1px solid var(--border);background:var(--sidebar);overflow:auto}
  [role=tab]{display:flex;flex-direction:column;gap:1px;padding:7px 10px;border:0;border-radius:6px;background:transparent;text-align:left;color:var(--text)}
  [role=tab]:hover:not([aria-selected=true]){background:var(--hover)}
  [role=tab][aria-selected=true]{background:var(--selected)}
  .name{font-size:13px;font-weight:600}
  [aria-selected=true] .name{font-weight:700}
  .status{font-size:11px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .panel{flex:1;min-width:0;padding:18px 24px;overflow:auto;overscroll-behavior:contain}
  .panel-error{margin:0 0 12px}
</style>
