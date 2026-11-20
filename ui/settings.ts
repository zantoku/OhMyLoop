// App-wide Settings (docs/design/settings.md, concept GA): the sections in
// their fixed order, the one-line status each shows in the sidebar, the state
// of the two downloadable tools, and why Settings can't open right now.
// Pure functions of app state; no UI or device access.
import type { SeparationStatus } from './separation';
import type { AnalysisStatus } from './timing';
import type { LibraryStatus } from './types';
import type { ThemePreference } from './theme';
import type { NotesFont } from './notes-font';
import type { KeymapSource } from './keymap';

export const SETTINGS_SECTIONS = ['library', 'appearance', 'audio', 'shortcuts', 'stems', 'beats'] as const;
export type SettingsSection = typeof SETTINGS_SECTIONS[number];
export const SECTION_NAMES: Record<SettingsSection, string> = {
  library: 'Library', appearance: 'Appearance', audio: 'Audio', shortcuts: 'Shortcuts', stems: 'Stem separation', beats: 'Beat analysis'
};
export function isSettingsSection(value: unknown): value is SettingsSection {
  return SETTINGS_SECTIONS.includes(value as SettingsSection);
}

export type StatusTone = 'normal' | 'success' | 'warning' | 'error';
export type SectionStatus = { text: string; tone: StatusTone };

/** The size of the beat analysis download (both model files), in bytes. */
export const BEAT_MODEL_BYTES = 83_000_000;
export function megabytes(bytes: number): string { return `${(bytes / 1e6).toFixed(1)} MB`; }
const percent = (fraction: number | null) => fraction === null ? '' : ` · ${Math.round(fraction * 100)} %`;
const fraction = (completed: number, total: number | null | undefined) => total ? Math.min(1, completed / total) : null;

export function libraryName(root: string): string {
  return root.split(/[\\/]/).filter(Boolean).at(-1) ?? root;
}

export type StemTools =
  | { kind: 'checking' | 'unsupported' | 'none' | 'ready' | 'damaged' | 'failed' }
  | { kind: 'installing'; phase: string; progress: number | null };
/** The state of the separation runtime and model taken together. */
export function stemTools(status: SeparationStatus | null): StemTools {
  if (!status) return { kind: 'checking' };
  const install = status.install;
  if (install.running) {
    const work = install.progress?.workflow ?? install.progress;
    return { kind: 'installing', phase: install.phase, progress: work ? fraction(work.completed, work.total) : null };
  }
  if (status.runtime.supported === false) return { kind: 'unsupported' };
  if (status.runtime.ready && status.model_ready) return { kind: 'ready' };
  if (install.phase === 'Failed') return { kind: 'failed' };
  // Tools on disk that no longer verify, or one half of a finished setup missing.
  if (status.runtime.path || status.model_ready) return { kind: 'damaged' };
  return { kind: 'none' };
}

export type BeatModel =
  | { kind: 'checking' | 'none' | 'ready' | 'failed' | 'busy' }
  | { kind: 'downloading'; phase: string; progress: number | null };
export function beatModel(status: AnalysisStatus | null): BeatModel {
  if (!status) return { kind: 'checking' };
  if (status.running && status.kind === 'install') return { kind: 'downloading', phase: status.phase, progress: fraction(status.progress?.completed ?? 0, status.progress?.total) };
  if (status.running) return { kind: 'busy' };
  if (status.model_ready) return { kind: 'ready' };
  if (status.kind === 'install' && status.phase === 'Failed') return { kind: 'failed' };
  return { kind: 'none' };
}

export type SettingsSummary = {
  library: LibraryStatus; songCount: number | null;
  theme: ThemePreference; notesFont: NotesFont;
  device: string | null; audioError: string | null; native: boolean;
  shortcuts: KeymapSource;
  separation: SeparationStatus | null; analysis: AnalysisStatus | null;
};

/** The one-line status under each section name in the sidebar. */
export function sectionStatus(section: SettingsSection, s: SettingsSummary): SectionStatus {
  switch (section) {
    case 'library': {
      if (!s.library.selected) return { text: 'No library yet', tone: 'normal' };
      if (!s.library.available) return { text: 'Not found', tone: 'warning' };
      const name = libraryName(s.library.selected.root);
      return { text: s.songCount === null ? name : `${name} · ${s.songCount} ${s.songCount === 1 ? 'song' : 'songs'}`, tone: 'normal' };
    }
    case 'appearance':
      return { text: `${{ auto: 'Auto', light: 'Light', dark: 'Dark' }[s.theme]} · ${s.notesFont === 'monospace' ? 'monospaced' : 'proportional'} notes`, tone: 'normal' };
    case 'audio':
      if (s.audioError) return { text: 'Device lost', tone: 'error' };
      return { text: s.device ?? (s.native ? 'No audio output' : 'Browser preview'), tone: 'normal' };
    case 'shortcuts':
      return { builtin: { text: 'Built-in', tone: 'normal' }, custom: { text: 'Custom file', tone: 'normal' }, rejected: { text: 'File rejected', tone: 'error' } }[s.shortcuts] as SectionStatus;
    case 'stems': {
      const tools = stemTools(s.separation);
      switch (tools.kind) {
        case 'installing': return { text: `${tools.phase}${percent(tools.progress)}`, tone: 'normal' };
        case 'ready': return { text: `Ready · ${megabytes(s.separation!.package_bytes)} model`, tone: 'success' };
        case 'damaged': return { text: 'Needs repair', tone: 'warning' };
        case 'failed': return { text: 'Setup failed', tone: 'error' };
        case 'unsupported': return { text: 'Unavailable here', tone: 'normal' };
        case 'checking': return { text: 'Checking…', tone: 'normal' };
        default: return { text: 'Not installed', tone: 'normal' };
      }
    }
    case 'beats': {
      const model = beatModel(s.analysis);
      switch (model.kind) {
        case 'downloading': return { text: `Downloading${percent(model.progress)}`, tone: 'normal' };
        case 'ready': return { text: 'Ready', tone: 'success' };
        case 'failed': return { text: 'Download failed', tone: 'error' };
        case 'busy': return { text: 'Analysing a song', tone: 'normal' };
        case 'checking': return { text: 'Checking…', tone: 'normal' };
        default: return { text: 'Not installed', tone: 'normal' };
      }
    }
  }
}

/** Which download started in Settings is running, if any (decision 6). */
export function settingsDownload(separation: SeparationStatus | null, analysis: AnalysisStatus | null): 'stems' | 'beats' | null {
  if (separation?.install.running) return 'stems';
  if (analysis?.running && analysis.kind === 'install') return 'beats';
  return null;
}

export type BackgroundWork = {
  separating: boolean; stemSetup: boolean; beatDownload: boolean; analysing: boolean; busy: string;
};
/**
 * Why Settings can't open, or null (decision 6): while background work runs
 * the Settings button and ⌘, are disabled and say why. A download started
 * inside Settings is not a blocker (App leaves it out), so Settings can reopen
 * on its progress and Cancel. `busy` covers imports and other app tasks.
 */
export function settingsBlockedReason(work: BackgroundWork): string | null {
  if (work.separating) return 'Settings are unavailable while a song is split into stems.';
  if (work.stemSetup) return 'Settings are unavailable while the stem separation tools download.';
  if (work.beatDownload) return 'Settings are unavailable while the beat analysis model downloads.';
  if (work.analysing) return 'Settings are unavailable while beat analysis runs.';
  if (work.busy) return `Settings are unavailable until this finishes: ${work.busy}`;
  return null;
}
