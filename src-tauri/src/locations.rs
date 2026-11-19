//! Shows app-owned files and folders in the system file manager, or opens them
//! in their default app. The webview names a location, never a path, so it
//! can only reach the library, the shortcut file and the model folders.
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Location {
    Library,
    ShortcutFile,
}

/// The path itself when it exists, else its nearest existing ancestor, so a
/// shortcut file that hasn't been written yet still shows where it belongs.
pub fn nearest_existing(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|candidate| !candidate.as_os_str().is_empty() && candidate.exists())
        .map(Path::to_path_buf)
}

fn launch(mut command: Command) -> Result<()> {
    let mut child = command
        .spawn()
        .context("Could not start the file manager")?;
    // Reap the helper without blocking the caller; its exit code is not
    // meaningful on every platform (Windows Explorer returns 1 on success).
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// Selects `path` in Finder, File Explorer or the Linux file manager. A
/// folder is revealed in its parent, like a file, rather than opened.
pub fn reveal(path: &Path) -> Result<()> {
    let target = nearest_existing(path).context("This location doesn't exist yet")?;
    // A missing file falls back to showing its nearest existing folder's contents.
    let selects = target == path;
    launch(reveal_command(&target, selects))
}

#[cfg(target_os = "macos")]
fn reveal_command(target: &Path, selects: bool) -> Command {
    let mut command = Command::new("open");
    if selects {
        command.arg("-R");
    }
    command.arg(target);
    command
}

#[cfg(windows)]
fn reveal_command(target: &Path, selects: bool) -> Command {
    use std::os::windows::process::CommandExt;
    let mut command = Command::new("explorer");
    if selects {
        // Explorer parses its own command line: the path is quoted after
        // `/select,` and must not be re-escaped by Rust's argument quoting.
        command.raw_arg(format!("/select,\"{}\"", target.display()));
    } else {
        command.arg(target);
    }
    command
}

#[cfg(not(any(target_os = "macos", windows)))]
fn reveal_command(target: &Path, _selects: bool) -> Command {
    // xdg-open can't select an item, so it opens the containing folder.
    let folder = if target.is_file() {
        target
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| target.to_path_buf())
    } else {
        target.to_path_buf()
    };
    let mut command = Command::new("xdg-open");
    command.arg(folder);
    command
}

/// Opens an existing file in its default app.
pub fn open(path: &Path) -> Result<()> {
    ensure!(path.is_file(), "There's no file at {} yet", path.display());
    let command = if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg("-t").arg(path);
        command
    } else if cfg!(target_os = "windows") {
        let mut command = Command::new("explorer");
        command.arg(path);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        command
    };
    launch(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_path_falls_back_to_its_nearest_existing_folder() {
        let root = std::env::temp_dir().join(format!("ohmyloop-locations-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("keymap.json");
        assert_eq!(
            nearest_existing(&root.join("missing/keymap.json")),
            Some(root.clone())
        );
        std::fs::write(&file, b"{}").unwrap();
        assert_eq!(nearest_existing(&file), Some(file.clone()));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn opening_requires_an_existing_file() {
        let missing =
            std::env::temp_dir().join(format!("ohmyloop-missing-{}.json", std::process::id()));
        assert!(open(&missing).is_err());
    }

    #[test]
    fn locations_are_named_not_paths() {
        assert_eq!(
            serde_json::from_str::<Location>("\"shortcut_file\"").unwrap(),
            Location::ShortcutFile
        );
        assert!(serde_json::from_str::<Location>("\"/etc/passwd\"").is_err());
    }
}
