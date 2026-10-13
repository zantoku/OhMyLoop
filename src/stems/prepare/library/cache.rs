//! Serialized preparation and immutable, reader-owned cache generations.
use super::*;
use std::time::UNIX_EPOCH;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct Freshness {
    bytes: u64,
    modified_seconds: u64,
    modified_nanos: u32,
}
fn freshness(path: &Path) -> Result<Freshness> {
    let metadata = fs::metadata(path)?;
    let modified = metadata.modified()?.duration_since(UNIX_EPOCH)?;
    Ok(Freshness {
        bytes: metadata.len(),
        modified_seconds: modified.as_secs(),
        modified_nanos: modified.subsec_nanos(),
    })
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct CacheKey {
    policy: u32,
    rate: u32,
    record: StoredStemSet,
    reference: Freshness,
    stems: Vec<Freshness>,
}
#[derive(Serialize, Deserialize)]
struct CachedPcm {
    file: String,
    rate: u32,
    frames: u64,
    channels: u32,
}
#[derive(Serialize, Deserialize)]
struct CacheRecord {
    version: u32,
    generation: String,
    key: CacheKey,
    attenuation: f32,
    reference: CachedPcm,
    stems: Vec<CachedPcm>,
}
fn cached(pcm: &Pcm) -> CachedPcm {
    CachedPcm {
        file: pcm.path.file_name().unwrap().to_string_lossy().into_owned(),
        rate: pcm.rate,
        frames: pcm.frames,
        channels: pcm.channels,
    }
}
fn restore(pcm: &CachedPcm, directory: &Path, rate: u32, frames: u64) -> Result<Pcm> {
    ensure!(
        pcm.rate == rate && pcm.frames == frames && (pcm.channels == 1 || pcm.channels == 2),
        "Invalid cached PCM geometry"
    );
    let path = safe_file(directory, &pcm.file)?;
    ensure!(
        fs::metadata(&path)?.len() == frames.checked_mul(8).context("PCM size overflow")?,
        "Truncated prepared PCM"
    );
    Ok(Pcm {
        path,
        rate,
        frames,
        channels: pcm.channels,
    })
}

// The desktop can start overlapping opens before stale-publication guards run.
// Serialize cache writers and keep published PCM generations immutable. Readers
// retain their generation until their last factory/source drops, including Windows
// where open audio files cannot be removed.
fn preparation_lock(cache: &Path) -> Result<Arc<std::sync::Mutex<()>>> {
    type Locks = std::collections::HashMap<PathBuf, std::sync::Weak<std::sync::Mutex<()>>>;
    static LOCKS: std::sync::OnceLock<std::sync::Mutex<Locks>> = std::sync::OnceLock::new();
    let mut locks = LOCKS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| anyhow::anyhow!("Preparation registry unavailable"))?;
    locks.retain(|_, lock| lock.strong_count() > 0);
    if let Some(lock) = locks.get(cache).and_then(std::sync::Weak::upgrade) {
        return Ok(lock);
    }
    let lock = Arc::new(std::sync::Mutex::new(()));
    locks.insert(cache.to_path_buf(), Arc::downgrade(&lock));
    Ok(lock)
}
type Generations = std::collections::HashMap<PathBuf, std::sync::Weak<TemporaryDirectory>>;
fn generations() -> &'static std::sync::Mutex<Generations> {
    static GENERATIONS: std::sync::OnceLock<std::sync::Mutex<Generations>> =
        std::sync::OnceLock::new();
    GENERATIONS.get_or_init(Default::default)
}
fn generation_owner(directory: &Path) -> Result<Arc<TemporaryDirectory>> {
    let mut owners = generations()
        .lock()
        .map_err(|_| anyhow::anyhow!("Preparation ownership unavailable"))?;
    owners.retain(|_, owner| owner.strong_count() > 0);
    if let Some(owner) = owners.get(directory).and_then(std::sync::Weak::upgrade) {
        return Ok(owner);
    }
    let owner = Arc::new(TemporaryDirectory(directory.to_path_buf(), true.into()));
    owners.insert(directory.to_path_buf(), Arc::downgrade(&owner));
    Ok(owner)
}
fn retire_generations(cache: &Path, current: &Path) -> Result<()> {
    for entry in fs::read_dir(cache)? {
        let entry = entry?;
        let directory = entry.path();
        if directory == current
            || !entry.file_type()?.is_dir()
            || uuid::Uuid::parse_str(&entry.file_name().to_string_lossy()).is_err()
        {
            continue;
        }
        directory_safe(&directory)?;
        let owner = generations()
            .lock()
            .map_err(|_| anyhow::anyhow!("Preparation ownership unavailable"))?
            .get(&directory)
            .and_then(std::sync::Weak::upgrade);
        if let Some(owner) = owner {
            owner.1.store(false, std::sync::atomic::Ordering::Release);
        } else if let Err(error) = fs::remove_dir_all(directory)
            && error.kind() != ErrorKind::NotFound
        {
            return Err(error.into());
        }
    }
    Ok(())
}

