//! Bounded, read-only access to user keymap JSON. Action semantics live in the UI model.
use serde::Serialize;
use serde_json::Value;
use std::{fs::File, io::Read, path::Path};

const MAX_BYTES: u64 = 64 * 1024;

#[derive(Debug, Serialize)]
pub struct KeymapFile {
    pub path: String,
    pub configuration: Option<Value>,
    pub warning: Option<String>,
}

pub fn read(path: &Path) -> KeymapFile {
    let mut source = KeymapFile {
        path: path.to_string_lossy().into_owned(),
        configuration: None,
        warning: None,
    };
    let result = (|| -> anyhow::Result<Option<Value>> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let mut bytes = Vec::new();
        file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
        anyhow::ensure!(bytes.len() as u64 <= MAX_BYTES, "Keymap exceeds 64 KiB");
        let configuration: Value = serde_json::from_slice(&bytes)?;
        anyhow::ensure!(configuration.is_object(), "Keymap must be a JSON object");
        Ok(Some(configuration))
    })();
    match result {
        Ok(configuration) => source.configuration = configuration,
        Err(error) => {
            source.warning = Some(format!(
                "Cannot load {}: {error:#}. Using default shortcuts.",
                path.display()
            ))
        }
    }
    source
}
