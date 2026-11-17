use ohmyloop::{
    audio::write_wav,
    stems::{BlockSource, MixState, PreparedComparison, Role, SourceFactory},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

struct Fixture {
    root: PathBuf,
    reference: PathBuf,
    sets: Vec<PathBuf>,
    frames: u64,
    rate: u32,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn sha(path: &Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}
fn write_manifest(set: &Path, mut value: Value) {
    value["job_key"] = json!(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&value["identity"]).unwrap())
    ));
    fs::write(
        set.join("manifest.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
}
fn fixture(roles: &[Role], second: bool) -> Fixture {
    let root = std::env::temp_dir().join(format!("ohmyloop-stem-test-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&root).unwrap();
    let root = root.canonicalize().unwrap();
    let rate = 44_100;
    let frames = 4097;
    let reference = root.join("reference.wav");
    write_wav(&reference, rate, &vec![[0.1, -0.1]; frames as usize]).unwrap();
    let mut sets = vec![];
    for index in 0..if second { 2 } else { 1 } {
        let set = root.join(format!("set{index}"));
        fs::create_dir(&set).unwrap();
        write_wav(
            &set.join("prepared.wav"),
            rate,
            &vec![[0.1, -0.1]; frames as usize],
        )
        .unwrap();
        let mut stems = serde_json::Map::new();
        for role in roles {
            let file = format!("{}.wav", role.name());
            let amplitude = (role.index() + 1) as f32 * 0.1;
            if index == 0 {
                write_wav(
                    &set.join(&file),
                    rate,
                    &vec![[amplitude, -amplitude]; frames as usize],
                )
                .unwrap()
            } else {
                write_wav(
                    &set.join(&file),
                    rate,
                    &vec![[-amplitude, amplitude]; frames as usize],
                )
                .unwrap()
            };
            stems.insert(role.name().into(),json!({"file":file,"sha256":sha(&set.join(format!("{}.wav",role.name()))),"rate":rate,"frames":frames,"channels":2}));
        }
        let mut value = json!({"version":1,"status":"complete","identity":{"preparation":{"version":1,"source_sha256":sha(&reference),"source_rate":rate,"source_frames":frames,"source_span":{"start":0,"end_exclusive":frames},"prepared_rate":rate,"prepared_frames":frames,"channels":2,"prepared_sha256":sha(&set.join("prepared.wav"))}},"stems":stems});
        value["identity"]["preparation"]["preprocessing"] =
            json!("symphonia-stereo-rubato-delay-compensated-v1");
        value["identity"]["preparation"]["geometry_policy"] = json!(
            "floor(seconds * source_rate); floor(span_frames * 44100 / source_rate); no context"
        );
        value["corrections"] = json!([]);
        value["identity"]["settings"] = json!({"normalization":"none per stem"});
        value["identity"]["model_lock"] = json!({"version":1,"model":"demucs-6s","files":{
            "5c90dfd2-34c22ccb.th":{"sha256":"34c22ccb381c6f9fdbf324f04e1e2fe21aaaf293f5ded163a162697ff9a02ddd","bytes":54996327,"url":"https://example.test/checkpoint"},
            "htdemucs_6s.yaml":{"sha256":"207405151270af8fd81c2373c25d27950916682ac91dca7884a11ce13dad6f58","bytes":21,"url":"https://example.test/config"}
        }});
        write_manifest(&set, value);
        sets.push(set);
    }
    Fixture {
        root,
        reference,
        sets,
        frames,
        rate,
    }
}

#[test]
fn all_mute_masks_gain_solo_and_reference_bypass() {
    let f = fixture(&Role::ALL, true);
    let prepared = PreparedComparison::prepare(&f.reference, &f.sets, f.rate, |_| {}).unwrap();
    let mut source = prepared.reader().unwrap();
    source.select_source(1).unwrap();
    let a = prepared.shared_attenuation;
    assert!((a - 0.7 / 2.1).abs() < 1e-6);
    let mut output = [[0.; 2]; 13];
    for mask in 0..64 {
        let mut mix = MixState::default();
        for role in Role::ALL {
            mix.tracks[role.index()].mute = mask & (1 << role.index()) != 0;
        }
        source.set_mix(mix).unwrap();
        assert_eq!(source.read_frames(128, &mut output).unwrap(), 13);
        let expected = Role::ALL
            .iter()
            .filter(|r| mask & (1 << r.index()) == 0)
            .map(|r| (r.index() + 1) as f32 * 0.1)
            .sum::<f32>()
            * a;
        assert!((output[0][0] - expected).abs() < 1e-6, "mask {mask}");
        assert!(output.iter().flatten().all(|v| v.abs() <= 0.700001));
    }
    let mut mix = MixState::default();
    mix.tracks[Role::Bass.index()].solo = true;
    mix.tracks[Role::Drums.index()].solo = true;
    mix.tracks[Role::Drums.index()].mute = true;
    mix.tracks[Role::Bass.index()].gain = 0.4;
    source.set_mix(mix).unwrap();
    source.read_frames(0, &mut output).unwrap();
    assert!((output[0][0] - 0.04 * a).abs() < 1e-6);
    source.select_source(2).unwrap();
    source.read_frames(0, &mut output).unwrap();
    assert!((output[0][0] + 0.04 * a).abs() < 1e-6);
    source.select_source(0).unwrap();
    source.read_frames(0, &mut output).unwrap();
    assert!((output[0][0] - 0.1 * a).abs() < 1e-6);
    mix.tracks[0].gain = f32::NAN;
    assert!(source.set_mix(mix).is_err());
    mix.tracks[0].gain = 1.01;
    assert!(source.set_mix(mix).is_err());
    mix.tracks[0].gain = -0.1;
    assert!(source.set_mix(mix).is_err());
}

#[test]
fn missing_solo_role_stays_silent_and_seek_eof_is_file_backed() {
    let f = fixture(&[Role::Bass], false);
    let prepared = PreparedComparison::prepare(&f.reference, &f.sets, f.rate, |_| {}).unwrap();
    let mut mix = MixState::default();
    mix.tracks[Role::Guitar.index()].solo = true;
    let mut source = prepared.open(1, mix).unwrap();
    let mut out = [[9.; 2]; 32];
    assert_eq!(source.read_frames(f.frames - 7, &mut out).unwrap(), 7);
    assert_eq!(&out[..7], &[[0.; 2]; 7]);
    assert_eq!(source.read_frames(f.frames, &mut out).unwrap(), 0);
    assert!(prepared.open(3, mix).is_err());
    assert_eq!(prepared.source_roles(1).unwrap(), vec![Role::Bass]);
}

#[test]
fn absolute_sum_headroom_handles_cancellation_and_gain_endpoints() {
    let f = fixture(&[Role::Bass, Role::Drums], false);
    let set = &f.sets[0];
    write_wav(
        &set.join("bass.wav"),
        f.rate,
        &vec![[1.0; 2]; f.frames as usize],
    )
    .unwrap();
    write_wav(
        &set.join("drums.wav"),
        f.rate,
        &vec![[-1.0; 2]; f.frames as usize],
    )
    .unwrap();
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(set.join("manifest.json")).unwrap()).unwrap();
    for role in ["bass", "drums"] {
        manifest["stems"][role]["sha256"] = json!(sha(&set.join(format!("{role}.wav"))));
    }
    write_manifest(set, manifest);
    let prepared = PreparedComparison::prepare(&f.reference, &f.sets, f.rate, |_| {}).unwrap();
    assert!((prepared.shared_attenuation - 0.35).abs() < 1e-6);
    let mut source = prepared.open(1, MixState::default()).unwrap();
    let mut out = [[0.; 2]; 1];
    source.read_frames(0, &mut out).unwrap();
    assert_eq!(out, [[0.; 2]]);
    let mut mix = MixState::default();
    mix.tracks[Role::Drums.index()].gain = 0.;
    source = prepared.open(1, mix).unwrap();
    source.read_frames(0, &mut out).unwrap();
    assert!((out[0][0] - 0.35).abs() < 1e-6);
}

#[test]
fn reference_mono_and_cross_rate_impulses_compensate_resampler_delay() {
    for (rate, to) in [(44_100, 48_000), (48_000, 44_100)] {
        let f = fixture(&[Role::Bass], false);
        let mut writer = hound::WavWriter::create(
            &f.reference,
            hound::WavSpec {
                channels: 1,
                sample_rate: rate,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .unwrap();
        let input_len = 4097u64;
        let impulse = 1000u64;
        for n in 0..input_len {
            writer
                .write_sample(if n == impulse { 0.1f32 } else { 0.0 })
                .unwrap();
        }
        writer.finalize().unwrap();
        let prepared = PreparedComparison::prepare(&f.reference, &[], to, |_| {}).unwrap();
        assert_eq!(prepared.frames, input_len * to as u64 / rate as u64);
        let mut source = prepared.reader().unwrap();
        let mut output = vec![[0.0; 2]; prepared.frames as usize];
        source.read_frames(0, &mut output).unwrap();
        let peak = output
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a[0].abs().total_cmp(&b[0].abs()))
            .unwrap()
            .0 as i64;
        let expected = (impulse as f64 * to as f64 / rate as f64).round() as i64;
        assert!(
            (peak - expected).abs() <= 1,
            "{rate}->{to}: peak={peak} expected={expected}"
        );
        assert!(output.iter().all(|x| x[0] == x[1]));
    }
}

#[test]
fn manifest_geometry_provenance_future_paths_and_hashes_fail_without_mutation() {
    let f = fixture(&[Role::Bass], false);
    let path = f.sets[0].join("manifest.json");
    let original = fs::read(&path).unwrap();
    let value: Value = serde_json::from_slice(&original).unwrap();
    let source_hash = sha(&f.reference);
    let stem_hash = sha(&f.sets[0].join("bass.wav"));
    for case in 0..9 {
        let mut v = value.clone();
        match case {
            0 => v["version"] = json!(2),
            1 => v["status"] = json!("running"),
            2 => v["identity"]["preparation"]["source_sha256"] = json!("wrong"),
            3 => v["identity"]["preparation"]["source_span"]["start"] = json!(1),
            4 => v["stems"]["bass"]["file"] = json!("../reference.wav"),
            5 => v["stems"]["bass"]["sha256"] = json!("wrong"),
            6 => v["stems"]["bass"]["frames"] = json!(11),
            7 => {
                let track = v["stems"]["bass"].take();
                v["stems"] = json!({"banjo":track})
            }
            _ => v["identity"]["preparation"]["source_frames"] = json!(10),
        }
        write_manifest(&f.sets[0], v);
        let before = fs::read(&path).unwrap();
        assert!(
            PreparedComparison::prepare(&f.reference, &f.sets, f.rate, |_| {}).is_err(),
            "case {case}"
        );
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(sha(&f.reference), source_hash);
        assert_eq!(sha(&f.sets[0].join("bass.wav")), stem_hash);
    }
}

#[test]
fn duplicate_roles_are_rejected_before_json_identity_canonicalization() {
    let f = fixture(&[Role::Bass], false);
    let path = f.sets[0].join("manifest.json");
    let value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let track = serde_json::to_string(&value["stems"]["bass"]).unwrap();
    let original = serde_json::to_string(&value).unwrap();
    let replacement = format!("\"stems\":{{\"bass\":{track},\"bass\":{track}}}");
    let serialized = original.replace(&format!("\"stems\":{{\"bass\":{track}}}"), &replacement);
    assert_ne!(serialized, original);
    fs::write(path, serialized).unwrap();
    let error = PreparedComparison::prepare(&f.reference, &f.sets, f.rate, |_| {})
        .err()
        .unwrap();
    assert!(format!("{error:#}").contains("Duplicate stem role"));
}

#[test]
fn truncated_reference_wav_is_a_preparation_error() {
    let f = fixture(&[Role::Bass], false);
    let file = fs::OpenOptions::new()
        .write(true)
        .open(&f.reference)
        .unwrap();
    file.set_len(100).unwrap();
    assert!(PreparedComparison::prepare(&f.reference, &[], f.rate, |_| {}).is_err());
}

#[test]
fn stem_rate_conversion_uses_direct_reference_floor_mapping() {
    let f = fixture(&[Role::Bass], false);
    let stem_rate = 48_000u32;
    let stem_frames = f.frames * stem_rate as u64 / f.rate as u64;
    let mut reference = vec![[0.0; 2]; f.frames as usize];
    reference[1000] = [0.1; 2];
    write_wav(&f.reference, f.rate, &reference).unwrap();
    let mut bank = vec![[0.0; 2]; stem_frames as usize];
    bank[(1000u64 * stem_rate as u64 / f.rate as u64) as usize] = [0.1; 2];
    let set = &f.sets[0];
    write_wav(&set.join("prepared.wav"), stem_rate, &bank).unwrap();
    write_wav(&set.join("bass.wav"), stem_rate, &bank).unwrap();
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(set.join("manifest.json")).unwrap()).unwrap();
    let prep = &mut manifest["identity"]["preparation"];
    prep["source_sha256"] = json!(sha(&f.reference));
    prep["prepared_rate"] = json!(stem_rate);
    prep["prepared_frames"] = json!(stem_frames);
    prep["prepared_sha256"] = json!(sha(&set.join("prepared.wav")));
    manifest["stems"]["bass"]["rate"] = json!(stem_rate);
    manifest["stems"]["bass"]["frames"] = json!(stem_frames);
    manifest["stems"]["bass"]["sha256"] = json!(sha(&set.join("bass.wav")));
    write_manifest(set, manifest);
    for device in [44_100, 48_000] {
        let prepared = PreparedComparison::prepare(&f.reference, &f.sets, device, |_| {}).unwrap();
        assert_eq!(prepared.frames, f.frames * device as u64 / f.rate as u64);
        let mut source = prepared.open(1, MixState::default()).unwrap();
        let mut output = vec![[0.0; 2]; prepared.frames as usize];
        source.read_frames(0, &mut output).unwrap();
        let peak = output
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a[0].abs().total_cmp(&b[0].abs()))
            .unwrap()
            .0 as i64;
        let expected = (1000.0 * device as f64 / f.rate as f64).round() as i64;
        assert!((peak - expected).abs() <= 2);
    }
}

#[test]
fn owned_attachment_removes_prototype_dependency_and_cache_reuses_without_hashing() {
    use ohmyloop::stems::library::{SavedMix, attach, prepare};
    let f = fixture(&[Role::Bass, Role::Drums], false);
    let owned = f.root.join(uuid::Uuid::new_v4().to_string());
    let cache = f.root.join("cache");
    let mut record = attach(&f.reference, &f.sets[0], &owned, f.rate, |_| {}).unwrap();
    assert_eq!(owned.file_name().unwrap().to_str().unwrap(), record.id);
    assert!(!owned.join("prepared.wav").exists());
    assert_eq!(fs::read_dir(&owned).unwrap().count(), 3);
    fs::remove_dir_all(&f.sets[0]).unwrap();
    let initial = prepare(&record, &f.reference, &owned, &cache, f.rate, |_| {}).unwrap();
    let attenuation = initial.shared_attenuation;
    let cache_before = fs::read(cache.join("cache.json")).unwrap();
    drop(initial);
    assert!(cache.exists());
    let mut mix = MixState::default();
    mix.tracks[Role::Bass.index()].gain = 0.5;
    mix.tracks[Role::Bass.index()].solo = true;
    record.mix = SavedMix::from_mix(mix);
    assert!(!record.mix.to_mix().tracks[0].solo);
    let mut phases = vec![];
    let reused = prepare(&record, &f.reference, &owned, &cache, f.rate, |p| {
        phases.push(p.phase)
    })
    .unwrap();
    assert_eq!(phases, ["Reuse prepared playback"]);
    assert_eq!(fs::read(cache.join("cache.json")).unwrap(), cache_before);
    assert_eq!(attenuation, reused.shared_attenuation);
    let mut source = reused.open(record.selection(mix), mix).unwrap();
    let mut out = [[0.; 2]; 1];
    source.read_frames(32, &mut out).unwrap();
    assert!((out[0][0] - 0.05 * attenuation).abs() < 1e-6);
    drop(source);
    drop(reused);
    // An ordinary manual edit rebuilds without enforcing the recorded hash.
    write_wav(
        &owned.join("bass.wav"),
        f.rate,
        &vec![[0.6; 2]; f.frames as usize],
    )
    .unwrap();
    let mut phases = vec![];
    let edited = prepare(&record, &f.reference, &owned, &cache, f.rate, |p| {
        phases.push(p.phase)
    })
    .unwrap();
    assert!(!phases.iter().any(|p| p.starts_with("Hash")));
    assert!(phases.iter().any(|p| p.starts_with("Decode")));
    assert!((edited.shared_attenuation - 0.875).abs() < 1e-5);
    drop(edited);
    let resampled = prepare(&record, &f.reference, &owned, &cache, 48_000, |_| {}).unwrap();
    assert_eq!(resampled.frames, f.frames * 48_000 / f.rate as u64);
    assert_eq!(fs::read_dir(&cache).unwrap().count(), 2);
    drop(resampled);
    // Missing/truncated preparation rebuilds; missing authoritative stems fail.
    let pointer: Value =
        serde_json::from_slice(&fs::read(cache.join("cache.json")).unwrap()).unwrap();
    let generation = cache.join(pointer["generation"].as_str().unwrap());
    assert_eq!(fs::read_dir(&generation).unwrap().count(), 3);
    let entry = fs::read_dir(&generation)
        .unwrap()
        .find(|e| {
            e.as_ref()
                .unwrap()
                .path()
                .extension()
                .is_some_and(|x| x == "pcm")
        })
        .unwrap()
        .unwrap();
    fs::write(entry.path(), [0; 8]).unwrap();
    assert!(prepare(&record, &f.reference, &owned, &cache, 48_000, |_| {}).is_ok());
    fs::remove_file(owned.join("bass.wav")).unwrap();
    assert!(prepare(&record, &f.reference, &owned, &cache, 48_000, |_| {}).is_err());
}

#[test]
fn owned_records_validate_versions_provenance_paths_and_present_role_selection() {
    use ohmyloop::stems::library::attach;
    let f = fixture(&[Role::Bass], false);
    let owned = f.root.join(uuid::Uuid::new_v4().to_string());
    let record = attach(&f.reference, &f.sets[0], &owned, f.rate, |_| {}).unwrap();
    let mut mix = MixState::default();
    mix.tracks[Role::Guitar.index()].mute = true;
    assert_eq!(record.selection(mix), 0);
    mix.tracks[Role::Bass.index()].solo = true;
    assert_eq!(record.selection(mix), 1);
    for case in 0..6 {
        let mut invalid = record.clone();
        match case {
            0 => invalid.version = 2,
            1 => invalid.tracks[0].file = "../reference.wav".into(),
            2 => invalid.tracks[0].role = "banjo".into(),
            3 => invalid.tracks.push(invalid.tracks[0].clone()),
            4 => invalid.provenance["preparation"]["source_span"]["start"] = json!(1),
            _ => invalid.mix.tracks[0].gain = 1.1,
        }
        assert!(invalid.validate().is_err(), "case {case}");
    }
}

#[test]
#[cfg(unix)]
fn owned_preparation_rejects_symlink_and_attachment_failure_keeps_inputs() {
    use ohmyloop::stems::library::{attach, prepare};
    let f = fixture(&[Role::Bass], false);
    let owned = f.root.join(uuid::Uuid::new_v4().to_string());
    let record = attach(&f.reference, &f.sets[0], &owned, f.rate, |_| {}).unwrap();
    fs::remove_file(owned.join("bass.wav")).unwrap();
    std::os::unix::fs::symlink(f.sets[0].join("bass.wav"), owned.join("bass.wav")).unwrap();
    assert!(
        prepare(
            &record,
            &f.reference,
            &owned,
            &f.root.join("cache"),
            f.rate,
            |_| {}
        )
        .is_err()
    );
    let invalid = f.root.join("invalid-owned");
    let before = sha(&f.sets[0].join("bass.wav"));
    fs::write(f.sets[0].join("bass.wav"), b"broken").unwrap();
    assert!(attach(&f.reference, &f.sets[0], &invalid, f.rate, |_| {}).is_err());
    assert!(!invalid.exists());
    assert_ne!(sha(&f.sets[0].join("bass.wav")), before);
}

#[test]
fn managed_stem_publication_preserves_edits_rejects_stale_and_cleans_reference_replacement() {
    use ohmyloop::{
        managed::ManagedLibrary,
        project::SessionId,
        stems::library::{attach, prepare},
    };
    let f = fixture(&[Role::Bass], false);
    let library = ManagedLibrary::create(&f.root).unwrap();
    let imported = library
        .commit_import(library.import(&f.reference, |_, _| {}).unwrap())
        .unwrap();
    let key = imported.record.id.to_string();
    let id = uuid::Uuid::new_v4().to_string();
    let owned = library.stem_directory(&key, &id).unwrap();
    let mut attached = attach(&f.reference, &f.sets[0], &owned, f.rate, |_| {}).unwrap();
    attached.mix.tracks[0].gain = 0.5;
    library
        .save_stems(
            &key,
            imported.record.reference_revision,
            Some(attached.clone()),
        )
        .unwrap();
    let mut document = imported.record.document(SessionId::new()).unwrap();
    document.notes = "Authored notes survive attachment".into();
    library.save_document(&document).unwrap();
    assert_eq!(library.record(&key).unwrap().stems, Some(attached.clone()));
    let before = fs::read(library.root().join(format!("songs/{key}/song.json"))).unwrap();
    assert!(
        library
            .save_stems(&key, imported.record.reference_revision + 1, None)
            .is_err()
    );
    assert_eq!(
        fs::read(library.root().join(format!("songs/{key}/song.json"))).unwrap(),
        before
    );
    let cache = library.stem_cache_directory(&key).unwrap();
    let playback = prepare(
        &attached,
        &library.asset_path(&imported.record).unwrap(),
        &owned,
        &cache,
        f.rate,
        |_| {},
    )
    .unwrap();
    drop(playback);
    let replacement = library
        .prepare_replace(&key, &f.reference, true, |_, _| {})
        .unwrap();
    let replaced = library.commit_replace(replacement).unwrap();
    assert!(library.record(&key).unwrap().stems.is_none());
    assert!(!owned.exists());
    assert!(!cache.exists());
    assert_eq!(
        replaced.record.document(SessionId::new()).unwrap().notes,
        document.notes
    );
    // Reattach to the new reference, then preserve a future-version record byte-for-byte.
    let id = uuid::Uuid::new_v4().to_string();
    let owned = library.stem_directory(&key, &id).unwrap();
    let attached = attach(&f.reference, &f.sets[0], &owned, f.rate, |_| {}).unwrap();
    library
        .save_stems(&key, replaced.record.reference_revision, Some(attached))
        .unwrap();
    let song_path = library.root().join(format!("songs/{key}/song.json"));
    let saved = fs::read(&song_path).unwrap();
    let mut future: Value = serde_json::from_slice(&saved).unwrap();
    future["stems"]["version"] = json!(2);
    fs::write(&song_path, serde_json::to_vec(&future).unwrap()).unwrap();
    let future_bytes = fs::read(&song_path).unwrap();
    assert!(
        library
            .save_stems(&key, replaced.record.reference_revision, None)
            .is_err()
    );
    assert!(library.save_document(&document).is_err());
    assert_eq!(fs::read(&song_path).unwrap(), future_bytes);
    fs::write(&song_path, saved).unwrap();
    library.delete(&key).unwrap();
    assert!(!owned.exists());
    assert!(f.reference.exists());
    assert!(f.sets[0].join("bass.wav").exists());
}

#[test]
fn attachment_requires_known_preprocessing_coordinate_policy_and_model_artifacts() {
    use ohmyloop::stems::library::attach;
    let f = fixture(&[Role::Bass], false);
    let path = f.sets[0].join("manifest.json");
    let original: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for case in 0..12 {
        let mut invalid = original.clone();
        match case {
            0 => {
                invalid["identity"]["preparation"]
                    .as_object_mut()
                    .unwrap()
                    .remove("preprocessing");
            }
            1 => {
                invalid["identity"]["preparation"]["preprocessing"] =
                    json!("unknown-shifted-preprocessing")
            }
            2 => {
                invalid["identity"]["preparation"]
                    .as_object_mut()
                    .unwrap()
                    .remove("geometry_policy");
            }
            3 => {
                invalid["identity"]["preparation"]["geometry_policy"] =
                    json!("ceil-with-context-and-delay")
            }
            4 => {
                invalid["identity"]
                    .as_object_mut()
                    .unwrap()
                    .remove("model_lock");
            }
            5 => invalid["identity"]["model_lock"]["model"] = json!("unsupported-model"),
            6 => invalid["identity"]["model_lock"]["files"] = json!({}),
            7 => {
                invalid["identity"]["model_lock"]["files"]["5c90dfd2-34c22ccb.th"]["sha256"] =
                    json!("0".repeat(64))
            }
            8 => invalid["identity"]["model_lock"]["version"] = json!(2),
            9 => invalid["identity"]["settings"]["normalization"] = json!("normalize each stem"),
            10 => invalid["corrections"] = json!([{"offset_frames":100}]),
            _ => {
                invalid.as_object_mut().unwrap().remove("corrections");
            }
        }
        write_manifest(&f.sets[0], invalid);
        let before = fs::read(&path).unwrap();
        let destination = f.root.join(uuid::Uuid::new_v4().to_string());
        assert!(
            attach(&f.reference, &f.sets[0], &destination, f.rate, |_| {}).is_err(),
            "case {case}"
        );
        assert!(!destination.exists());
        assert_eq!(fs::read(&path).unwrap(), before);
    }
    // Both declared prototype backends are accepted; no model files/runtime are required.
    let mut roformer = original;
    roformer["identity"]["model_lock"] = json!({"version":1,"model":"roformer-sw","files":{
        "BS-Roformer-SW.ckpt":{"sha256":"24e7d35ee9c64415673d3fd33e06a67cac2c103c5df6267ba1576459c775916e","bytes":699412152,"url":"https://example.test/checkpoint"},
        "BS-Roformer-SW.yaml":{"sha256":"b558996f1e25eb48798bd6502505a5de94c4f966d6edfb1a0420f06cc40b501a","bytes":4653,"url":"https://example.test/config"}
    }});
    let mut missing_padding = roformer.clone();
    write_manifest(&f.sets[0], missing_padding.clone());
    assert!(
        attach(
            &f.reference,
            &f.sets[0],
            &f.root.join(uuid::Uuid::new_v4().to_string()),
            f.rate,
            |_| {}
        )
        .is_err()
    );
    missing_padding["identity"]["settings"]["roformer_short_input"] =
        json!("uncompensated-padding");
    write_manifest(&f.sets[0], missing_padding);
    assert!(
        attach(
            &f.reference,
            &f.sets[0],
            &f.root.join(uuid::Uuid::new_v4().to_string()),
            f.rate,
            |_| {}
        )
        .is_err()
    );
    roformer["identity"]["settings"]["roformer_short_input"] =
        json!("right-zero-pad-to-config-chunk; crop-declared-padding");
    let n = roformer["identity"]["preparation"]["prepared_frames"]
        .as_u64()
        .unwrap();
    roformer["corrections"] = json!([{"kind":"roformer-short-input-right-zero-pad-and-crop","input_frames":n,"padded_frames":409600,"right_padding_frames":409600-n,"crop_start":0,"crop_end_exclusive":n}]);
    write_manifest(&f.sets[0], roformer.clone());
    let record = attach(
        &f.reference,
        &f.sets[0],
        &f.root.join(uuid::Uuid::new_v4().to_string()),
        f.rate,
        |_| {},
    )
    .unwrap();
    assert_eq!(
        record.provenance["alignment_corrections"],
        roformer["corrections"]
    );
    let restored: ohmyloop::stems::library::StoredStemSet =
        serde_json::from_slice(&serde_json::to_vec(&record).unwrap()).unwrap();
    restored.validate().unwrap();
    let mut bad = restored.clone();
    bad.provenance["alignment_corrections"][0]["crop_start"] = json!(1);
    assert!(bad.validate().is_err());
    let mut legacy = restored;
    legacy
        .provenance
        .as_object_mut()
        .unwrap()
        .remove("alignment_corrections");
    legacy.validate().unwrap();
}

#[test]
fn overlapping_preparation_keeps_active_generations_readable_and_retires_when_released() {
    use ohmyloop::stems::library::{attach, prepare};
    use std::sync::{Arc, mpsc};
    let f = fixture(&[Role::Bass], false);
    let owned = f.root.join(uuid::Uuid::new_v4().to_string());
    let record = Arc::new(attach(&f.reference, &f.sets[0], &owned, f.rate, |_| {}).unwrap());
    let cache = f.root.join("cache");
    let original = prepare(&record, &f.reference, &owned, &cache, f.rate, |_| {}).unwrap();
    let pointer: Value =
        serde_json::from_slice(&fs::read(cache.join("cache.json")).unwrap()).unwrap();
    let old_directory = cache.join(pointer["generation"].as_str().unwrap());
    let mut active = original.open(1, MixState::default()).unwrap();
    drop(original);
    let (started_tx, started_rx) = mpsc::channel();
    let (continue_tx, continue_rx) = mpsc::channel();
    let (second_tx, second_rx) = mpsc::channel();
    let reference = f.reference.clone();
    let first = std::thread::spawn({
        let record = Arc::clone(&record);
        let owned = owned.clone();
        let cache = cache.clone();
        let reference = reference.clone();
        move || {
            let mut blocked = false;
            prepare(&record, &reference, &owned, &cache, 48_000, |progress| {
                if !blocked && progress.phase.starts_with("Decode") {
                    blocked = true;
                    started_tx.send(()).unwrap();
                    continue_rx.recv().unwrap();
                }
            })
            .unwrap()
        }
    });
    started_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    let second = std::thread::spawn({
        let record = Arc::clone(&record);
        let owned = owned.clone();
        let cache = cache.clone();
        move || {
            prepare(&record, &reference, &owned, &cache, 44_100, |_| {
                let _ = second_tx.send(());
            })
            .unwrap()
        }
    });
    // The second caller must not enter preparation while the first owns the cache.
    assert!(
        second_rx
            .recv_timeout(std::time::Duration::from_millis(30))
            .is_err()
    );
    let mut samples = [[0.; 2]; 32];
    active.read_frames(100, &mut samples).unwrap();
    assert!(samples.iter().all(|s| (s[0] - 0.1).abs() < 1e-6));
    continue_tx.send(()).unwrap();
    let first = first.join().unwrap();
    let second = second.join().unwrap();
    assert!(old_directory.exists());
    active.read_frames(200, &mut samples).unwrap();
    for prepared in [&first, &second] {
        let mut reader = prepared.open(1, MixState::default()).unwrap();
        assert_eq!(reader.read_frames(200, &mut samples).unwrap(), 32);
        assert!(samples.iter().flatten().all(|s| s.is_finite()));
    }
    drop(active);
    assert!(!old_directory.exists());
    drop(first);
    drop(second);
    // Only the most recently published device-rate generation remains on disk.
    assert_eq!(fs::read_dir(&cache).unwrap().count(), 2);
    let mut phases = vec![];
    let reused = prepare(&record, &f.reference, &owned, &cache, 44_100, |p| {
        phases.push(p.phase)
    })
    .unwrap();
    assert_eq!(phases, ["Reuse prepared playback"]);
    assert_eq!(reused.output_rate, 44_100);
    let pointer_before = fs::read(cache.join("cache.json")).unwrap();
    fs::write(owned.join("bass.wav"), b"invalid edited audio").unwrap();
    assert!(prepare(&record, &f.reference, &owned, &cache, 48_000, |_| {}).is_err());
    assert_eq!(fs::read(cache.join("cache.json")).unwrap(), pointer_before);
    assert_eq!(fs::read_dir(&cache).unwrap().count(), 2);
    let mut surviving_reader = reused.open(1, MixState::default()).unwrap();
    assert_eq!(surviving_reader.read_frames(100, &mut samples).unwrap(), 32);
}

#[test]
fn attachment_workflow_covers_actual_work_and_waits_for_publication() {
    use ohmyloop::stems::library::{WorkflowTracker, attach, prepare};
    let f = fixture(&[Role::Bass, Role::Drums], false);
    let owned = f.root.join(uuid::Uuid::new_v4().to_string());
    let mut tracker = WorkflowTracker::attachment(&f.sets[0]).unwrap();
    let mut snapshots = Vec::new();
    snapshots.push(tracker.begin_first().workflow.unwrap());
    let record = attach(&f.reference, &f.sets[0], &owned, 48_000, |p| {
        snapshots.push(tracker.update(p).workflow.unwrap());
    })
    .unwrap();
    let cache = owned.join("playback-cache");
    let prepared = prepare(&record, &f.reference, &owned, &cache, 48_000, |p| {
        snapshots.push(tracker.update(p).workflow.unwrap());
    })
    .unwrap();
    snapshots.push(tracker.begin("attachment.publish").workflow.unwrap());
    let total = snapshots[0].total;
    let ids = snapshots[0]
        .steps
        .iter()
        .map(|step| &step.id)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), total);
    assert!(
        snapshots[0]
            .steps
            .iter()
            .all(|step| step.label.split_whitespace().count() <= 8)
    );
    assert!(
        snapshots[0]
            .steps
            .iter()
            .all(|step| !step.label.contains(".wav") && !step.label.contains("/"))
    );
    for snapshot in &snapshots {
        assert_eq!(snapshot.total, total);
        assert!(
            snapshot.completed < total as f64,
            "Attachment reached 100% before publication"
        );
    }
    for pair in snapshots.windows(2) {
        assert!(
            pair[1].completed >= pair[0].completed,
            "Overall progress moved backwards: {} -> {}",
            pair[0].completed,
            pair[1].completed
        );
    }
    let final_progress = tracker.finish().workflow.unwrap();
    assert_eq!(final_progress.completed, total as f64);
    assert!(
        final_progress
            .steps
            .iter()
            .all(|step| step.state == "complete")
    );
    drop(prepared);
    let mut reuse = Vec::new();
    let _prepared = prepare(&record, &f.reference, &owned, &cache, 48_000, |p| {
        reuse.push(p.workflow.unwrap())
    })
    .unwrap();
    assert_eq!(reuse.len(), 1);
    assert_eq!(reuse[0].steps.len(), 1);
    assert_eq!(reuse[0].steps[0].label, "Reuse prepared playback");
    assert_eq!(reuse[0].completed, 1.0);
}

