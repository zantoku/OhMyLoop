//! Explicit shared installation of the pinned Roformer weights and configuration.
use super::{control::Control, disk};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};
pub const ID: &str = "roformer-sw-v1";
pub const PROFILE: &str = "roformer-sw-cpu2-v1";
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub name: String,
    pub bytes: u64,
    pub sha256: String,
    pub url: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    pub version: u32,
    pub id: String,
    pub profile: String,
    pub runtime_lock_sha256: String,
    pub files: Vec<Artifact>,
}
pub fn descriptor() -> Descriptor {
    serde_json::from_str(include_str!(
        "../../resources/separation/roformer-sw-v1.json"
    ))
    .expect("embedded descriptor")
}
impl Descriptor {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 1 && self.id == ID && self.profile == PROFILE && self.files.len() == 2,
            "Unsupported model package"
        );
        let mut names = std::collections::HashSet::new();
        for f in &self.files {
            ensure!(
                matches!(
                    f.name.as_str(),
                    "BS-Roformer-SW.ckpt" | "BS-Roformer-SW.yaml"
                ) && names.insert(&f.name)
                    && f.bytes > 0
                    && f.url.starts_with("https://")
                    && f.sha256.len() == 64
                    && f.sha256.bytes().all(|c| c.is_ascii_hexdigit()),
                "Invalid model artifact descriptor"
            );
        }
        Ok(())
    }
    pub fn bytes(&self) -> u64 {
        self.files.iter().map(|f| f.bytes).sum()
    }
    pub fn lock(&self) -> serde_json::Value {
        serde_json::json!({"version":1,"model":"roformer-sw","files":self.files.iter().map(|f|(f.name.clone(),serde_json::json!({"bytes":f.bytes,"sha256":f.sha256,"url":f.url}))).collect::<serde_json::Map<_,_>>()})
    }
}
pub fn path(root: &Path) -> PathBuf {
    root.join(ID)
}
fn safe(root: &Path, name: &str) -> Result<PathBuf> {
    let target = root.join(name);
    ensure!(
        !fs::symlink_metadata(root)?.file_type().is_symlink(),
        "Model directory must not be a symlink"
    );
    let m = fs::symlink_metadata(&target)?;
    ensure!(
        m.is_file() && !m.file_type().is_symlink(),
        "Unsafe model artifact"
    );
    Ok(target)
}
pub fn present(root: &Path) -> bool {
    let d = descriptor();
    d.files.iter().all(|f| {
        safe(root, &f.name)
            .ok()
            .and_then(|p| fs::metadata(p).ok())
            .is_some_and(|m| m.len() == f.bytes)
    }) && read_bounded(&root.join("package.json"), 1024 * 1024)
        .ok()
        .and_then(|b| serde_json::from_slice::<Descriptor>(&b).ok())
        .is_some_and(|v| serde_json::to_value(v).ok() == serde_json::to_value(d).ok())
}
pub fn digest(path: &Path, control: &Control, mut report: impl FnMut(u64, u64)) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let total = file.metadata()?.len();
    let mut hash = Sha256::new();
    let mut b = vec![0; 256 * 1024];
    let mut read = 0;
    loop {
        control.check()?;
        let n = file.read(&mut b)?;
        if n == 0 {
            break;
        }
        hash.update(&b[..n]);
        read += n as u64;
        report(read, total);
    }
    Ok(format!("{:x}", hash.finalize()))
}
pub fn verify(root: &Path, control: &Control, mut report: impl FnMut(u64, u64)) -> Result<()> {
    let d = descriptor();
    d.validate()?;
    let mut done = 0;
    for f in &d.files {
        let p = safe(root, &f.name)?;
        ensure!(
            fs::metadata(&p)?.len() == f.bytes,
            "Model size mismatch; reinstall the model"
        );
        let hash = digest(&p, control, |n, _| report(done + n, d.bytes()))?;
        ensure!(
            hash == f.sha256,
            "Model checksum mismatch; reinstall the model"
        );
        done += f.bytes;
    }
    Ok(())
}
/// A partial artifact owns its cleanup until checksum-verified publication.
struct PartialArtifact {
    partial: PathBuf,
    target: PathBuf,
    file: Option<fs::File>,
    hash: Sha256,
    bytes: u64,
}
impl PartialArtifact {
    fn new(target: &Path) -> Result<Self> {
        let partial = target.with_extension("partial");
        if partial.exists() {
            fs::remove_file(&partial)?;
        }
        Ok(Self {
            file: Some(
                fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&partial)?,
            ),
            partial,
            target: target.to_path_buf(),
            hash: Sha256::new(),
            bytes: 0,
        })
    }
    fn append(&mut self, block: &[u8], artifact: &Artifact, control: &Control) -> Result<()> {
        control.check()?;
        self.bytes = self
            .bytes
            .checked_add(block.len() as u64)
            .context("Download size overflow")?;
        ensure!(
            self.bytes <= artifact.bytes,
            "Model download exceeds declared size"
        );
        self.file
            .as_mut()
            .context("Download closed")?
            .write_all(block)?;
        self.hash.update(block);
        Ok(())
    }
    fn publish(mut self, artifact: &Artifact, control: &Control) -> Result<()> {
        ensure!(
            self.bytes == artifact.bytes
                && format!("{:x}", self.hash.clone().finalize()) == artifact.sha256,
            "Model download is incomplete or checksum mismatch"
        );
        self.file.as_ref().unwrap().sync_all()?;
        control.check()?;
        self.file.take();
        fs::rename(&self.partial, &self.target)?;
        Ok(())
    }
}
impl Drop for PartialArtifact {
    fn drop(&mut self) {
        self.file.take();
        let _ = fs::remove_file(&self.partial);
    }
}
#[cfg(test)]
fn receive(
    mut stream: impl Read,
    target: &Path,
    artifact: &Artifact,
    control: &Control,
    mut report: impl FnMut(u64),
) -> Result<()> {
    let mut partial = PartialArtifact::new(target)?;
    let mut b = vec![0; 256 * 1024];
    loop {
        control.check()?;
        let n = stream.read(&mut b)?;
        if n == 0 {
            break;
        }
        partial.append(&b[..n], artifact, control)?;
        report(partial.bytes);
    }
    partial.publish(artifact, control)
}

