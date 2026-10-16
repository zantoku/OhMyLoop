//! Shared durable writes for versioned local JSON stores.
use anyhow::{Context, Result};
use std::{
    fs::{self, File},
    io::{Read, Write},
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

pub(crate) fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
    anyhow::ensure!(bytes.len() as u64 <= limit, "Document exceeds size limit");
    Ok(bytes)
}
