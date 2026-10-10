# First desktop screen

The waveform is the primary workspace: a guitarist can move from navigating
a verse to isolating a short phrase in the same view. Saved section ranges
and temporary loops are independent. Explicit section ends allow nested
phrases, unlike a list of consecutive section boundaries.

Palette: canvas `#f3f5fa`, panel `#ffffff`, ink `#24314c`, waveform field
`#233552`, action blue `#526fc0`, loop amber `#dda54e`. Muted sage and plum
distinguish saved sections. Warm loop handles evoke a physical practice tool;
the surrounding controls stay quiet.

Type: Avenir Next where available, followed by Segoe UI and Noto Sans. Use
tabular figures for musical positions and control values. No remote fonts.
The scale is 12/14/16/20/28 px, with readable sentence-case labels.

```
OhMyLoop                         Song title                   Open song
┌─ Sections ───────┬─────────────────────────────────────────────────┐
│ Verse            │ overview waveform + current viewport             │
│ Chorus           │                                                 │
│ Solo             │ detailed waveform + saved ranges + A/B loop      │
│                  │                                                 │
│ Add section      │ selection bounds / zoom / save selection         │
├──────────────────┴─────────────────────────────────────────────────┤
│ Previous  Play/Pause  Next     position / duration      Loop on/off  │
│ Speed 75%  ─────────────      Pitch -3 st  ─────────────             │
└────────────────────────────────────────────────────────────────────┘
```

Use left alignment throughout. The waveform, rather than decorative cards,
gets the visual emphasis. Avoid gradients, dashboard statistics, and motion
unrelated to playback or user actions. Keyboard focus and contrast are
essential. The playhead shows audio progress; sliders change only their own
parameter.

Interaction defaults:

- Click the waveform to seek; drag to select a temporary loop.
- Scroll over the waveform to zoom at the pointer; middle-button drag pans
  the view without seeking or editing loop boundaries.
- Click or left-button drag the whole-song minimap to move the detailed
  viewport at its current zoom level, bounded by the start and end of the song.
- Resize loop boundaries using handles or exact time inputs.
- Select a saved section to seek to its start, preserving play/pause state.
  With loop mode enabled, selecting a section moves the loop to that range.
- Save the current range as a named section; edit its name and bounds later.
- Space toggles playback; L toggles looping; A/B set loop boundaries;
  comma jumps to A, period jumps to B, and Enter rewinds to song start;
  seeking to B turns looping off while preserving play/pause state;
  X clears the temporary A–B range and turns looping off;
  arrow keys seek; brackets navigate sections; Cmd/Ctrl+O opens a song.
- Shortcuts do not fire while typing into an input or dialog.
- Timeline focus outlines appear during keyboard navigation; pointer clicks
  and dragging keep the minimap and waveform free of focus outlines.
- Command buttons, hover descriptions, shortcut hints, and the shortcut dialog
  resolve from the shared action/keymap model. File overrides can be reloaded
  from the dialog, and invalid configuration falls back to defaults with a warning.

This differs from a generic media player by giving equal space to phrase
selection and section navigation. A single waveform workspace, two prominent
independent practice controls, and restrained section colors serve the guitar
practice workflow without turning the screen into a DAW mixer.

## Measure-mapping screen

Current implementation: updated 2026-10-10.

The practice header opens **Measure map**; **Back to practice** returns from the
dedicated screen in the same window. It shares the song and
transport, but has a separate viewport and editing selection. Its waveform
uses the existing zoom, pan and minimap conventions while showing measure
boundaries, beat anchors and meter changes instead of A–B editing handles.

Metronome and paged Follow playhead default on for the song session and follow
the displayed authored map/draft. View navigation preserves following; explicit
seeks reveal the playhead even while paused. Waveform clicks select the containing
measure at the exact clicked time; measure-lane clicks seek to the measure start.
Playback selects its current measure independently of viewport following.

A musical ruler and editing panel support pulse interpretation, pickups,
incomplete measures and protected corrections. Automatic analysis supplies
an editable draft; manual mapping remains available without a model. The
interaction contract and implementation scope are recorded in
[MEASURE_MAPPING.md](MEASURE_MAPPING.md).

The waveform carries the map review. Detected suggestions are optional and beat
lines thin with density instead of overlapping labels. Analysis and draft creation
live in the right sidebar; model installation is a secondary disclosure. A compact
play/speed overlay sits at the waveform's lower left. The full-width panel beneath
it holds selected-measure tools, grouped into Measure, Join / Split / Shift,
and Free time. Keep the existing palette and type hierarchy; give
vertical space to the waveform rather than a second large speed/pitch panel.

The Measure panel gives the compact `4 / 4` meter pair, kind and one Apply changes
action the primary row. Two 48 px numeric inputs use tabular figures and an 18 px
type size; the slash is larger and quiet. Keep the existing Avenir/Segoe/Noto
type stack, pale `#f9fafc` surface, `#e0e6f0` dividers, `#334966` text and muted
`#7a879d` hints. A small lock beside the title uses the map's green accent when
protected. Advanced folds away grouping, pulse and boundary positions; it can
scroll within the editing panel without filling the default view. The sidebar's
Batch tools owns global numbering (Preview/Apply) and a future grouping-operation
TODO for measures sharing a time signature. Create measures remains a separate
draft form with derived detected-measure counts and the epsilon field.

Join / Split / Shift arranges all three tools horizontally with vertical
separators. The compact split dropdown shows only beat numbers; the shift tool
contains a pulse-count input and Shift left/right buttons. At narrow widths,
stack the tools with horizontal separators to keep controls accessible. Show
the selected pulse unit and explain the endpoint effects beside the controls.
Shift retains authored timing, absorbing consumed bars at fixed endpoints and
growing meters to avoid overfull bars. Join/Split/Shift and notation use direct
validated edits with Undo. Numerical anchor corrections are behind their own
Advanced disclosure.
Free time has a selected-measure removal action and an uncounted-range picker
with meter/pulse and Create measure from range. Reclaimed measures return to
the Measure panel. Inactive panel markup is omitted while form/command state
remains mounted, so WebKit accessibility follows panel changes reliably.
