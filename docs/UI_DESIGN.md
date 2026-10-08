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
- Resize loop boundaries using handles or exact time inputs.
- Select a saved section to seek to its start, preserving play/pause state.
  With loop mode enabled, selecting a section moves the loop to that range.
- Save the current range as a named section; edit its name and bounds later.
- Space toggles playback; L toggles looping; A/B set loop boundaries;
  X clears the temporary A–B range and turns looping off;
  arrow keys seek; brackets navigate sections; Cmd/Ctrl+O opens a song.
- Shortcuts do not fire while typing into an input or dialog.

This differs from a generic media player by giving equal space to phrase
selection and section navigation. A single waveform workspace, two prominent
independent practice controls, and restrained section colors serve the guitar
practice workflow without turning the screen into a DAW mixer.
