use ohmyloop::{
    audio::{Audio, write_wav},
    desktop::Desktop,
    library::{Metadata, Store, read_tags},
    project::{Section, song_key},
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "ohmyloop-library-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn metadata(title: &str) -> Metadata {
    Metadata {
        title: title.into(),
        artist: "Player".into(),
        album: "Practice takes".into(),
    }
}
fn tagged_wave(path: &std::path::Path) {
    write_wav(path, 48000, &Audio::tone(48000, 0.5, 220.0).frames).unwrap();
    let bytes = fs::read(path).unwrap();
    let mut info = b"INFO".to_vec();
    for (key, text) in [
        (b"IART", "Guitarist"),
        (b"INAM", "A tagged take"),
        (b"IPRD", "Workshop"),
    ] {
        let mut value = text.as_bytes().to_vec();
        value.push(0);
        info.extend(key);
        info.extend((value.len() as u32).to_le_bytes());
        info.extend(&value);
        if value.len() % 2 != 0 {
            info.push(0);
        }
    }
    let mut tagged = bytes[..12].to_vec();
    tagged.extend(b"LIST");
    tagged.extend((info.len() as u32).to_le_bytes());
    tagged.extend(info);
    tagged.extend(&bytes[12..]);
    let size = (tagged.len() - 8) as u32;
    tagged[4..8].copy_from_slice(&size.to_le_bytes());
    fs::write(path, tagged).unwrap();
}
#[test]
fn catalogue_restores_edits_updates_location_and_retains_added_date() {
    let temp = Temp::new();
    let store = Store::new(&temp.0);
    let key = "a".repeat(64);
    assert!(store.read().unwrap().songs.is_empty());
    store
        .opened(
            key.clone(),
            "/first/song.wav".into(),
            metadata("Original"),
            100,
        )
        .unwrap();
    let mut edited = metadata("  My title  ");
    edited.artist = "  My artist  ".into();
    store.update(&key, edited).unwrap();
    store
        .opened(
            key.clone(),
            "/moved/song.wav".into(),
            metadata("File tags"),
            300,
        )
        .unwrap();
    let restored = Store::new(&temp.0).read().unwrap();
    assert_eq!(restored.songs.len(), 1);
    let entry = restored.entry(&key).unwrap();
    assert_eq!(entry.metadata.title, "My title");
    assert_eq!(entry.metadata.artist, "My artist");
    assert_eq!(entry.date_added, 100);
    assert_eq!(entry.last_opened, 300);
    assert_eq!(entry.file_location, "/moved/song.wav");
    store
        .opened(
            key.clone(),
            "/moved/song.wav".into(),
            metadata("Ignored"),
            200,
        )
        .unwrap();
    assert_eq!(store.read().unwrap().entry(&key).unwrap().last_opened, 300);
}
#[test]
fn invalid_metadata_and_catalogues_leave_original_bytes_intact() {
    let temp = Temp::new();
    let store = Store::new(&temp.0);
    let key = "a".repeat(64);
    store
        .opened(key.clone(), "/song.wav".into(), metadata("Title"), 100)
        .unwrap();
    let path = temp.0.join("library.json");
    let valid = fs::read(&path).unwrap();
    for title in [" ".to_owned(), "x".repeat(1025), "bad\0title".to_owned()] {
        assert!(store.update(&key, metadata(&title)).is_err());
        assert_eq!(fs::read(&path).unwrap(), valid);
    }
    assert!(store.update("missing", metadata("Title")).is_err());
    let catalogue: serde_json::Value = serde_json::from_slice(&valid).unwrap();
    let mut variants = vec![];
    let mut future = catalogue.clone();
    future["version"] = serde_json::json!(99);
    variants.push(future);
    let mut duplicate = catalogue.clone();
    duplicate["songs"]
        .as_array_mut()
        .unwrap()
        .push(catalogue["songs"][0].clone());
    variants.push(duplicate);
    let mut bad_date = catalogue.clone();
    bad_date["songs"][0]["last_opened"] = serde_json::json!(0);
    variants.push(bad_date);
    for value in variants {
        let bytes = serde_json::to_vec(&value).unwrap();
        fs::write(&path, &bytes).unwrap();
        assert!(store.read().is_err());
        assert!(
            store
                .opened(key.clone(), "/song.wav".into(), metadata("Title"), 300)
                .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
    fs::write(&path, b"broken JSON").unwrap();
    assert!(store.read().is_err());
    assert_eq!(fs::read(&path).unwrap(), b"broken JSON");
}
#[test]
fn reads_embedded_tags_and_falls_back_to_filename_without_modifying_audio() {
    let temp = Temp::new();
    let path = temp.0.join("filename.wav");
    tagged_wave(&path);
    let before = fs::read(&path).unwrap();
    let tags = read_tags(&path).unwrap();
    assert_eq!(
        tags,
        Metadata {
            artist: "Guitarist".into(),
            title: "A tagged take".into(),
            album: "Workshop".into()
        }
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    let plain = temp.0.join("Untagged take.wav");
    write_wav(&plain, 48000, &Audio::tone(48000, 0.1, 440.0).frames).unwrap();
    assert_eq!(read_tags(&plain).unwrap().title, "Untagged take");
}
#[test]
fn desktop_reopens_library_song_with_practice_data_and_rejects_changed_files() {
    let temp = Temp::new();
    let path = temp.0.join("song.wav");
    tagged_wave(&path);
    let key = song_key(&path).unwrap();
    let desktop = Desktop::start(temp.0.join("practice")).unwrap();
    let first = desktop
        .load(Some(path.to_string_lossy().into_owned()))
        .unwrap();
    assert_eq!(first.song.as_ref().unwrap().title, "A tagged take");
    assert!(first.library_error.is_none());
    let mut document = first.document.unwrap();
    document.sections.push(Section {
        id: "phrase".into(),
        name: "Phrase".into(),
        start: 0.01,
        end: 0.25,
    });
    desktop.update(document.clone()).unwrap();
    desktop
        .update_library(key.clone(), metadata("My take"))
        .unwrap();
    let reopened = desktop.open_library(key.clone()).unwrap();
    assert_eq!(reopened.song.unwrap().title, "My take");
    assert_eq!(reopened.document.unwrap().sections, document.sections);
    desktop.shutdown();
    let moved = temp.0.join("moved.wav");
    fs::rename(&path, &moved).unwrap();
    let restored = Desktop::start(temp.0.join("practice")).unwrap();
    assert!(restored.open_library(key.clone()).is_err());
    restored
        .load(Some(
            moved.canonicalize().unwrap().to_string_lossy().into_owned(),
        ))
        .unwrap();
    assert_eq!(restored.library().unwrap().songs.len(), 1);
    assert_eq!(
        restored
            .library()
            .unwrap()
            .entry(&key)
            .unwrap()
            .file_location,
        moved.canonicalize().unwrap().to_string_lossy()
    );
    let before = restored.library().unwrap();
    write_wav(&moved, 48000, &Audio::tone(48000, 0.1, 880.0).frames).unwrap();
    assert!(restored.open_library(key.clone()).is_err());
    assert_eq!(restored.library().unwrap(), before);
    assert_eq!(restored.status().unwrap().song_key, Some(key));
    restored.shutdown();
}
#[test]
fn corrupt_library_reports_error_but_keeps_existing_practice_workflow() {
    let temp = Temp::new();
    let practice = temp.0.join("practice");
    fs::create_dir_all(&practice).unwrap();
    let library = practice.join("library.json");
    fs::write(&library, b"broken JSON").unwrap();
    let path = temp.0.join("song.wav");
    tagged_wave(&path);
    let desktop = Desktop::start(practice).unwrap();
    let opened = desktop
        .load(Some(path.to_string_lossy().into_owned()))
        .unwrap();
    assert!(opened.song.is_some());
    assert!(opened.library_error.unwrap().contains("left intact"));
    assert!(desktop.library().is_err());
    assert_eq!(fs::read(library).unwrap(), b"broken JSON");
    desktop.shutdown();
}
