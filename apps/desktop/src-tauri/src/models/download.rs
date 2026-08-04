//! Model download-on-demand (P1-P0-2 / P1-P0-8, ADR-0017) — feature `model-download`.
//!
//! The ONE network surface behind model acquisition. Everything else in `models/`
//! stays download-free. This module consumes the registry's validated
//! `ModelDownloadPlan` (HTTPS-only, checksum-pinned sources) and installs through
//! the existing verify-before-install path, so a corrupt or wrong file is hashed
//! and rejected, never loaded. Off by default; compiled only under
//! `--features model-download`. No telemetry, no implicit/background fetch — the
//! caller invokes this on an explicit user action only (ADR-0017 §4).

use super::{ModelEntry, ModelInstallOutcome, ModelRegistryError};
use std::fs;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum ModelDownloadError {
    #[error("model {id} requires a license review before it can be downloaded")]
    LicenseReviewRequired { id: String },
    #[error("registry: {0}")]
    Registry(#[from] ModelRegistryError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("all {tried} pinned source(s) failed for model {id}; last error: {last}")]
    AllSourcesFailed {
        id: String,
        tried: usize,
        last: String,
    },
}

/// Download `entry`'s artifact from its pinned HTTPS sources and install it,
/// verifying sha256 before the file is accepted. Tries sources in registry order;
/// a fetch failure or checksum mismatch falls through to the next source. Returns
/// the installed artifact on the first source that both downloads and verifies.
pub fn download_and_install(
    entry: &ModelEntry,
    models_dir: &Path,
) -> Result<ModelInstallOutcome, ModelDownloadError> {
    // License-gated models (e.g. Gemma) are never fetched by this surface until
    // their review clears (ADR-0007/0010). The plan carries the flag; refuse here.
    if entry.license_review_required {
        return Err(ModelDownloadError::LicenseReviewRequired {
            id: entry.id.clone(),
        });
    }

    // Building the plan validates the checksum is real and the sources are HTTPS.
    let plan = entry.download_plan(models_dir)?;
    fs::create_dir_all(models_dir)?;

    // Download to a temp path next to the destination so install can canonicalize
    // it, then hand off to the checksum-verifying installer.
    let tmp = plan.destination_path.with_extension("part");
    let mut last_error = String::from("no sources attempted");

    for url in &plan.sources {
        match fetch_to_path(url, &tmp) {
            Ok(()) => match entry.install_artifact_from_path(&tmp, models_dir) {
                Ok(outcome) => {
                    let _ = fs::remove_file(&tmp);
                    return Ok(outcome);
                }
                Err(e) => {
                    // Checksum mismatch or install failure — discard and try next.
                    let _ = fs::remove_file(&tmp);
                    last_error = format!("{url}: {e}");
                }
            },
            Err(e) => {
                let _ = fs::remove_file(&tmp);
                last_error = format!("{url}: {e}");
            }
        }
    }

    Err(ModelDownloadError::AllSourcesFailed {
        id: entry.id.clone(),
        tried: plan.sources.len(),
        last: last_error,
    })
}

/// GET a pinned HTTPS URL and stream it to `dest`. ureq (rustls) blocking client.
fn fetch_to_path(url: &str, dest: &Path) -> Result<(), String> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| format!("request failed: {e}"))?;
    let mut reader = response.into_reader();
    let mut file = fs::File::create(dest).map_err(|e| format!("create {dest:?}: {e}"))?;
    std::io::copy(&mut reader, &mut file).map_err(|e| format!("write {dest:?}: {e}"))?;
    Ok(())
}
