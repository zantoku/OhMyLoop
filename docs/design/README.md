# Design pass: workflow and screen inventory

OhMyLoop works, but its layout grew one feature at a time. This pass redesigns
it screen by screen from functional briefs, not from the current layout, so
designs follow the user's tasks rather than the order features were built.

## Workflow

For each screen:

1. **Brief** (`docs/design/<screen>.md`). Purpose, user stories tagged by
   frequency, states, fixed constraints and open questions. Every story names
   the action IDs from `ui/actions.ts` it covers. Together, the Actions
   columns form the coverage checklist, so no screen action is lost. The brief describes no layout and includes no
   screenshots.
2. **Story review.** The maintainer drops, merges or re-ranks stories.
   Frequency drives hierarchy: *constant* tasks stay visible and
   keyboard-reachable; *rare* tasks may move into menus or disclosures.
3. **Designs.** Two or three distinct layouts from the brief and the shared
   foundations below. Pick one, then refine it with comments.
4. **Implementation.** One PR per screen. Behaviour, action IDs and tests stay
   unchanged unless the brief says otherwise. Include before/after screenshots
   and use the brief's Actions columns as the acceptance checklist. Fold the
   result back into [UI_DESIGN.md](../UI_DESIGN.md).

Order: **Practice** first (it settles the app shell and shared components),
then **Measure map**, then Library, Notes and Stems.

## Screen inventory

| Screen | Job | Entered from | Briefed |
| --- | --- | --- | --- |
| Practice | Hear a song at a workable speed and pitch, loop passages, return to them | Library, start-up, every other screen | [practice.md](practice.md) |
| Measure map | Build and correct the song's measure map against its reference recording | Practice | [measure-map.md](measure-map.md) |
| Library | Find, import, open and describe songs in the managed library | Header, empty start | [library.md](library.md) |
| Notes | Keep plain-text notes for one song | Practice | [notes.md](notes.md) |
| Stems | Separate or attach a stem set and follow its processing | Practice, processing notice | [stems.md](stems.md) |
| Settings | App-wide library, stem tools, analysis model, appearance, shortcuts and audio output | Library header, ⌘, from any screen, macOS app menu, links from other screens | [settings.md](settings.md) |

Dialogs and overlays:

| Dialog | Owner | Job |
| --- | --- | --- |
| Section editor | Practice | Name a section and set its exact bounds |
| Keyboard shortcuts | Global | List bindings; reload the shortcut configuration file |
| Library setup / duplicate / delete | Library | Create or open a library on the first import; resolve duplicate imports; confirm deletion |
| Recording replacement | Library | Replace the reference recording, keeping or clearing timing |
| Quit confirmation | Global | Quit while processing runs |
| Stem replacement confirmation | Stems | Replace an existing stem set |

## Shared building blocks

These recur on several screens. Their design is settled during the Practice
pass and reused afterwards.

- **App shell:** brand, current song title and duration, navigation between
  screens, global notices, and a status bar showing save state, audio device
  and shortcut help. Appearance moved to app-wide Settings (Library review).
- **Notices:** errors and running work, often with one recovery action (Retry,
  Reload, View). Several can be active at once.
- **Waveform workspace:** a whole-song minimap with a viewport, a detailed
  waveform with playhead and an overlay ruler (measure numbers in mapped audio,
  timecode in free time), and zoom/pan. Practice adds a section lane and an
  A–B loop. Measure map adds boundaries, beats and detected suggestions.
- **Transport:** play/pause, position, speed. Practice has the full set;
  Measure map has a compact version.
- **Musical position:** current measure and beat; previous/next measure; go to
  a measure number. Shown on Practice and Measure map whenever a map exists.
- **Metronome:** on/off and click level on Practice; always on at full level on
  the Measure map (measure-map decision 7).
- **Screen header:** dedicated screens (Notes, Measure map, Stems) have a
  leading X that returns to Practice.

## Foundations that carry over

These are recorded decisions. Designs may restyle them, but should not drop
them without flagging the change. Details are in [UI_DESIGN.md](../UI_DESIGN.md)
and `ui/theme.css`.

- Desktop app for Linux, macOS and Windows. Default window 1180 × 810; content
  fills larger windows. Minimum window 1180 × 680, with every element
  visible; narrow layouts are out of scope for now.
- Light and dark themes (auto / dark / light). Palette: canvas, panel, ink,
  action blue, loop amber, sage and plum for sections. Avenir Next / Segoe UI /
  Noto Sans; no remote fonts; tabular figures for times and values; type scale
  12/14/16/20/28.
- The waveform gets the emphasis. No decorative cards, gradients, dashboard
  statistics or motion unrelated to playback.
- Every command comes from the shared action/keymap model, so buttons,
  tooltips and the shortcut list stay consistent. Shortcuts never fire while
  typing.
- Close controls are an X in the top leading corner. Determinate circular
  progress starts at 12 o'clock and fills clockwise.
- Visible keyboard focus during keyboard navigation; none on pointer use.
