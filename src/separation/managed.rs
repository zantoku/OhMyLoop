//! App-owned managed runtime recipes, installation and completion records.
use super::{command, control::Control, disk, package, runtime::Runtime};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
    time::Duration,
};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MinimumOs {
    pub family: String,
    pub version: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Definition {
    pub schema_version: u32,
    pub id: String,
    pub target: String,
    pub python_version: String,
    pub python_key: String,
    pub python_installation: String,
    pub python_build: String,
    pub python_executable: String,
    pub python_downloads: String,
    pub requirements: String,
    pub uv: String,
    pub uv_version: String,
    pub ffmpeg: String,
    pub ffmpeg_version: String,
    pub artifact_manifest: String,
    pub minimum_os: MinimumOs,
    pub download_bytes: u64,
    pub installed_bytes: u64,
    pub temporary_bytes: u64,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Completion {
    schema_version: u32,
    id: String,
    recipe_sha256: String,
}
#[derive(Clone)]
pub(super) struct Managed {
    pub root: PathBuf,
    pub resources: PathBuf,
    pub definition: Definition,
    recipe_sha256: String,
}
pub(super) const STAGES: [(&str, &str); 6] = [
    ("runtime.python", "Download private Python"),
    ("runtime.environment", "Create private environment"),
    ("runtime.dependencies", "Install separation tools"),
    ("runtime.check", "Check separation tools"),
    ("runtime.cache", "Finish tools setup"),
    ("model.download", "Download model"),
];
pub(super) fn target() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Some("aarch64-apple-darwin"),
        ("linux", "x86_64") => Some("x86_64-unknown-linux-gnu"),
        ("windows", "x86_64") => Some("x86_64-pc-windows-msvc"),
        _ => None,
    }
}
fn relative(value: &str) -> Result<&Path> {
    let path = Path::new(value);
    ensure!(
        !value.is_empty() && path.components().all(|c| matches!(c, Component::Normal(_))),
        "Unsafe runtime recipe path"
    );
    Ok(path)
}
fn normalized_root(root: &Path) -> Result<PathBuf> {
    ensure!(root.is_absolute(), "Tools storage must be absolute");
    let mut parent = root.parent().context("Missing tools storage parent")?;
    let mut suffix = vec![
        root.file_name()
            .context("Missing tools storage name")?
            .to_os_string(),
    ];
    while !parent.exists() {
        suffix.push(
            parent
                .file_name()
                .context("Missing tools storage ancestor")?
                .to_os_string(),
        );
        parent = parent.parent().context("Missing tools storage ancestor")?;
    }
    // Resolve OS aliases such as /var -> /private/var at the trusted app-data
    // boundary. Children owned by this service are checked independently below.
    let mut normalized = parent.canonicalize()?;
    for component in suffix.into_iter().rev() {
        normalized.push(component);
    }
    ensure_no_symlinks(&normalized)?;
    Ok(normalized)
}
impl Managed {
    pub fn load(resources: &Path, root: PathBuf) -> Result<Self> {
        let target =
            target().context("Stem separation is not supported on this OS/architecture")?;
        let resources = resources.join("runtime").join(target);
        let resources = resources.canonicalize()?;
        let root = normalized_root(&root)?;
        let bytes = package::read_bounded(&resources.join("definition.json"), 1024 * 1024)?;
        let mut definition: Definition = serde_json::from_slice(&bytes)?;
        ensure!(
            definition.schema_version == 1
                && definition.target == target
                && definition.python_version == "3.12.13",
            "Unsupported separation runtime definition"
        );
        ensure!(
            definition.id.starts_with("roformer-sw-cpu2-v1-")
                && definition
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
            "Unsafe runtime identity"
        );
        for value in [
            &definition.python_key,
            &definition.python_installation,
            &definition.python_executable,
            &definition.python_downloads,
            &definition.requirements,
            &definition.uv,
            &definition.ffmpeg,
            &definition.artifact_manifest,
        ] {
            relative(value)?;
        }
        ensure!(
            !definition.python_build.is_empty()
                && definition.temporary_bytes >= definition.installed_bytes,
            "Invalid runtime build or disk estimate"
        );
        check_os(&definition.minimum_os)?;
        let mut hash = Sha256::new();
        hash.update(&bytes);
        for path in [
            &definition.requirements,
            &definition.python_downloads,
            &definition.artifact_manifest,
        ] {
            hash.update(package::read_bounded(
                &resources.join(path),
                4 * 1024 * 1024,
            )?);
        }
        hash.update(package::read_bounded(
            &resources
                .parent()
                .context("Missing runtime resources")?
                .parent()
                .context("Missing separation resources")?
                .join("worker.py"),
            1024 * 1024,
        )?);
        let recipe_sha256 = format!("{:x}", hash.finalize());
        // The installation directory identifies the exact recipe, not merely
        // the model/profile name. Even a changed wheel or worker gets a new
        // final path, preserving the prior installation until verification.
        definition.id = effective_id(&definition.id, &recipe_sha256);
        Ok(Self {
            root,
            resources,
            definition,
            recipe_sha256,
        })
    }
    pub fn environment(&self) -> PathBuf {
        self.root.join("runtimes").join(&self.definition.id)
    }
    pub fn python_root(&self) -> PathBuf {
        self.root.join("python").join(&self.definition.id)
    }
    pub fn python(&self) -> PathBuf {
        self.environment().join(if cfg!(windows) {
            "Scripts/python.exe"
        } else {
            "bin/python"
        })
    }
    pub fn cache(&self) -> PathBuf {
        self.root.join("cache").join(&self.definition.id)
    }
    pub fn ffmpeg(&self) -> PathBuf {
        self.resources.join(&self.definition.ffmpeg)
    }
    pub fn ready(&self) -> bool {
        self.python().is_file()
            && self.ffmpeg().is_file()
            && package::read_bounded(&self.environment().join("complete.json"), 4096)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Completion>(&bytes).ok())
                .is_some_and(|c| {
                    c.schema_version == 1
                        && c.id == self.definition.id
                        && c.recipe_sha256 == self.recipe_sha256
                })
    }
    fn uv(&self) -> Command {
        let mut cmd = Command::new(self.resources.join(&self.definition.uv));
        controlled_environment(&mut cmd, &self.ffmpeg(), &self.root);
        cmd.env("UV_NO_CONFIG", "1")
            .env("UV_NO_PROGRESS", "1")
            .env("UV_LINK_MODE", "copy")
            .env("UV_CACHE_DIR", self.cache())
            .env("UV_PYTHON_INSTALL_DIR", self.python_root())
            .env("UV_PYTHON_PREFERENCE", "only-managed")
            .env("UV_PYTHON_DOWNLOADS", "never")
            .env("UV_NO_BUILD", "1");
        cmd
    }
    pub fn install(
        &self,
        runtime: &Runtime,
        control: &Control,
        mut stage: impl FnMut(&str),
    ) -> Result<()> {
        if self.ready() {
            return Ok(());
        }
        self.ensure_owned_root()?;
        disk::admit(&self.root, self.definition.temporary_bytes)?;
        self.remove_environment()?;
        fs::create_dir_all(self.environment())?;
        let mut published = false;
        let result = (|| {
            stage("runtime.python");
            let metadata = reqwest::Url::from_file_path(
                self.resources.join(&self.definition.python_downloads),
            )
            .map_err(|()| anyhow::anyhow!("Invalid Python recipe path"))?;
            command::run(
                self.uv()
                    .args([
                        "python",
                        "install",
                        "--no-bin",
                        "--no-registry",
                        "--python-downloads-json-url",
                    ])
                    .arg(metadata.as_str())
                    .arg(&self.definition.python_key)
                    .env("UV_PYTHON_DOWNLOADS", "automatic"),
                control,
                Duration::from_secs(30 * 60),
            )?;
            control.check()?;
            stage("runtime.environment");
            let python = self
                .python_root()
                .join(&self.definition.python_installation)
                .join(&self.definition.python_executable);
            ensure!(
                python.is_file(),
                "Managed interpreter was not installed at its pinned location"
            );
            command::run(
                self.uv()
                    .args(["venv", "--python"])
                    .arg(python)
                    .arg(self.environment()),
                control,
                Duration::from_secs(120),
            )?;
            stage("runtime.dependencies");
            let requirements = String::from_utf8(package::read_bounded(
                &self.resources.join(&self.definition.requirements),
                4 * 1024 * 1024,
            )?)?;
            let resource_url = reqwest::Url::from_directory_path(&self.resources)
                .map_err(|()| anyhow::anyhow!("Invalid resources path"))?;
            let requirements = requirements.replace(
                "${RESOURCE_DIR}",
                resource_url.as_str().trim_end_matches('/'),
            );
            let lock = self.environment().join("requirements.lock");
            fs::write(&lock, requirements)?;
            command::run(
                self.uv()
                    .args(["pip", "install", "--python"])
                    .arg(self.python())
                    .args([
                        "--no-deps",
                        "--require-hashes",
                        "--only-binary",
                        ":all:",
                        "--no-index",
                        "--link-mode",
                        "copy",
                        "--requirements",
                    ])
                    .arg(&lock),
                control,
                Duration::from_secs(60 * 60),
            )?;
            stage("runtime.check");
            runtime.native_check(control)?;
            stage("runtime.cache");
            command::run(
                self.uv().args(["cache", "clean"]),
                control,
                Duration::from_secs(120),
            )?;
            // Check again with the download cache gone: copy mode must really be independent.
            runtime.native_check(control)?;
            control.check()?;
            let record = Completion {
                schema_version: 1,
                id: self.definition.id.clone(),
                recipe_sha256: self.recipe_sha256.clone(),
            };
            crate::storage::atomic_write(
                &self.environment().join("complete.json"),
                &serde_json::to_vec(&record)?,
            )?;
            published = true;
            self.retire_previous()?;
            Ok(())
        })();
        if result.is_err() && !published {
            self.remove_environment()
                .context("Could not remove incomplete separation environment")?;
        }
        result
    }
    fn ensure_owned_root(&self) -> Result<()> {
        ensure!(self.root.is_absolute(), "Tools storage must be absolute");
        // Refuse symlink ancestors before mutating any runtime-owned path.
        ensure_no_symlinks(&self.root)?;
        fs::create_dir_all(&self.root)?;
        for sub in ["runtimes", "python", "cache"] {
            let path = self.root.join(sub);
            ensure_no_symlinks(&path)?;
            fs::create_dir_all(path)?;
        }
        Ok(())
    }
    fn remove_environment(&self) -> Result<()> {
        remove_owned(&self.root, &self.environment())
    }
    pub fn remove(&self) -> Result<()> {
        self.ensure_owned_root()?;
        self.remove_environment()?;
        remove_owned(&self.root, &self.cache())?;
        remove_owned(&self.root, &self.python_root())
    }
    pub fn invalidate(&self) -> Result<()> {
        let path = self.environment().join("complete.json");
        ensure_no_symlinks(&path)?;
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
    fn retire_previous(&self) -> Result<()> {
        for sub in ["runtimes", "python", "cache"] {
            for entry in fs::read_dir(self.root.join(sub))? {
                let entry = entry?;
                let name = entry.file_name();
                let Some(name) = name.to_str() else { continue };
                if name != self.definition.id && name.starts_with("roformer-sw-cpu2-v1-") {
                    remove_owned(&self.root, &entry.path())?;
                }
            }
        }
        Ok(())
    }
}
fn effective_id(base: &str, recipe: &str) -> String {
    format!("{base}-{}", &recipe[..16])
}
fn ensure_no_symlinks(path: &Path) -> Result<()> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) => ensure!(
                !meta.file_type().is_symlink(),
                "Tools storage contains a symlink: {}",
                ancestor.display()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}
