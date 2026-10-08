use ohmyloop::{
    audio::{Audio, write_wav},
    desktop::{Action, Desktop},
    dsp::Controls,
    project::{Document, Section, Store, song_key, waveform},
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

struct Temp(PathBuf);
impl Temp {
    fn new(label: &str) -> Self {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("ohmyloop-{label}-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn nested_sections_keep_ids_and_temporary_loops_are_independent() {
    let temp = Temp::new("document");
    let store = Store::new(temp.0.clone()).unwrap();
    let mut doc = Document::new("test-song".into());
    doc.sections = vec![
        Section {
            id: "verse".into(),
            name: "Verse".into(),
            start: 1.0,
            end: 8.0,
        },
        Section {
            id: "phrase".into(),
            name: "Bend".into(),
            start: 2.0,
            end: 2.5,
        },
    ];
    doc.loop_selection = Some(ohmyloop::engine::LoopRegion {
        start: 3.0,
        end: 4.0,
        fade_ms: 5.0,
    });
    doc.loop_enabled = true;
    store.save(&doc, 10.0).unwrap();
    doc.sections[1].name = "Bend and release".into();
    doc.sections[1].start = 2.1;
    store.save(&doc, 10.0).unwrap();
    let loaded = store.load("test-song", 10.0).unwrap();
    assert_eq!(loaded, doc);
    assert_eq!(loaded.sections[1].id, "phrase");
    assert_eq!(loaded.loop_selection.unwrap().start, 3.0);
    doc.sections[1].id = "verse".into();
    assert!(store.save(&doc, 10.0).is_err());
    assert_eq!(store.load("test-song", 10.0).unwrap(), loaded);
}

#[test]
fn corrupted_or_future_documents_are_preserved_instead_of_overwritten() {
    let temp = Temp::new("invalid-document");
    let store = Store::new(temp.0.clone()).unwrap();
    fs::write(temp.0.join("song.json"), b"{broken JSON").unwrap();
    assert!(store.load("song", 12.0).is_err());
    assert_eq!(fs::read(temp.0.join("song.json")).unwrap(), b"{broken JSON");
    let mut doc = Document::new("song".into());
    doc.version = 99;
    assert!(store.save(&doc, 12.0).is_err());
    assert!(store.load("../escape", 12.0).is_err());
}

#[test]
fn content_identity_survives_file_moves_and_detects_replacement() {
    let temp = Temp::new("identity");
    let (a, b) = (temp.0.join("song.wav"), temp.0.join("renamed.wav"));
    write_wav(&a, 48_000, &Audio::tone(48_000, 0.25, 440.0).frames).unwrap();
    fs::copy(&a, &b).unwrap();
    assert_eq!(song_key(&a).unwrap(), song_key(&b).unwrap());
    write_wav(&a, 48_000, &Audio::tone(48_000, 0.25, 880.0).frames).unwrap();
    assert_ne!(song_key(&a).unwrap(), song_key(&b).unwrap());
}

#[test]
fn waveform_preserves_transients_including_the_last_partial_bin() {
    let mut audio = Audio {
        rate: 48_000,
        frames: vec![[0.0; 2]; 131_073],
    };
    audio.frames[47_111] = [-0.9, 0.8];
    audio.frames[131_072] = [0.7, -0.6];
    let peaks = waveform(&audio);
    assert!(peaks.len() <= 65_536);
    assert!(peaks.contains(&[-0.9, 0.8]));
    assert_eq!(peaks.last(), Some(&[-0.6, 0.7]));
}

#[test]
fn bandlimited_conversion_preserves_duration_pitch_and_stereo() {
    let mut audio = Audio::tone(44_100, 1.0, 440.0);
    for f in &mut audio.frames {
        f[1] *= -0.5;
    }
    let converted = audio.resample(48_000).unwrap();
    assert_eq!(converted.frames.len(), 48_000);
    let central = &converted.frames[12_000..36_000];
    let crosses = central
        .windows(2)
        .filter(|p| p[0][0] <= 0.0 && p[1][0] > 0.0)
        .count();
    assert!((crosses as f64 * 2.0 - 440.0).abs() <= 2.0);
    assert!(central.iter().all(|f| (f[1] + f[0] * 0.5).abs() < 1e-6));
}

#[test]
fn downsampling_suppresses_frequencies_above_the_new_nyquist_limit() {
    let audio = Audio::tone(48_000, 1.0, 18_000.0);
    let converted = audio.resample(16_000).unwrap();
    let rms = (converted.frames[4000..12000]
        .iter()
        .map(|f| f[0] as f64 * f[0] as f64)
        .sum::<f64>()
        / 8000.0)
        .sqrt();
    assert!(rms < 0.002, "alias RMS: {rms}");
}

#[test]
fn desktop_controller_restores_sections_controls_and_source_position() {
    let temp = Temp::new("controller");
    let desktop = Desktop::start(temp.0.clone()).unwrap();
    let loaded = desktop.load(None).unwrap();
    assert_eq!(loaded.song.unwrap().duration, 12.0);
    let mut doc = loaded.document.unwrap();
    doc.controls = Controls {
        speed: 0.75,
        semitones: -3.0,
    };
    doc.sections[0].name = "First riff".into();
    let updated = desktop.update(doc.clone()).unwrap();
    assert!(
        updated.song.is_none(),
        "Mutations must not resend waveform data"
    );
    desktop.act(Action::Seek { seconds: 2.125 }).unwrap();
    desktop.shutdown();
    let restored = Desktop::start(temp.0.clone()).unwrap();
    let snapshot = restored.restore().unwrap();
    let restored_doc = snapshot.document.unwrap();
    assert_eq!(restored_doc.controls, doc.controls);
    assert_eq!(restored_doc.sections[0].id, doc.sections[0].id);
    assert_eq!(restored_doc.sections[0].name, "First riff");
    assert!((snapshot.transport.position - 2.125).abs() < 0.001);
    restored.shutdown();
}
