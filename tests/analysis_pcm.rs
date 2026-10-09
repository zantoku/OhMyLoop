use ohmyloop::{
    analysis_pcm::{MODEL_RATE, write_model_pcm, write_model_pcm_with_progress},
    audio::Audio,
};
use std::sync::atomic::AtomicBool;
#[test]
fn prepared_pcm_has_exact_duration_and_no_leading_or_tail_delay() {
    for rate in [8000, 22050, 44100, 48000, 96000] {
        let mut audio = Audio {
            rate,
            frames: vec![[0.0; 2]; rate as usize],
        };
        let points = [
            rate as usize / 10,
            rate as usize / 2,
            rate as usize * 9 / 10,
        ];
        for &at in &points {
            audio.frames[at] = [1.0, 1.0];
        }
        let path = std::env::temp_dir().join(format!(
            "ohmyloop-analysis-pcm-{}-{rate}.wav",
            std::process::id()
        ));
        let mut progress = vec![];
        write_model_pcm_with_progress(&audio, &path, &AtomicBool::new(false), |done, total| {
            progress.push((done, total))
        })
        .unwrap();
        assert_eq!(progress.first(), Some(&(0, rate as u64)));
        assert_eq!(progress.last(), Some(&(rate as u64, rate as u64)));
        assert!(progress.windows(2).all(|pair| pair[0].0 <= pair[1].0));
        let reader = hound::WavReader::open(&path).unwrap();
        assert_eq!(reader.spec().channels, 1);
        assert_eq!(reader.spec().sample_rate, MODEL_RATE);
        let samples = reader
            .into_samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(samples.len(), MODEL_RATE as usize);
        for at in points {
            let expected = (at as f64 * f64::from(MODEL_RATE) / f64::from(rate)).round() as usize;
            let found = (expected - 10..=expected + 10)
                .max_by(|a, b| samples[*a].abs().total_cmp(&samples[*b].abs()))
                .unwrap();
            assert!(
                found.abs_diff(expected) <= 1,
                "rate={rate}: expected={expected},found={found}"
            );
        }
        std::fs::remove_file(path).unwrap();
    }
}
#[test]
fn preparation_is_cancellable_and_rejects_nonfinite_input() {
    let path = std::env::temp_dir().join(format!(
        "ohmyloop-analysis-cancel-{}.wav",
        std::process::id()
    ));
    let mut audio = Audio::tone(48000, 0.2, 440.0);
    assert!(write_model_pcm(&audio, &path, &AtomicBool::new(true)).is_err());
    audio.frames[0][0] = f32::NAN;
    assert!(write_model_pcm(&audio, &path, &AtomicBool::new(false)).is_err());
    let _ = std::fs::remove_file(path);
}
