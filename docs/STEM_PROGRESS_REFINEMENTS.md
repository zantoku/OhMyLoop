# Stem progress refinements: implementation brief

Status: implemented and verified on 2026-10-16; all product decisions Q1–Q6
remain confirmed. Independent Standards and Spec reviews have no remaining findings.
This is a follow-up to issue #6, not production runtime packaging (#7).

## Outcome

Make separation's overall progress reflect its measured time distribution, follow
the active checklist step without fighting manual scrolling, and orient all
determinate circular progress at 12 o'clock clockwise. Preserve the existing
separation, manual attachment and practice workflows and their ownership rules.

## Scope and exclusions

- Calibrated time weighting applies only to separation. Manual attachment and
  playback-cache preparation retain their task-unit weighting.
- Checklist following and pie orientation apply wherever the shared processing
  checklist is used. Do not change mixer knob sweeps or indeterminate spinner
  orientation: they are not determinate progress indicators.
- No ETA, artificial creeping progress, adaptive/per-user weights, timing history,
  new user-facing timing breakdown, runtime packaging, DSP/model-setting changes,
  or unrelated cleanup. Do not alter publication, cancellation or silence policy.

## Timing evidence and calibration

Run two complete Bones separations with the selected installed Roformer-SW model
and fixed CPU profile. Use fresh generated outputs each time and isolated managed
storage; read the original recording only. Do not mutate the maintainer's library,
current stems or model installation. Exclude model download from measured job time.
Model acquisition, if needed, must complete before timing begins.

Record per-stage wall-clock durations and end-to-end totals using monotonic clocks.
Include native verification/preparation, worker startup/input preparation, model
loading, inference, output writing, validation/playback preparation and publication.
Instrument actual boundaries before throttling UI updates. Existing worker events
currently report loading only after Python import/input work and writing after the
first file has been written: correct these attribution gaps. Include all meaningful
job time once, without overlapping intervals or presenting unmeasured work as zero.
Finalize timing on success, error or cancellation without changing those outcomes.

Write a maintainer-facing results table and methodology in the verification docs:
both runs, source duration/geometry, hardware, exact build mode, model/runtime/CPU
profile, cache conditions, totals, resulting fixed weights and limitations. Do not
commit recordings, generated audio, model files, machine-specific absolute source
paths or runtime/cache data. No persistent per-user diagnostics or new UI is needed.
Use the runnable development path for this iteration; optimized production runtime
packaging is out of scope. Do not describe development weights as calibrated for
unmeasured production builds. Record whether runs were consecutive/warm; do not
purge global OS caches or interfere with the maintainer's active app/jobs.

Choose one fixed weight profile from the two measured runs; keep weights explicit
and centralized with their provenance. Existing sampled debug evidence (~72% of
total in inference versus ~2.7% current task allocation) motivates this work but
is not sufficient calibration. These weights estimate relative progress, not ETA.

## Progress contract

- Separation's overall fraction is the sum of completed stage weights plus the
  active stage's measured fraction of its weight, divided by the fixed total.
  The overall denominator/profile must not change during an attempt.
- Use genuine byte/frame/chunk fractions where available. During unknown-duration
  work, hold the overall bar at its current position and show that active step as
  indeterminate. Advance at completion; do not fake a time-based within-stage rate.
- Keep per-step pies tied to that step's actual units, independently of its overall
  weight. Short labels, green completion, skipped states and scrollability remain.
- Overall progress and completed checklist states must not regress within a job.
  Reset cleanly for a new attempt. Reach 100% only after successful publication;
  failed/cancelled work must never display successful completion.
- Correct the mismatch between canonical checklist role order and alphabetical
  manifest validation traversal. Do not prematurely complete later rows and then
  reopen them. Handle legitimately omitted/reused stages consistently, preserving
  correct progress for near-silent stems and existing attachment/cache workflows.
- Model download retains separate byte-based progress. No weighting change there.

## Checklist following

- Opening/reopening a running workflow reveals its active row immediately.
- As processing advances, scroll only the checklist as needed to reveal the active
  row, without stealing keyboard focus or moving the containing page.
- Manual scrolling away pauses following. Resume when the user scrolls back to
  the active row. No follow toggle or other new control.
- Do not confuse a programmatic scroll with a user override. Avoid unnecessary
  scrolling on every progress tick; keep rows that are already visible still.
- On completion/failure/cancel, leave the list inspectable; do not jump to the top.
  A new attempt resets following and reveals its active work.
- Respect reduced-motion preferences. Scrolling must remain usable by pointer,
  touch/trackpad and keyboard at narrow and normal window sizes.

## Circular progress

Determinate circular progress starts at 12 o'clock and fills clockwise, per
[UI_DESIGN.md](UI_DESIGN.md#progress-indicators). The current checklist's extra
minus-90-degree transform rotates the conic gradient away from the desired origin.
Check shared usages; avoid rotating unrelated controls or indeterminate spinners.

## Verification and handoff

Add meaningful focused coverage for weighted monotonic progress, measured/unknown
fractions, proper role order, skipped/reused steps, terminal outcomes, new attempts
and unchanged manual attachment weighting. Cover checklist follow/pause/resume and
screen reopening through a small testable seam; use browser verification for real
scroll geometry, keyboard/manual scrolling and pie orientation, since jsdom does
not establish layout behavior. Keep new logic in cohesive modules, not App.svelte.

Run Rust formatting, focused tests, strict Clippy for changed native code, Svelte
checking and relevant frontend tests/build; run full native/frontend suites once
at the end. Tests use generated audio/fake events and no model download. Actual
timing runs are separate maintainer evidence, not automated copyrighted fixtures.
Document exact commands and distinguish automated, browser, model/hardware and
subjective audio checks. No subjective listening claim is necessary for this work.

After implementation, report changed files, commands/results, benchmark tables,
calibrated weights and unresolved limitations. Leave changes uncommitted and do
not publish/close GitHub issues. The parent agent will perform independent reviews
and iterate on findings before handing this back to the maintainer.
