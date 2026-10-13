use ohmyloop::{
    audio::{Audio, write_wav},
    library::Metadata,
    managed::{ManagedLibrary, PreferenceData, Preferences},
    project::SessionId,
};
use std::{fs, path::PathBuf};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("ohmyloop-managed-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn audio(&self, name: &str, seconds: f64) -> PathBuf {
        let path = self.0.join(name);
        let audio = Audio::tone(44100, seconds, 440.0);
        write_wav(&path, audio.rate, &audio.frames).unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn independent_copies_relative_move_and_permanent_deletion() {
    let f = Fixture::new();
    let source = f.audio("source.wav", 0.1);
    let original = fs::read(&source).unwrap();
    let lib = ManagedLibrary::create(&f.0).unwrap();
    let mut progress = vec![];
    let p = lib
        .import(&source, |done, total| progress.push((done, total)))
        .unwrap();
    assert!(lib.catalogue().unwrap().songs.is_empty());
    let p = lib.commit_import(p).unwrap();
    let key = p.record.id.to_string();
    assert_eq!(progress.last().unwrap().0, original.len() as u64);
    assert_eq!(lib.duplicates(&p.record.asset.digest).unwrap().len(), 1);
    let q = lib
        .commit_import(lib.import(&source, |_, _| {}).unwrap())
        .unwrap();
    assert_ne!(p.record.id, q.record.id);
    assert_ne!(
        lib.asset_path(&p.record).unwrap(),
        lib.asset_path(&q.record).unwrap()
    );
    lib.update_metadata(
        &key,
        Metadata {
            title: "Edited".into(),
            artist: "Artist".into(),
            album: "".into(),
        },
    )
    .unwrap();
    let mut document = p.record.document(SessionId::new()).unwrap();
    document.notes = "My notes".into();
    lib.save_document(&document).unwrap();
    let oldid = lib.id().to_owned();
    let newroot = f.0.join("moved");
    fs::rename(lib.root(), &newroot).unwrap();
    let moved = ManagedLibrary::open(&newroot).unwrap();
    assert_eq!(moved.id(), oldid);
    let reopened = moved.open_song(&key).unwrap();
    assert_eq!(reopened.record.notes, "My notes");
    assert_eq!(reopened.metadata.title, "Edited");
    moved.delete(&key).unwrap();
    assert!(moved.record(&key).is_err());
    assert!(moved.open_song(&q.record.id.to_string()).is_ok());
    assert_eq!(fs::read(source).unwrap(), original);
}
#[test]
fn replacement_preserves_current_edits_and_removes_only_previous_asset() {
    let f = Fixture::new();
    let source = f.audio("source.wav", 0.1);
    let replacement = f.audio("new.wav", 0.1);
    let changed = f.audio("short.wav", 0.05);
    let lib = ManagedLibrary::create(&f.0).unwrap();
    let p = lib
        .commit_import(lib.import(&source, |_, _| {}).unwrap())
        .unwrap();
    let key = p.record.id.to_string();
    let oldpath = lib.asset_path(&p.record).unwrap();
    let pending = lib
        .prepare_replace(&key, &replacement, true, |_, _| {})
        .unwrap();
    let mut doc = p.record.document(SessionId::new()).unwrap();
    doc.notes = "Written while copying".into();
    doc.last_position = 0.025;
    lib.save_document(&doc).unwrap();
    let next = lib.commit_replace(pending).unwrap();
    assert_eq!(next.record.notes, doc.notes);
    assert_eq!(next.record.timeline.last_position, 0.025);
    assert!(!oldpath.exists());
    assert!(
        lib.prepare_replace(&key, &changed, true, |_, _| {})
            .is_err()
    );
    assert!(lib.open_song(&key).is_ok());
    let reset = lib
        .commit_replace(
            lib.prepare_replace(&key, &changed, false, |_, _| {})
                .unwrap(),
        )
        .unwrap();
    assert_eq!(reset.record.timeline.last_position, 0.0);
    assert_eq!(reset.record.notes, doc.notes);
}
#[test]
fn changed_audio_invalid_schemas_and_failed_decode_preserve_sources() {
    let f = Fixture::new();
    let source = f.audio("source.wav", 0.1);
    let lib = ManagedLibrary::create(&f.0).unwrap();
    let p = lib
        .commit_import(lib.import(&source, |_, _| {}).unwrap())
        .unwrap();
    let path = lib.asset_path(&p.record).unwrap();
    fs::write(&path, b"changed").unwrap();
    assert!(
        lib.open_song(&p.record.id.to_string())
            .unwrap_err_string()
            .contains("changed")
    );
    let invalid = f.0.join("bad.wav");
    fs::write(&invalid, b"not audio").unwrap();
    assert!(lib.import(&invalid, |_, _| {}).is_err());
    assert_eq!(fs::read(invalid).unwrap(), b"not audio");
    assert_eq!(lib.catalogue().unwrap().songs.len(), 1);
    let songpath = lib.root().join(format!("songs/{}/song.json", p.record.id));
    let mut json: serde_json::Value =
        serde_json::from_slice(&fs::read(&songpath).unwrap()).unwrap();
    json["version"] = 99.into();
    let bytes = serde_json::to_vec(&json).unwrap();
    fs::write(&songpath, &bytes).unwrap();
    assert!(lib.save_record(&p.record).is_err());
    assert_eq!(fs::read(&songpath).unwrap(), bytes);
}
trait ErrorString {
    fn unwrap_err_string(self) -> String;
}
impl<T> ErrorString for anyhow::Result<T> {
    fn unwrap_err_string(self) -> String {
        match self {
            Ok(_) => panic!("expected failure"),
            Err(e) => e.to_string(),
        }
    }
}
#[test]
fn unsafe_paths_and_preferences_are_not_overwritten() {
    let f = Fixture::new();
    let source = f.audio("source.wav", 0.1);
    let lib = ManagedLibrary::create(&f.0).unwrap();
    let p = lib
        .commit_import(lib.import(&source, |_, _| {}).unwrap())
        .unwrap();
    let mut record = p.record;
    record.asset.path = "../source.wav".into();
    assert!(lib.save_record(&record).is_err());
    assert!(ManagedLibrary::create(&f.0).is_err());
    let prefs = Preferences::new(&f.0);
    prefs.save(&PreferenceData::default()).unwrap();
    let path = f.0.join("managed-library.json");
    fs::write(&path, b"{\"version\":99}").unwrap();
    assert!(prefs.save(&PreferenceData::default()).is_err());
    assert_eq!(fs::read(path).unwrap(), b"{\"version\":99}");
}

#[test]
fn failed_document_save_keeps_previous_bytes_and_projection_failures_are_warnings() {
    let f = Fixture::new();
    let source = f.audio("source.wav", 0.1);
    let lib = ManagedLibrary::create(&f.0).unwrap();
    let p = lib
        .commit_import(lib.import(&source, |_, _| {}).unwrap())
        .unwrap();
    let path = lib.root().join(format!("songs/{}/song.json", p.record.id));
    let bytes = fs::read(&path).unwrap();
    fs::create_dir(path.with_extension("json.tmp")).unwrap();
    let mut record = p.record.clone();
    record.notes = "Not saved".into();
    assert!(lib.save_record(&record).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    fs::remove_dir(path.with_extension("json.tmp")).unwrap();
    let projection = lib.root().join("catalogue.json");
    fs::write(&projection, b"invalid projection").unwrap();
    let q = lib
        .commit_import(lib.import(&source, |_, _| {}).unwrap())
        .unwrap();
    assert!(!q.warnings.is_empty());
    assert!(lib.open_song(&q.record.id.to_string()).is_ok());
    assert_eq!(lib.catalogue().unwrap().songs.len(), 2);
    assert_eq!(fs::read(&projection).unwrap(), b"invalid projection");
    assert!(lib.discard_prepared(&q).is_err());
    assert!(lib.open_song(&q.record.id.to_string()).is_ok());
}

#[test]
fn stale_replacement_discards_only_its_uncommitted_copy() {
    let f = Fixture::new();
    let source = f.audio("source.wav", 0.1);
    let lib = ManagedLibrary::create(&f.0).unwrap();
    let p = lib
        .commit_import(lib.import(&source, |_, _| {}).unwrap())
        .unwrap();
    let key = p.record.id.to_string();
    let first = lib.prepare_replace(&key, &source, true, |_, _| {}).unwrap();
    let stale = lib.prepare_replace(&key, &source, true, |_, _| {}).unwrap();
    let stale_path = lib.asset_path(&stale.prepared.record).unwrap();
    let committed = lib.commit_replace(first).unwrap();
    assert!(lib.commit_replace(stale).is_err());
    assert!(!stale_path.exists());
    assert!(lib.asset_path(&committed.record).unwrap().exists());
    assert!(lib.open_song(&key).is_ok());
}
#[cfg(unix)]
#[test]
fn symlinked_documents_assets_and_deletion_targets_are_rejected() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let source = f.audio("source.wav", 0.1);
    let lib = ManagedLibrary::create(&f.0).unwrap();
    let p = lib
        .commit_import(lib.import(&source, |_, _| {}).unwrap())
        .unwrap();
    let key = p.record.id.to_string();
    let directory = lib.root().join(format!("songs/{key}"));
    symlink(&source, directory.join("external")).unwrap();
    assert!(lib.delete(&key).is_err());
    assert!(source.exists());
    fs::remove_file(directory.join("external")).unwrap();
    let path = lib.asset_path(&p.record).unwrap();
    fs::remove_file(&path).unwrap();
    symlink(&source, &path).unwrap();
    assert!(lib.open_song(&key).is_err());
    assert!(lib.save_record(&p.record).is_err());
}
