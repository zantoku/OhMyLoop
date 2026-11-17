//! One pre-release reset of recognized content-keyed metadata, never audio.
use crate::{dsp::Controls, engine::LoopRegion, storage::atomic_write, timing::MeasureMap};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::Path};

const MARKER: &str = "uuid-schema.json";
const LEGACY_MODEL: &str = "beat-this-final0-onnx-5f810deb-v1";
const LEGACY_PREPROCESSING: &str = "mono-average-rubato016-phase-align-v1";
const METADATA_LIMIT: u64 = 16 * 1024 * 1024;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SchemaMarker {
    version: u32,
    layout: String,
}

/// Complete the authorized legacy reset before any UUID records are loaded/saved.
/// `true` means recognized old metadata was removed on this launch. A valid
/// marker makes later launches a no-op; unknown markers are preserved and fail.
pub fn reset_legacy(directory: &Path) -> Result<bool> {
    let metadata = fs::symlink_metadata(directory)?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "Practice data must be a real directory for the legacy reset"
    );
    let marker = directory.join(MARKER);
    match fs::symlink_metadata(&marker) {
        Ok(metadata) => {
            ensure!(
                metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() <= 4096,
                "Unrecognized UUID schema marker; the original was left intact"
            );
            let state: SchemaMarker = serde_json::from_slice(&fs::read(&marker)?)
                .context("Cannot read UUID schema marker; the original was left intact")?;
            ensure!(
                state.version == 1 && state.layout == "uuid-song-records",
                "Unsupported UUID schema marker; the original was left intact"
            );
            return Ok(false);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("Cannot inspect UUID schema marker"),
    }

    let mut removed = false;
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !matches!(name, "library.json" | "last-song.json")
            && !name
                .strip_suffix(".json")
                .is_some_and(|key| hash_key(key) || key == "builtin-demo")
        {
            continue;
        }
        let Some(bytes) = regular_bytes(&path, METADATA_LIMIT)? else {
            continue;
        };
        let recognized = match name {
            "library.json" => legacy_catalogue(&bytes),
            "last-song.json" => serde_json::from_slice::<String>(&bytes)
                .is_ok_and(|source| !source.is_empty() && !source.contains('\0')),
            name => name.strip_suffix(".json").is_some_and(|key| {
                (hash_key(key) || key == "builtin-demo") && legacy_practice(&bytes, key)
            }),
        };
        if recognized {
            fs::remove_file(&path).context("Legacy metadata cleanup failed; retry on launch")?;
            removed = true;
        }
    }

    // The previous desktop kept completed timing caches beside `practice`.
    // Recognize exact cache filenames/content; never remove directories or jobs.
    if let Some(parent) = directory.parent() {
        let cache = parent.join("analysis");
        match fs::symlink_metadata(&cache) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                for entry in fs::read_dir(cache)? {
                    let path = entry?.path();
                    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                        continue;
                    };
                    let suffix = format!("-{LEGACY_MODEL}-{LEGACY_PREPROCESSING}.json");
                    if !name.strip_suffix(&suffix).is_some_and(hash_key) {
                        continue;
                    }
                    let Some(bytes) = regular_bytes(&path, 32 * 1024 * 1024)? else {
                        continue;
                    };
                    if legacy_analysis(&path, &bytes) {
                        fs::remove_file(&path)
                            .context("Legacy analysis cleanup failed; retry on launch")?;
                        removed = true;
                    }
                }
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("Cannot inspect legacy analysis cache"),
        }
    }
    if let Ok(metadata) = fs::symlink_metadata(marker.with_extension("json.tmp")) {
        ensure!(
            !metadata.file_type().is_symlink(),
            "UUID schema staging file is a symlink; the original was left intact"
        );
    }
    atomic_write(
        &marker,
        &serde_json::to_vec(&SchemaMarker {
            version: 1,
            layout: "uuid-song-records".into(),
        })?,
    )
    .context("Cannot complete legacy reset; retry on launch")?;
    Ok(removed)
}

fn regular_bytes(path: &Path, limit: u64) -> Result<Option<Vec<u8>>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > limit {
        return Ok(None);
    }
    Ok(Some(fs::read(path)?))
}

