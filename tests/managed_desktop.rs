use ohmyloop::{
    audio::{Audio, write_wav},
    desktop::{Action, Desktop},
    managed::ManagedLibrary,
    project::SongId,
};
use std::{fs, path::PathBuf};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("ohmyloop-managed-desktop-{}", SongId::new()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn make_source(temp: &Temp, name: &str, hz: f64) -> String {
    let path = temp.0.join(name);
    write_wav(&path, 1000, &Audio::tone(1000, 4.0, hz).frames).unwrap();
    path.to_string_lossy().into_owned()
}
#[test]
fn managed_desktop_import_replace_restart_switch_and_delete() {
    let temp = Temp::new();
    let data = temp.0.join("app-data");
    let source = make_source(&temp, "source.wav", 220.0);
    let original = fs::read(&source).unwrap();
    let desktop = Desktop::start(data.clone()).unwrap();
    assert!(desktop.restore_managed().unwrap().song.is_none());
    desktop.load(None).unwrap();
    assert!(desktop.managed_status().unwrap().selected.is_none());
    let switched = desktop
        .select_library(temp.0.to_string_lossy().into_owned(), true, false)
        .unwrap();
    assert!(switched.song.is_none());
    assert!(switched.transport.song_key.is_none());
    let root = switched.library_status.selected.unwrap().root;
    let first = desktop.import_managed(source.clone(), |_, _| {}).unwrap();
    let song = first.song.unwrap();
    assert!(!first.transport.playing);
    assert_ne!(song.path.as_deref(), Some(source.as_str()));
    let mut document = first.document.unwrap();
    document.notes = "stay with UUID".into();
    desktop.update(document).unwrap();
    assert_eq!(
        desktop.import_candidates(source.clone()).unwrap()[0].song_key,
        song.key
    );
    let replacement = make_source(&temp, "replacement.wav", 330.0);
    let replaced = desktop
        .replace_reference(song.reference_token.clone(), replacement, true)
        .unwrap();
    assert_eq!(replaced.song.as_ref().unwrap().key, song.key);
    assert_eq!(replaced.document.unwrap().notes, "stay with UUID");
    assert!(!std::path::Path::new(song.path.as_ref().unwrap()).exists());
    assert!(desktop.act(Action::Pause, song.reference_token).is_err());
    desktop.shutdown();
    let restarted = Desktop::start(data).unwrap();
    let restored = restarted.restore_managed().unwrap();
    assert_eq!(restored.song.as_ref().unwrap().key, song.key);
    assert!(
        restarted
            .select_library(
                temp.0.join("missing").to_string_lossy().into_owned(),
                false,
                false
            )
            .is_err()
    );
    assert_eq!(
        restarted.managed_status().unwrap().selected.unwrap().root,
        root
    );
    let deleted = restarted.delete_library_song(song.key).unwrap();
    assert!(deleted.song.is_none());
    assert!(deleted.document.is_none());
    assert!(deleted.transport.song_key.is_none());
    assert!(restarted.library().unwrap().songs.is_empty());
    assert_eq!(fs::read(source).unwrap(), original);
    restarted.shutdown();
}
#[test]
fn import_worker_remains_responsive_and_stale_library_commit_cleans_copy() {
    let temp = Temp::new();
    let source = make_source(&temp, "source.wav", 220.0);
    let desktop = Desktop::start(temp.0.join("data")).unwrap();
    desktop
        .select_library(temp.0.to_string_lossy().into_owned(), true, false)
        .unwrap();
    let root = desktop.managed_status().unwrap().selected.unwrap().root;
    let mut switched = false;
    let result = desktop.import_managed(source, |copied, _| {
        assert!(desktop.status().is_ok());
        if copied == 0 && !switched {
            switched = true;
            desktop.select_library(root.clone(), false, false).unwrap();
        }
    });
    assert!(result.is_err());
    assert!(
        ManagedLibrary::open(std::path::Path::new(&root))
            .unwrap()
            .catalogue()
            .unwrap()
            .songs
            .is_empty()
    );
    assert_eq!(
        fs::read_dir(std::path::Path::new(&root).join("songs"))
            .unwrap()
            .count(),
        0
    );
    desktop.shutdown();
}
#[test]
fn unavailable_root_is_reported_and_locate_requires_original_library_uuid() {
    let temp = Temp::new();
    let desktop = Desktop::start(temp.0.join("data")).unwrap();
    desktop
        .select_library(temp.0.to_string_lossy().into_owned(), true, false)
        .unwrap();
    let root = temp.0.join("OhMyLoop Library");
    let moved = temp.0.join("moved");
    fs::rename(&root, &moved).unwrap();
    assert!(!desktop.managed_status().unwrap().available);
    assert!(desktop.retry_library().is_err());
    let another = temp.0.join("another-parent");
    fs::create_dir(&another).unwrap();
    let different = ManagedLibrary::create(&another).unwrap();
    assert!(
        desktop
            .select_library(different.root().to_string_lossy().into_owned(), false, true)
            .is_err()
    );
    assert!(
        desktop
            .select_library(moved.to_string_lossy().into_owned(), false, true)
            .unwrap()
            .library_status
            .available
    );
    desktop.shutdown();
}

#[test]
fn failed_pending_save_blocks_switch_and_import_without_unloading() {
    let temp = Temp::new();
    let path = make_source(&temp, "source.wav", 220.0);
    let desktop = Desktop::start(temp.0.join("data")).unwrap();
    desktop
        .select_library(temp.0.to_string_lossy().into_owned(), true, false)
        .unwrap();
    let first = desktop.import_managed(path.clone(), |_, _| {}).unwrap();
    let song = first.song.unwrap();
    let root = desktop.managed_status().unwrap().selected.unwrap().root;
    let record_path = std::path::Path::new(&root).join(format!("songs/{}/song.json", song.key));
    let bytes = fs::read(&record_path).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["version"] = 99.into();
    fs::write(&record_path, serde_json::to_vec(&value).unwrap()).unwrap();
    let other_parent = temp.0.join("other");
    fs::create_dir(&other_parent).unwrap();
    let other = ManagedLibrary::create(&other_parent).unwrap();
    assert!(
        desktop
            .select_library(other.root().to_string_lossy().into_owned(), false, false)
            .is_err()
    );
    assert_eq!(
        desktop.status().unwrap().song_key.as_deref(),
        Some(song.key.as_str())
    );
    assert_eq!(
        desktop.managed_status().unwrap().selected.unwrap().root,
        root
    );
    assert!(desktop.import_managed(path, |_, _| {}).is_err());
    assert_eq!(
        desktop.status().unwrap().song_key.as_deref(),
        Some(song.key.as_str())
    );
    fs::write(record_path, bytes).unwrap();
    desktop.shutdown();
}

#[test]
fn committed_delete_reports_pointer_warning_without_phantom_active_song() {
    let temp = Temp::new();
    let path = make_source(&temp, "source.wav", 220.0);
    let data = temp.0.join("data");
    let desktop = Desktop::start(data.clone()).unwrap();
    desktop
        .select_library(temp.0.to_string_lossy().into_owned(), true, false)
        .unwrap();
    let song = desktop
        .import_managed(path, |_, _| {})
        .unwrap()
        .song
        .unwrap();
    fs::write(data.join("managed-library.json"), br#"{"version":99}"#).unwrap();
    let deleted = desktop.delete_library_song(song.key).unwrap();
    assert!(deleted.song.is_none());
    assert!(deleted.transport.song_key.is_none());
    assert!(deleted.library_error.unwrap().contains("last-song pointer"));
    assert!(desktop.library().unwrap().songs.is_empty());
    desktop.shutdown();
}
