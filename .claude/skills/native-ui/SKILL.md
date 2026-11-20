---
name: native-ui
description: Native desktop feel for OhMyLoop's Tauri/Svelte UI. Use when building or changing anything in `ui/` (components, styles, keyboard handling, focus, menus, dialogs) or `src-tauri` menus and windows.
---

# Native desktop UI

OhMyLoop is a Tauri webview, but it must feel like a **native** desktop app,
with macOS as the reference platform. A web page says "web" through:

- focus rings after clicks
- a pointing-hand cursor
- selectable labels
- the browser's right-click menu
- a page that rubber-bands
- shortcuts that die after clicking a control

Every rule below removes one of these tells. The design briefs in
`docs/design/` say what a screen contains; this skill says how every screen
behaves. The [Foundations](../../../docs/design/README.md#foundations-that-carry-over)
and [KEYMAP.md](../../../docs/KEYMAP.md) still apply.

## Keyboard: one focus model

The **workspace** is the screen's main surface: the waveform on Practice and
on the Measure map, the song list in the Library. It holds **home focus**.
Global shortcuts fire whenever focus is at home or on any control that does
not own the key.

- **Single routing point.** Shortcuts go through the global handler, then the
  keymap, then the `ActionDispatcher`. A component handles a key itself only
  when that key is part of its own documented behaviour.
- **Key owners.** Only these own keys:
  - a text field, textarea, select or contenteditable, while focused
  - an open modal dialog or popover
  - a composite control (slider, knob, waveform seek surface, loop handle,
    map line), and only for the keys it documents, such as the arrows,
    Home/End and Page Up/Down. Every other key bubbles on to the global
    handler.
  - a toolbar (`data-nav-keys`), for the arrows that move between its
    buttons, and only while its focus arrived by keyboard (it marks itself
    `data-keyboard-focus` on focus during keyboard navigation). Don't rely on
    the modality alone: on Windows and Linux the arrow press itself switches
    it to keyboard, so a button focused by a click would take the arrows.

  Decide ownership by the key, never by the element's role. Swallowing every
  key inside `[role=slider]` is the bug this rule exists to prevent.
- **Clicks leave focus where it was.** As in native macOS apps, clicking a
  button, toggle, segmented control or tab does not move keyboard focus to
  it. Prevent the focus change on `mousedown` (WebKit and Chromium move focus
  there; cancelling `pointerdown` doesn't stop it), and commit a focused
  field before cancelling, so its `change` still fires. Afterwards,
  Space plays, as it does everywhere. Only Tab-focused buttons activate with
  Space or Enter.
- **Committing hands focus back.** In a number or short text field:
  - Enter commits and returns home focus.
  - Esc restores the previous value and returns home focus.
  - Leaving the field (blur) commits.
- **Esc unwinds** one level per press: the open menu or popover, then the
  dialog, then a field or control back to home focus. Esc in the Notes text
  closes the Notes drawer (flushing first, like N) and returns home focus; if
  the save fails or the notes are over-long, the drawer stays open and focus
  stays in the text, and while the app is busy Esc only returns home.
- **⌘ shortcuts live in the native menu bar** (Tauri `menu` API). Build the
  menus from the action model, so labels, accelerators and enabled state
  match the buttons and the shortcut list. Single-key shortcuts (Space,
  letters, arrows) stay in the webview handler. A replacement menu keeps the
  predefined macOS items: About, Settings… ⌘, Hide, Quit, and Edit's
  Undo/Redo/Cut/Copy/Paste/Select All. Without them, ⌘C and ⌘V stop working
  in text fields. Undo and Redo go to the text field when one is focused,
  otherwise to the app's undo.
- **Windows and Linux have no menu bar** (decided 2026-11-19). The minimum
  window keeps its full 680 px for content, which matches current Windows
  apps and GNOME's header-bar style. There, Ctrl shortcuts go through the
  webview handler with the same routing rules, and every menu command is also
  reachable from a button on screen (Settings through the Library's Settings
  button).

## Focus rings

- Show rings only during **keyboard navigation**:
  - Track the input modality on `<html>`: `pointerdown` sets `pointer`, and
    Tab or arrow navigation sets `keyboard`.
  - Draw rings under `[data-modality=keyboard] :focus-visible`.
  - A pointer interaction never shows a ring, even when code moves focus.
- A ring hugs its control: offset 0–1 px, following its border radius, in
  `var(--focus)`. A composite control (waveform, map line) shows focus on its
  own drawn element, not on a box around the whole region.
- Containers that hold focus for scripting (`tabindex="-1"`) never show a
  ring.

## Pointer, selection and chrome

- **Cursors carry meaning.**
  - Use the arrow for buttons, toggles, tabs, rows and disabled controls.
  - Use the hand only for links that open a web page.
  - Use `ew-resize` (or `col-resize`) for draggable lines and handles,
    `grab`/`grabbing` while panning, and `text` in text fields.
  - Never use `not-allowed`.
- **Chrome text is not selectable.** Set `user-select: none` app-wide and
  re-enable selection for:
  - text fields
  - notes
  - text a person may copy: error details, file paths, versions
- **No browser context menu.** Suppress `contextmenu` everywhere except text
  fields, which keep the native Cut/Copy/Paste and spelling menu. Custom
  context menus appear only where a design specifies them.
- **The window never scrolls.** Lock `html` and `body` (`overflow: hidden`,
  `overscroll-behavior: none`). Only designated panes scroll: lists, drawers
  and dialog sections. Leave scrollbars unstyled so they follow the system's
  overlay scrollbars.
- **Nothing drags out.** Set `draggable="false"` and `-webkit-user-drag: none`
  on images, SVGs and links, unless a drag is a designed feature.
- **Native controls keep native behaviour.** Use real `<input>`, `<select>`,
  `<textarea>`, `<dialog>` and `<button>`, styled with tokens and
  `accent-color`. Rebuild a control only when the design needs something
  native controls can't do, and then give it the matching ARIA role and keys.
- **Feedback is immediate.** Hover and pressed states change without
  transitions longer than about 120 ms. Nothing animates that isn't playback,
  a progress indicator or an opening drawer or dialog.

## Dialogs and transient UI

- Modal dialogs use `<dialog>` with `showModal()` and trap focus. They open
  with focus on the first field, or else on the default button.
- Enter triggers the default (trailing, primary) button and Esc cancels. The
  leading X closes the dialog, following the Foundations.
- When a dialog, popover or drawer closes, focus returns to where it was
  before it opened, or else home.
- Tooltips come from `keymap.title(...)` (the native `title` attribute) and
  name the shortcut.

## Done means

A UI change is done when every check below holds on the screens it touches,
in light and dark mode. Report the ones you checked in the running app
(`npm run desktop`) separately from those covered only by tests.

1. Tab moves through every control in visual order with a hugging ring, and
   clicking anything shows no ring.
2. After clicking each button, slider, field and waveform element, Space
   still toggles playback and the screen's letter shortcuts still fire. While
   typing in a text field, none fire.
3. Esc unwinds one level per press, ending at home focus. Enter in a field
   commits and returns home.
4. ⌘C, ⌘V, ⌘Z and ⌘A work in text fields, and the app's ⌘ shortcuts work
   from the menu bar wherever focus is.
5. Right-clicking chrome shows nothing. Drag-selecting across labels selects
   nothing. Scrolling at a pane's edge doesn't bounce the window.
6. Key routing has a vitest case for each new key owner. Assert that the keys
   it doesn't own reach the dispatcher.