fn hash_key(key: &str) -> bool {
    key.len() == 64 && key.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyPractice {
    version: u32,
    song_key: String,
    controls: Controls,
    sections: Vec<super::Section>,
    selected_section: Option<String>,
    loop_selection: Option<LoopRegion>,
    loop_enabled: bool,
    last_position: f64,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    measure_map: Option<MeasureMap>,
}

fn legacy_practice(bytes: &[u8], key: &str) -> bool {
    let Ok(value) = serde_json::from_slice::<LegacyPractice>(bytes) else {
        return false;
    };
    let mut ids = HashSet::new();
    matches!(value.version, 2..=4)
        && value.song_key == key
        && value.controls.validate().is_ok()
        && value.notes.len() <= 1_048_576
        && value.sections.len() <= 1000
        && value.sections.iter().all(|section| {
            !section.id.is_empty()
                && section.id.len() <= 128
                && ids.insert(&section.id)
                && !section.name.trim().is_empty()
                && section.name.len() <= 128
                && LoopRegion {
                    start: section.start,
                    end: section.end,
                    fade_ms: 0.0,
                }
                .validate(f64::MAX)
                .is_ok()
        })
        && value
            .selected_section
            .as_ref()
            .is_none_or(|id| ids.contains(id))
        && value
            .loop_selection
            .is_none_or(|region| region.validate(f64::MAX).is_ok())
        && (!value.loop_enabled || value.loop_selection.is_some())
        && value.last_position.is_finite()
        && value.last_position >= 0.0
        && (value.version != 2 || value.measure_map.is_none())
        && value.measure_map.is_none_or(|map| map.validate().is_ok())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyCatalogue {
    version: u32,
    songs: Vec<LegacyEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyEntry {
    song_key: String,
    metadata: crate::library::Metadata,
    file_location: String,
    date_added: u64,
    last_opened: u64,
}

fn legacy_catalogue(bytes: &[u8]) -> bool {
    let Ok(value) = serde_json::from_slice::<LegacyCatalogue>(bytes) else {
        return false;
    };
    let mut ids = HashSet::new();
    value.version == 1
        && value.songs.len() <= 10_000
        && value.songs.iter().all(|entry| {
            hash_key(&entry.song_key)
                && ids.insert(&entry.song_key)
                && !entry.file_location.is_empty()
                && entry.file_location.len() <= 16_384
                && !entry.file_location.contains('\0')
                && entry.date_added <= entry.last_opened
                && entry.last_opened <= 8_640_000_000_000_000
                && !entry.metadata.title.trim().is_empty()
                && [
                    &entry.metadata.title,
                    &entry.metadata.artist,
                    &entry.metadata.album,
                ]
                .iter()
                .all(|text| text.trim() == **text && text.len() <= 1024 && !text.contains('\0'))
        })
}

fn legacy_analysis(path: &Path, bytes: &[u8]) -> bool {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return false;
    };
    let Some(candidates) = value.get("candidates") else {
        return false;
    };
    let Some(key) = candidates.get("song_key").and_then(|key| key.as_str()) else {
        return false;
    };
    let expected = format!("{key}-{LEGACY_MODEL}-{LEGACY_PREPROCESSING}.json");
    hash_key(key)
        && path.file_name().and_then(|name| name.to_str()) == Some(expected.as_str())
        && candidates["version"].as_u64() == Some(1)
        && candidates["model"].as_str() == Some(LEGACY_MODEL)
        && candidates["preprocessing"].as_str() == Some(LEGACY_PREPROCESSING)
        && candidates["rate"].as_u64().is_some_and(|rate| rate > 0)
        && candidates["frames"]
            .as_u64()
            .is_some_and(|frames| frames > 0)
        && candidates["map_revision"].as_u64().is_some()
        && candidates["elapsed_seconds"]
            .as_f64()
            .is_some_and(|elapsed| elapsed.is_finite() && elapsed >= 0.0)
        && candidates["beats"].as_array().is_some_and(|beats| {
            beats.len() <= 80_000
                && beats.iter().all(|beat| {
                    beat["frame"].as_u64().is_some_and(|frame| {
                        frame < candidates["frames"].as_u64().unwrap_or_default()
                    }) && beat["score"].as_f64().is_some_and(f64::is_finite)
                        && beat["downbeat"].as_bool().is_some()
                })
                && beats
                    .windows(2)
                    .all(|pair| pair[0]["frame"].as_u64() < pair[1]["frame"].as_u64())
        })
        && ["beat_logits", "downbeat_logits"].iter().all(|field| {
            value[field].as_array().is_some_and(|logits| {
                logits
                    .iter()
                    .all(|logit| logit.as_f64().is_some_and(f64::is_finite))
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "ohmyloop-uuid-reset-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn practice(&self) -> PathBuf {
            let path = self.0.join("practice");
            fs::create_dir_all(&path).unwrap();
            path
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn write_json(path: &Path, value: serde_json::Value) {
        fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
    }
    fn practice_value(key: &str) -> serde_json::Value {
        json!({"version":4,"song_key":key,"controls":{"speed":1.0,"semitones":0.0},
            "sections":[],"selected_section":null,"loop_selection":null,
            "loop_enabled":false,"last_position":0.0,"notes":"old notes","measure_map":null})
    }
    fn cache_value(key: &str) -> serde_json::Value {
        json!({"candidates":{"version":1,"song_key":key,"map_revision":0,
            "rate":48000,"frames":48000,"model":LEGACY_MODEL,"preprocessing":LEGACY_PREPROCESSING,
            "elapsed_seconds":1.0,"beats":[]},"beat_logits":[],"downbeat_logits":[]})
    }
    fn cache_name(key: &str) -> String {
        format!("{key}-{LEGACY_MODEL}-{LEGACY_PREPROCESSING}.json")
    }

    #[test]
    fn removes_only_recognized_metadata_and_preserves_audio_preferences_and_caches() {
        let temp = Temp::new();
        let dir = temp.practice();
        let key = "a".repeat(64);
        write_json(&dir.join(format!("{key}.json")), practice_value(&key));
        write_json(
            &dir.join("builtin-demo.json"),
            practice_value("builtin-demo"),
        );
        write_json(&dir.join("library.json"), json!({"version":1,"songs":[]}));
        write_json(&dir.join("last-song.json"), json!("/external/song.wav"));
        let analysis = temp.0.join("analysis");
        fs::create_dir(&analysis).unwrap();
        write_json(&analysis.join(cache_name(&key)), cache_value(&key));
        let preserved = [
            dir.join("song.wav"),
            dir.join("keymap.json"),
            dir.join("manifest.json"),
            dir.join("bass.wav"),
            analysis.join("unknown.json"),
            analysis.join("model.onnx"),
        ];
        for path in &preserved {
            fs::write(path, b"keep these bytes").unwrap();
        }
        assert!(reset_legacy(&dir).unwrap());
        assert!(!dir.join(format!("{key}.json")).exists());
        assert!(!dir.join("builtin-demo.json").exists());
        assert!(!dir.join("library.json").exists());
        assert!(!dir.join("last-song.json").exists());
        assert!(!analysis.join(cache_name(&key)).exists());
        for path in &preserved {
            assert_eq!(fs::read(path).unwrap(), b"keep these bytes");
        }
        write_json(&dir.join("library.json"), json!({"version":2,"songs":[]}));
        write_json(
            &dir.join("last-song.json"),
            json!({"version":2,"song_id":"new"}),
        );
        let uuid_data = dir.join("new-uuid-record.json");
        fs::write(&uuid_data, b"new song notes").unwrap();
        assert!(!reset_legacy(&dir).unwrap());
        assert_eq!(fs::read(uuid_data).unwrap(), b"new song notes");
        assert!(dir.join("library.json").exists());
        assert!(dir.join("last-song.json").exists());
    }

    #[test]
    fn preserves_malformed_future_and_wrong_identity_records_without_marker() {
        let temp = Temp::new();
        let dir = temp.practice();
        let key = "b".repeat(64);
        let doc = dir.join(format!("{key}.json"));
        let mut future = practice_value(&key);
        future["version"] = json!(99);
        write_json(&doc, future);
        write_json(&dir.join("library.json"), json!({"version":99,"songs":[]}));
        fs::write(dir.join("last-song.json"), b"broken JSON").unwrap();
        let wrong = dir.join(format!("{}.json", "c".repeat(64)));
        write_json(&wrong, practice_value(&key));
        let malformed = dir.join(format!("{}.json", "d".repeat(64)));
        fs::write(&malformed, b"broken JSON").unwrap();
        let before: Vec<_> = [
            &doc,
            &wrong,
            &malformed,
            &dir.join("library.json"),
            &dir.join("last-song.json"),
        ]
        .iter()
        .map(|path| ((*path).clone(), fs::read(path).unwrap()))
        .collect();
        assert!(!reset_legacy(&dir).unwrap());
        for (path, bytes) in before {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
    }

    #[test]
    fn interrupted_cleanup_retries_and_marker_failure_does_not_mark_success() {
        let temp = Temp::new();
        let dir = temp.practice();
        let key = "e".repeat(64);
        let doc = dir.join(format!("{key}.json"));
        write_json(&doc, practice_value(&key));
        // Deterministic publication failure, even when tests run as administrator.
        fs::create_dir(dir.join("uuid-schema.json.tmp")).unwrap();
        assert!(reset_legacy(&dir).is_err());
        assert!(!doc.exists());
        assert!(!dir.join(MARKER).exists());
        fs::remove_dir(dir.join("uuid-schema.json.tmp")).unwrap();
        write_json(
            &dir.join("builtin-demo.json"),
            practice_value("builtin-demo"),
        );
        assert!(reset_legacy(&dir).unwrap());
        assert!(!reset_legacy(&dir).unwrap());
    }

    #[test]
    fn unsupported_or_malformed_marker_stops_before_deletion() {
        let temp = Temp::new();
        let dir = temp.practice();
        let key = "f".repeat(64);
        let doc = dir.join(format!("{key}.json"));
        write_json(&doc, practice_value(&key));
        for bytes in [
            b"broken JSON".as_slice(),
            br#"{"version":99,"layout":"uuid-song-records"}"#,
        ] {
            fs::write(dir.join(MARKER), bytes).unwrap();
            assert!(reset_legacy(&dir).is_err());
            assert!(doc.exists());
            assert_eq!(fs::read(dir.join(MARKER)).unwrap(), bytes);
        }
    }

    #[cfg(unix)]
    #[test]
    fn does_not_follow_metadata_or_analysis_directory_symlinks() {
        use std::os::unix::fs::symlink;
        let temp = Temp::new();
        let dir = temp.practice();
        let key = "a".repeat(64);
        let outside = temp.0.join("outside");
        fs::create_dir(&outside).unwrap();
        let doc = outside.join("document.json");
        write_json(&doc, practice_value(&key));
        let cached = outside.join(cache_name(&key));
        write_json(&cached, cache_value(&key));
        symlink(&doc, dir.join(format!("{key}.json"))).unwrap();
        symlink(&outside, temp.0.join("analysis")).unwrap();
        assert!(!reset_legacy(&dir).unwrap());
        assert!(doc.exists());
        assert!(cached.exists());
        assert!(
            fs::symlink_metadata(dir.join(format!("{key}.json")))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn matching_name_alone_cannot_delete_invalid_or_future_analysis() {
        let temp = Temp::new();
        let dir = temp.practice();
        let cache = temp.0.join("analysis");
        fs::create_dir(&cache).unwrap();
        for (i, field, value) in [
            (0, "version", json!(99)),
            (
                1,
                "beats",
                json!([{"frame":999999,"score":1.0,"downbeat":true}]),
            ),
        ] {
            let key = format!("{i:x}").repeat(64);
            let mut data = cache_value(&key);
            data["candidates"][field] = value;
            write_json(&cache.join(cache_name(&key)), data);
        }
        assert!(!reset_legacy(&dir).unwrap());
        assert_eq!(fs::read_dir(&cache).unwrap().count(), 2);
    }
}