#[test]
fn failed_attachment_progress_never_completes_remaining_work() {
    use ohmyloop::stems::library::{WorkflowTracker, attach};
    let f = fixture(&[Role::Bass], false);
    let path = f.sets[0].join("bass.wav");
    let mut bytes = fs::read(&path).unwrap();
    bytes[44] ^= 1;
    fs::write(path, bytes).unwrap();
    let mut workflow = WorkflowTracker::attachment(&f.sets[0]).unwrap();
    let mut snapshots = Vec::new();
    assert!(
        attach(
            &f.reference,
            &f.sets[0],
            &f.root.join("failed-owned"),
            f.rate,
            |p| {
                snapshots.push(workflow.update(p).workflow.unwrap());
            }
        )
        .is_err()
    );
    let final_snapshot = snapshots.last().unwrap();
    assert!(final_snapshot.completed < final_snapshot.total as f64);
    assert_eq!(final_snapshot.steps.last().unwrap().state, "pending");
}

#[test]
fn attachment_decodes_owned_once_and_reuses_identical_pcm_without_rescanning() {
    use ohmyloop::stems::library::{attach, prepare};
    let f = fixture(&Role::ALL, false);
    let owned = f.root.join(uuid::Uuid::new_v4().to_string());
    let mut steps = std::collections::BTreeSet::new();
    let record = attach(&f.reference, &f.sets[0], &owned, 48000, |p| {
        if let Some(id) = p.step {
            steps.insert(id);
        }
    })
    .unwrap();
    let mut reopened = Vec::new();
    let adopted = prepare(
        &record,
        &f.reference,
        &owned,
        &owned.join("playback-cache"),
        48000,
        |p| reopened.push(p.phase),
    )
    .unwrap();
    assert_eq!(reopened, ["Reuse prepared playback"]);
    assert_eq!(steps.iter().filter(|id| id.ends_with(".hash")).count(), 8);
    assert_eq!(steps.iter().filter(|id| id.ends_with(".decode")).count(), 8);
    assert_eq!(
        steps.iter().filter(|id| id.ends_with(".resample")).count(),
        7
    );
    assert_eq!(
        steps.iter().filter(|id| id.ends_with(".headroom")).count(),
        1
    );
    assert!(
        !steps
            .iter()
            .any(|id| id.starts_with("cache.") && id.ends_with(".decode"))
    );
    let expected = PreparedComparison::prepare(&f.reference, &f.sets, 48000, |_| {}).unwrap();
    assert_eq!(adopted.shared_attenuation, expected.shared_attenuation);
    for selection in [0, 1] {
        let mut actual = adopted.open(selection, MixState::default()).unwrap();
        let mut before = expected.open(selection, MixState::default()).unwrap();
        let mut a = [[0.; 2]; 128];
        let mut b = a;
        assert_eq!(
            actual.read_frames(1024, &mut a).unwrap(),
            before.read_frames(1024, &mut b).unwrap()
        );
        assert_eq!(a, b);
    }
}

