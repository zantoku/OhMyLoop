//! Bounded protocol parsing and native identity verification.
use super::package;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, Read},
    path::Path,
};
pub const MAX_LINE: usize = 64 * 1024;
pub const MAX_DOCUMENT: u64 = 1024 * 1024;
pub const DIAGNOSTIC_BYTES: usize = 16 * 1024;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Update {
    pub version: u32,
    pub job_id: String,
    pub stage: String,
    pub completed: Option<u64>,
    pub total: Option<u64>,
    pub unit: Option<String>,
}
pub fn read_updates(
    mut input: impl BufRead,
    job: &str,
    mut publish: impl FnMut(Update),
) -> Result<()> {
    let mut previous: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    let mut previous_stage = 0;
    let mut line = Vec::with_capacity(MAX_LINE);
    loop {
        line.clear();
        loop {
            let available = input.fill_buf()?;
            if available.is_empty() {
                ensure!(line.is_empty(), "Unterminated worker progress line");
                return Ok(());
            }
            let count = available
                .iter()
                .position(|&b| b == b'\n')
                .map_or(available.len(), |n| n + 1);
            ensure!(
                line.len() + count <= MAX_LINE,
                "Worker progress exceeds line limit"
            );
            line.extend_from_slice(&available[..count]);
            let end = available[count - 1] == b'\n';
            input.consume(count);
            if end {
                break;
            }
        }
        let update: Update = serde_json::from_slice(&line)?;
        ensure!(
            update.version == 1 && update.job_id == job,
            "Stale or unsupported worker progress"
        );
        ensure!(
            matches!(
                update.stage.as_str(),
                "worker.input" | "worker.load" | "worker.infer" | "worker.write" | "worker.finish"
            ),
            "Unknown worker stage"
        );
        let rank = match update.stage.as_str() {
            "worker.input" => 0,
            "worker.load" => 1,
            "worker.infer" => 2,
            "worker.write" => 3,
            _ => 4,
        };
        ensure!(rank >= previous_stage, "Worker stage moved backwards");
        previous_stage = rank;
        match (update.completed, update.total, update.unit.as_deref()) {
            (None, None, None) => ensure!(
                matches!(
                    update.stage.as_str(),
                    "worker.input" | "worker.load" | "worker.finish"
                ),
                "Missing measured inference progress"
            ),
            (Some(n), Some(t), Some(unit)) => {
                ensure!(
                    t > 0
                        && n <= t
                        && matches!(
                            (update.stage.as_str(), unit),
                            ("worker.infer", "chunks") | ("worker.write", "tracks")
                        ),
                    "Invalid measured progress"
                );
                if let Some((last, total)) = previous.get(&update.stage) {
                    ensure!(n >= *last && t == *total, "Worker progress moved backwards");
                }
                previous.insert(update.stage.clone(), (n, t));
            }
            _ => anyhow::bail!("Incomplete measured progress"),
        };
        publish(update);
    }
}
pub fn read_result(path: &Path, job: &str, prep: &Value) -> Result<Value> {
    ensure!(
        fs::metadata(path)?.len() <= MAX_DOCUMENT,
        "Worker result exceeds limit"
    );
    let result =
        serde_json::from_slice::<UniqueValue>(&package::read_bounded(path, MAX_DOCUMENT)?)?.0;
    ensure!(
        result["version"] == 1
            && result["job_id"] == job
            && result["package_id"] == package::ID
            && result["profile_id"] == package::PROFILE
            && result["preparation"] == *prep
            && result["actual_device"] == "cpu"
            && result["python"] == "3.12.13",
        "Worker result identity mismatch"
    );
    for (name, version) in [
        ("audio-separator", "0.41.1"),
        (
            "torch",
            if cfg!(target_os = "macos") {
                "2.8.0"
            } else {
                "2.8.0+cpu"
            },
        ),
        (
            "torchaudio",
            if cfg!(target_os = "macos") {
                "2.8.0"
            } else {
                "2.8.0+cpu"
            },
        ),
        ("numpy", "2.2.6"),
        ("soundfile", "0.13.1"),
        ("librosa", "0.11.0"),
    ] {
        ensure!(
            result["packages"][name] == version,
            "Worker runtime version mismatch"
        );
    }
    let stems = result["stems"]
        .as_object()
        .context("Missing worker stems")?;
    ensure!(stems.len() == 6, "Incomplete worker outputs");
    for role in crate::stems::Role::ALL {
        let name = role.name();
        let s = stems.get(name).context("Missing canonical role")?;
        ensure!(
            s["file"] == format!("{name}.wav")
                && s["rate"] == 44100
                && s["channels"] == 2
                && s["frames"] == prep["prepared_frames"],
            "Worker stem geometry or path mismatch"
        );
    }
    Ok(result)
}
pub fn manifest(prep: &Value, result: &Value) -> Result<Value> {
    let settings = settings();
    let identity = json!({"preparation":prep,"model_lock":package::descriptor().lock(),"settings":settings,"runtime":{"definition_id":result["runtime_id"],"python":result["python"],"packages":result["packages"],"actual_device":result["actual_device"]},"model_package":{"id":package::ID,"profile":package::PROFILE,"descriptor_version":1,"worker_protocol":1,"runtime_lock_sha256":package::descriptor().runtime_lock_sha256}});
    let key = format!("{:x}", Sha256::digest(serde_json::to_vec(&identity)?));
    let manifest = json!({"version":1,"status":"complete","job_key":key,"identity":identity,"stems":result["stems"],"corrections":result["corrections"]});
    crate::stems::library::validate_corrections(&manifest)?;
    Ok(manifest)
}
pub fn settings() -> Value {
    json!({"device":"cpu","threads":2,"seed":0,"normalization":"none per stem","roformer_short_input":"right-zero-pad-to-config-chunk; crop-declared-padding"})
}
pub fn tail(mut input: impl Read) -> std::io::Result<Vec<u8>> {
    let mut tail = Vec::new();
    let mut b = [0; 4096];
    loop {
        let n = input.read(&mut b)?;
        if n == 0 {
            return Ok(tail);
        }
        tail.extend_from_slice(&b[..n]);
        if tail.len() > DIAGNOSTIC_BYTES {
            tail.drain(..tail.len() - DIAGNOSTIC_BYTES);
        }
    }
}
/// Reject duplicate JSON keys before translating a result into owned provenance.
struct UniqueValue(Value);
impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UniqueValue;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("JSON without duplicate object keys")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> std::result::Result<UniqueValue, M::Error> {
                let mut value = serde_json::Map::new();
                while let Some((key, entry)) = map.next_entry::<String, UniqueValue>()? {
                    if value.insert(key, entry.0).is_some() {
                        return Err(serde::de::Error::custom("Duplicate JSON field"));
                    }
                }
                Ok(UniqueValue(Value::Object(value)))
            }
            fn visit_seq<S: serde::de::SeqAccess<'de>>(
                self,
                mut sequence: S,
            ) -> std::result::Result<UniqueValue, S::Error> {
                let mut values = Vec::new();
                while let Some(value) = sequence.next_element::<UniqueValue>()? {
                    values.push(value.0);
                }
                Ok(UniqueValue(Value::Array(values)))
            }
            fn visit_str<E: serde::de::Error>(
                self,
                v: &str,
            ) -> std::result::Result<UniqueValue, E> {
                Ok(UniqueValue(Value::String(v.into())))
            }
            fn visit_bool<E: serde::de::Error>(
                self,
                v: bool,
            ) -> std::result::Result<UniqueValue, E> {
                Ok(UniqueValue(Value::Bool(v)))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<UniqueValue, E> {
                Ok(UniqueValue(Value::from(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<UniqueValue, E> {
                Ok(UniqueValue(Value::from(v)))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<UniqueValue, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| UniqueValue(Value::Number(n)))
                    .ok_or_else(|| serde::de::Error::custom("Non-finite JSON number"))
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<UniqueValue, E> {
                Ok(UniqueValue(Value::Null))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn worker_boundaries_cover_input_loading_inference_writes_and_exit_in_order() {
        let events = [
            ("worker.input", None, None, None),
            ("worker.load", None, None, None),
            ("worker.infer", Some(0), Some(2), Some("chunks")),
            ("worker.infer", Some(2), Some(2), Some("chunks")),
            ("worker.write", Some(0), Some(6), Some("tracks")),
            ("worker.write", Some(6), Some(6), Some("tracks")),
            ("worker.finish", None, None, None),
        ];
        let mut input = events.into_iter().map(|(stage, completed, total, unit)| format!("{}\n", json!({"version":1,"job_id":"job","stage":stage,"completed":completed,"total":total,"unit":unit}))).collect::<String>();
        let mut stages = Vec::new();
        read_updates(std::io::Cursor::new(input.as_bytes()), "job", |u| {
            stages.push(u.stage)
        })
        .unwrap();
        assert_eq!(stages.first().unwrap(), "worker.input");
        assert_eq!(stages.last().unwrap(), "worker.finish");
        input.push_str(&format!("{}\n", json!({"version":1,"job_id":"job","stage":"worker.input","completed":null,"total":null,"unit":null})));
        assert!(read_updates(std::io::Cursor::new(input.as_bytes()), "job", |_| {}).is_err());
    }
    #[test]
    fn bounded_progress_rejects_corruption_and_stale_ids() {
        assert!(serde_json::from_str::<UniqueValue>(r#"{"stems":{"bass":1,"bass":2}}"#).is_err());
        let u = json!({"version":1,"job_id":"job","stage":"worker.infer","completed":1,"total":2,"unit":"chunks"});
        let text = format!("{u}\n");
        let mut count = 0;
        read_updates(std::io::Cursor::new(text.as_bytes()), "job", |_| count += 1).unwrap();
        assert_eq!(count, 1);
        assert!(read_updates(std::io::Cursor::new(text.as_bytes()), "stale", |_| {}).is_err());
        assert!(
            read_updates(
                std::io::Cursor::new(vec![b'x'; MAX_LINE + 1]),
                "job",
                |_| {}
            )
            .is_err()
        );
        let backwards = format!(
            "{u}\n{}\n",
            json!({"version":1,"job_id":"job","stage":"worker.infer","completed":0,"total":2,"unit":"chunks"})
        );
        assert!(read_updates(std::io::Cursor::new(backwards), "job", |_| {}).is_err());
        assert_eq!(
            tail(std::io::Cursor::new(vec![b'x'; 100000]))
                .unwrap()
                .len(),
            DIAGNOSTIC_BYTES
        );
    }
}
