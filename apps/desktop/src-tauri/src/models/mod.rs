//! Local model registry and artifact verification.
//!
//! The registry is descriptive: it names the models Kaydence can use and where
//! their verified artifacts should live. This module deliberately stays
//! download-free: it can produce validated download plans, but it does not fetch
//! bytes, so model readiness remains a local, auditable filesystem check.

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
const HTTPS_SCHEME_PREFIX: &str = concat!("https", "://");

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
    #[error("model {id} selected artifact is not a readable file: {}", path.display())]
    InvalidInstallSource { id: String, path: PathBuf },
    #[error(
        "model {id} selected artifact checksum mismatch: expected {expected}, actual {actual}; source left untouched"
    )]
    InstallChecksumMismatch {
        id: String,
        expected: String,
        actual: String,
    },
    #[error("model {id} does not have usable download sources yet")]
    PlaceholderSources { id: String },
    #[error("model {id} has an invalid download source: {url}")]
    InvalidDownloadSource { id: String, url: String },
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
        selected_asr_model_id: Option<&str>,
    ) -> Vec<(&ModelEntry, Result<ModelArtifactStatus, ModelRegistryError>)> {
        self.models
            .iter()
            .filter(|model| {
                model.first_run_required
                    || (model.task == ModelTask::Asr
                        && selected_asr_model_id == Some(model.id.as_str()))
            })
            .map(|model| (model, model.verify_artifact(models_dir)))
            .collect()
    }

    pub fn download_plan(
        &self,
        id: &str,
        models_dir: &Path,
    ) -> Result<ModelDownloadPlan, ModelRegistryError> {
        self.require(id)?.download_plan(models_dir)
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
    pub first_run_required: bool,
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

    pub fn download_plan(
        &self,
        models_dir: &Path,
    ) -> Result<ModelDownloadPlan, ModelRegistryError> {
        if self.is_checksum_placeholder() {
            return Err(ModelRegistryError::PlaceholderChecksum {
                id: self.id.clone(),
            });
        }

        let destination_path = self.artifact_path(models_dir)?;
        let sources = self.validated_sources()?;
        Ok(ModelDownloadPlan {
            id: self.id.clone(),
            task: self.task,
            file: self.file.clone(),
            destination_path,
            sha256: self.sha256.trim().to_ascii_lowercase(),
            size_mb: self.size_mb,
            license: self.license.clone(),
            license_review_required: self.license_review_required,
            sources,
        })
    }

    pub fn install_artifact_from_path(
        &self,
        source_path: &Path,
        models_dir: &Path,
    ) -> Result<ModelInstallOutcome, ModelRegistryError> {
        if self.is_checksum_placeholder() {
            return Err(ModelRegistryError::PlaceholderChecksum {
                id: self.id.clone(),
            });
        }

        let source_path = fs::canonicalize(source_path)?;
        if !source_path.is_file() {
            return Err(ModelRegistryError::InvalidInstallSource {
                id: self.id.clone(),
                path: source_path,
            });
        }

        let expected = self.sha256.trim().to_ascii_lowercase();
        let actual = hash_file_sha256(&source_path)?;
        if actual != expected {
            return Err(ModelRegistryError::InstallChecksumMismatch {
                id: self.id.clone(),
                expected,
                actual,
            });
        }

        fs::create_dir_all(models_dir)?;
        let destination_path = self.artifact_path(models_dir)?;
        match self.verify_artifact(models_dir) {
            Ok(ModelArtifactStatus::Ready { path, size_bytes }) => {
                return Ok(ModelInstallOutcome {
                    id: self.id.clone(),
                    path,
                    size_bytes,
                });
            }
            Ok(ModelArtifactStatus::Missing { .. })
            | Err(ModelRegistryError::ChecksumMismatch { .. }) => {}
            Err(err) => return Err(err),
        }

        let temp_path = install_temp_path(&destination_path)?;
        if let Err(err) = fs::copy(&source_path, &temp_path) {
            let _ = fs::remove_file(&temp_path);
            return Err(err.into());
        }

        let copied = match hash_file_sha256(&temp_path) {
            Ok(copied) => copied,
            Err(err) => {
                let _ = fs::remove_file(&temp_path);
                return Err(err.into());
            }
        };
        if copied != expected {
            let _ = fs::remove_file(&temp_path);
            return Err(ModelRegistryError::InstallChecksumMismatch {
                id: self.id.clone(),
                expected,
                actual: copied,
            });
        }

        fs::rename(&temp_path, &destination_path)?;
        let ModelArtifactStatus::Ready { path, size_bytes } = self.verify_artifact(models_dir)?
        else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "installed model artifact was not found after rename",
            )
            .into());
        };

        Ok(ModelInstallOutcome {
            id: self.id.clone(),
            path,
            size_bytes,
        })
    }

    fn validated_sources(&self) -> Result<Vec<String>, ModelRegistryError> {
        let mut sources = Vec::new();
        for source in &self.sources {
            let source = source.trim();
            if source.is_empty() || source.starts_with("TODO") {
                return Err(ModelRegistryError::PlaceholderSources {
                    id: self.id.clone(),
                });
            }
            if !is_https_download_source(source) {
                return Err(ModelRegistryError::InvalidDownloadSource {
                    id: self.id.clone(),
                    url: source.to_string(),
                });
            }
            if !sources.iter().any(|seen| seen == source) {
                sources.push(source.to_string());
            }
        }

        if sources.is_empty() {
            return Err(ModelRegistryError::PlaceholderSources {
                id: self.id.clone(),
            });
        }
        Ok(sources)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDownloadPlan {
    pub id: String,
    pub task: ModelTask,
    pub file: String,
    pub destination_path: PathBuf,
    pub sha256: String,
    pub size_mb: u64,
    pub license: String,
    pub license_review_required: bool,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInstallOutcome {
    pub id: String,
    pub path: PathBuf,
    pub size_bytes: u64,
}

fn is_sha256_hex(value: &str) -> bool {
    let value = value.trim();
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_https_download_source(source: &str) -> bool {
    source.starts_with(HTTPS_SCHEME_PREFIX)
        && !source.bytes().any(|byte| byte.is_ascii_whitespace())
        && !source.bytes().any(|byte| byte.is_ascii_control())
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

fn install_temp_path(destination_path: &Path) -> io::Result<PathBuf> {
    let parent = destination_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "model artifact destination has no parent directory",
        )
    })?;
    let file_name = destination_path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "model artifact destination has no filename",
        )
    })?;
    let file_name = file_name.to_string_lossy();

    for index in 0..1000 {
        let candidate = parent.join(format!(
            ".{file_name}.{}.{}.installing",
            std::process::id(),
            index
        ));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "too many temporary model install files",
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
            first_run_required: false,
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

    fn downloadable_entry(id: &str, file: &str) -> ModelEntry {
        let mut model = entry(id, file, sha256_for(b"expected"));
        let primary = format!("{}models.example.test/fixture.onnx", HTTPS_SCHEME_PREFIX);
        let mirror = format!("{}mirror.example.test/fixture.onnx", HTTPS_SCHEME_PREFIX);
        model.sources = vec![format!(" {primary} "), primary, mirror];
        model
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
        let results = registry
            .verify_required_first_run_models(&repo_models_dir(), Some("whisper-large-v3-turbo"));

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.id, "whisper-large-v3-turbo");
        assert!(results.iter().all(|(_, result)| matches!(
            result,
            Err(ModelRegistryError::PlaceholderChecksum { .. })
        )));
        assert!(results
            .iter()
            .any(|(model, _)| model.has_placeholder_sources()));
    }

    #[test]
    fn first_run_scope_includes_selected_asr_and_explicit_support_models_only() {
        let dir = tmp();
        let selected = entry("selected-asr", "selected.onnx", sha256_for(b"selected"));
        let other = entry("other-asr", "other.onnx", sha256_for(b"other"));
        let mut support = entry("required-vad", "vad.onnx", sha256_for(b"vad"));
        support.task = ModelTask::Vad;
        support.lane = None;
        support.first_run_required = true;
        let registry = ModelRegistry {
            schema_version: SUPPORTED_SCHEMA_VERSION,
            models: vec![selected, other, support],
        };

        let results = registry.verify_required_first_run_models(&dir, Some("selected-asr"));
        let ids = results
            .iter()
            .map(|(model, _)| model.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ids, ["selected-asr", "required-vad"]);
        let _ = std::fs::remove_dir_all(dir);
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
    fn download_plan_normalizes_sources_and_destination() {
        let dir = tmp();
        let model = downloadable_entry("fixture-asr", "fixture.onnx");

        let plan = model.download_plan(&dir).unwrap();

        assert_eq!(plan.id, "fixture-asr");
        assert_eq!(plan.task, ModelTask::Asr);
        assert_eq!(plan.file, "fixture.onnx");
        assert_eq!(plan.destination_path, dir.join("fixture.onnx"));
        assert_eq!(plan.sha256, sha256_for(b"expected"));
        assert_eq!(plan.size_mb, 1);
        assert_eq!(plan.license, "Apache-2.0");
        assert!(!plan.license_review_required);
        assert_eq!(
            plan.sources,
            vec![
                format!("{}models.example.test/fixture.onnx", HTTPS_SCHEME_PREFIX),
                format!("{}mirror.example.test/fixture.onnx", HTTPS_SCHEME_PREFIX)
            ]
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn registry_download_plan_requires_registered_model() {
        let dir = tmp();
        let registry = ModelRegistry {
            schema_version: SUPPORTED_SCHEMA_VERSION,
            models: vec![downloadable_entry("fixture-asr", "fixture.onnx")],
        };

        assert!(registry.download_plan("fixture-asr", &dir).is_ok());
        let err = registry.download_plan("missing", &dir).unwrap_err();

        assert!(matches!(err, ModelRegistryError::ModelNotFound(id) if id == "missing"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn download_plan_rejects_placeholder_checksum() {
        let dir = tmp();
        let mut model = downloadable_entry("fixture-asr", "fixture.onnx");
        model.sha256 = "TODO".to_string();

        let err = model.download_plan(&dir).unwrap_err();

        assert!(matches!(
            err,
            ModelRegistryError::PlaceholderChecksum { id } if id == "fixture-asr"
        ));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn download_plan_rejects_placeholder_sources() {
        let dir = tmp();
        for sources in [
            Vec::new(),
            vec!["TODO_primary".to_string()],
            vec![" ".to_string()],
        ] {
            let mut model = entry("fixture-asr", "fixture.onnx", sha256_for(b"expected"));
            model.sources = sources;

            let err = model.download_plan(&dir).unwrap_err();

            assert!(matches!(
                err,
                ModelRegistryError::PlaceholderSources { id } if id == "fixture-asr"
            ));
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn download_plan_rejects_non_https_or_unsafe_sources() {
        let dir = tmp();
        for source in [
            format!("{}models.example.test/fixture.onnx", concat!("http", "://")),
            format!("{}tmp/fixture.onnx", concat!("file", "://")),
            format!("{}models.example.test/fixture onnx", HTTPS_SCHEME_PREFIX),
        ] {
            let mut model = entry("fixture-asr", "fixture.onnx", sha256_for(b"expected"));
            model.sources = vec![source.clone()];

            let err = model.download_plan(&dir).unwrap_err();

            assert!(matches!(
                err,
                ModelRegistryError::InvalidDownloadSource { id, url: rejected }
                    if id == "fixture-asr" && rejected == source
            ));
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn installs_reviewed_local_artifact_into_model_store() {
        let dir = tmp();
        let source_dir = tmp();
        let bytes = b"reviewed local artifact";
        let model = entry("fixture-asr", "fixture.onnx", sha256_for(bytes));
        let source = source_dir.join("candidate.onnx");
        std::fs::write(&source, bytes).unwrap();

        let outcome = model.install_artifact_from_path(&source, &dir).unwrap();

        assert_eq!(
            outcome,
            ModelInstallOutcome {
                id: "fixture-asr".to_string(),
                path: dir.join("fixture.onnx"),
                size_bytes: bytes.len() as u64,
            }
        );
        assert_eq!(std::fs::read(dir.join("fixture.onnx")).unwrap(), bytes);
        assert_eq!(std::fs::read(source).unwrap(), bytes);
        assert_eq!(
            model.verify_artifact(&dir).unwrap(),
            ModelArtifactStatus::Ready {
                path: dir.join("fixture.onnx"),
                size_bytes: bytes.len() as u64,
            }
        );
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(source_dir);
    }

    #[test]
    fn install_refuses_unreviewed_checksum_without_copying() {
        let dir = tmp();
        let source_dir = tmp();
        let model = entry("fixture-asr", "fixture.onnx", sha256_for(b"expected"));
        let source = source_dir.join("candidate.onnx");
        std::fs::write(&source, b"actual").unwrap();

        let err = model.install_artifact_from_path(&source, &dir).unwrap_err();

        assert!(matches!(
            err,
            ModelRegistryError::InstallChecksumMismatch { id, actual, .. }
                if id == "fixture-asr" && actual == sha256_for(b"actual")
        ));
        assert!(!dir.join("fixture.onnx").exists());
        assert_eq!(std::fs::read(source).unwrap(), b"actual");
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(source_dir);
    }

    #[test]
    fn install_quarantines_bad_existing_artifact_before_replacement() {
        let dir = tmp();
        let source_dir = tmp();
        let model = entry("fixture-asr", "fixture.onnx", sha256_for(b"expected"));
        let destination = dir.join("fixture.onnx");
        std::fs::write(&destination, b"stale bad").unwrap();
        let source = source_dir.join("candidate.onnx");
        std::fs::write(&source, b"expected").unwrap();

        let outcome = model.install_artifact_from_path(&source, &dir).unwrap();

        assert_eq!(outcome.path, destination);
        assert_eq!(
            std::fs::read(dir.join("fixture.onnx")).unwrap(),
            b"expected"
        );
        let quarantined = std::fs::read_dir(dir.join(QUARANTINE_DIR_NAME))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(quarantined.len(), 1);
        assert_eq!(std::fs::read(quarantined[0].path()).unwrap(), b"stale bad");
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(source_dir);
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
