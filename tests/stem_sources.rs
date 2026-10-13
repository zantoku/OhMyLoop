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
        let value = json!({"version":1,"status":"complete","identity":{"preparation":{"version":1,"source_sha256":sha(&reference),"source_rate":rate,"source_frames":frames,"source_span":{"start":0,"end_exclusive":frames},"prepared_rate":rate,"prepared_frames":frames,"channels":2,"prepared_sha256":sha(&set.join("prepared.wav"))}},"stems":stems});
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