#[test]
fn near_silent_tracks_are_validated_then_excluded_with_truthful_progress() {
    use ohmyloop::stems::library::{WorkflowTracker, attach, prepare};
    let f = fixture(&[Role::Bass, Role::Drums, Role::Guitar, Role::Piano], false);
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(f.sets[0].join("manifest.json")).unwrap()).unwrap();
    for role in [Role::Bass, Role::Drums, Role::Guitar, Role::Piano] {
        let mut samples = vec![[0.; 2]; f.frames as usize];
        match role {
            Role::Bass => {}
            Role::Drums => samples.fill([0.0005, -0.0005]),
            Role::Guitar => samples.fill([0.001, -0.001]),
            Role::Piano => samples[1000] = [0.002, 0.],
            _ => unreachable!(),
        }
        let path = f.sets[0].join(format!("{}.wav", role.name()));
        write_wav(&path, f.rate, &samples).unwrap();
        manifest["stems"][role.name()]["sha256"] = json!(sha(&path));
    }
    write_manifest(&f.sets[0], manifest);
    let owned = f.root.join(uuid::Uuid::new_v4().to_string());
    let mut workflow = WorkflowTracker::attachment(&f.sets[0]).unwrap();
    let mut snapshots = Vec::new();
    let mut ids = std::collections::BTreeSet::new();
    let record = attach(&f.reference, &f.sets[0], &owned, 48000, |p| {
        if let Some(id) = &p.step {
            ids.insert(id.clone());
        }
        snapshots.push(workflow.update(p).workflow.unwrap());
    })
    .unwrap();
    assert_eq!(record.roles(), [Role::Guitar, Role::Piano]);
    for role in ["bass", "drums"] {
        assert!(!owned.join(format!("{role}.wav")).exists());
        assert!(f.sets[0].join(format!("{role}.wav")).exists());
        assert!(ids.contains(&format!("validation.{role}.hash")));
        assert!(ids.contains(&format!("validation.{role}.decode")));
        assert!(!ids.contains(&format!("validation.{role}.resample")));
    }
    for s in &snapshots {
        assert!(s.completed < s.total as f64);
    }
    let done = workflow.finish().workflow.unwrap();
    assert_eq!(done.completed, done.total as f64);
    let skipped: Vec<_> = done
        .steps
        .iter()
        .filter(|s| s.state == "skipped")
        .map(|s| s.label.as_str())
        .collect();
    assert_eq!(skipped, ["Skip near-silent bass", "Skip near-silent drums"]);
    let mut events = Vec::new();
    let _prepared = prepare(
        &record,
        &f.reference,
        &owned,
        &owned.join("playback-cache"),
        48000,
        |p| events.push(p.phase),
    )
    .unwrap();
    assert_eq!(events, ["Reuse prepared playback"]);
}

