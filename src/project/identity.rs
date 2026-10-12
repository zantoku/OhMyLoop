//! Durable IDs and ephemeral reference/session identity.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
macro_rules! id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(Uuid);
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }
        impl TryFrom<String> for $name {
            type Error = anyhow::Error;
            fn try_from(value: String) -> Result<Self> {
                let uuid = Uuid::parse_str(&value)?;
                ensure!(
                    !uuid.is_nil() && uuid.hyphenated().to_string() == value,
                    "Expected a canonical nonzero UUID"
                );
                Ok(Self(uuid))
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> String {
                value.to_string()
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}
id!(SongId);
id!(AudioAssetId);
id!(TimelineId);
id!(SessionId);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
    pub song: SongId,
    pub asset: AudioAssetId,
    pub timeline: TimelineId,
    pub revision: u64,
    pub session: SessionId,
}
impl Reference {
    pub fn token(&self) -> String {
        format!(
            "{}~{}~{}~{}~{}",
            self.song, self.asset, self.timeline, self.revision, self.session
        )
    }
    pub fn parse(token: &str) -> Result<Self> {
        let p: Vec<_> = token.split('~').collect();
        ensure!(p.len() == 5, "Invalid recording session");
        let revision = p[3].parse()?;
        ensure!(revision > 0, "Invalid recording revision");
        Ok(Self {
            song: p[0].to_owned().try_into()?,
            asset: p[1].to_owned().try_into()?,
            timeline: p[2].to_owned().try_into()?,
            revision,
            session: p[4].to_owned().try_into()?,
        })
    }
    /// Cache identity excludes session, but includes the immutable asset and timeline revision.
    pub fn cache_key(&self) -> String {
        format!(
            "{}-{}-{}-{}",
            self.song, self.asset, self.timeline, self.revision
        )
    }
}
