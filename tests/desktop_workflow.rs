use ohmyloop::{
    audio::{Audio, write_wav},
    desktop::{Action, Desktop},
    dsp::Controls,
    project::{Document, Section, SessionId, SongRecord, Store, asset_digest, waveform},
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
    let record = SongRecord::new(
        "/fixture.wav".into(),
        "a".repeat(64),
        &Audio::tone(1000, 10.0, 220.0),
    );
    store.save_record(&record).unwrap();
    let key = record.id.to_string();
    let mut doc = record.document(SessionId::new()).unwrap();
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
    let mut loaded = store.load(&key, 10.0).unwrap();
    loaded.reference_token = doc.reference_token.clone();
    assert_eq!(loaded, doc);
    assert_eq!(loaded.sections[1].id, "phrase");
    assert_eq!(loaded.loop_selection.unwrap().start, 3.0);
    doc.sections[1].id = "verse".into();
    assert!(store.save(&doc, 10.0).is_err());
    let mut reread = store.load(&key, 10.0).unwrap();
    reread.reference_token = loaded.reference_token.clone();
    assert_eq!(reread, loaded);
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
fn asset_digest_survives_file_moves_and_detects_replacement() {
    let temp = Temp::new("identity");
    let (a, b) = (temp.0.join("song.wav"), temp.0.join("renamed.wav"));
    write_wav(&a, 48_000, &Audio::tone(48_000, 0.25, 440.0).frames).unwrap();
    fs::copy(&a, &b).unwrap();
    assert_eq!(asset_digest(&a).unwrap(), asset_digest(&b).unwrap());
    write_wav(&a, 48_000, &Audio::tone(48_000, 0.25, 880.0).frames).unwrap();
    assert_ne!(asset_digest(&a).unwrap(), asset_digest(&b).unwrap());
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
    let path = temp.0.join("fixture.wav");
    write_wav(&path, 1000, &Audio::tone(1000, 12.0, 220.0).frames).unwrap();
    let loaded = desktop
        .load(Some(path.to_string_lossy().into_owned()))
        .unwrap();
    assert_eq!(loaded.song.as_ref().unwrap().duration, 12.0);
    let mut doc = loaded.document.unwrap();
    doc.controls = Controls {
        speed: 0.75,
        semitones: -3.0,
    };
    doc.sections.push(Section {
        id: "riff".into(),
        name: "First riff".into(),
        start: 0.0,
        end: 2.0,
    });
    let updated = desktop.update(doc.clone()).unwrap();
    assert!(
        updated.song.is_none(),
        "Mutations must not resend waveform data"
    );
    desktop
        .act(Action::Seek { seconds: 2.125 }, doc.reference_token.clone())
        .unwrap();
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

#[test]
fn endpoint_seeks_keep_ranges_and_disable_looping_at_b_or_outside() {
    let temp = Temp::new("endpoint-seeks");
    let desktop = Desktop::start(temp.0.clone()).unwrap();
    let loaded = desktop.load(None).unwrap();
    let mut doc = loaded.document.unwrap();
    let region = ohmyloop::engine::LoopRegion {
        start: 1.25,
        end: 2.5,
        fade_ms: 5.0,
    };
    doc.loop_selection = Some(region);
    doc.loop_enabled = true;
    desktop.update(doc).unwrap();
    let at_a = desktop
        .act(
            Action::Seek {
                seconds: region.start,
            },
            desktop.status().unwrap().reference_token.unwrap(),
        )
        .unwrap();
    assert!(at_a.document.as_ref().unwrap().loop_enabled);
    assert!(!at_a.transport.playing);
    assert!((at_a.transport.position - region.start).abs() < 0.001);
    let at_b = desktop
        .act(
            Action::Seek {
                seconds: region.end,
            },
            desktop.status().unwrap().reference_token.unwrap(),
        )
        .unwrap();
    let mut doc = at_b.document.unwrap();
    assert!(!doc.loop_enabled);
    assert_eq!(doc.loop_selection.unwrap(), region);
    assert!((at_b.transport.position - region.end).abs() < 0.001);
    doc.loop_enabled = true;
    desktop.update(doc).unwrap();
    let at_start = desktop
        .act(
            Action::Seek { seconds: 0.0 },
            desktop.status().unwrap().reference_token.unwrap(),
        )
        .unwrap();
    assert!(!at_start.document.unwrap().loop_enabled);
    assert!(at_start.transport.position < 0.001);
    desktop.shutdown();
}

#[test]
fn loading_a_map_with_the_wrong_decoded_rate_preserves_the_saved_file() {
    let temp = Temp::new("wrong-map-rate");
    let path = temp.0.join("source.wav");
    write_wav(&path, 44_100, &Audio::tone(44_100, 1.0, 440.0).frames).unwrap();
    let directory = temp.0.join("practice");
    let store = Store::new(directory.clone()).unwrap();
    let record = SongRecord::new(
        path.to_string_lossy().into_owned(),
        asset_digest(&path).unwrap(),
        &Audio::load(&path).unwrap(),
    );
    let key = record.id.to_string();
    store.save_record(&record).unwrap();
    let library = ohmyloop::library::Store::new(&directory);
    library
        .opened(
            key.clone(),
            path.to_string_lossy().into_owned(),
            ohmyloop::library::Metadata {
                title: "Source".into(),
                ..Default::default()
            },
            1,
        )
        .unwrap();
    let file = directory.join(format!("{key}.json"));
    let mut json: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    json["timeline"]["measure_map"] =
        serde_json::to_value(ohmyloop::timing::MeasureMap::empty(48000, 48000)).unwrap();
    fs::write(&file, serde_json::to_vec(&json).unwrap()).unwrap();
    let before = fs::read(&file).unwrap();
    let desktop = Desktop::start(directory).unwrap();
    assert!(
        desktop
            .load(Some(path.to_string_lossy().into_owned()))
            .is_err()
    );
    assert_eq!(fs::read(file).unwrap(), before);
    desktop.shutdown();
}

#[test]
fn review_maps_are_source_bound_and_ephemeral_even_when_clicks_are_off() {
    use ohmyloop::timing::{Edit, MeasureMap, Meter};
    let temp = Temp::new("review-map");
    let desktop = Desktop::start(temp.0.clone()).unwrap();
    let loaded = desktop.load(None).unwrap();
    let song = loaded.song.unwrap();
    let mut doc = loaded.document.unwrap();
    let accepted = MeasureMap::empty(
        song.rate,
        (song.duration * f64::from(song.rate)).round() as u64,
    )
    .edit(Edit::Build {
        start: 0,
        end: 8 * u64::from(song.rate),
        count: 2,
        meter: Meter::common(),
    })
    .unwrap()
    .map;
    doc.measure_map = Some(accepted.clone());
    desktop.update(doc).unwrap();
    desktop
        .act(Action::Seek { seconds: 5.0 }, song.reference_token.clone())
        .unwrap();
    let draft = accepted
        .edit(Edit::SetOrigin { first_number: 17 })
        .unwrap()
        .map;
    let status = desktop
        .metronome(song.reference_token.clone(), false, 0.5, Some(draft))
        .unwrap();
    assert_eq!(status.musical_position.unwrap().label, "18");
    assert_eq!(
        desktop
            .act(Action::Pause, song.reference_token.clone())
            .unwrap()
            .document
            .unwrap()
            .measure_map,
        Some(accepted.clone())
    );
    assert!(
        desktop
            .metronome("old-song".into(), false, 0.5, None)
            .is_err()
    );
    assert_eq!(
        desktop.status().unwrap().musical_position.unwrap().label,
        "18"
    );
    assert!(
        desktop
            .metronome(song.reference_token.clone(), false, f32::NAN, None)
            .is_err()
    );
    assert!(
        desktop
            .metronome(
                song.reference_token.clone(),
                false,
                0.5,
                Some(MeasureMap::empty(1, 12))
            )
            .is_err()
    );
    let status = desktop
        .metronome(song.reference_token, false, 0.5, None)
        .unwrap();
    assert_eq!(status.musical_position.unwrap().label, "2");
    desktop.shutdown();
}

#[test]
fn notes_are_song_level_and_survive_replacing_the_only_timeline() {
    let temp = Temp::new("notes");
    let store = Store::new(temp.0.clone()).unwrap();
    let audio = Audio::tone(1000, 10.0, 220.0);
    let mut record = SongRecord::new("/a.wav".into(), "a".repeat(64), &audio);
    store.save_record(&record).unwrap();
    let mut doc = record.document(SessionId::new()).unwrap();
    doc.notes = "  Drop D\n\nWork on bridge. **Plain text**  ".into();
    doc.sections.push(Section {
        id: "verse".into(),
        name: "Verse".into(),
        start: 1.0,
        end: 2.0,
    });
    store.save(&doc, 10.0).unwrap();
    record = store.record(&doc.song_key).unwrap();
    record
        .replace(
            "/b.wav".into(),
            "b".repeat(64),
            &Audio::tone(1000, 5.0, 330.0),
            false,
        )
        .unwrap();
    store.save_record(&record).unwrap();
    let current = store.load(&doc.song_key, 5.0).unwrap();
    assert_eq!(current.notes, doc.notes);
    assert!(current.sections.is_empty());
    assert!(record.timeline.sections.is_empty());
    let valid = fs::read(temp.0.join(format!("{}.json", doc.song_key))).unwrap();
    let mut invalid = current;
    invalid.notes = "é".repeat(600000);
    assert!(store.save(&invalid, 5.0).is_err());
    assert_eq!(
        fs::read(temp.0.join(format!("{}.json", doc.song_key))).unwrap(),
        valid
    );
}