#[test]
fn rejected_owned_corruption_reference_races_and_all_quiet_sets_leave_no_new_assets() {
    use ohmyloop::stems::library::{WorkflowTracker, attach};
    for scenario in 0..3 {
        let f = fixture(&[Role::Bass], false);
        let owned = f.root.join(uuid::Uuid::new_v4().to_string());
        if scenario == 2 {
            let path = f.sets[0].join("bass.wav");
            write_wav(&path, f.rate, &vec![[0.0005; 2]; f.frames as usize]).unwrap();
            let mut manifest: Value =
                serde_json::from_slice(&fs::read(f.sets[0].join("manifest.json")).unwrap())
                    .unwrap();
            manifest["stems"]["bass"]["sha256"] = json!(sha(&path));
            write_manifest(&f.sets[0], manifest);
        }
        let mut changed = false;
        let mut workflow = WorkflowTracker::attachment(&f.sets[0]).unwrap();
        let mut snapshots = Vec::new();
        let result = attach(&f.reference, &f.sets[0], &owned, 48000, |p| {
            if !changed
                && ((scenario == 0
                    && p.step.as_deref() == Some("owned.bass.copy")
                    && p.total == Some(p.completed))
                    || (scenario == 1 && p.step.as_deref() == Some("validation.reference.decode")))
            {
                changed = true;
                let path = if scenario == 0 {
                    owned.join("bass.wav")
                } else {
                    f.reference.clone()
                };
                let mut bytes = fs::read(&path).unwrap();
                if scenario == 0 {
                    bytes[44] ^= 1;
                    bytes.push(0);
                } else {
                    bytes[128] ^= 1;
                }
                let modified = fs::metadata(&path).unwrap().modified().unwrap();
                fs::write(&path, bytes).unwrap();
                fs::File::options()
                    .write(true)
                    .open(path)
                    .unwrap()
                    .set_modified(modified + std::time::Duration::from_secs(1))
                    .unwrap();
            }
            snapshots.push(workflow.update(p).workflow.unwrap());
        });
        let error = result.unwrap_err().to_string();
        if scenario == 1 {
            assert!(
                error.contains("changed during attachment preparation"),
                "{error}"
            );
        }
        if scenario == 2 {
            assert!(error.contains("No audible stems found"), "{error}");
        }
        assert!(!owned.exists());
        assert!(f.sets[0].join("bass.wav").exists());
        let last = snapshots.last().unwrap();
        assert!(last.completed < last.total as f64);
        assert_eq!(last.steps.last().unwrap().state, "pending");
    }
}

