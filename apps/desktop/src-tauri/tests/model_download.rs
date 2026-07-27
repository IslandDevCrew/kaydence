//! Model download proof (P1-P0-2 / P1-P0-8, ADR-0015).
//!
//! Compiles only under `--features model-download`. Performs a REAL network
//! fetch of the registry-pinned default whisper model into a temp dir and
//! asserts it installs (sha256-verified). Opt-in only: without KAYDENCE_DOWNLOAD_TEST
//! it SKIPS loudly rather than hitting the network in a normal `cargo test`, so
//! the surface stays dormant unless a human asks for the proof (ADR-0015 §4).
//!
//!   KAYDENCE_DOWNLOAD_TEST=1 cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml \
//!     --features model-download --test model_download -- --nocapture
#![cfg(feature = "model-download")]

use kaydence_lib::models::{download, ModelRegistry};

const OPT_IN: &str = "KAYDENCE_DOWNLOAD_TEST";
const MODEL_ID: &str = "whisper-base-en-q5_1";

#[test]
fn downloads_and_verifies_the_pinned_default_model() {
    if std::env::var(OPT_IN).is_err() {
        eprintln!("SKIP: set {OPT_IN}=1 to run the real network download proof (ADR-0015).");
        return;
    }

    let registry_path = kaydence_lib::models::source_tree_registry_path();
    let registry = ModelRegistry::load(&registry_path).expect("registry loads");
    let entry = registry.require(MODEL_ID).expect("model is registered");

    let tmp = std::env::temp_dir().join(format!("kayd-dl-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);

    let outcome = download::download_and_install(entry, &tmp)
        .expect("download + sha256 verify + install must succeed on a pinned source");

    eprintln!(
        "installed {} ({} bytes) at {}",
        outcome.id,
        outcome.size_bytes,
        outcome.path.display()
    );
    assert!(outcome.path.is_file(), "installed artifact must exist");
    assert!(outcome.size_bytes > 1_000_000, "q5_1 model is ~57 MB");

    // A second call is a no-op fast path: the verified artifact already exists.
    let again = download::download_and_install(entry, &tmp).expect("re-install verifies existing");
    assert_eq!(again.path, outcome.path);

    let _ = std::fs::remove_dir_all(&tmp);
}