/// Metadata captured before validation, never refreshed into an already-proven cache key.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct InputSnapshot {
    reference: Freshness,
    stems: Vec<Freshness>,
}
impl InputSnapshot {
    /// Project proven metadata onto the retained track subset without reading files again.
    pub(super) fn retained(
        &self,
        original: &StoredStemSet,
        retained: &StoredStemSet,
    ) -> Result<Self> {
        ensure!(
            self.stems.len() == original.tracks.len()
                && original.version == retained.version
                && original.id == retained.id
                && original.reference_sha256 == retained.reference_sha256
                && original.reference_rate == retained.reference_rate
                && original.reference_frames == retained.reference_frames
                && original.provenance == retained.provenance,
            "Filtered stem record disagrees with validated input identity"
        );
        let mut previous = None;
        let mut stems = Vec::with_capacity(retained.tracks.len());
        for track in &retained.tracks {
            let index = original
                .tracks
                .iter()
                .position(|candidate| candidate == track)
                .context("Filtered stem track was not validated")?;
            ensure!(
                previous.is_none_or(|previous| index > previous),
                "Filtered stem roles are duplicated or reordered"
            );
            previous = Some(index);
            stems.push(self.stems[index].clone());
        }
        Ok(Self {
            reference: self.reference.clone(),
            stems,
        })
    }
}
pub(super) fn verify_inputs(
    record: &StoredStemSet,
    reference: &Path,
    owned: &Path,
    checked: &InputSnapshot,
) -> Result<()> {
    unchanged(record, reference, owned, checked)
}
pub(super) fn capture_inputs(
    record: &StoredStemSet,
    reference: &Path,
    owned: &Path,
) -> Result<InputSnapshot> {
    directory_safe(owned)?;
    Ok(InputSnapshot {
        reference: freshness(reference)?,
        stems: record
            .tracks
            .iter()
            .map(|track| freshness(&safe_file(owned, &track.file)?))
            .collect::<Result<_>>()?,
    })
}
fn unchanged(
    record: &StoredStemSet,
    reference: &Path,
    owned: &Path,
    checked: &InputSnapshot,
) -> Result<()> {
    ensure!(
        capture_inputs(record, reference, owned)? == *checked,
        "Reference or stems changed during attachment preparation"
    );
    Ok(())
}

/// Adopt validated PCM instead of decoding, resampling and scanning it again.
pub(super) fn adopt(
    record: &StoredStemSet,
    reference: &Path,
    owned: &Path,
    cache: &Path,
    validated: PreparedComparison,
    checked: &InputSnapshot,
    progress: impl FnMut(PreparationProgress),
) -> Result<PreparedComparison> {
    adopt_with_transfer(
        record,
        reference,
        owned,
        cache,
        validated,
        checked,
        progress,
        |source, target, move_file, report| {
            transfer_pcm(source, target, move_file, |s, t| fs::rename(s, t), report)
        },
    )
}