fn remove_owned(root: &Path, path: &Path) -> Result<()> {
    ensure!(
        path.starts_with(root) && path != root,
        "Refusing to remove unowned tools path"
    );
    ensure!(
        path.strip_prefix(root)?
            .components()
            .all(|c| matches!(c, Component::Normal(_))),
        "Refusing a traversing tools path"
    );
    ensure_no_symlinks(path)?;
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}
pub(super) fn controlled_environment(command: &mut Command, ffmpeg: &Path, root: &Path) {
    let inherited = [
        "SystemRoot",
        "WINDIR",
        "TEMP",
        "TMP",
        "HOME",
        "USERPROFILE",
        "LOCALAPPDATA",
        "APPDATA",
        "LANG",
        "LC_ALL",
    ]
    .into_iter()
    .filter_map(|key| std::env::var_os(key).map(|v| (key, v)))
    .collect::<Vec<_>>();
    command
        .env_clear()
        .envs(inherited)
        .env("PATH", ffmpeg.parent().unwrap_or(Path::new("")))
        .env("PYTHONNOUSERSITE", "1")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env("HF_HUB_OFFLINE", "1")
        .env("TRANSFORMERS_OFFLINE", "1")
        .env("NUMBA_CACHE_DIR", root.join("native-cache/numba"))
        .env("MPLCONFIGDIR", root.join("native-cache/matplotlib"))
        .env("XDG_CACHE_HOME", root.join("native-cache"));
}
fn check_os(minimum: &MinimumOs) -> Result<()> {
    #[cfg(target_os = "macos")]
    if minimum.family == "macos" {
        let output = Command::new("/usr/bin/sw_vers")
            .arg("-productVersion")
            .output()?;
        let version = String::from_utf8(output.stdout)?;
        ensure!(
            version_at_least(version.trim(), &minimum.version),
            "Stem separation requires macOS {} or later",
            minimum.version
        );
    }
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    if minimum.family == "glibc" {
        unsafe extern "C" {
            fn gnu_get_libc_version() -> *const std::ffi::c_char;
        }
        // SAFETY: glibc returns a static NUL-terminated version string.
        let version = unsafe { std::ffi::CStr::from_ptr(gnu_get_libc_version()) }.to_str()?;
        ensure!(
            version_at_least(version, &minimum.version),
            "Stem separation requires glibc {} or later",
            minimum.version
        );
    }
    #[cfg(windows)]
    if minimum.family == "windows" {
        #[repr(C)]
        struct Version {
            size: u32,
            major: u32,
            minor: u32,
            build: u32,
            platform: u32,
            description: [u16; 128],
        }
        #[link(name = "ntdll")]
        unsafe extern "system" {
            fn RtlGetVersion(version: *mut Version) -> i32;
        }
        let mut version = Version {
            size: std::mem::size_of::<Version>() as u32,
            major: 0,
            minor: 0,
            build: 0,
            platform: 0,
            description: [0; 128],
        };
        // SAFETY: initialized OSVERSIONINFOW-compatible storage with its exact size.
        ensure!(
            unsafe { RtlGetVersion(&mut version) } == 0,
            "Could not check Windows runtime compatibility"
        );
        ensure!(
            version_at_least(
                &format!("{}.{}.{}", version.major, version.minor, version.build),
                &minimum.version
            ),
            "Stem separation requires Windows {} or later",
            minimum.version
        );
    }
    Ok(())
}
fn version_at_least(actual: &str, minimum: &str) -> bool {
    let parse = |v: &str| {
        v.split('.')
            .map(|s| s.parse::<u32>())
            .collect::<std::result::Result<Vec<_>, _>>()
    };
    match (parse(actual), parse(minimum)) {
        (Ok(mut a), Ok(mut b)) => {
            let n = a.len().max(b.len());
            a.resize(n, 0);
            b.resize(n, 0);
            a >= b
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_and_owned_paths_reject_unsafe_input() {
        assert!(version_at_least("14.0.1", "14.0"));
        assert!(!version_at_least("13.6", "14.0"));
        for path in ["../outside", "/outside", "tools/../outside", ""] {
            assert!(relative(path).is_err());
        }
        assert!(relative("tools/uv").is_ok());
        let root = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir(&root).unwrap();
        assert!(remove_owned(&root, &root).is_err());
        assert!(remove_owned(&root, &root.join("../other")).is_err());
        fs::remove_dir(root).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn symlink_roots_never_delete_external_files() {
        use std::os::unix::fs::symlink;
        let root = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir(&root).unwrap();
        let outside = root.join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("preserve"), b"song").unwrap();
        symlink(&outside, root.join("link")).unwrap();
        assert!(remove_owned(&root, &root.join("link")).is_err());
        assert!(outside.join("preserve").is_file());
        fs::remove_dir_all(root).unwrap();
    }
    #[cfg(unix)]
    fn fixture() -> (Managed, Runtime, PathBuf) {
        use std::os::unix::fs::PermissionsExt;
        let scratch = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("managed tools ü {}", uuid::Uuid::new_v4()));
        let resources = scratch.join("resources");
        fs::create_dir_all(resources.join("tools")).unwrap();
        let definition = Definition {
            schema_version: 1,
            id: effective_id("roformer-sw-cpu2-v1-fixture", &"a".repeat(64)),
            target: "fixture".into(),
            python_version: "3.12.13".into(),
            python_key: "cpython-fixture".into(),
            python_installation: "cpython-fixture".into(),
            python_build: "fixture".into(),
            python_executable: "bin/python3".into(),
            python_downloads: "python.json".into(),
            requirements: "requirements.lock".into(),
            uv: "tools/uv".into(),
            uv_version: "fixture".into(),
            ffmpeg: "tools/ffmpeg".into(),
            ffmpeg_version: "fixture".into(),
            artifact_manifest: "artifacts.json".into(),
            minimum_os: MinimumOs {
                family: "fixture".into(),
                version: "0".into(),
            },
            download_bytes: 0,
            installed_bytes: 1,
            temporary_bytes: 1024,
        };
        let managed = Managed {
            root: scratch.join("owned"),
            resources: resources.clone(),
            definition,
            recipe_sha256: "a".repeat(64),
        };
        let q = |p: &Path| format!("'{}'", p.display().to_string().replace('\'', "'\\''"));
        let probe = resources.join("tools/probe");
        fs::write(
            &probe,
            format!(
                "#!/bin/sh\n[ ! -e {} ] || exit 8\nexit 0\n",
                q(&resources.join("fail-probe"))
            ),
        )
        .unwrap();
        fs::set_permissions(&probe, fs::Permissions::from_mode(0o700)).unwrap();
        let uv = format!(
            r#"#!/bin/sh
case "$1" in
python) /bin/mkdir -p "$UV_PYTHON_INSTALL_DIR/cpython-fixture/bin"; /bin/cp {probe} "$UV_PYTHON_INSTALL_DIR/cpython-fixture/bin/python3";;
venv) for last; do :; done; /bin/mkdir -p "$last/bin"; /bin/cp {probe} "$last/bin/python";;
pip) /bin/mkdir -p "$UV_CACHE_DIR"; echo cached > "$UV_CACHE_DIR/download"; [ ! -e {fail} ] || exit 7; if [ -e {hang} ]; then /bin/sleep 60 & wait; fi;;
cache) /bin/rm -rf "$UV_CACHE_DIR";;
*) exit 2;;
esac
"#,
            probe = q(&probe),
            fail = q(&resources.join("fail-install")),
            hang = q(&resources.join("hang-install"))
        );
        fs::write(resources.join("tools/uv"), uv).unwrap();
        fs::set_permissions(
            resources.join("tools/uv"),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        fs::write(resources.join("tools/ffmpeg"), b"fixture").unwrap();
        fs::write(resources.join("requirements.lock"), b"fixture").unwrap();
        fs::write(resources.join("python.json"), b"{}").unwrap();
        fs::write(resources.join("worker.py"), b"fixture").unwrap();
        let mut runtime = Runtime::fixture(managed.python(), resources.join("worker.py"));
        runtime.managed = Some(std::sync::Arc::new(managed.clone()));
        (managed, runtime, scratch)
    }
    #[cfg(unix)]
    #[test]
    fn explicit_retry_reuses_downloads_and_publishes_only_after_native_check() {
        let (managed, runtime, scratch) = fixture();
        let control = Control::default();
        fs::write(managed.resources.join("fail-install"), b"1").unwrap();
        assert!(managed.install(&runtime, &control, |_| {}).is_err());
        assert!(!managed.ready());
        assert!(!managed.environment().exists());
        assert!(managed.cache().join("download").is_file());
        fs::remove_file(managed.resources.join("fail-install")).unwrap();
        fs::write(managed.resources.join("fail-probe"), b"1").unwrap();
        assert!(managed.install(&runtime, &control, |_| {}).is_err());
        assert!(!managed.ready());
        assert!(!managed.environment().exists());
        fs::remove_file(managed.resources.join("fail-probe")).unwrap();
        managed.install(&runtime, &control, |_| {}).unwrap();
        assert!(managed.ready());
        assert!(managed.python_root().exists());
        assert!(!managed.cache().exists());
        fs::write(managed.resources.join("fail-install"), b"1").unwrap();
        managed
            .install(&runtime, &control, |_| {
                panic!("Compatible runtime must be reused offline")
            })
            .unwrap();
        let unrelated = scratch.join("models");
        fs::create_dir(&unrelated).unwrap();
        fs::write(unrelated.join("preserve"), b"weights").unwrap();
        managed.remove().unwrap();
        assert!(!managed.ready());
        assert!(unrelated.join("preserve").is_file());
        fs::remove_dir_all(scratch).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn failed_replacement_retains_previous_and_success_retires_it() {
        let (managed, runtime, scratch) = fixture();
        managed
            .install(&runtime, &Control::default(), |_| {})
            .unwrap();
        let mut next = managed.clone();
        next.recipe_sha256 = "b".repeat(64);
        next.definition.id = effective_id("roformer-sw-cpu2-v1-fixture", &next.recipe_sha256);
        assert_ne!(next.definition.id, managed.definition.id);
        let mut next_runtime = Runtime::fixture(next.python(), runtime.worker.clone());
        next_runtime.managed = Some(std::sync::Arc::new(next.clone()));
        fs::write(next.resources.join("fail-install"), b"1").unwrap();
        assert!(
            next.install(&next_runtime, &Control::default(), |_| {})
                .is_err()
        );
        assert!(managed.ready());
        fs::remove_file(next.resources.join("fail-install")).unwrap();
        next.install(&next_runtime, &Control::default(), |_| {})
            .unwrap();
        assert!(next.ready());
        assert!(!managed.environment().exists());
        assert!(!managed.python_root().exists());
        fs::remove_dir_all(scratch).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn cancellation_reaps_installer_and_cleans_incomplete_environment() {
        let (managed, runtime, scratch) = fixture();
        fs::write(managed.resources.join("hang-install"), b"1").unwrap();
        let control = Control::default();
        let cancel = control.clone();
        let cache = managed.cache();
        let cancel_thread = std::thread::spawn(move || {
            let began = std::time::Instant::now();
            while !cache.join("download").exists() && began.elapsed() < Duration::from_secs(3) {
                std::thread::sleep(Duration::from_millis(10));
            }
            cancel.cancel();
        });
        let began = std::time::Instant::now();
        assert!(managed.install(&runtime, &control, |_| {}).is_err());
        cancel_thread.join().unwrap();
        assert!(began.elapsed() < Duration::from_secs(4));
        assert!(!managed.ready());
        assert!(!managed.environment().exists());
        assert!(managed.cache().join("download").exists());
        fs::remove_dir_all(scratch).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn retirement_failure_never_rolls_back_a_verified_new_environment() {
        use std::os::unix::fs::symlink;
        let (managed, runtime, scratch) = fixture();
        fs::create_dir_all(managed.root.join("runtimes")).unwrap();
        let outside = scratch.join("preserve");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("song"), b"original").unwrap();
        symlink(
            &outside,
            managed
                .root
                .join("runtimes/roformer-sw-cpu2-v1-retired-link"),
        )
        .unwrap();
        assert!(
            managed
                .install(&runtime, &Control::default(), |_| {})
                .is_err()
        );
        assert!(
            managed.ready(),
            "Post-publication cleanup failure removed the verified new runtime"
        );
        assert!(outside.join("song").is_file());
        fs::remove_dir_all(scratch).unwrap();
    }
}
