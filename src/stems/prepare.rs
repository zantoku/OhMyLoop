use super::{MixState, Pcm, PcmReader, Role, SourceFactory, StemSource, TemporaryDirectory};
use anyhow::{Context, Result, ensure};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufWriter, ErrorKind, Read, Write},
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use symphonia::core::{
    audio::SampleBuffer, codecs::DecoderOptions, errors::Error, formats::FormatOptions,
    io::MediaSourceStream, meta::MetadataOptions, probe::Hint,
};

/// Progress is byte/frame work within the named phase, not a guessed ETA.
#[derive(Clone, Debug)]
pub struct PreparationProgress {
    pub phase: String,
    pub completed: u64,
    pub total: Option<u64>,
}

pub struct PreparedComparison {
    pub reference_rate: u32,
    pub reference_frames: u64,
    pub output_rate: u32,
    pub frames: u64,
    pub shared_attenuation: f32,
    pub temp_bytes: u64,
    pub elapsed: Duration,
    reference: Pcm,
    sets: Vec<Vec<(Role, Pcm)>>,
    temporary: Arc<TemporaryDirectory>,
}

impl PreparedComparison {
    pub fn prepare(
        reference: &Path,
        sets: &[PathBuf],
        output_rate: u32,
        mut progress: impl FnMut(PreparationProgress),
    ) -> Result<Self> {
        ensure!(sets.len() <= 2, "At most two stem sets are supported");
        ensure!(
            (8_000..=192_000).contains(&output_rate),
            "Unsupported output rate"
        );
        let started = Instant::now();
        let directory =
            std::env::temp_dir().join(format!("ohmyloop-stems-{}", uuid::Uuid::new_v4()));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&directory)?;
        let temporary = Arc::new(TemporaryDirectory(directory.clone()));
        let mut preparer = Preparer {
            directory,
            next: 0,
            progress: &mut progress,
        };
        let reference_hash = preparer.digest(reference)?;
        let decoded = preparer.decode(reference)?;
        let reference_rate = decoded.rate;
        let reference_frames = decoded.frames;
        let frames = coordinate_frames(reference_frames, output_rate, reference_rate)?;
        let reference = preparer.resample(decoded, output_rate, frames)?;
        let mut banks = Vec::new();
        for set in sets {
            let manifest_path = set.join("manifest.json");
            ensure!(
                fs::metadata(&manifest_path)?.len() <= 4 * 1024 * 1024,
                "Manifest exceeds 4 MiB limit"
            );
            let bytes = fs::read(&manifest_path)?;
            let manifest: Manifest =
                serde_json::from_slice(&bytes).context("Invalid prototype manifest")?;
            let raw: serde_json::Value = serde_json::from_slice(&bytes)?;
            ensure!(
                manifest.version == 1 && manifest.status == "complete",
                "Unsupported or incomplete prototype manifest"
            );
            let identity = raw.get("identity").context("Missing manifest identity")?;
            ensure!(
                format!("{:x}", Sha256::digest(serde_json::to_vec(identity)?)) == manifest.job_key,
                "Invalid manifest job identity"
            );
            let preparation = manifest.identity.preparation;
            ensure!(
                preparation.version == 1 && preparation.channels == 2,
                "Unsupported prototype preparation"
            );
            ensure!(
                preparation.source_sha256 == reference_hash,
                "Stem set reference digest mismatch"
            );
            ensure!(
                preparation.source_rate == reference_rate
                    && preparation.source_frames == reference_frames,
                "Stem set reference geometry mismatch"
            );
            ensure!(
                preparation.source_span.start == 0
                    && preparation.source_span.end_exclusive == reference_frames,
                "Stem set must cover the entire reference from zero"
            );
            ensure!(
                (8_000..=192_000).contains(&preparation.prepared_rate),
                "Unsupported prepared sample rate"
            );
            ensure!(
                preparation.prepared_frames
                    == coordinate_frames(
                        reference_frames,
                        preparation.prepared_rate,
                        reference_rate
                    )?,
                "Prepared length disagrees with declared coordinate mapping"
            );
            let prepared_path = safe_file(set, "prepared.wav")?;
            ensure!(
                preparer.digest(&prepared_path)? == preparation.prepared_sha256,
                "Prepared input hash mismatch"
            );
            let prepared_pcm = preparer.decode(&prepared_path)?;
            ensure!(
                prepared_pcm.rate == preparation.prepared_rate
                    && prepared_pcm.frames == preparation.prepared_frames
                    && prepared_pcm.channels == 2,
                "Prepared input geometry mismatch"
            );
            fs::remove_file(&prepared_pcm.path)?;
            ensure!(
                !manifest.stems.is_empty() && manifest.stems.len() <= 6,
                "Stem set must have one to six supported roles"
            );
            let mut bank = Vec::new();
            for (name, track) in manifest.stems {
                let role = Role::parse(&name)?;
                ensure!(
                    track.frames == preparation.prepared_frames
                        && track.rate == preparation.prepared_rate
                        && track.channels == 2,
                    "Declared geometry mismatch for {name}"
                );
                let path = safe_file(set, &track.file)?;
                ensure!(
                    preparer.digest(&path)? == track.sha256,
                    "Stem hash mismatch: {name}"
                );
                let pcm = preparer.decode(&path)?;
                ensure!(
                    pcm.rate == track.rate
                        && pcm.frames == track.frames
                        && pcm.channels == track.channels,
                    "Decoded geometry mismatch: {name}"
                );
                bank.push((role, preparer.resample(pcm, output_rate, frames)?));
            }
            banks.push(bank);
        }
        let peak = preparer.peak(&reference, &banks)?;
        // 0.70 reserves 0.30 for fixed-pitch metronome/count-in and DSP
        // overshoot. This is a pre-DSP bound, not a guarantee of no overshoot.
        let shared_attenuation = if peak > 0.0 {
            (0.70 / peak).min(1.0) as f32
        } else {
            1.0
        };
        let temp_bytes = fs::read_dir(&preparer.directory)?.try_fold(0u64, |sum, entry| {
            Ok::<_, std::io::Error>(sum + entry?.metadata()?.len())
        })?;
        Ok(Self {
            reference_rate,
            reference_frames,
            output_rate,
            frames,
            shared_attenuation,
            temp_bytes,
            elapsed: started.elapsed(),
            reference,
            sets: banks,
            temporary,
        })
    }
    pub fn source_count(&self) -> usize {
        self.sets.len() + 1
    }
    pub fn source_roles(&self, index: usize) -> Result<Vec<Role>> {
        ensure!(index <= self.sets.len(), "Source index out of range");
        Ok(if index == 0 {
            vec![]
        } else {
            self.sets[index - 1].iter().map(|(r, _)| *r).collect()
        })
    }
    pub fn reader(&self) -> Result<StemSource> {
        let sets = self
            .sets
            .iter()
            .map(|bank| {
                bank.iter()
                    .map(|(role, pcm)| Ok((*role, PcmReader::new(pcm.clone())?)))
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(StemSource {
            reference: PcmReader::new(self.reference.clone())?,
            sets,
            selection: 0,
            mix: MixState::default(),
            attenuation: self.shared_attenuation,
            scratch: vec![[0.0; 2]; 8192],
            _temporary: Arc::clone(&self.temporary),
        })
    }
}
impl SourceFactory for PreparedComparison {
    fn reference_geometry(&self) -> (u32, u64) {
        (self.reference_rate, self.reference_frames)
    }
    fn rate(&self) -> u32 {
        self.output_rate
    }
    fn len_frames(&self) -> u64 {
        self.frames
    }
    fn open(&self, selection: usize, mix: MixState) -> Result<Box<dyn super::BlockSource>> {
        let mut reader = self.reader()?;
        reader.select_source(selection)?;
        reader.set_mix(mix)?;
        Ok(Box::new(reader))
    }
}

/// The prototype declares floor(source_frames * destination_rate/source_rate).
/// Device PCM uses the same direct reference mapping, avoiding double rounding.
pub fn coordinate_frames(frames: u64, to: u32, from: u32) -> Result<u64> {
    ensure!(from > 0 && to > 0, "Invalid sample rate");
    let value = (frames as u128 * to as u128 / from as u128)
        .try_into()
        .context("Frame conversion overflow")?;
    ensure!(value > 0, "Audio duration is too short");
    Ok(value)
}

#[derive(Deserialize)]
struct Manifest {
    version: u32,
    status: String,
    job_key: String,
    identity: Identity,
    #[serde(deserialize_with = "unique_tracks")]
    stems: BTreeMap<String, Track>,
}
#[derive(Deserialize)]
struct Identity {
    preparation: Preparation,
}
#[derive(Deserialize)]
struct Preparation {
    version: u32,
    channels: u32,
    source_sha256: String,
    source_rate: u32,
    source_frames: u64,
    source_span: Span,
    prepared_rate: u32,
    prepared_frames: u64,
    prepared_sha256: String,
}
#[derive(Deserialize)]
struct Span {
    start: u64,
    end_exclusive: u64,
}
#[derive(Deserialize)]
struct Track {
    file: String,
    sha256: String,
    frames: u64,
    rate: u32,
    channels: u32,
}

fn unique_tracks<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<BTreeMap<String, Track>, D::Error> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = BTreeMap<String, Track>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("unique stem role keys")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> std::result::Result<Self::Value, M::Error> {
            let mut tracks = BTreeMap::new();
            while let Some((role, track)) = map.next_entry::<String, Track>()? {
                if tracks.insert(role, track).is_some() {
                    return Err(serde::de::Error::custom("Duplicate stem role"));
                }
            }
            Ok(tracks)
        }
    }
    deserializer.deserialize_map(Visitor)
}

