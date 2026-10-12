use ohmyloop::{
    audio::{Audio, write_wav},
    desktop::{Action, Desktop},
    project::{AudioAssetId, Reference, SessionId, SongId, SongRecord, Store, TimelineId},
    timing::{Edit, MeasureMap, Meter},
};
use std::{fs, path::PathBuf};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("ohmyloop-uuid-{}", SongId::new()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn wave(path: &std::path::Path, rate: u32, seconds: f64, hz: f64) {
    write_wav(path, rate, &Audio::tone(rate, seconds, hz).frames).unwrap();
}
#[test]
fn typed_ids_reject_invalid_noncanonical_and_nil_values() {
    for value in [
        "song",
        "../song",
        "00000000-0000-0000-0000-000000000000",
        "FFFFFFFF-FFFF-4FFF-8FFF-FFFFFFFFFFFF",
    ] {
        assert!(serde_json::from_value::<SongId>(value.into()).is_err());
        assert!(serde_json::from_value::<AudioAssetId>(value.into()).is_err());
        assert!(serde_json::from_value::<TimelineId>(value.into()).is_err());
    }
    let song = SongId::new();
    assert_eq!(
        serde_json::from_str::<SongId>(&serde_json::to_string(&song).unwrap()).unwrap(),
        song
    );
    assert!(Reference::parse("../song").is_err());
}
#[test]
fn replacements_duplicates_retained_timelines_and_stale_sessions_are_explicit() {
    let temp = Temp::new();
    let data = temp.0.join("practice");
    let a = temp.0.join("original.wav");
    let b = temp.0.join("gain.wav");
    let c = temp.0.join("trim.wav");
    let d = temp.0.join("rate.wav");
    wave(&a, 1000, 4.0, 220.0);
    wave(&b, 1000, 4.0, 330.0);
    wave(&c, 1000, 2.0, 440.0);
    wave(&d, 2000, 4.0, 330.0);
    let desktop = Desktop::start(data.clone()).unwrap();
    let first = desktop
        .load(Some(a.to_string_lossy().into_owned()))
        .unwrap();
    let original = first.song.unwrap();
    let mut doc = first.document.unwrap();
    doc.notes = "Song notes survive recording changes".into();
    doc.controls.speed = 0.75;
    doc.sections.push(ohmyloop::project::Section {
        id: "verse".into(),
        name: "Verse".into(),
        start: 1.0,
        end: 3.0,
    });
    doc.measure_map = Some(
        MeasureMap::empty(1000, 4000)
            .edit(Edit::Build {
                start: 0,
                end: 4000,
                count: 2,
                meter: Meter::common(),
            })
            .unwrap()
            .map,
    );
    desktop.update(doc.clone()).unwrap();
    desktop
        .act(
            Action::Seek { seconds: 1.5 },
            original.reference_token.clone(),
        )
        .unwrap();
    let replacement = desktop
        .replace_reference(
            original.reference_token.clone(),
            b.to_string_lossy().into_owned(),
            true,
        )
        .unwrap();
    let same = replacement.song.unwrap();
    let same_doc = replacement.document.unwrap();
    assert_eq!(same.key, original.key);
    assert_ne!(same.asset_id, original.asset_id);
    assert_eq!(same.timeline_id, original.timeline_id);
    assert_eq!(same.reference_revision, 2);
    assert_eq!(same_doc.notes, doc.notes);
    assert_eq!(same_doc.controls, doc.controls);
    assert_eq!(same_doc.measure_map, doc.measure_map);
    assert_eq!(same_doc.sections, doc.sections);
    assert!((same_doc.last_position - 1.5).abs() < 0.001);
    assert!(desktop.source(original.reference_token.clone()).is_err());
    assert!(desktop.update(doc.clone()).is_err());
    assert!(
        desktop
            .act(Action::Pause, original.reference_token.clone())
            .is_err()
    );
    assert!(
        desktop
            .metronome(original.reference_token.clone(), false, 0.5, None)
            .is_err()
    );
    assert!(
        desktop
            .count_in(original.reference_token.clone(), false, false, 0.5)
            .is_err()
    );
    assert!(
        desktop
            .replace_reference(
                same.reference_token.clone(),
                c.to_string_lossy().into_owned(),
                true
            )
            .is_err()
    );
    assert!(
        desktop
            .replace_reference(
                same.reference_token.clone(),
                d.to_string_lossy().into_owned(),
                true
            )
            .is_err()
    );
    assert_eq!(
        desktop.status().unwrap().reference_token,
        Some(same.reference_token.clone())
    );
    let trimmed = desktop
        .replace_reference(
            same.reference_token.clone(),
            c.to_string_lossy().into_owned(),
            false,
        )
        .unwrap();
    let trimmed_song = trimmed.song.unwrap();
    let trimmed_doc = trimmed.document.unwrap();
    assert_eq!(trimmed_song.key, original.key);
    assert_ne!(trimmed_song.timeline_id, original.timeline_id);
    assert_eq!(trimmed_doc.notes, doc.notes);
    assert_eq!(trimmed_doc.controls, doc.controls);
    assert!(trimmed_doc.sections.is_empty());
    assert!(trimmed_doc.measure_map.is_none());
    assert_eq!(trimmed_doc.last_position, 0.0);
    let prior = desktop
        .select_recording(
            trimmed_song.reference_token.clone(),
            original.asset_id.to_string(),
        )
        .unwrap();
    let prior_song = prior.song.unwrap();
    let prior_doc = prior.document.unwrap();
    assert_eq!(prior_doc.sections, doc.sections);
    assert_eq!(prior_doc.measure_map, doc.measure_map);
    assert_eq!(prior_song.retained_recordings.len(), 3);
    let reopened = desktop.open_library(original.key.clone()).unwrap();
    let reopened_song = reopened.song.unwrap();
    assert_eq!(reopened_song.asset_id, original.asset_id);
    assert_ne!(reopened_song.session_id, prior_song.session_id);
    assert!(desktop.source(prior_song.reference_token).is_err());
    assert!(desktop.update(prior_doc).is_err());
    let existing = desktop
        .load(Some(a.to_string_lossy().into_owned()))
        .unwrap();
    assert_eq!(existing.song.unwrap().key, original.key);
    let duplicate = desktop
        .load_separate(a.to_string_lossy().into_owned())
        .unwrap();
    let duplicate_song = duplicate.song.unwrap();
    assert_ne!(duplicate_song.key, original.key);
    assert!(duplicate.document.unwrap().notes.is_empty());
    assert_eq!(desktop.library().unwrap().songs.len(), 2);
    desktop.shutdown();
    let restored = Desktop::start(data).unwrap();
    let snapshot = restored.restore().unwrap();
    assert_eq!(snapshot.song.as_ref().unwrap().key, duplicate_song.key);
    assert_ne!(snapshot.song.unwrap().session_id, duplicate_song.session_id);
    restored.shutdown();
    assert!(a.exists() && b.exists() && c.exists() && d.exists());
}
#[test]
fn future_and_malformed_uuid_records_are_never_reset_or_overwritten() {
    let temp = Temp::new();
    let store = Store::new(temp.0.clone()).unwrap();
    let record = SongRecord::new(
        "/fixture.wav".into(),
        "a".repeat(64),
        &Audio::tone(1000, 4.0, 220.0),
    );
    store.save_record(&record).unwrap();
    let path = temp.0.join(format!("{}.json", record.id));
    let mut value = serde_json::to_value(&record).unwrap();
    value["version"] = 99.into();
    let bytes = serde_json::to_vec(&value).unwrap();
    fs::write(&path, &bytes).unwrap();
    let store = Store::new(temp.0.clone()).unwrap();
    assert!(store.record(&record.id.to_string()).is_err());
    assert!(
        store
            .save(&record.document(SessionId::new()).unwrap(), 4.0)
            .is_err()
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    fs::write(&path, b"{invalid UUID record").unwrap();
    assert!(
        Store::new(temp.0.clone())
            .unwrap()
            .record(&record.id.to_string())
            .is_err()
    );
    assert_eq!(fs::read(path).unwrap(), b"{invalid UUID record");
}
#[test]
fn library_recovers_changed_or_missing_reference_after_restart_without_reidentifying_song() {
    let temp = Temp::new();
    let data = temp.0.join("practice");
    let original = temp.0.join("source.wav");
    wave(&original, 1000, 4.0, 220.0);
    let desktop = Desktop::start(data.clone()).unwrap();
    let loaded = desktop
        .load(Some(original.to_string_lossy().into_owned()))
        .unwrap();
    let song = loaded.song.unwrap();
    let mut doc = loaded.document.unwrap();
    doc.notes = "Kept notes".into();
    doc.sections.push(ohmyloop::project::Section {
        id: "riff".into(),
        name: "Riff".into(),
        start: 1.0,
        end: 2.0,
    });
    desktop.update(doc.clone()).unwrap();
    desktop.shutdown();
    wave(&original, 1000, 4.0, 330.0);
    let desktop = Desktop::start(data.clone()).unwrap();
    assert!(desktop.restore().is_err());
    assert!(desktop.open_library(song.key.clone()).is_err());
    let token = desktop.library_reference(song.key.clone()).unwrap();
    let recovered = desktop
        .replace_library_reference(
            song.key.clone(),
            token.clone(),
            original.to_string_lossy().into_owned(),
            true,
        )
        .unwrap();
    let recovered_song = recovered.song.unwrap();
    assert_eq!(recovered_song.key, song.key);
    assert_eq!(recovered_song.timeline_id, song.timeline_id);
    assert_eq!(recovered.document.unwrap().sections, doc.sections);
    assert!(
        desktop
            .replace_library_reference(
                song.key.clone(),
                token,
                original.to_string_lossy().into_owned(),
                true
            )
            .is_err()
    );
    desktop.shutdown();
    fs::remove_file(&original).unwrap();
    let trimmed = temp.0.join("trimmed.wav");
    wave(&trimmed, 1000, 2.0, 440.0);
    let desktop = Desktop::start(data).unwrap();
    assert!(desktop.restore().is_err());
    let token = desktop.library_reference(song.key.clone()).unwrap();
    let recovered = desktop
        .replace_library_reference(
            song.key.clone(),
            token,
            trimmed.to_string_lossy().into_owned(),
            false,
        )
        .unwrap();
    assert_eq!(recovered.song.as_ref().unwrap().key, song.key);
    assert_ne!(recovered.song.unwrap().timeline_id, song.timeline_id);
    let doc = recovered.document.unwrap();
    assert_eq!(doc.notes, "Kept notes");
    assert!(doc.sections.is_empty());
    desktop.shutdown();
}
#[test]
fn same_song_reopen_flushes_the_current_position_before_restoration() {
    let temp = Temp::new();
    let path = temp.0.join("source.wav");
    wave(&path, 1000, 4.0, 220.0);
    let desktop = Desktop::start(temp.0.join("practice")).unwrap();
    let first = desktop
        .load(Some(path.to_string_lossy().into_owned()))
        .unwrap()
        .song
        .unwrap();
    desktop
        .act(Action::Seek { seconds: 2.25 }, first.reference_token)
        .unwrap();
    let reopened = desktop.open_library(first.key).unwrap();
    assert!((reopened.transport.position - 2.25).abs() < 0.001);
    desktop.shutdown();
}
#[test]
fn manual_open_preserves_malformed_future_and_oversized_last_song_records() {
    for bytes in [
        b"{malformed last song".to_vec(),
        b"{\"version\":99,\"song_id\":\"11111111-1111-4111-8111-111111111111\"}".to_vec(),
        vec![b' '; 4097],
    ] {
        let temp = Temp::new();
        let data = temp.0.join("practice");
        Store::new(data.clone()).unwrap();
        let last = data.join("last-song.json");
        fs::write(&last, &bytes).unwrap();
        let audio = temp.0.join("song.wav");
        wave(&audio, 1000, 1.0, 220.0);
        let desktop = Desktop::start(data).unwrap();
        assert!(desktop.restore().is_err());
        let demo = desktop.load(None).unwrap().song.unwrap();
        let opened = desktop
            .load(Some(audio.to_string_lossy().into_owned()))
            .unwrap();
        assert!(opened.library_error.as_ref().unwrap().contains("last-song"));
        assert_eq!(fs::read(last).unwrap(), bytes);
        assert_ne!(desktop.status().unwrap().song_key, Some(demo.key));
        desktop.shutdown();
    }
}
#[test]
fn corrupt_or_unwritable_projections_do_not_block_restoration_or_strand_reference_commits() {
    let temp = Temp::new();
    let data = temp.0.join("practice");
    let a = temp.0.join("original.wav");
    let b = temp.0.join("replacement.wav");
    wave(&a, 1000, 4.0, 220.0);
    wave(&b, 1000, 4.0, 330.0);
    let desktop = Desktop::start(data.clone()).unwrap();
    let first = desktop
        .load(Some(a.to_string_lossy().into_owned()))
        .unwrap();
    let song = first.song.unwrap();
    let mut doc = first.document.unwrap();
    doc.notes = "Keep these notes".into();
    desktop.update(doc).unwrap();
    desktop.shutdown();
    let catalogue = data.join("library.json");
    let valid = fs::read(&catalogue).unwrap();
    fs::write(&catalogue, b"{corrupt catalogue").unwrap();
    let desktop = Desktop::start(data.clone()).unwrap();
    let restored = desktop.restore().unwrap();
    assert_eq!(restored.song.as_ref().unwrap().key, song.key);
    assert_eq!(
        restored.document.as_ref().unwrap().notes,
        "Keep these notes"
    );
    assert!(restored.library_error.is_some());
    assert_eq!(fs::read(&catalogue).unwrap(), b"{corrupt catalogue");
    let token = restored.song.unwrap().reference_token;
    let changed = desktop
        .replace_reference(token, b.to_string_lossy().into_owned(), true)
        .unwrap();
    let current = changed.song.unwrap();
    assert!(changed.library_error.is_some());
    assert_eq!(
        Store::new(data.clone())
            .unwrap()
            .record(&song.key)
            .unwrap()
            .reference
            .asset,
        current.asset_id
    );
    assert!(desktop.source(current.reference_token.clone()).is_ok());
    // Write failure of a valid catalogue and an unknown future last-song record
    // preserve their bytes while a retained-reference commit still activates.
    fs::write(&catalogue, &valid).unwrap();
    fs::create_dir(data.join("library.json.tmp")).unwrap();
    let last = data.join("last-song.json");
    let future = b"{\"version\":99,\"song_id\":\"11111111-1111-4111-8111-111111111111\"}";
    fs::write(&last, future).unwrap();
    let selected = desktop
        .select_recording(current.reference_token, song.asset_id.to_string())
        .unwrap();
    let selected_song = selected.song.unwrap();
    assert_eq!(selected_song.asset_id, song.asset_id);
    assert!(
        selected
            .library_error
            .as_ref()
            .unwrap()
            .contains("last-song")
    );
    assert_eq!(fs::read(&catalogue).unwrap(), valid);
    assert_eq!(fs::read(last).unwrap(), future);
    let mut doc = selected.document.unwrap();
    doc.notes = "Still editable after projection failure".into();
    desktop.update(doc).unwrap();
    assert_eq!(
        Store::new(data)
            .unwrap()
            .record(&song.key)
            .unwrap()
            .reference
            .asset,
        selected_song.asset_id
    );
    desktop.shutdown();
}
