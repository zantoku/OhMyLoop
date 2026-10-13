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
