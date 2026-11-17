//! Fixed development CPU2 stage weights from two successful full-song Bones runs.
//! Apple M5 Pro / native dev profile / 44.1kHz cache, 2026-10-16. These are
//! arithmetic mean seconds, normalized once per attempt by WorkflowTracker.
//! Admitted job setup/thread startup is included in model.verify. No per-user learning.
//! Method, complete timings and limits: docs/STEM_SEPARATION_VERIFICATION.md.
const MEAN_SECONDS: &[(&str, f64)] = &[
    ("model.verify", 21.230250666),
    ("source.hash", 0.145414208),
    ("source.decode", 2.722949647),
    ("source.check", 0.145012708),
    ("source.resample", 0.000006563),
    ("source.write", 0.833338604),
    ("source.pcm", 3.176635000),
    ("worker.start", 0.034765084),
    ("worker.input", 1.362147334),
    ("worker.load", 0.583050604),
    ("worker.infer", 307.184001562),
    ("worker.write", 0.744259562),
    ("worker.finish", 0.278199104),
    ("result.validate", 0.002853166),
    ("validation.reference.hash", 0.151661104),
    ("validation.reference.decode", 3.236570979),
    ("validation.reference.resample", 0.000360187),
    ("validation.prepared.hash", 3.276469063),
    ("validation.prepared.decode", 1.783311854),
    ("validation.bass.hash", 3.262624876),
    ("validation.bass.decode", 1.777973021),
    ("validation.bass.resample", 0.000030104),
    ("validation.drums.hash", 3.257867812),
    ("validation.drums.decode", 1.771822708),
    ("validation.drums.resample", 0.000033500),
    ("validation.guitar.hash", 3.251762375),
    ("validation.guitar.decode", 1.766405313),
    ("validation.guitar.resample", 0.000030417),
    ("validation.other.hash", 3.246982229),
    ("validation.other.decode", 1.765434855),
    ("validation.other.resample", 0.000029354),
    ("validation.piano.hash", 3.237450854),
    ("validation.piano.decode", 1.769330833),
    ("validation.piano.resample", 0.000037229),
    ("validation.vocals.hash", 3.242083688),
    ("validation.vocals.decode", 1.773006479),
    ("validation.vocals.resample", 0.000022896),
    ("validation.headroom", 5.482470125),
    ("cache.adopt", 0.000862750),
    ("cache.save", 0.007438979),
    ("attachment.publish", 0.006067791),
];
pub(super) fn weight(stage: &str) -> f64 {
    MEAN_SECONDS
        .iter()
        .find(|(id, _)| *id == stage)
        .map(|(_, seconds)| *seconds)
        .expect("Every separation checklist step needs an explicit calibrated weight")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profile_is_explicit_unique_and_positive() {
        let mut ids = std::collections::HashSet::new();
        for &(id, seconds) in MEAN_SECONDS {
            assert!(ids.insert(id));
            assert!(seconds.is_finite() && seconds > 0.);
        }
    }
}
