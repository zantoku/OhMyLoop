# OhMyLoop

Language for songs, practice timing and separated audio. These definitions describe
the planned integrated library as well as the existing practice workflow.

## Language

**Song**:
A library item with a stable identity, descriptive metadata, notes and practice
material, independent of the particular audio files used to play it.
_Avoid_: File, hash

**Audio asset**:
A particular stored recording or derived audio file associated with a song.
_Avoid_: Song

**Reference recording**:
The song's selected full recording used for waveform review and automatic timing
analysis, against which its practice timeline is established.
_Avoid_: Original mix (when referring to the timing reference)

**Reference timeline**:
The recording-relative positions to which sections, loops and musical timing
belong. Changing playback speed, pitch or audible stems does not change it.

**Stem**:
A separated estimate of an instrument or musical part within a recording.

**Stem set**:
A group of stems produced together from the same recording and aligned to the
same reference timeline.
_Avoid_: Song, backing track

**Mix**:
The selected playback source and levels, mutes and solos used to hear it.

**Measure map**:
The user's accepted mapping between reference-timeline positions and musical
measures, pulses and notation, including uncounted passages.

**Timing evidence**:
Automatically detected suggestions from a particular reference recording, kept
separate from the user's accepted measure map.

**Managed library**:
The user-selected collection that holds imported recordings, song data and
generated stems under the application's management.
