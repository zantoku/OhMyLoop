# Issue #5 scheduled implementation handoff

User authorized implementation and confirmed the full shared understanding.
The earlier delayed start is cancelled. The maintainer explicitly requested
implementation **now** on 2026-10-13. Start immediately; do not wait on the old timer.
This handoff is a task briefing, not a production feature.

## Coordinator

The original task coordinates implementation and independent review. The separate
waiting task has been instructed to stand down. User explicitly authorized an
implementation subagent, review upon completion and iteration on real findings.

## Frozen source of requirements

- `docs/STEM_DESKTOP_IMPLEMENTATION_PLAN.md`: agreed scope, all 17 accepted technical
  answers and acceptance. No remaining product questions; do not reopen agreed
  choices absent a demonstrated contradiction.
- `docs/UI_DESIGN.md`: selected visual design, transport mockup A.
- `ui/TrackMixer.prototype.svelte`: throwaway primary visual reference. Rewrite as
  production components; do not ship its switcher, fake state or fixture path.
- GitHub issue #5 in zantoku/OhMyLoop: same agreed contract; #3 and #4 completed.
- Root AGENTS.md, docs/agents/domain.md, GLOSSARY.md and ADRs 0005–0009.
  Recent #5 decisions supersede old multi-set/history/recovery proposals in
  broader planning docs. Preserve CLI/harnesses until the later #8 cleanup.

## Coordinator setup at implementation start

Inspect git status and HEAD. Preserve all pre-existing tracked, staged and untracked
changes; the prior planning docs and prototype are authorized input, not accidental
implementation changes. Capture the baseline SHA, existing diff/index diff and
pre-existing untracked-file list under a task-specific /tmp review directory.
Freeze the current agreed plan/UI document there for independent reviewers.
Do not reset, stash, overwrite unrelated user changes, commit or push without an
explicit later instruction. Work in the current checkout unless isolation is needed;
if using a worktree, ensure it actually includes the agreed uncommitted specs and
prototype instead of silently starting from stale HEAD.

## Implementation agent brief

Deliver the entire #5 package, not another proof or a partial demo. Start by reading
the requirements and tracing existing storage/native/session seams. Use cohesive
modules with explicit state and interfaces. Native owns validation/storage/cache/
playback; frontend owns presentation and desired mix/stems sessions. App should
coordinate rather than acquire file/process/storage logic. Centralize actions,
keybindings/descriptions/tooltips and typing isolation. Validate versioned data at
its boundary, preserve malformed/future records, and retain supported UUID data.

Core behaviour:

- One song / one set; minimal Stems screen next to Notes/Measure map. Attach existing
  known prototype manifests, translate verified provenance to app-owned records,
  copy only declared owned audio. No runtime prepared.wav dependency. Hash/finite/
  geometry/provenance/fullsong zerooffset checks on attachment, including owned
  output validation and safe library-relative paths.
- Confirm replacement/removal. Copy/validate new set, atomically save association,
  then remove old owned files. Ordinary errors preserve previous association and
  surface cleanup warnings. No journaling/restart recovery, background attachment,
  cancellation, history, A/B, inference, export or alignment guessing.
- Reference replacement always removes stems/cache, even when preserving the map;
  warn before confirmation. Remove stems retains reference/maps/sections/notes.
- Use native shared pre-mix/Rubber Band source seam. Original iff all present tracks
  unity/unmuted/unsoloed; any adjustment selects stems. Six canonical roles only,
  display present tracks, gain 0–100%, solo union, mute wins, all muted silence.
  Matching-role gains/mutes preserved on replacement and reopening, solo transient.
- Fixed shared headroom applies to Original and Stems whenever attached. Do not
  change factor per control or normalize stems independently. Show attenuation in
  Stems details; preserve existing final clamp and callback invariants.
- Persist prepared PCM/headroom under each song, current output-rate only. Keys
  include recorded content identity and preparation/resampling policy/config.
  Size/mtime freshness checks invalidate typical manual edits; no repeated stem
  hashing or guaranteed detection of edits preserving size/time. Reference integrity
  remains as before. Missing/invalid cache rebuilds with progress, playback disabled
  until ready. Missing/invalid stems fall back visibly to Original, disable mixer,
  preserve requested settings. Runtime read failures stop and show error.
- Attach/replace/remove pauses and retains position, blocks conflicting song/library
  changes and remains paused afterward. Screen entry/exit alone preserves transport.
  Guard library/song/reference/set generations and stale asynchronous replies.
- Production visual A: compact centered knobs, desktop 26px gaps, names/percentages/
  M/S in transport; position below Play with no Paused label. Condensed speed/pitch
  below. Equal-width permanently bordered Count-in above Loop; loop/hourglass share
  one border and use same hourglass path. Cycle Off / Loop / Loop-with-count-in.
  Only loop icon highlights in state 2; both icons in state 3. Loop-with-count-in
  enables Count-in; disabling Count-in downgrades loop; loop off keeps Count-in.
- L follows three-state cycle, Off/Loop only without accepted measure map. Preserve
  mapping count-in suppression. Count-in/repeat session-only reset on another song/
  restart; retain existing persisted loop selection/enabled. Practice-only 1–6 mutes
  fixed roles; missing roles noop. Knobs drag/wheel/key arrows/doubleclick100, live
  coalesced latest-value serialized native updates; saving debounced independently.
- Preserve consumed position, speed/pitch, loops/click/countin and Notes/Map drafts,
  viewports on mix and screen changes. Waveform/timing analysis always reference.
  Keep current decoded reference path; do not claim whole-desktop bounded memory.

Resolve routine implementation choices autonomously, without expanding scope.
Respect audio worker/callback ownership and measured #4 responsiveness/resource
criteria, with desktop evidence rather than treating proof results as new validation.
Existing local BONES sets are optional listening inputs, never copyrighted fixtures.
Find reference through current managed library rather than assume old root M4A path.

## Verification and independent review

Use generated audio for meaningful native tests: validation/path/schema errors,
replacement failure ordering, cache reuse/invalidation/rate change, source automatic
selection, roles/gain/mute/solo/restoration, fallback/read failure, reference cleanup,
stale replies, actual DSP live changes/seek/loops/countin. Frontend tests cover
sessions/coalescing, control availability, central actions/typing, screen transport/
viewport/drafts and reference-only analysis ownership. Do not mirror implementation
with trivial tests. Run formatting, strict relevant lint, focused tests and broaden
once changes justify it. Standard native setup is CARGO_HOME="$PWD/.cargo-home"
with locked/offline Cargo; frontend npm check/test/build as appropriate.

Inspect real rendering at desktop/narrow widths; distinguish mock data, actual
hardware smoke and maintainer subjective listening. Report Linux/Windows honestly.
Do not modify/delete user audio or evaluation inputs, install global dependencies,
close the GitHub issue or publish completion before review/user acceptance.

After implementer finishes, dispatch independent Standards and Spec review agents
in parallel, using .agents/skills/code-review instructions and the frozen contract.
For uncommitted work, adapt diff to the captured pre-existing state so reviewers
review implementation rather than attribute earlier user/planning changes to it.
The root/user supplied baseline is the captured implementation-start snapshot.
Fix real findings, rerun appropriate checks and obtain follow-up reviews. Report
Standards and Spec separately with remaining finding counts, changes, commands/
results, platform/hardware/listening evidence and practical limitations.