#[test]
fn all_near_silent_replacement_keeps_existing_managed_association_and_playback() {
    use ohmyloop::{
        managed::ManagedLibrary,
        stems::library::{attach, prepare},
    };
    let f = fixture(&[Role::Bass], false);
    let library = ManagedLibrary::create(&f.root).unwrap();
    let imported = library
        .commit_import(library.import(&f.reference, |_, _| {}).unwrap())
        .unwrap();
    let key = imported.record.id.to_string();
    let reference = library.asset_path(&imported.record).unwrap();
    let old_id = uuid::Uuid::new_v4().to_string();
    let old_directory = library.stem_directory(&key, &old_id).unwrap();
    let old = attach(&reference, &f.sets[0], &old_directory, 48000, |_| {}).unwrap();
    library
        .save_stems(&key, imported.record.reference_revision, Some(old.clone()))
        .unwrap();
    let old_audio = prepare(
        &old,
        &reference,
        &old_directory,
        &old_directory.join("playback-cache"),
        48000,
        |_| {},
    )
    .unwrap();
    let mut reader = old_audio.open(1, MixState::default()).unwrap();
    let mut samples = [[0.; 2]; 16];
    reader.read_frames(100, &mut samples).unwrap();
    let before_audio = samples;
    let document = library.root().join(format!("songs/{key}/song.json"));
    let before_document = fs::read(&document).unwrap();
    let path = f.sets[0].join("bass.wav");
    write_wav(&path, f.rate, &vec![[0.0005; 2]; f.frames as usize]).unwrap();
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(f.sets[0].join("manifest.json")).unwrap()).unwrap();
    manifest["stems"]["bass"]["sha256"] = json!(sha(&path));
    write_manifest(&f.sets[0], manifest);
    let new_directory = library
        .stem_directory(&key, &uuid::Uuid::new_v4().to_string())
        .unwrap();
    assert!(
        attach(&reference, &f.sets[0], &new_directory, 48000, |_| {})
            .unwrap_err()
            .to_string()
            .contains("No audible stems found")
    );
    assert!(!new_directory.exists());
    assert_eq!(fs::read(document).unwrap(), before_document);
    assert!(old_directory.join("bass.wav").exists());
    reader.read_frames(100, &mut samples).unwrap();
    assert_eq!(samples, before_audio);
}

