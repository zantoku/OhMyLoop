//! Read initial tags with the existing decoder's probe; never write audio files.
use super::Metadata;
use anyhow::Result;
use std::{fs::File, path::Path};
use symphonia::core::{
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::{MetadataOptions, StandardTagKey, Tag},
    probe::Hint,
};

fn collect(metadata: &mut Metadata, tags: &[Tag]) {
    for tag in tags {
        let target = match tag.std_key {
            Some(StandardTagKey::Artist) => &mut metadata.artist,
            Some(StandardTagKey::TrackTitle) => &mut metadata.title,
            Some(StandardTagKey::Album) => &mut metadata.album,
            _ => continue,
        };
        let text = tag.value.to_string();
        if target.is_empty() {
            *target = text
                .trim_matches(|c: char| c.is_whitespace() || c == '\0')
                .to_owned();
        }
    }
}
pub fn read_tags(path: &Path) -> Result<Metadata> {
    let stream = MediaSourceStream::new(Box::new(File::open(path)?), Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(extension);
    }
    let mut probe = symphonia::default::get_probe().format(
        &hint,
        stream,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;
    let mut metadata = Metadata::default();
    if let Some(revision) = probe.metadata.get().and_then(|m| m.current().cloned()) {
        collect(&mut metadata, revision.tags());
    }
    // Some formats publish their initial tags while reading the first packet.
    let _ = probe.format.next_packet();
    if let Some(revision) = probe.format.metadata().current() {
        collect(&mut metadata, revision.tags());
    }
    if metadata.title.is_empty() {
        metadata.title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled song")
            .to_owned();
    }
    Ok(metadata)
}