fn install_with(
    root: &Path,
    d: &Descriptor,
    control: &Control,
    mut report: impl FnMut(u64, u64),
    mut fetch: impl FnMut(&Artifact, &Path, &Control, &mut dyn FnMut(u64)) -> Result<()>,
) -> Result<()> {
    d.validate()?;
    fs::create_dir_all(root)?;
    ensure!(
        !fs::symlink_metadata(root)?.file_type().is_symlink(),
        "Unsafe model directory"
    );
    let mut complete = std::collections::HashSet::new();
    let mut done = 0;
    for f in &d.files {
        control.check()?;
        if safe(root, &f.name)
            .ok()
            .is_some_and(|p| fs::metadata(&p).is_ok_and(|m| m.len() == f.bytes))
            && digest(&root.join(&f.name), control, |_, _| {}).is_ok_and(|h| h == f.sha256)
        {
            complete.insert(f.name.clone());
            done += f.bytes;
        }
    }
    disk::admit(
        root,
        d.bytes()
            .checked_sub(done)
            .context("Package size overflow")?
            + 1024 * 1024,
    )?;
    report(done, d.bytes());
    for f in &d.files {
        control.check()?;
        if complete.contains(&f.name) {
            continue;
        }
        let target = root.join(&f.name);
        if target.exists() {
            fs::remove_file(&target)?;
        }
        if let Err(error) = fetch(f, &target, control, &mut |n| report(done + n, d.bytes())) {
            let partial = target.with_extension("partial");
            if let Err(cleanup) = fs::remove_file(&partial)
                && cleanup.kind() != std::io::ErrorKind::NotFound
            {
                return Err(error.context(format!(
                    "Partial model cleanup failed at {}: {cleanup}",
                    partial.display()
                )));
            }
            return Err(error);
        }
        done += f.bytes;
    }
    control.check()?;
    crate::storage::atomic_write(&root.join("package.json"), &serde_json::to_vec(d)?)?;
    Ok(())
}
pub fn install(root: &Path, control: &Control, report: impl FnMut(u64, u64)) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(5))
        .build()?;
    install_with(
        root,
        &descriptor(),
        control,
        report,
        |f, target, control, report| {
            runtime.block_on(async {
                let mut request = Box::pin(client.get(&f.url).send());
                let mut response = loop {
                    control.check()?;
                    if let Ok(result) =
                        tokio::time::timeout(Duration::from_millis(25), request.as_mut()).await
                    {
                        break result?.error_for_status()?;
                    }
                };
                if let Some(bytes) = response.content_length() {
                    ensure!(bytes == f.bytes, "Unexpected model download size");
                }
                let mut partial = PartialArtifact::new(target)?;
                loop {
                    control.check()?;
                    let mut next = Box::pin(response.chunk());
                    let block = loop {
                        control.check()?;
                        if let Ok(result) =
                            tokio::time::timeout(Duration::from_millis(25), next.as_mut()).await
                        {
                            break result?;
                        }
                    };
                    let Some(block) = block else {
                        break;
                    };
                    partial.append(&block, f, control)?;
                    report(partial.bytes);
                }
                partial.publish(f, control)
            })
        },
    )
}