#[test]
fn generated_owned_outputs_validate_without_wav_copy_and_cancel_at_native_stage() {
    use ohmyloop::stems::{
        WorkControl,
        library::{attach_generated, prepare},
    };
    let f = fixture(&Role::ALL, false);
    let owned = f.root.join(uuid::Uuid::new_v4().to_string());
    fs::create_dir(&owned).unwrap();
    for entry in fs::read_dir(&f.sets[0]).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), owned.join(entry.file_name())).unwrap();
    }
    #[cfg(unix)]
    let inode = {
        use std::os::unix::fs::MetadataExt;
        fs::metadata(owned.join("bass.wav")).unwrap().ino()
    };
    let mut events = Vec::new();
    let (record, prepared) =
        attach_generated(&f.reference, &owned, f.rate, &WorkControl::default(), |p| {
            events.push(p)
        })
        .unwrap();
    assert_eq!(record.tracks.len(), 6);
    assert!(events.iter().all(|p| !p.phase.starts_with("Copy")));
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(fs::metadata(owned.join("bass.wav")).unwrap().ino(), inode);
    }
    let mut reuse = Vec::new();
    let cached = prepare(
        &record,
        &f.reference,
        &owned,
        &owned.join("playback-cache"),
        f.rate,
        |p| reuse.push(p),
    )
    .unwrap();
    assert!(
        reuse
            .iter()
            .all(|p| p.step.as_deref() == Some("cache.reuse"))
    );
    drop((prepared, cached));
    let cancelled = f.root.join(uuid::Uuid::new_v4().to_string());
    fs::create_dir(&cancelled).unwrap();
    for entry in fs::read_dir(&f.sets[0]).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), cancelled.join(entry.file_name())).unwrap();
    }
    let control = WorkControl::default();
    let result = attach_generated(&f.reference, &cancelled, f.rate, &control, |p| {
        if p.step.as_deref() == Some("validation.bass.decode") {
            control.cancel();
        }
    });
    assert!(result.is_err());
    assert!(!cancelled.join("playback-cache/cache.json").exists());
}

