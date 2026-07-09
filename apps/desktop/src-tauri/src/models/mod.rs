//! Local model registry and artifact verification.
//!
//! The registry is descriptive: it names the models Kaydence can use and where
//! their verified artifacts should live. This module deliberately stays
//! download-free so model readiness remains a local, auditable filesystem check.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{self, Read},
    path::{Component, Path, PathBuf},
};

pub const SUPPORTED_SCHEMA_VERSION: u16 = 1;
pub const QUARANTINE_DIR_NAME: &str = "quarantine";
const HASH_BUFFER_BYTES: usize = 64 * 1024;

pub fn source_tree_models_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join("models")
}

pub fn source_tree_registry_path() -> PathBuf {
    source_tree_models_dir().join("registry.json")
}

#[derive(Debug, thiserror::Error)]
pub enum ModelRegistryError {
    #[error("unsupported model registry schema version {actual}; expected {expected}")]
    UnsupportedSchema { actual: u16, expected: u16 },
    #[error("duplicate model id in registry: {0}")]
    DuplicateModelId(String),
    #[error("model {0} is not registered")]
    ModelNotFound(String),
    #[error("model {id} has an unsafe artifact path: {file}")]
    InvalidArtifactPath { id: String, file: String },
    #[error("model {id} does not have a usable sha256 yet")]
    PlaceholderChecksum { id: String },
    #[error(
        "model {id} checksum mismatch: expected {expected}, actual {actual}; quarantined at {}",
        quarantined_to.display()
    )]
    ChecksumMismatch {
        id: String,
        expected: String,
        actual: String,
        quarantined_to: PathBuf,
    },
    #[error("model {id} checksum mismatch and quarantine failed for {}: {source}", path.display())]
    QuarantineFailed {
        id: String,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("model registry json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("model registry io: {0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ModelRegistry {
    pub schema_version: u16,
    pub models: Vec<ModelEntry>,
}

impl ModelRegistry {
    pub fn load(path: &Path) -> Result<Self, ModelRegistryError> {
        let json = std::fs::read_to_string(path)?;
        Self::from_json(&json)
    }

    pub fn from_json(json: &str) -> Result<Self, ModelRegistryError> {
        let registry: Self = serde_json::from_str(json)?;
        registry.validate()?;
        Ok(registry)
    }

    pub fn get(&self, id: &str) -> Option<&ModelEntry> {
        self.models.iter().find(|model| model.id == id)
    }

    pub fn require(&self, id: &str) -> Result<&ModelEntry, ModelRegistryError> {
        self.get(id)
            .ok_or_else(|| ModelRegistryError::ModelNotFound(id.to_string()))
    }

    pub fn recommended_for(&self, task: ModelTask) -> Vec<&ModelEntry> {
        self.models
            .iter()
            .filter(|model| model.task == task && model.recommended)
            .collect()
    }

    pub fn verify_required_first_run_models(
        &self,
        models_dir: &Path,
    ) -> Vec<(&ModelEntry, Result<ModelArtifactStatus, ModelRegistryError>)> {
        self.models
            .iter()
            .filter(|model| {
                model.recommended && matches!(model.task, ModelTask::Asr | ModelTask::Vad)
            })
            .map(|model| (model, model.verify_artifact(models_dir)))
            .collect()
    }

    fn validate(&self) -> Result<(), ModelRegistryError> {
        if self.schema_version != SUPPORTED_SCHEMA_VERSION {
            return Err(ModelRegistryError::UnsupportedSchema {
                actual: self.schema_version,
                expected: SUPPORTED_SCHEMA_VERSION,
            });
        }

        let mut ids = HashSet::new();
        for model in &self.models {
            if !ids.insert(model.id.clone()) {
                return Err(ModelRegistryError::DuplicateModelId(model.id.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub task: ModelTask,
    pub lane: Option<String>,
    pub runtime: String,
    pub file: String,
    pub sha256: String,
    pub size_mb: u64,
    pub license: String,
    #[serde(default)]
    pub license_review_required: bool,
    pub min_hw: String,
    #[serde(default)]
    pub recommended: bool,
    #[serde(default)]
    pub default_for: Vec<String>,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub note: Option<String>,
}

impl ModelEntry {
    pub fn is_checksum_placeholder(&self) -> bool {
        !is_sha256_hex(&self.sha256)
    }

    pub fn has_placeholder_sources(&self) -> bool {
        self.sources.is_empty()
            || self.sources.iter().any(|source| {
                let source = source.trim();
                source.is_empty() || source.starts_with("TODO")
            })
    }

    pub fn artifact_path(&self, models_dir: &Path) -> Result<PathBuf, ModelRegistryError> {
        if self.file.contains(['/', '\\']) {
            return Err(ModelRegistryError::InvalidArtifactPath {
                id: self.id.clone(),
                file: self.file.clone(),
            });
        }

        let file = Path::new(&self.file);
        let mut components = file.components();
        match (components.next(), components.next()) {
            (Some(Component::Normal(_)), None) => Ok(models_dir.join(file)),
            _ => Err(ModelRegistryError::InvalidArtifactPath {
                id: self.id.clone(),
                file: self.file.clone(),
            }),
        }
    }

    pub fn verify_artifact(
        &self,
        models_dir: &Path,
    ) -> Result<ModelArtifactStatus, ModelRegistryError> {
        if self.is_checksum_placeholder() {
            return Err(ModelRegistryError::PlaceholderChecksum {
                id: self.id.clone(),
            });
        }

        let path = self.artifact_path(models_dir)?;
        if !path.exists() {
            return Ok(ModelArtifactStatus::Missing { path });
        }

        let actual = hash_file_sha256(&path)?;
        let expected = self.sha256.trim().to_ascii_lowercase();
        if actual != expected {
            let quarantined_to = quarantine_artifact(&path, &actual).map_err(|source| {
                ModelRegistryError::QuarantineFailed {
                    id: self.id.clone(),
                    path: path.clone(),
                    source,
                }
            })?;
            return Err(ModelRegistryError::ChecksumMismatch {
                id: self.id.clone(),
                expected,
                actual,
                quarantined_to,
            });
        }

        Ok(ModelArtifactStatus::Ready {
            size_bytes: path.metadata()?.len(),
            path,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelTask {
    Asr,
    Vad,
    Cleanup,
    Prediction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelArtifactStatus {
    Missing { path: PathBuf },
    Ready { path: PathBuf, size_bytes: u64 },
}

fn is_sha256_hex(value: &str) -> bool {
    let value = value.trim();
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn hash_file_sha256(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; HASH_BUFFER_BYTES];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex_lower(&hasher.finalize()))
}

fn quarantine_artifact(path: &Path, actual_hash: &str) -> io::Result<PathBuf> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "model artifact path has no parent directory",
        )
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "model artifact path has no filename",
        )
    })?;
    let file_name = file_name.to_string_lossy();
    let short_hash = actual_hash.get(..12).unwrap_or(actual_hash);
    let quarantine_dir = parent.join(QUARANTINE_DIR_NAME);
    fs::create_dir_all(&quarantine_dir)?;

    for index in 0..1000 {
        let suffix = if index == 0 {
            String::new()
        } else {
            format!(".{index}")
        };
        let candidate = quarantine_dir.join(format!("{file_name}.{short_hash}{suffix}.bad"));
        if !candidate.exists() {
            fs::rename(path, &candidate)?;
            return Ok(candidate);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "too many quarantined artifacts with the same checksum",
    ))
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kaydence-models-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn registry_path() -> PathBuf {
        source_tree_registry_path()
    }

    fn repo_models_dir() -> PathBuf {
        source_tree_models_dir()
    }

    fn entry(id: &str, file: &str, sha256: String) -> ModelEntry {
        ModelEntry {
            id: id.to_string(),
            task: ModelTask::Asr,
            lane: Some("cpu".to_string()),
            runtime: "onnxruntime".to_string(),
            file: file.to_string(),
            sha256,
            size_mb: 1,
            license: "Apache-2.0".to_string(),
            license_review_required: false,
            min_hw: "any".to_string(),
            recommended: true,
            default_for: Vec::new(),
            sources: vec!["TODO_primary".to_string()],
            note: None,
        }
    }

    fn sha256_for(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex_lower(&hasher.finalize())
    }

    #[test]
    fn parses_current_registry_and_exposes_first_run_candidates() {
        let registry = ModelRegistry::load(&registry_path()).unwrap();

        assert!(registry.require("parakeet-v3").is_ok());
        assert!(registry.require("whisper-large-v3-turbo").is_ok());
        assert!(registry.require("silero-vad").is_ok());
        assert!(registry.recommended_for(ModelTask::Asr).len() >= 2);
        assert!(!registry.recommended_for(ModelTask::Vad).is_empty());
    }

    #[test]
    fn current_placeholder_hashes_fail_closed_for_first_run_readiness() {
        let registry = ModelRegistry::load(&registry_path()).unwrap();
        let results = registry.verify_required_first_run_models(&repo_models_dir());

        assert!(!results.is_empty());
        assert!(results.iter().all(|(_, result)| matches!(
            result,
            Err(ModelRegistryError::PlaceholderChecksum { .. })
        )));
        assert!(results
            .iter()
            .any(|(model, _)| model.has_placeholder_sources()));
    }

    #[test]
    fn verifies_local_artifact_with_matching_checksum() {
        let dir = tmp();
        let bytes = b"kaydence local model fixture";
        let model = entry("fixture-asr", "fixture.onnx", sha256_for(bytes));
        let path = dir.join("fixture.onnx");
        let mut file = File::create(&path).unwrap();
        file.write_all(bytes).unwrap();
        drop(file);

        let status = model.verify_artifact(&dir).unwrap();

        assert_eq!(
            status,
            ModelArtifactStatus::Ready {
                path,
                size_bytes: bytes.len() as u64
            }
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn reports_missing_artifact_after_checksum_is_usable() {
        let dir = tmp();
        let model = entry("fixture-asr", "missing.onnx", sha256_for(b"expected"));

        let status = model.verify_artifact(&dir).unwrap();

        assert_eq!(
            status,
            ModelArtifactStatus::Missing {
                path: dir.join("missing.onnx")
            }
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn checksum_mismatch_quarantines_artifact() {
        let dir = tmp();
        let model = entry("fixture-asr", "fixture.onnx", sha256_for(b"expected"));
        let artifact_path = dir.join("fixture.onnx");
        std::fs::write(&artifact_path, b"actual").unwrap();

        let err = model.verify_artifact(&dir).unwrap_err();

        let quarantined_to = match err {
            ModelRegistryError::ChecksumMismatch {
                id,
                actual,
                quarantined_to,
                ..
            } => {
                assert_eq!(id, "fixture-asr");
                assert_eq!(actual, sha256_for(b"actual"));
                quarantined_to
            }
            other => panic!("unexpected error: {other:?}"),
        };
        assert!(!artifact_path.exists());
        assert_eq!(
            quarantined_to
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy(),
            QUARANTINE_DIR_NAME
        );
        assert!(quarantined_to
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains(&sha256_for(b"actual")[..12]));
        assert_eq!(std::fs::read(quarantined_to).unwrap(), b"actual");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn quarantine_uses_unique_names_for_repeated_bad_artifacts() {
        let dir = tmp();
        let model = entry("fixture-asr", "fixture.onnx", sha256_for(b"expected"));
        let actual_hash = sha256_for(b"actual");
        let quarantine_dir = dir.join(QUARANTINE_DIR_NAME);
        std::fs::create_dir_all(&quarantine_dir).unwrap();
        std::fs::write(
            quarantine_dir.join(format!("fixture.onnx.{}.bad", &actual_hash[..12])),
            b"previous bad artifact",
        )
        .unwrap();
        std::fs::write(dir.join("fixture.onnx"), b"actual").unwrap();

        let err = model.verify_artifact(&dir).unwrap_err();

        let quarantined_to = match err {
            ModelRegistryError::ChecksumMismatch { quarantined_to, .. } => quarantined_to,
            other => panic!("unexpected error: {other:?}"),
        };
        assert_eq!(
            quarantined_to.file_name().unwrap().to_string_lossy(),
            format!("fixture.onnx.{}.1.bad", &actual_hash[..12])
        );
        assert_eq!(std::fs::read(quarantined_to).unwrap(), b"actual");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn artifact_paths_must_be_plain_filenames() {
        let dir = tmp();
        for file in [
            "../escape.onnx",
            "nested/model.onnx",
            "/tmp/model.onnx",
            "nested\\model.onnx",
        ] {
            let model = entry("fixture-asr", file, sha256_for(b"expected"));
            let err = model.verify_artifact(&dir).unwrap_err();
            assert!(matches!(
                err,
                ModelRegistryError::InvalidArtifactPath { .. }
            ));
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let json = r#"{
            "schema_version": 1,
            "models": [
                {
                    "id": "same",
                    "task": "asr",
                    "runtime": "onnxruntime",
                    "file": "a.onnx",
                    "sha256": "TODO",
                    "size_mb": 1,
                    "license": "MIT",
                    "min_hw": "any"
                },
                {
                    "id": "same",
                    "task": "vad",
                    "runtime": "onnxruntime",
                    "file": "b.onnx",
                    "sha256": "TODO",
                    "size_mb": 1,
                    "license": "MIT",
                    "min_hw": "any"
                }
            ]
        }"#;

        let err = ModelRegistry::from_json(json).unwrap_err();

        assert!(matches!(err, ModelRegistryError::DuplicateModelId(id) if id == "same"));
    }
}
