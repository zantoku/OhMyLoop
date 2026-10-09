use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
pub const MODEL_ID: &str = "beat-this-final0-onnx-5f810deb-v1";
pub const MEL_HASH: &str = "fdd59e65c515331308e4c8841edf99972deca646bdf6197744c2a5b7755e3de9";
pub const BEAT_HASH: &str = "5f810debe53459b559127fb55bbad40035bb47cc567b20e501670f968c770f02";
const MEL_URL: &str = "https://raw.githubusercontent.com/danigb/beat-this-rs/1ae768e78f1ad83b0ed3886241dc29ffde853c40/models/mel_spectrogram.onnx";
const BEAT_URL: &str =
    "https://github.com/danigb/beat-this-rs/releases/download/model-large/beat_this.onnx";
pub fn verify(path: &Path, expected: &str) -> Result<()> {
    let mut file = File::open(path).context(
        "Analysis model is missing. Install the model or select an installed-model folder.",
    )?;
    ensure!(
        file.metadata()?.len() <= 90_000_000,
        "Model exceeds expected size"
    );
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    ensure!(
        format!("{:x}", hash.finalize()) == expected,
        "Analysis model checksum does not match. Reinstall the pinned model; your map is unchanged."
    );
    Ok(())
}
pub fn present(directory: &Path) -> bool {
    directory.join("mel_spectrogram.onnx").is_file() && directory.join("beat_this.onnx").is_file()
}
pub fn verify_models(directory: &Path) -> Result<()> {
    verify(&directory.join("mel_spectrogram.onnx"), MEL_HASH)?;
    verify(&directory.join("beat_this.onnx"), BEAT_HASH)
}
pub fn install(
    directory: &Path,
    cancelled: &AtomicBool,
    mut phase: impl FnMut(&str),
) -> Result<()> {
    fs::create_dir_all(directory)?;
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(180))
        .build()?;
    for (name, url, hash) in [
        ("mel_spectrogram.onnx", MEL_URL, MEL_HASH),
        ("beat_this.onnx", BEAT_URL, BEAT_HASH),
    ] {
        ensure!(
            !cancelled.load(Ordering::Acquire),
            "Model installation cancelled"
        );
        let destination = directory.join(name);
        if verify(&destination, hash).is_ok() {
            continue;
        }
        phase(if name.starts_with("mel") {
            "Downloading audio frontend"
        } else {
            "Downloading beat model (about 83 MB)"
        });
        let temporary = directory.join(format!("{name}.download-{}", std::process::id()));
        let result = (|| -> Result<()> {
            let mut response = client.get(url).send()?.error_for_status()?;
            let mut file = File::create(&temporary)?;
            let mut buffer = [0_u8; 65536];
            let mut size = 0_u64;
            loop {
                ensure!(
                    !cancelled.load(Ordering::Acquire),
                    "Model installation cancelled"
                );
                let read = response.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                size += read as u64;
                ensure!(size <= 90_000_000, "Model download exceeded expected size");
                file.write_all(&buffer[..read])?;
            }
            file.sync_all()?;
            drop(file);
            verify(&temporary, hash)?;
            ensure!(
                !cancelled.load(Ordering::Acquire),
                "Model installation cancelled"
            );
            fs::rename(&temporary, &destination)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result?;
    }
    fs::write(
        directory.join("LICENSE.txt"),
        include_str!("../../licenses/beat-this-MIT.txt"),
    )?;
    fs::write(
        directory.join("PROVENANCE.json"),
        include_str!("../../config/beat-model.json"),
    )?;
    verify_models(directory)
}
