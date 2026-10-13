# Stem attachment and desktop mixer — issue #5

Status: scope, visual design and technical decisions agreed; awaiting the
maintainer's final shared-understanding confirmation before implementation.

## Agreed scope

- One song, at most one attached stem set. No set history or A/B comparison UI.
- Attach existing completed prototype outputs; integrated Roformer-SW inference
  and model installation belong to #6.
- Validate provenance, hashes, finite samples and full-song reference-timeline
  mapping, then copy into managed storage. Never keep scratch-directory links.
- Confirm replacement. Save the validated new set before removing the prior one.
  Preserve gain/mute controls for matching roles; solo remains transient.
- Infer audible source: Original iff every track is at 100%, unmuted and
  unsoloed; otherwise Stems. No persisted source selector.
- Preserve transport, source-coordinate annotations, speed/pitch, metronome and
  count-in when changing the mix. Reference audio owns waveform and timing
  analysis; mix changes do not redefine the accepted measure map.
- Visual design is captured in [UI_DESIGN.md](UI_DESIGN.md). Use production
  components rewritten from the mockup, not prototype code as-is.

## Accepted technical decisions — round 1

1. Add a minimal Stems screen beside Notes and Measure map: attachment/replacement,
   current-set details and useful errors. #6 extends this screen with inference.
2. Copy and validate into a new owned set directory, atomically save the song's
   association, then remove the prior set. Ordinary failures retain the prior
   association and attempt immediate cleanup. Surface cleanup warnings. No journal
   or restart recovery; interruption may leave unreferenced files.
3. Translate verified prototype output into a versioned app-owned stem-set record.
   Retain declared stems and verified provenance, rather than keeping prepared.wav
   as an ongoing playback dependency. Validate prepared.wav during attachment;
   validate owned copies before activation. Playback PCM/headroom are rebuildable
   caches, not authoritative stem assets.

## Accepted technical decisions — round 2

4. Persist prepared playback PCM and measured headroom under the song directory.
   Cache validity includes content digests, output rate and preparation/resampling
   policy versions/configuration. These are rebuildable, not authoritative audio.
   Invalidate/remove obsolete caches on reference/stem replacement and song deletion.
5. Every reference replacement removes the attached stem set, including a
   timing-compatible replacement retaining the measure map. Explain this in the
   replacement dialog. Do not keep previous-reference stems.
6. Support the six explicit semantic roles bass/drums/guitar/piano/vocals/other;
   display only present tracks, reject unknown roles. Preserve gains/mutes for
   matching roles on set replacement, clear solo, and use only present roles in
   the automatic Original/Stems predicate.
7. Keep the existing decoded reference Audio path for waveforms and timing
   analysis in #5; bounded stem preparation/playback does not imply bounded
   whole-desktop memory. A reference-analysis memory refactor is separate work.
8. The loop shortcut cycles the same three states as the button. Without an
   accepted measure map, only Off/Loop are available. Count-in and repetition
   count-in remain session state, reset on another song/restart. Persist loop
   selection/enabled as today. Preserve count-in suppression during map editing.

## Accepted technical decisions — round 3

9. On song open, wait for missing/invalid playback preparation with visible
   stage/progress and playback disabled until ready. A valid cache skips preparation.
10. Do not rehash authoritative stem files on every open. Validate and record
    hashes at attachment; users manually editing managed stem files take
    responsibility. Reference-recording integrity checks remain as implemented.
    Reopening still needs ordinary file/format/geometry safety checks; exact cache
    invalidation for external edits is addressed separately.
11. Missing/invalid stems on open visibly fall back to Original and disable mix
    controls, preserving saved gain/mute settings for successful retry. Missing
    playback cache rebuilds. Runtime read failures stop playback and report errors.
12. Add confirmed Remove stems on the Stems screen: remove owned stems/cache,
    reset mix to Original, retain reference/map/sections/notes.
13. Practice-only keys 1–6 toggle canonical bass/drums/guitar/piano/vocals/other
    mutes; missing roles do nothing, typing is isolated. Knobs support live vertical
    drag, wheel, keyboard arrows and double-click reset to 100%. Serialize/coalesce
    playback commands to latest desired controls; debounce persistence separately.

## Accepted technical decisions — round 4

14. Use stem size/modification timestamps as cheap preparation-cache freshness
    checks; do not rehash on ordinary reopening or enforce integrity after manual
    edits. Changed metadata rebuilds preparation. Same-size edits preserving
    timestamps are not guaranteed to be detected. Validate attachment hashes once;
    recorded provenance describes that verified attachment.
15. Use the fixed shared reference/stem headroom factor for every mix state,
    including automatic Original playback while a set is attached. Show attenuation
    in Stems details; gain/mute/solo never changes the factor. Preserve the tested
    headroom policy and final output clamp, with no new limiter.
16. Retain playback preparation for only the current output sample rate; rebuild
    and replace obsolete caches on rate changes. No cache quotas/eviction UI,
    storage-size settings or disk-management UI in #5.
17. Attach/replace/remove pauses playback and preserves position, stays paused
    afterward, shows progress and disables conflicting song/library changes.
    No background attachment or cancellation. Merely entering/leaving Stems
    preserves transport like Notes. Guard publication with library/song/reference/
    set revisions to reject stale replies.

## Implementation boundaries and acceptance

Use cohesive native attachment/cache/source modules and frontend stems/mix sessions,
not filesystem/process work in App event handlers. Extend versioned storage at its
boundary, preserve malformed/future documents, and retain supported existing UUID
records; no new blanket data wipe. Paths remain library-relative and guarded against
escaping/symlink traversal. Update central action identities, descriptions, bindings
and help together. Keep no-allocation/blocking/filesystem work in the audio callback.

Gain is attenuation-only, 0–100%. Retain native solo-union/mute-wins semantics;
all muted means silence. Restore gains/mutes on reopening, clear transient solo on
song/set changes, and resolve automatic source from present roles only. Count-in
and loop changes retain established transport/map semantics and click timing.

No inference/model setup (#6), export, arbitrary alignment, track effects, multiple
sets, A/B UI, history, journaling/restart recovery or background jobs in this slice.
Supported attachment is the existing version-1 prototype contract: verify the exact
reference input, supported preparation, all declared outputs, finite decoded PCM,
geometry and full-song zero-offset mapping. Unknown/unsupported provenance is
reported, never inferred from matching lengths. Both known evaluation backends may
be attached if they meet that contract; Roformer-SW is the production model choice.

Verification uses generated audio and focused native/frontend tests for attachment,
publication failures/replacement, cache reuse/invalidation, automatic source choice,
role math/restoration, unavailable-stem fallback and stale replies. Verify
Practice/Mapping/Notes/Stems transitions retain transport, viewport and map/note
drafts; analysis/waveforms always use reference audio. Exercise real DSP position,
loop, speed/pitch and count-in during live mix changes, including latest-request
coalescing. Existing #4 measurements remain targets, not proof of desktop behaviour.
Run formatting/lint/relevant checks, inspect compact/narrow-window UI and typing/
keyboard operation, and report actual hardware/listening/platform evidence separately.

The UI prototype remains throwaway; rewrite the selected design as production
components. Preserve CLI/native harnesses and their meaningful tests until #8 cleanup.

## Superseded planning

The original issue #5 and docs/STEMS.md anticipate multiple sets and A/B. The
agreed scope above replaces those requirements. ADR 0009's A/B evidence describes
the completed native proof; it does not require a desktop A/B UI. Recoverable publication in the original issue/proposed ADR 0008 is superseded for
#5 by the accepted direct-copy/atomic-association policy, separately from #6 jobs.
