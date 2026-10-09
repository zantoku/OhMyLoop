#![cfg(feature = "beat-analysis")]
use ohmyloop::{analysis::AnalysisManager, audio::Audio};
use std::{
    fs,
    sync::Arc,
    time::{Duration, Instant},
};
#[test]
fn missing_or_corrupt_models_fail_without_results_or_damage_to_input_files() {
    let directory =
        std::env::temp_dir().join(format!("ohmyloop-model-test-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let models = directory.join("models");
    fs::create_dir_all(&models).unwrap();
    let manager = AnalysisManager::new(models.clone(), directory.join("cache")).unwrap();
    let audio = Arc::new(Audio::tone(48_000, 0.1, 440.0));
    assert!(
        manager
            .analyze(
                std::env::current_exe().unwrap(),
                audio.clone(),
                "song".into(),
                0
            )
            .is_err()
    );
    assert!(!manager.status().running);
    assert_eq!(manager.status().model_path, models.to_string_lossy());
    assert!(manager.status().progress.is_none());
    fs::write(models.join("mel_spectrogram.onnx"), b"invalid mel").unwrap();
    fs::write(models.join("beat_this.onnx"), b"invalid beats").unwrap();
    manager
        .analyze(std::env::current_exe().unwrap(), audio, "song".into(), 7)
        .unwrap();
    let began = Instant::now();
    while manager.status().running {
        assert!(began.elapsed() < Duration::from_secs(2));
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(manager.status().phase, "Failed");
    assert!(manager.status().result.is_none());
    assert!(manager.status().error.unwrap().contains("checksum"));
    assert!(manager.worker_pid().is_none());
    assert_eq!(
        fs::read(models.join("beat_this.onnx")).unwrap(),
        b"invalid beats"
    );
    manager.shutdown();
    fs::remove_dir_all(directory).unwrap();
}
