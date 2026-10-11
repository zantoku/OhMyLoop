//! Shared durable writes for versioned local JSON stores.
use anyhow::{Context, Result};
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};
pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = path.with_extension("json.tmp");
    let mut file = File::create(&temporary).context("Cannot save local data")?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(temporary, path).context("Cannot replace saved local data")?;
    Ok(())
}
