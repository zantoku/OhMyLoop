use super::*;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LibraryPointer {
    pub root: String,
    pub id: LibraryId,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreferenceData {
    pub version: u32,
    pub active: Option<LibraryPointer>,
    pub last_song: Option<String>,
    pub import_explained: bool,
}
impl Default for PreferenceData {
    fn default() -> Self {
        Self {
            version: 1,
            active: None,
            last_song: None,
            import_explained: false,
        }
    }
}
impl PreferenceData {
    fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 1,
            "Unsupported managed preferences; original preserved"
        );
        if let Some(pointer) = &self.active {
            ensure!(
                !pointer.root.is_empty()
                    && pointer.root.len() <= 16384
                    && !pointer.root.contains('\0'),
                "Invalid library pointer"
            );
        }
        if let Some(song) = &self.last_song {
            crate::project::SongId::try_from(song.clone())?;
            ensure!(self.active.is_some(), "Last song requires selected library");
        }
        Ok(())
    }
}
pub struct Preferences {
    path: PathBuf,
}
impl Preferences {
    pub fn new(directory: &Path) -> Self {
        Self {
            path: directory.join("managed-library.json"),
        }
    }
    pub fn read(&self) -> Result<PreferenceData> {
        let metadata = match fs::symlink_metadata(&self.path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(PreferenceData::default());
            }
            Err(e) => return Err(e.into()),
        };
        ensure!(
            metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() <= 16384,
            "Invalid managed preferences; original preserved"
        );
        let data: PreferenceData = serde_json::from_slice(&fs::read(&self.path)?)?;
        data.validate()?;
        Ok(data)
    }
    pub fn save(&self, data: &PreferenceData) -> Result<()> {
        self.read()?;
        data.validate()?;
        let tmp = self.path.with_extension("json.tmp");
        if let Ok(metadata) = fs::symlink_metadata(&tmp) {
            ensure!(
                !metadata.file_type().is_symlink(),
                "Invalid preference temporary path"
            );
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        crate::storage::atomic_write(&self.path, &serde_json::to_vec_pretty(data)?)
    }
}
