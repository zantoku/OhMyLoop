use super::*;
use std::path::Component;
impl ManagedLibrary {
    pub(super) fn checked_path(&self, relative: &Path, must_exist: bool) -> Result<PathBuf> {
        let root_metadata =
            fs::symlink_metadata(&self.root).context("Library folder is unavailable")?;
        ensure!(
            root_metadata.is_dir() && !root_metadata.file_type().is_symlink(),
            "Library root changed or is unavailable"
        );
        ensure!(!relative.as_os_str().is_empty(), "Empty managed path");
        let mut path = self.root.clone();
        for component in relative.components() {
            let Component::Normal(name) = component else {
                anyhow::bail!("Managed paths must be relative without traversal")
            };
            ensure!(
                !name.to_string_lossy().contains('\\'),
                "Invalid managed path separator"
            );
            path.push(name);
            match fs::symlink_metadata(&path) {
                Ok(metadata) => ensure!(
                    !metadata.file_type().is_symlink(),
                    "Symlinks are not permitted in managed paths"
                ),
                Err(error) if !must_exist && error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error).context("Managed file is missing or unavailable"),
            }
        }
        Ok(path)
    }
}
pub(super) fn reject_tree_symlinks(directory: &Path) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        ensure!(
            !kind.is_symlink(),
            "Song contains a symlink; deletion refused"
        );
        if kind.is_dir() {
            reject_tree_symlinks(&entry.path())?;
        }
    }
    Ok(())
}