#[test]
fn all_role_validation_follows_checklist_without_reopening_completed_rows() {
    use ohmyloop::stems::library::{WorkflowTracker, attach};
    let f = fixture(&Role::ALL, false);
    let owned = f.root.join(uuid::Uuid::new_v4().to_string());
    let mut tracker = WorkflowTracker::attachment(&f.sets[0]).unwrap();
    let mut previous = tracker.begin_first().workflow.unwrap();
    let mut decoded = Vec::new();
    let mut separation = WorkflowTracker::separation();
    let mut separation_previous = separation.begin("result.validate").workflow.unwrap();
    attach(&f.reference, &f.sets[0], &owned, 48000, |p| {
        if let Some(id) = &p.step
            && id.starts_with("validation.")
            && id.ends_with(".decode")
            && !id.contains("reference")
            && !id.contains("prepared")
            && decoded.last() != Some(id)
        {
            decoded.push(id.clone());
        }
        let separation_next = separation.update(p.clone()).workflow.unwrap();
        if let Some(id) = &p.step
            && let Some(step) = separation_next.steps.iter().find(|s| &s.id == id)
        {
            assert_eq!(
                step.state, "active",
                "Actual stage {id} was prematurely completed"
            );
        }
        for old in &separation_previous.steps {
            if old.state == "complete" || old.state == "skipped" {
                let state = separation_next
                    .steps
                    .iter()
                    .find(|s| s.id == old.id)
                    .unwrap()
                    .state;
                assert!(
                    state == "complete" || state == "skipped",
                    "Separation reopened {}",
                    old.id
                );
            }
        }
        separation_previous = separation_next;
        let next = tracker.update(p).workflow.unwrap();
        for old in &previous.steps {
            if old.state == "complete" || old.state == "skipped" {
                let state = next.steps.iter().find(|s| s.id == old.id).unwrap().state;
                assert!(
                    state == "complete" || state == "skipped",
                    "Reopened {}",
                    old.id
                );
            }
        }
        assert!(next.completed >= previous.completed);
        assert_eq!(next.total, previous.total);
        previous = next;
    })
    .unwrap();
    let mut expected = Role::ALL.map(|r| format!("validation.{}.decode", r.name()));
    expected.sort();
    assert_eq!(decoded, expected);
    assert_eq!(previous.total, previous.steps.len()); // attachment retains task units
}