fn safe_file(root: &Path, name: &str) -> Result<PathBuf> {
    let relative = Path::new(name);
    ensure!(
        !name.is_empty()
            && !name.contains('\\')
            && relative
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
        "Unsafe manifest track path"
    );
    let mut path = root.to_path_buf();
    for component in relative.components() {
        path.push(component);
        ensure!(
            !fs::symlink_metadata(&path)?.file_type().is_symlink(),
            "Manifest paths must not contain symlinks"
        );
    }
    ensure!(path.is_file(), "Missing manifest file");
    Ok(path)
}

struct Preparer<'a, F: FnMut(PreparationProgress)> {
    directory: PathBuf,
    next: usize,
    progress: &'a mut F,
}
impl<F: FnMut(PreparationProgress)> Preparer<'_, F> {
    fn path(&mut self) -> PathBuf {
        let path = self.directory.join(format!("{}.pcm", self.next));
        self.next += 1;
        path
    }
    fn notify(&mut self, phase: impl Into<String>, completed: u64, total: Option<u64>) {
        (self.progress)(PreparationProgress {
            phase: phase.into(),
            completed,
            total,
        })
    }
    fn digest(&mut self, path: &Path) -> Result<String> {
        let mut file = File::open(path)?;
        let size = file.metadata()?.len();
        let mut hash = Sha256::new();
        let mut buffer = vec![0; 256 * 1024];
        let mut read = 0;
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hash.update(&buffer[..n]);
            read += n as u64;
            self.notify(format!("Hash {}", path.display()), read, Some(size));
        }
        Ok(format!("{:x}", hash.finalize()))
    }
    fn decode(&mut self, path: &Path) -> Result<Pcm> {
        let stream = MediaSourceStream::new(Box::new(File::open(path)?), Default::default());
        let mut hint = Hint::new();
        if let Some(extension) = path.extension().and_then(|x| x.to_str()) {
            hint.with_extension(extension);
        }
        let mut format = symphonia::default::get_probe()
            .format(
                &hint,
                stream,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )?
            .format;
        let track = format.default_track().context("No default audio track")?;
        let id = track.id;
        let expected_frames = track.codec_params.n_frames;
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())?;
        let output = self.path();
        let mut writer = BufWriter::new(File::create(&output)?);
        let mut frames = 0u64;
        let mut rate = None;
        let mut input_channels = None;
        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(Error::IoError(e)) if e.kind() == ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            };
            if packet.track_id() != id {
                continue;
            }
            let decoded = decoder.decode(&packet)?;
            let spec = *decoded.spec();
            ensure!(
                rate.is_none_or(|r| r == spec.rate),
                "Sample rate changed mid-file"
            );
            rate = Some(spec.rate);
            let channels = spec.channels.count();
            ensure!(
                channels == 1 || channels == 2,
                "Only mono/stereo audio is supported"
            );
            ensure!(
                input_channels.is_none_or(|c| c == channels),
                "Channel layout changed mid-file"
            );
            input_channels = Some(channels);
            ensure!(
                decoded.capacity() <= 1_048_576,
                "Decoder packet exceeds bounded preparation limit"
            );
            let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
            buffer.copy_interleaved_ref(decoded);
            for sample in buffer.samples().chunks_exact(channels) {
                ensure!(
                    sample.iter().all(|x| x.is_finite()),
                    "Non-finite decoded sample"
                );
                writer.write_all(&sample[0].to_le_bytes())?;
                writer.write_all(&sample[channels - 1].to_le_bytes())?;
                frames += 1;
            }
            self.notify(
                format!("Decode {}", path.display()),
                frames,
                expected_frames,
            );
        }
        writer.flush()?;
        ensure!(frames > 0, "Empty decoded audio");
        // WAV declares an exact PCM frame count. Compressed containers may
        // report codec padding or estimates, so their decoded count is primary.
        if path
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("wav"))
        {
            ensure!(
                expected_frames.is_none_or(|n| n == frames),
                "Truncated WAV input"
            );
        }
        let rate = rate.context("No decoded sample rate")?;
        ensure!(
            (8_000..=192_000).contains(&rate),
            "Unsupported input sample rate"
        );
        Ok(Pcm {
            path: output,
            rate,
            frames,
            channels: input_channels.context("Missing channel layout")? as u32,
        })
    }
    fn resample(&mut self, pcm: Pcm, rate: u32, count: u64) -> Result<Pcm> {
        if pcm.rate == rate {
            ensure!(pcm.frames == count, "PCM coordinate geometry mismatch");
            return Ok(pcm);
        }
        let mut converter = SincFixedIn::<f32>::new(
            rate as f64 / pcm.rate as f64,
            1.0,
            SincInterpolationParameters {
                sinc_len: 128,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Cubic,
                oversampling_factor: 128,
                window: WindowFunction::BlackmanHarris2,
            },
            1024,
            2,
        )?;
        let mut discard = converter.output_delay();
        // SincFixedIn's first convolution is centered on the first input
        // sample (its internal negative start index already advances past
        // half the kernel). Provide explicit half-kernel zero pre-roll before
        // discarding output_delay; otherwise discarding alone shifts the
        // recording earlier by ~64 input frames. Impulse tests guard this.
        let pre_roll = 64u64;
        let mut input = vec![vec![0.0; 1024]; 2];
        let mut interleaved = vec![[0.0; 2]; 1024];
        let mut reader = PcmReader::new(pcm.clone())?;
        let path = self.path();
        let mut writer = BufWriter::new(File::create(&path)?);
        let mut cursor = 0u64;
        let mut written = 0u64;
        while written < count {
            interleaved.fill([0.0; 2]);
            let leading = pre_roll.saturating_sub(cursor).min(1024) as usize;
            reader.read(cursor.saturating_sub(pre_roll), &mut interleaved[leading..])?;
            cursor += 1024;
            for (channel, plane) in input.iter_mut().enumerate() {
                for (sample, frame) in plane.iter_mut().zip(&interleaved) {
                    *sample = frame[channel];
                }
            }
            let output = converter.process(&input, None)?;
            for (&left, &right) in output[0].iter().zip(&output[1]) {
                if discard > 0 {
                    discard -= 1;
                    continue;
                }
                if written == count {
                    break;
                }
                ensure!(
                    left.is_finite() && right.is_finite(),
                    "Non-finite resampler output"
                );
                writer.write_all(&left.to_le_bytes())?;
                writer.write_all(&right.to_le_bytes())?;
                written += 1;
            }
            self.notify("Resample (delay compensated)", written, Some(count));
        }
        writer.flush()?;
        drop(reader);
        fs::remove_file(pcm.path)?;
        Ok(Pcm {
            path,
            rate,
            frames: count,
            channels: pcm.channels,
        })
    }
    fn peak(&mut self, reference: &Pcm, sets: &[Vec<(Role, Pcm)>]) -> Result<f64> {
        let mut peak = 0.0f64;
        let mut buffer = vec![[0.0; 2]; 8192];
        let mut reader = PcmReader::new(reference.clone())?;
        let mut pos = 0;
        while pos < reference.frames {
            let n = reader.read(pos, &mut buffer)?;
            for x in buffer[..n].iter().flatten() {
                peak = peak.max(x.abs() as f64)
            }
            pos += n as u64;
            self.notify("Scan reference headroom", pos, Some(reference.frames));
        }
        for bank in sets {
            let mut readers = bank
                .iter()
                .map(|(_, pcm)| PcmReader::new(pcm.clone()))
                .collect::<Result<Vec<_>>>()?;
            let mut sums = vec![[0.0f64; 2]; 8192];
            let mut pos = 0;
            while pos < reference.frames {
                let n = reference.frames.saturating_sub(pos).min(8192) as usize;
                sums[..n].fill([0.0; 2]);
                for reader in &mut readers {
                    ensure!(
                        reader.read(pos, &mut buffer[..n])? == n,
                        "Unexpected prepared EOF"
                    );
                    for (sum, frame) in sums.iter_mut().zip(&buffer[..n]) {
                        sum[0] += frame[0].abs() as f64;
                        sum[1] += frame[1].abs() as f64;
                    }
                }
                for x in sums[..n].iter().flatten() {
                    peak = peak.max(*x)
                }
                pos += n as u64;
                self.notify(
                    "Scan stem absolute-sum headroom",
                    pos,
                    Some(reference.frames),
                );
            }
        }
        Ok(peak)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stems::BlockSource;

    #[test]
    fn prepared_temporary_lifetime_and_read_failures() {
        let input =
            std::env::temp_dir().join(format!("ohmyloop-source-{}.wav", uuid::Uuid::new_v4()));
        crate::audio::write_wav(&input, 44_100, &[[0.1; 2]; 5000]).unwrap();
        let prepared = PreparedComparison::prepare(&input, &[], 44_100, |_| {}).unwrap();
        let directory = prepared.temporary.0.clone();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        let mut source = prepared.reader().unwrap();
        // Read failure is observable through the block seam, never substituted.
        File::options()
            .write(true)
            .open(&prepared.reference.path)
            .unwrap()
            .set_len(8)
            .unwrap();
        assert!(source.read_frames(100, &mut [[0.0; 2]; 16]).is_err());
        drop(prepared);
        assert!(directory.exists());
        drop(source);
        assert!(!directory.exists());
        fs::remove_file(input).unwrap();
    }
}