fn transfer_pcm(
    source: &Path,
    target: &Path,
    move_file: bool,
    mut rename: impl FnMut(&Path, &Path) -> std::io::Result<()>,
    report: &mut dyn FnMut(u64),
) -> Result<()> {
    let bytes = fs::metadata(source)?.len();
    if move_file && rename(source, target).is_ok() {
        report(bytes);
        return Ok(());
    }
    // Cross-volume temp/library paths (and shared temporary owners) use a bounded copy.
    let mut input = File::open(source)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)?;
    let mut buffer = vec![0; 256 * 1024];
    loop {
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        output.write_all(&buffer[..n])?;
        report(n as u64);
    }
    output.sync_all()?;
    ensure!(
        fs::metadata(target)?.len() == bytes,
        "Truncated adopted PCM"
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn adopt_with_transfer(
    record: &StoredStemSet,
    reference_path: &Path,
    owned: &Path,
    cache: &Path,
    validated: PreparedComparison,
    checked: &InputSnapshot,
    progress: impl FnMut(PreparationProgress),
    mut transfer: impl FnMut(&Path, &Path, bool, &mut dyn FnMut(u64)) -> Result<()>,
) -> Result<PreparedComparison> {
    record.validate()?;
    ensure!(
        validated.reference_rate == record.reference_rate
            && validated.reference_frames == record.reference_frames
            && validated.sets.len() == 1
            && validated.sets[0].len() == record.tracks.len(),
        "Validated preparation disagrees with stem record"
    );
    let output_rate = validated.output_rate;
    let frames = coordinate_frames(record.reference_frames, output_rate, record.reference_rate)?;
    ensure!(
        validated.frames == frames
            && validated.shared_attenuation.is_finite()
            && validated.shared_attenuation > 0.0
            && validated.shared_attenuation <= 1.0,
        "Invalid validated playback geometry or attenuation"
    );
    for (track, (role, _)) in record.tracks.iter().zip(&validated.sets[0]) {
        ensure!(
            Role::parse(&track.role)? == *role,
            "Validated track order disagrees with record"
        );
    }
    directory_safe(cache)?;
    fs::create_dir_all(cache)?;
    let resolved = cache.canonicalize()?;
    let cache = resolved.as_path();
    let lock = preparation_lock(cache)?;
    let _exclusive = lock
        .lock()
        .map_err(|_| anyhow::anyhow!("Preparation lock unavailable"))?;
    unchanged(record, reference_path, owned, checked)?;
    let mut identity = record.clone();
    identity.mix = SavedMix::default();
    let key = CacheKey {
        policy: POLICY,
        rate: output_rate,
        record: identity,
        reference: checked.reference.clone(),
        stems: checked.stems.clone(),
    };
    let directory = cache.join(uuid::Uuid::new_v4().to_string());
    fs::create_dir(&directory)?;
    let started = Instant::now();
    let mut progress = throttled_progress(progress);
    let total = frames
        .checked_mul(8)
        .and_then(|n| n.checked_mul(record.tracks.len() as u64 + 1))
        .context("PCM size overflow")?;
    let result = (|| {
        let (reference, bank) = {
            let mut completed = 0;
            let mut report = |bytes| {
                completed += bytes;
                progress(PreparationProgress {
                    phase: "Keep validated playback audio".into(),
                    completed,
                    total: Some(total),
                    step: Some("cache.adopt".into()),
                    workflow: None,
                });
            };
            report(0);
            let move_file = Arc::strong_count(&validated.temporary) == 1;
            let mut next = 0;
            let mut adopt_pcm = |pcm: &Pcm| -> Result<Pcm> {
                ensure!(
                    pcm.rate == output_rate
                        && pcm.frames == frames
                        && (pcm.channels == 1 || pcm.channels == 2),
                    "Invalid validated PCM geometry"
                );
                ensure!(
                    pcm.path.parent() == Some(validated.temporary.0.as_path()),
                    "Prepared PCM outside private directory"
                );
                ensure!(
                    fs::metadata(&pcm.path)?.len() == frames * 8,
                    "Truncated validated PCM"
                );
                let path = directory.join(format!("{next}.pcm"));
                next += 1;
                transfer(&pcm.path, &path, move_file, &mut report)?;
                ensure!(
                    fs::metadata(&path)?.len() == frames * 8,
                    "Truncated adopted PCM"
                );
                Ok(Pcm {
                    path,
                    ..pcm.clone()
                })
            };
            let reference = adopt_pcm(&validated.reference)?;
            let bank = validated.sets[0]
                .iter()
                .map(|(role, pcm)| Ok((*role, adopt_pcm(pcm)?)))
                .collect::<Result<Vec<_>>>()?;
            (reference, bank)
        };
        unchanged(record, reference_path, owned, checked)?;
        let saved = CacheRecord {
            version: VERSION,
            generation: directory
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            key,
            attenuation: validated.shared_attenuation,
            reference: cached(&reference),
            stems: bank.iter().map(|(_, pcm)| cached(pcm)).collect(),
        };
        progress(PreparationProgress {
            phase: "Save prepared playback".into(),
            completed: 0,
            total: Some(1),
            step: Some("cache.save".into()),
            workflow: None,
        });
        let bytes = serde_json::to_vec(&saved)?;
        let owner = generation_owner(&directory)?;
        unchanged(record, reference_path, owned, checked)?;
        directory_safe(&cache.join("cache.json.tmp"))?;
        crate::storage::atomic_write(&cache.join("cache.json"), &bytes)?;
        // After publication, cleanup failure must not invalidate the published generation.
        let _ = retire_generations(cache, &directory);
        progress(PreparationProgress {
            phase: "Save prepared playback".into(),
            completed: 1,
            total: Some(1),
            step: Some("cache.save".into()),
            workflow: None,
        });
        Ok(PreparedComparison {
            reference_rate: record.reference_rate,
            reference_frames: record.reference_frames,
            output_rate,
            frames,
            shared_attenuation: validated.shared_attenuation,
            temp_bytes: total,
            elapsed: started.elapsed(),
            reference,
            sets: vec![bank],
            temporary: owner,
        })
    })();
    if result.is_err()
        && let Err(cleanup) = fs::remove_dir_all(&directory)
    {
        return result.map_err(|error: anyhow::Error| {
            error.context(format!("Adoption cleanup also failed: {cleanup}"))
        });
    }
    result
}

/// No content hashes are computed here: cheap authoritative-file freshness checks
/// invalidate typical manual edits; cached PCM is disposable and may be rebuilt.
pub fn prepare(
    record: &StoredStemSet,
    reference: &Path,
    owned: &Path,
    cache: &Path,
    output_rate: u32,
    progress: impl FnMut(PreparationProgress),
) -> Result<PreparedComparison> {
    let mut progress = throttled_progress(progress);
    record.validate()?;
    ensure!(
        (8_000..=192_000).contains(&output_rate),
        "Unsupported output rate"
    );
    directory_safe(owned)?;
    directory_safe(cache)?;
    fs::create_dir_all(cache)?;
    let resolved_cache = cache.canonicalize()?;
    let cache = resolved_cache.as_path();
    let lock = preparation_lock(cache)?;
    let _exclusive = lock
        .lock()
        .map_err(|_| anyhow::anyhow!("Preparation lock unavailable"))?;
    let paths = record
        .tracks
        .iter()
        .map(|t| safe_file(owned, &t.file))
        .collect::<Result<Vec<_>>>()?;
    let mut identity = record.clone();
    identity.mix = SavedMix::default(); // Mix settings never invalidate PCM/headroom.
    let key = CacheKey {
        policy: POLICY,
        rate: output_rate,
        record: identity,
        reference: freshness(reference)?,
        stems: paths.iter().map(|p| freshness(p)).collect::<Result<_>>()?,
    };
    let frames = coordinate_frames(record.reference_frames, output_rate, record.reference_rate)?;
    let cache_record = cache.join("cache.json");
    let started = Instant::now();
    let reuse = (|| -> Result<PreparedComparison> {
        let path = safe_file(cache, "cache.json")?;
        ensure!(
            fs::metadata(&path)?.len() <= 4 * 1024 * 1024,
            "Oversized preparation cache"
        );
        let saved: CacheRecord = serde_json::from_slice(&fs::read(path)?)?;
        ensure!(
            saved.version == VERSION
                && saved.key == key
                && saved.stems.len() == record.tracks.len(),
            "Stale preparation cache"
        );
        ensure!(
            saved.attenuation.is_finite() && saved.attenuation > 0.0 && saved.attenuation <= 1.0,
            "Invalid cached attenuation"
        );
        uuid::Uuid::parse_str(&saved.generation).context("Invalid preparation generation")?;
        let directory = cache.join(&saved.generation);
        directory_safe(&directory)?;
        let reference = restore(&saved.reference, &directory, output_rate, frames)?;
        let bank = record
            .tracks
            .iter()
            .zip(&saved.stems)
            .map(|(t, p)| {
                Ok((
                    Role::parse(&t.role)?,
                    restore(p, &directory, output_rate, frames)?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(PreparedComparison {
            reference_rate: record.reference_rate,
            reference_frames: record.reference_frames,
            output_rate,
            frames,
            shared_attenuation: saved.attenuation,
            temp_bytes: frames * 8 * (bank.len() as u64 + 1),
            elapsed: started.elapsed(),
            reference,
            sets: vec![bank],
            temporary: generation_owner(&directory)?,
        })
    })();
    if let Ok(prepared) = reuse {
        retire_generations(cache, &prepared.temporary.0)?;
        progress(PreparationProgress {
            phase: "Reuse prepared playback".into(),
            completed: 1,
            total: Some(1),
            step: Some("cache.reuse".into()),
            workflow: WorkflowTracker::reuse().finish().workflow,
        });
        return Ok(prepared);
    }
    let roles = record
        .tracks
        .iter()
        .map(|track| track.role.clone())
        .collect::<Vec<_>>();
    let mut workflow = WorkflowTracker::cache(&roles);
    progress(workflow.begin("cache.reference.decode"));
    let directory = cache.join(uuid::Uuid::new_v4().to_string());
    fs::create_dir(&directory)?;
    let result = (|| {
        let mut report = |p| progress(workflow.update(p));
        let mut preparer = Preparer {
            directory: directory.clone(),
            next: 0,
            progress: &mut report,
            step: None,
            decoded_peak: 0.,
        };
        let reference_path = reference;
        preparer.stage("cache.reference.decode");
        let decoded = preparer.decode(reference_path)?;
        ensure!(
            decoded.rate == record.reference_rate && decoded.frames == record.reference_frames,
            "Reference geometry changed"
        );
        preparer.stage("cache.reference.resample");
        let reference = preparer.resample(decoded, output_rate, frames)?;
        let mut bank = Vec::new();
        for (track, path) in record.tracks.iter().zip(&paths) {
            preparer.stage(format!("cache.{}.decode", track.role));
            let decoded = preparer.decode(path)?;
            ensure!(
                decoded.rate == track.rate
                    && decoded.frames == track.frames
                    && decoded.channels == track.channels,
                "Stem geometry changed: {}",
                track.role
            );
            preparer.stage(format!("cache.{}.resample", track.role));
            bank.push((
                Role::parse(&track.role)?,
                preparer.resample(decoded, output_rate, frames)?,
            ));
        }
        preparer.stage("cache.headroom");
        let peak = preparer.peak(&reference, &[bank.clone()])?;
        preparer.stage("cache.save");
        let attenuation = if peak > 0.0 {
            (0.70 / peak).min(1.0) as f32
        } else {
            1.0
        };
        // Detect files edited during preparation without treating metadata as integrity.
        ensure!(
            freshness(reference_path)? == key.reference
                && paths
                    .iter()
                    .map(|p| freshness(p))
                    .collect::<Result<Vec<_>>>()?
                    == key.stems,
            "Prepared reference missing"
        );
        let saved = CacheRecord {
            version: VERSION,
            generation: directory
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            key,
            attenuation,
            reference: cached(&reference),
            stems: bank.iter().map(|(_, p)| cached(p)).collect(),
        };
        let temporary_pointer = cache_record.with_extension("json.tmp");
        directory_safe(&temporary_pointer)?;
        crate::storage::atomic_write(&cache_record, &serde_json::to_vec(&saved)?)?;
        retire_generations(cache, &directory)?;
        Ok(PreparedComparison {
            reference_rate: record.reference_rate,
            reference_frames: record.reference_frames,
            output_rate,
            frames,
            shared_attenuation: attenuation,
            temp_bytes: frames * 8 * (bank.len() as u64 + 1),
            elapsed: started.elapsed(),
            reference,
            sets: vec![bank],
            temporary: generation_owner(&directory)?,
        })
    })();
    if result.is_ok() {
        progress(workflow.finish());
    }
    match result {
        Err(error) => match fs::remove_dir_all(&directory) {
            Ok(()) => Err(error),
            Err(cleanup) => {
                Err(error.context(format!("Preparation cache cleanup also failed: {cleanup}")))
            }
        },
        success => success,
    }
}

#[cfg(test)]
mod adoption_tests {
    use super::*;
    use serde_json::json;

    struct Fixture {
        root: PathBuf,
        reference: PathBuf,
        owned: PathBuf,
        cache: PathBuf,
        record: StoredStemSet,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
    fn fixture() -> Fixture {
        let root =
            std::env::temp_dir().join(format!("ohmyloop-adoption-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let reference = root.join("reference.wav");
        let owned = root.join("owned");
        fs::create_dir(&owned).unwrap();
        fs::write(&reference, b"reference metadata fixture").unwrap();
        fs::write(owned.join("bass.wav"), b"stem metadata fixture").unwrap();
        let hash = "a".repeat(64);
        let record = StoredStemSet {
            version: VERSION,
            id: uuid::Uuid::new_v4().to_string(),
            reference_sha256: hash.clone(),
            reference_rate: 44100,
            reference_frames: 32,
            provenance: json!({"preparation": {"version":1,"source_sha256":hash,"source_rate":44100,
                "source_frames":32,"source_span":{"start":0,"end_exclusive":32},"prepared_rate":44100,
                "prepared_frames":32,"channels":2,"prepared_sha256":"a".repeat(64),
                "preprocessing":PREPROCESSING,"geometry_policy":GEOMETRY_POLICY},
                "settings":{"normalization":"none per stem"},
                "model_lock":{"version":1,"model":"demucs-6s","files":{
                    "5c90dfd2-34c22ccb.th":{"sha256":format!("34c22ccb{}", "a".repeat(56)),"bytes":1,"url":"https://example.test/model"},
                    "htdemucs_6s.yaml":{"sha256":"a".repeat(64),"bytes":1,"url":"https://example.test/config"}}}}),
            tracks: vec![StoredTrack {
                role: "bass".into(),
                file: "bass.wav".into(),
                sha256: "a".repeat(64),
                rate: 44100,
                frames: 32,
                channels: 2,
            }],
            mix: SavedMix::default(),
        };
        Fixture {
            cache: root.join("cache"),
            root,
            reference,
            owned,
            record,
        }
    }
    fn validated(f: &Fixture) -> PreparedComparison {
        let directory = f.root.join(uuid::Uuid::new_v4().to_string());
        fs::create_dir(&directory).unwrap();
        let reference = Pcm {
            path: directory.join("0.pcm"),
            rate: 44100,
            frames: 32,
            channels: 2,
        };
        let stem = Pcm {
            path: directory.join("1.pcm"),
            ..reference.clone()
        };
        fs::write(&reference.path, vec![1; 256]).unwrap();
        fs::write(&stem.path, vec![2; 256]).unwrap();
        PreparedComparison {
            reference_rate: 44100,
            reference_frames: 32,
            output_rate: 44100,
            frames: 32,
            shared_attenuation: 0.7,
            temp_bytes: 512,
            elapsed: Duration::ZERO,
            reference,
            sets: vec![vec![(Role::Bass, stem)]],
            temporary: Arc::new(TemporaryDirectory(directory, false.into())),
        }
    }
    #[test]
    fn adopts_pcm_without_decode_and_reuses_published_generation() {
        let f = fixture();
        let checked = capture_inputs(&f.record, &f.reference, &f.owned).unwrap();
        let original = validated(&f);
        let source_directory = original.temporary.0.clone();
        let p = adopt(
            &f.record,
            &f.reference,
            &f.owned,
            &f.cache,
            original,
            &checked,
            |_| {},
        )
        .unwrap();
        assert!(!source_directory.exists());
        assert_eq!(fs::read(&p.reference.path).unwrap(), vec![1; 256]);
        let reused = prepare(&f.record, &f.reference, &f.owned, &f.cache, 44100, |_| {}).unwrap();
        assert_eq!(reused.reference.path, p.reference.path);
        assert_eq!(reused.shared_attenuation, 0.7);
    }
    #[test]
    fn cross_device_transfer_copies_and_keeps_shared_source_intact() {
        let f = fixture();
        let checked = capture_inputs(&f.record, &f.reference, &f.owned).unwrap();
        let original = validated(&f);
        let source_directory = original.temporary.0.clone();
        let shared = original.temporary.clone();
        let p = adopt_with_transfer(
            &f.record,
            &f.reference,
            &f.owned,
            &f.cache,
            original,
            &checked,
            |_| {},
            |source, target, _, report| {
                transfer_pcm(
                    source,
                    target,
                    true,
                    |_, _| Err(std::io::Error::from_raw_os_error(18)),
                    report,
                )
            },
        )
        .unwrap();
        assert!(source_directory.join("0.pcm").exists());
        assert_eq!(fs::read(&p.sets[0][0].1.path).unwrap(), vec![2; 256]);
        drop(shared);
        assert!(!source_directory.exists());
    }
    #[test]
    fn edits_before_or_during_adoption_preserve_previous_pointer_and_cleanup() {
        for edit_reference in [false, true] {
            let f = fixture();
            let checked = capture_inputs(&f.record, &f.reference, &f.owned).unwrap();
            let previous = adopt(
                &f.record,
                &f.reference,
                &f.owned,
                &f.cache,
                validated(&f),
                &checked,
                |_| {},
            )
            .unwrap();
            let pointer = fs::read(f.cache.join("cache.json")).unwrap();
            let target = if edit_reference {
                &f.reference
            } else {
                &f.owned.join("bass.wav")
            };
            let mut transfers = 0;
            let result = adopt_with_transfer(
                &f.record,
                &f.reference,
                &f.owned,
                &f.cache,
                validated(&f),
                &checked,
                |_| {},
                |source, destination, moving, report| {
                    transfer_pcm(source, destination, moving, |s, t| fs::rename(s, t), report)?;
                    transfers += 1;
                    if transfers == 1 {
                        fs::write(target, b"changed size while adopting PCM")?;
                    }
                    Ok(())
                },
            );
            assert!(result.is_err());
            assert_eq!(fs::read(f.cache.join("cache.json")).unwrap(), pointer);
            assert_eq!(fs::read_dir(&f.cache).unwrap().count(), 2);
            assert!(previous.reference.path.exists());
            assert!(
                adopt(
                    &f.record,
                    &f.reference,
                    &f.owned,
                    &f.cache,
                    validated(&f),
                    &checked,
                    |_| {}
                )
                .is_err()
            );
        }
    }
    #[test]
    fn edit_at_pointer_publication_and_transfer_error_roll_back_generation() {
        let f = fixture();
        let checked = capture_inputs(&f.record, &f.reference, &f.owned).unwrap();
        let previous = adopt(
            &f.record,
            &f.reference,
            &f.owned,
            &f.cache,
            validated(&f),
            &checked,
            |_| {},
        )
        .unwrap();
        let pointer = fs::read(f.cache.join("cache.json")).unwrap();
        let result = adopt_with_transfer(
            &f.record,
            &f.reference,
            &f.owned,
            &f.cache,
            validated(&f),
            &checked,
            |_| {},
            |_, target, _, _| {
                fs::write(target, [0; 8])?;
                anyhow::bail!("Injected transfer failure")
            },
        );
        assert!(result.is_err());
        assert_eq!(fs::read(f.cache.join("cache.json")).unwrap(), pointer);
        assert_eq!(fs::read_dir(&f.cache).unwrap().count(), 2);
        let result = adopt(
            &f.record,
            &f.reference,
            &f.owned,
            &f.cache,
            validated(&f),
            &checked,
            |p| {
                if p.step.as_deref() == Some("cache.save") && p.completed == 0 {
                    fs::write(&f.reference, b"edited immediately before publication").unwrap();
                }
            },
        );
        assert!(result.is_err());
        assert_eq!(fs::read(f.cache.join("cache.json")).unwrap(), pointer);
        assert_eq!(fs::read_dir(&f.cache).unwrap().count(), 2);
        assert!(previous.reference.path.exists());
    }
    #[test]
    fn filtering_projects_original_metadata_and_rejects_edits_without_refreshing() {
        for edit_reference in [false, true] {
            let f = fixture();
            let previous_checked = capture_inputs(&f.record, &f.reference, &f.owned).unwrap();
            let previous = adopt(
                &f.record,
                &f.reference,
                &f.owned,
                &f.cache,
                validated(&f),
                &previous_checked,
                |_| {},
            )
            .unwrap();
            let pointer = fs::read(f.cache.join("cache.json")).unwrap();
            let mut original = f.record.clone();
            let mut silent = original.tracks[0].clone();
            silent.role = "drums".into();
            silent.file = "drums.wav".into();
            original.tracks.push(silent);
            fs::write(f.owned.join("drums.wav"), b"silent track metadata").unwrap();
            let checked = capture_inputs(&original, &f.reference, &f.owned).unwrap();
            verify_inputs(&original, &f.reference, &f.owned, &checked).unwrap();
            let projected = checked.retained(&original, &f.record).unwrap();
            assert_eq!(projected, previous_checked);
            let target = if edit_reference {
                f.reference.clone()
            } else {
                f.owned.join("bass.wav")
            };
            fs::write(target, b"changed after validation before subset projection").unwrap();
            assert!(verify_inputs(&original, &f.reference, &f.owned, &checked).is_err());
            assert_eq!(checked.retained(&original, &f.record).unwrap(), projected);
            fs::remove_file(f.owned.join("drums.wav")).unwrap();
            assert!(
                adopt(
                    &f.record,
                    &f.reference,
                    &f.owned,
                    &f.cache,
                    validated(&f),
                    &projected,
                    |_| {}
                )
                .is_err()
            );
            assert_eq!(fs::read(f.cache.join("cache.json")).unwrap(), pointer);
            assert!(previous.reference.path.exists());
            let mut invalid = original.clone();
            invalid.tracks.reverse();
            assert!(checked.retained(&original, &invalid).is_err());
            invalid = original.clone();
            invalid.tracks.push(invalid.tracks[0].clone());
            assert!(checked.retained(&original, &invalid).is_err());
            invalid = f.record.clone();
            invalid.tracks[0].role = "vocals".into();
            assert!(checked.retained(&original, &invalid).is_err());
        }
    }
}