#[test]
fn separation_weights_measured_chunks_hold_unknowns_and_require_publication_confirmation() {
    use ohmyloop::stems::{PreparationProgress, library::WorkflowTracker};
    fn update(
        tracker: &mut WorkflowTracker,
        step: &str,
        completed: u64,
        total: Option<u64>,
    ) -> ohmyloop::stems::library::WorkflowProgress {
        tracker
            .update(PreparationProgress {
                phase: "Processing".into(),
                step: Some(step.into()),
                completed,
                total,
                workflow: None,
            })
            .workflow
            .unwrap()
    }
    let mut tracker = WorkflowTracker::separation();
    let start = tracker.begin_first().workflow.unwrap();
    let before = update(&mut tracker, "worker.infer", 0, Some(40));
    let half = update(&mut tracker, "worker.infer", 20, Some(40));
    let end = update(&mut tracker, "worker.infer", 40, Some(40));
    assert_eq!(start.total, half.total);
    assert!((half.completed - before.completed) > half.total as f64 * 0.3);
    assert!((half.completed - before.completed - (end.completed - half.completed)).abs() < 1e-6);
    let unknown = update(&mut tracker, "worker.write", 0, None);
    let held = update(&mut tracker, "worker.write", 0, None);
    assert_eq!(unknown.completed, held.completed);
    assert_eq!(
        held.steps
            .iter()
            .find(|s| s.id == "worker.write")
            .unwrap()
            .total,
        None
    );
    let stale = update(&mut tracker, "worker.load", 0, None);
    assert_eq!(stale.completed, held.completed);
    assert_eq!(
        stale
            .steps
            .iter()
            .find(|s| s.id == "worker.load")
            .unwrap()
            .state,
        "complete"
    );
    let skipped = update(&mut tracker, "skip.piano", 1, Some(1));
    assert_eq!(
        skipped
            .steps
            .iter()
            .find(|s| s.id == "validation.piano.resample")
            .unwrap()
            .state,
        "skipped"
    );
    let prepublish = update(&mut tracker, "attachment.publish", 1, Some(1));
    assert!(prepublish.completed < prepublish.total as f64);
    let complete = tracker.finish().workflow.unwrap();
    assert_eq!(complete.completed, complete.total as f64);
    let mut retry = WorkflowTracker::separation();
    assert_eq!(retry.begin_first().workflow.unwrap().completed, 0.);
}
