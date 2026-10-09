// Stable action identities and presentation metadata. No UI or device access.
export type Requirement = 'none' | 'song' | 'range' | 'sections' | 'audio' | 'loop_on' | 'undo' | 'editor' | 'help' | 'zoom_out';
export type PayloadKind = 'command' | 'section' | 'speed' | 'editor_submit' | 'editor_close' | 'help_close';
function define(description: string, keys: readonly string[] = [], requires: Requirement = 'song', payload: PayloadKind = 'command', repeat = false) {
  return { description, keys, requires, payload, repeat, duringBusy: false };
}

export const ACTIONS = {
  play_pause: define('Play / pause', ['Space'], 'audio'),
  toggle_loop: define('Loop on / off', ['l']),
  set_loop_start: define('Set A at the playhead', ['a']),
  set_loop_end: define('Set B at the playhead', ['b']),
  jump_loop_start: define('Jump to A', [','], 'range'),
  jump_loop_end: define('Jump to B', ['.'], 'range'),
  jump_song_start: define('Rewind to song start', ['Enter']),
  clear_loop: define('Clear A–B range', ['x'], 'range'),
  seek_backward: define('Seek back 5 seconds', ['ArrowLeft'], 'song', 'command', true),
  seek_forward: define('Seek forward 5 seconds', ['ArrowRight'], 'song', 'command', true),
  seek_backward_fine: define('Seek back 0.1 seconds', ['Shift+ArrowLeft'], 'song', 'command', true),
  seek_forward_fine: define('Seek forward 0.1 seconds', ['Shift+ArrowRight'], 'song', 'command', true),
  previous_section: define('Previous section', ['['], 'sections'),
  next_section: define('Next section', [']'], 'sections'),
  add_section: define('Save a named section', ['n']),
  open_song: define('Open a song', ['Mod+o'], 'none'),
  disable_loop: define('Turn looping off', ['Escape'], 'loop_on'),
  show_shortcuts: { ...define('Show keyboard shortcuts', [], 'none'), duringBusy: true },
  reload_keymap: define('Reload shortcut configuration', [], 'none'),
  load_demo: define('Try the demo', [], 'none'),
  fit_loop: define('Fit the A–B passage in the view', [], 'range'),
  zoom_in: define('Zoom in'),
  zoom_out: define('Zoom out', [], 'zoom_out'),
  show_whole_song: define('Show the whole song'),
  reset_speed: define('Reset to original speed'),
  reset_pitch: define('Reset to original pitch'),
  retry_audio: define('Retry audio output'),
  retry_save: define('Retry saving practice settings'),
  undo_delete: define('Restore the removed section', [], 'undo'),
  dismiss_error: { ...define('Dismiss this message', [], 'none'), duringBusy: true },
  select_section: define('Jump to this section', [], 'song', 'section'),
  edit_section: define('Edit this section', [], 'song', 'section'),
  delete_section: define('Delete this section', [], 'song', 'section'),
  speed_preset: define('Set playback speed', [], 'song', 'speed'),
  save_section: define('Save section', [], 'editor', 'editor_submit'),
  close_editor: define('Close section editor', [], 'editor', 'editor_close'),
  close_shortcuts: { ...define('Close shortcuts', [], 'help', 'help_close'), duringBusy: true }
} as const;

export type ActionId = keyof typeof ACTIONS;
export const BUTTONS = {
  open_song: 'open_song', demo: 'load_demo', shortcuts: 'show_shortcuts',
  previous_section: 'previous_section', play: 'play_pause', next_section: 'next_section',
  loop: 'toggle_loop', clear_loop: 'clear_loop', fit_loop: 'fit_loop',
  jump_loop_start: 'jump_loop_start', jump_loop_end: 'jump_loop_end',
  add_section: 'add_section', save_selection: 'add_section',
  zoom_in: 'zoom_in', zoom_out: 'zoom_out', show_whole_song: 'show_whole_song',
  reset_speed: 'reset_speed', reset_pitch: 'reset_pitch', original_pitch: 'reset_pitch',
  retry_audio: 'retry_audio', retry_save: 'retry_save', undo_delete: 'undo_delete',
  dismiss_error: 'dismiss_error', select_section: 'select_section',
  edit_section: 'edit_section', delete_section: 'delete_section', speed_preset: 'speed_preset',
  save_section: 'save_section', close_editor: 'close_editor', cancel_editor: 'close_editor',
  close_shortcuts: 'close_shortcuts', reload_keymap: 'reload_keymap'
} as const satisfies Record<string, ActionId>;
export type ButtonId = keyof typeof BUTTONS;