pub fn remove(root: &Path) -> Result<()> {
    if root.exists() {
        ensure!(
            !fs::symlink_metadata(root)?.file_type().is_symlink(),
            "Unsafe model directory"
        );
        fs::remove_dir_all(root)?;
    }
    Ok(())
}
pub fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    crate::storage::read_limited(path, limit)
}

pub fn snapshot(root: &Path) -> Result<Vec<(u64, std::time::SystemTime)>> {
    descriptor()
        .files
        .iter()
        .map(|f| {
            let m = fs::metadata(safe(root, &f.name)?)?;
            Ok((m.len(), m.modified()?))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streaming_bounds_checksum_and_cancel() {
        let root = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir(&root).unwrap();
        let bytes = b"fixture";
        let a = Artifact {
            name: "fixture".into(),
            bytes: 7,
            sha256: format!("{:x}", Sha256::digest(bytes)),
            url: "https://example.invalid".into(),
        };
        let p = root.join("fixture");
        receive(&bytes[..], &p, &a, &Control::default(), |_| {}).unwrap();
        assert_eq!(fs::read(&p).unwrap(), bytes);
        fs::remove_file(&p).unwrap();
        assert!(receive(&b"toolong!"[..], &p, &a, &Control::default(), |_| {}).is_err());
        assert!(!p.with_extension("partial").exists());
        assert!(receive(&b"bad"[..], &p, &a, &Control::default(), |_| {}).is_err());
        let c = Control::default();
        c.cancel();
        assert!(receive(&bytes[..], &p, &a, &c, |_| {}).is_err());
        assert!(!p.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_install_retry_preserves_verified_files_and_ready_install_is_offline() {
        let root = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        let mut d = descriptor();
        for f in &mut d.files {
            f.bytes = 7;
            f.sha256 = format!("{:x}", Sha256::digest(b"fixture"));
        }
        let control = Control::default();
        let mut attempts = Vec::new();
        assert!(
            install_with(
                &root,
                &d,
                &control,
                |_, _| {},
                |a, p, c, report| {
                    attempts.push(a.name.clone());
                    if a.name.ends_with("yaml") {
                        return Err(anyhow::anyhow!("Offline"));
                    }
                    receive(&b"fixture"[..], p, a, c, report)
                }
            )
            .is_err()
        );
        assert!(root.join("BS-Roformer-SW.ckpt").is_file());
        assert!(!root.join("package.json").exists());
        attempts.clear();
        install_with(
            &root,
            &d,
            &control,
            |_, _| {},
            |a, p, c, report| {
                attempts.push(a.name.clone());
                receive(&b"fixture"[..], p, a, c, report)
            },
        )
        .unwrap();
        assert_eq!(attempts, vec!["BS-Roformer-SW.yaml"]);
        install_with(
            &root,
            &d,
            &control,
            |_, _| {},
            |_, _, _, _| panic!("Verified package must remain available offline"),
        )
        .unwrap();
        assert!(root.join("package.json").is_file());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn descriptor_is_pinned() {
        descriptor().validate().unwrap();
        assert_eq!(descriptor().bytes(), 699416805);
    }
}
