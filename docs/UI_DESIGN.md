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

## Planned measure-mapping screen

The next pass adds **Measure map** in the practice header and **Back to
practice** on a dedicated screen in the same window. It shares the song and
transport, but has a separate viewport and editing selection. Its waveform
uses the existing zoom, pan and minimap conventions while showing measure
boundaries, beat anchors and meter changes instead of A–B editing handles.

A musical ruler and inspector support pulse interpretation, pickups,
incomplete measures and protected corrections. Automatic analysis supplies
an editable draft; manual mapping remains available without a model. The
interaction contract and implementation scope are recorded in
[MEASURE_MAPPING.md](MEASURE_MAPPING.md). This screen is not implemented yet.
