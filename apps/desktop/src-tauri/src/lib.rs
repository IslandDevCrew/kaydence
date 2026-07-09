//! Kaydence backend library — the real product (root `AGENTS.md`).
//!
//! Wires the pipeline modules and the Tauri app shell. The runtime path is:
//! hotkeys -> audio(+VAD) -> engine -> cleanup ->
//! dictionary -> profiles -> inject -> history, with prediction/context as a
//! parallel local-only layer. Stages couple only through `events::SessionEvent`.

#[cfg(desktop)]
use std::path::{Path, PathBuf};
#[cfg(desktop)]
use std::sync::Arc;
use std::sync::Mutex;
#[cfg(desktop)]
use std::time::{Duration, Instant};

pub mod events;

pub mod audio;
pub mod cleanup;
pub mod context;
pub mod dictionary;
pub mod engine;
pub mod history;
pub mod hotkeys;
pub mod inject;
pub mod models;
pub mod pipeline;
pub mod prediction;
pub mod profiles;
pub mod settings;

/// Initial app/config snapshot for the presentation layer.
#[tauri::command]
fn app_snapshot(state: tauri::State<'_, RuntimeSnapshot>) -> settings::AppSnapshot {
    state.snapshot()
}

#[tauri::command]
fn select_asr_model(
    model_id: String,
    state: tauri::State<'_, RuntimeSnapshot>,
) -> Result<settings::AppSnapshot, String> {
    state
        .select_asr_model(&model_id)
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn refresh_model_readiness(
    app: tauri::AppHandle,
    state: tauri::State<'_, RuntimeSnapshot>,
) -> Result<settings::AppSnapshot, String> {
    #[cfg(desktop)]
    {
        use tauri::Manager;

        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|err| format!("App data directory unavailable: {err}"))?;
        state
            .refresh_models_from_app_data(&app_data_dir)
            .map_err(|err| err.to_string())
    }

    #[cfg(not(desktop))]
    {
        Ok(state.snapshot())
    }
}

#[tauri::command]
fn recent_history(
    app: tauri::AppHandle,
    state: tauri::State<'_, RuntimeSnapshot>,
    limit: Option<usize>,
) -> Result<Vec<history::HistorySession>, String> {
    #[cfg(desktop)]
    {
        use tauri::Manager;

        let retention_days = state.snapshot().settings.privacy.history_retention_days;
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|err| format!("App data directory unavailable: {err}"))?;
        let mut store =
            history::HistoryStore::open(&app_data_dir).map_err(|err| err.to_string())?;
        store
            .sweep_retention(retention_days, &app_data_dir)
            .map_err(|err| err.to_string())?;
        recover_history_audio(&mut store, &app_data_dir).map_err(|err| err.to_string())?;
        store
            .list_recent(limit.unwrap_or(5).clamp(1, 20))
            .map_err(|err| err.to_string())
    }

    #[cfg(not(desktop))]
    {
        let _ = app;
        let _ = state;
        let _ = limit;
        Ok(Vec::new())
    }
}

#[cfg(desktop)]
fn recover_history_audio(
    store: &mut history::HistoryStore,
    app_data_dir: &Path,
) -> Result<usize, HistoryRecoveryError> {
    let sessions_dir = app_data_dir.join("sessions");
    if !sessions_dir.exists() {
        return Ok(0);
    }

    let mut surfaced = 0;
    for entry in std::fs::read_dir(&sessions_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("wav") {
            continue;
        };
        let Some(session_id) = session_id_from_wal_path(&path) else {
            continue;
        };
        if store.session_has_audio(session_id)? {
            continue;
        }
        let Ok(recovered) = audio::wal::recover(&path) else {
            continue;
        };
        if store.record_recovered_audio(session_id, &recovered.path, recovered.samples)? {
            surfaced += 1;
        }
    }
    Ok(surfaced)
}

#[cfg(desktop)]
fn session_id_from_wal_path(path: &Path) -> Option<events::SessionId> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem| ulid::Ulid::from_string(stem).ok())
        .map(events::SessionId::new)
}

#[cfg(desktop)]
#[derive(Debug, thiserror::Error)]
enum HistoryRecoveryError {
    #[error("history recovery io: {0}")]
    Io(#[from] std::io::Error),
    #[error("audio recovery: {0}")]
    Wal(#[from] audio::wal::WalError),
    #[error("history: {0}")]
    History(#[from] history::HistoryError),
}

#[tauri::command]
fn delete_history_session(
    app: tauri::AppHandle,
    session_id: String,
) -> Result<Vec<history::HistorySession>, String> {
    #[cfg(desktop)]
    {
        use tauri::Manager;

        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|err| format!("App data directory unavailable: {err}"))?;
        let mut store =
            history::HistoryStore::open(&app_data_dir).map_err(|err| err.to_string())?;
        store
            .delete_session_and_audio(&session_id, &app_data_dir)
            .map_err(|err| err.to_string())?;
        store.list_recent(4).map_err(|err| err.to_string())
    }

    #[cfg(not(desktop))]
    {
        let _ = app;
        let _ = session_id;
        Ok(Vec::new())
    }
}

#[tauri::command]
fn export_history_session(
    app: tauri::AppHandle,
    session_id: String,
) -> Result<history::ExportSessionOutcome, String> {
    #[cfg(desktop)]
    {
        use tauri::Manager;

        let session_id = parse_history_session_id(&session_id)?;
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|err| format!("App data directory unavailable: {err}"))?;
        let mut store =
            history::HistoryStore::open(&app_data_dir).map_err(|err| err.to_string())?;
        recover_history_audio(&mut store, &app_data_dir).map_err(|err| err.to_string())?;
        store
            .export_session(session_id, &app_data_dir)
            .map_err(|err| err.to_string())
    }

    #[cfg(not(desktop))]
    {
        let _ = app;
        let _ = session_id;
        Ok(history::ExportSessionOutcome {
            exported: false,
            json_path: None,
            text_path: None,
        })
    }
}

#[tauri::command]
fn play_history_audio(
    app: tauri::AppHandle,
    session_id: String,
) -> Result<Option<history::HistoryAudioPlayback>, String> {
    #[cfg(desktop)]
    {
        use tauri::Manager;

        let session_id = parse_history_session_id(&session_id)?;
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|err| format!("App data directory unavailable: {err}"))?;
        let mut store =
            history::HistoryStore::open(&app_data_dir).map_err(|err| err.to_string())?;
        recover_history_audio(&mut store, &app_data_dir).map_err(|err| err.to_string())?;
        store
            .audio_playback(session_id, &app_data_dir)
            .map_err(|err| err.to_string())
    }

    #[cfg(not(desktop))]
    {
        let _ = app;
        let _ = session_id;
        Ok(None)
    }
}

#[tauri::command]
fn purge_history(app: tauri::AppHandle) -> Result<history::PurgeHistoryOutcome, String> {
    #[cfg(desktop)]
    {
        use tauri::Manager;

        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|err| format!("App data directory unavailable: {err}"))?;
        let mut store =
            history::HistoryStore::open(&app_data_dir).map_err(|err| err.to_string())?;
        store
            .purge_all(&app_data_dir)
            .map_err(|err| err.to_string())
    }

    #[cfg(not(desktop))]
    {
        let _ = app;
        Ok(history::PurgeHistoryOutcome {
            sessions_deleted: 0,
            audio_files_removed: 0,
            export_files_removed: 0,
        })
    }
}

fn parse_history_session_id(session_id: &str) -> Result<events::SessionId, String> {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        return Err("History session id cannot be empty".to_string());
    }
    ulid::Ulid::from_string(session_id)
        .map(events::SessionId::new)
        .map_err(|_| format!("Invalid history session id: {session_id}"))
}

#[derive(Debug)]
struct RuntimeSnapshot {
    inner: Mutex<settings::AppSnapshot>,
    settings_store: Mutex<Option<settings::SettingsStore>>,
}

impl Default for RuntimeSnapshot {
    fn default() -> Self {
        Self {
            inner: Mutex::new(settings::AppSnapshot::default()),
            settings_store: Mutex::new(None),
        }
    }
}

impl RuntimeSnapshot {
    fn snapshot(&self) -> settings::AppSnapshot {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    #[cfg(desktop)]
    fn mark_hotkey_registered(&self) {
        self.update_first_run(|first_run| {
            first_run.hotkey_registered = true;
            first_run.hotkey_registration_error = None;
        });
    }

    #[cfg(desktop)]
    fn mark_hotkey_registration_failed(&self, error: String) {
        self.update_first_run(|first_run| {
            first_run.hotkey_registered = false;
            first_run.hotkey_registration_error = Some(error);
        });
    }

    #[cfg(desktop)]
    fn set_settings_store(&self, app_data_dir: &Path) {
        *self
            .settings_store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) =
            Some(settings::SettingsStore::new(app_data_dir));
    }

    #[cfg(desktop)]
    fn apply_persisted_user_settings(&self) -> Result<(), settings::SettingsStoreError> {
        let Some(store) = self.settings_store() else {
            return Ok(());
        };
        let settings = store.load()?;
        if let Some(model_id) = settings.selected_asr_model_id {
            let _ = self.apply_asr_selection(&model_id);
        }
        Ok(())
    }

    #[cfg(desktop)]
    fn refresh_model_readiness(&self, registry_path: &Path, models_dir: &Path) {
        let readiness = first_run_model_readiness(registry_path, models_dir);
        self.update_first_run(|first_run| {
            first_run.model_ready = readiness.model_ready;
            first_run.model_readiness_error = readiness.model_readiness_error;
            first_run.required_models = readiness.required_models;
            first_run.asr_candidates = readiness.asr_candidates;
            first_run.recommended_asr_model_id = readiness.recommended_asr_model_id;
            first_run.selected_asr_model_id = readiness.selected_asr_model_id;
        });
    }

    #[cfg(desktop)]
    fn refresh_models(
        &self,
        registry_path: &Path,
        app_data_dir: &Path,
    ) -> Result<settings::AppSnapshot, settings::SettingsStoreError> {
        self.set_settings_store(app_data_dir);
        self.refresh_model_readiness(registry_path, &app_data_dir.join("models"));
        self.apply_persisted_user_settings()?;
        Ok(self.snapshot())
    }

    #[cfg(desktop)]
    fn refresh_models_from_app_data(
        &self,
        app_data_dir: &Path,
    ) -> Result<settings::AppSnapshot, settings::SettingsStoreError> {
        self.refresh_models(&models::source_tree_registry_path(), app_data_dir)
    }

    #[cfg(desktop)]
    fn mark_model_readiness_failed(&self, error: String) {
        self.update_first_run(|first_run| {
            first_run.model_ready = false;
            first_run.model_readiness_error = Some(error);
            first_run.required_models.clear();
            first_run.asr_candidates.clear();
            first_run.recommended_asr_model_id = None;
            first_run.selected_asr_model_id = None;
        });
    }

    fn select_asr_model(&self, model_id: &str) -> Result<settings::AppSnapshot, SelectModelError> {
        let model_id = model_id.trim();
        if model_id.is_empty() {
            return Err(SelectModelError::EmptyModelId);
        }
        if !self.has_asr_candidate(model_id) {
            return Err(SelectModelError::UnknownModel(model_id.to_string()));
        }

        if let Some(store) = self.settings_store() {
            let mut settings = store.load()?;
            settings.selected_asr_model_id = Some(model_id.to_string());
            store.save(&settings)?;
        }

        self.apply_asr_selection(model_id)
            .ok_or_else(|| SelectModelError::UnknownModel(model_id.to_string()))?;
        Ok(self.snapshot())
    }

    fn has_asr_candidate(&self, model_id: &str) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .settings
            .first_run
            .asr_candidates
            .iter()
            .any(|candidate| candidate.id == model_id)
    }

    fn apply_asr_selection(&self, model_id: &str) -> Option<()> {
        let mut snapshot = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let first_run = &mut snapshot.settings.first_run;
        if !first_run
            .asr_candidates
            .iter()
            .any(|candidate| candidate.id == model_id)
        {
            return None;
        }

        first_run.selected_asr_model_id = Some(model_id.to_string());
        for candidate in &mut first_run.asr_candidates {
            candidate.selected = candidate.id == model_id;
        }
        Some(())
    }

    fn settings_store(&self) -> Option<settings::SettingsStore> {
        self.settings_store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    #[cfg(desktop)]
    fn update_first_run(&self, update: impl FnOnce(&mut settings::FirstRunStatus)) {
        let mut snapshot = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        update(&mut snapshot.settings.first_run);
    }
}

#[derive(Debug, thiserror::Error)]
enum SelectModelError {
    #[error("ASR model id cannot be empty")]
    EmptyModelId,
    #[error("unknown ASR model id: {0}")]
    UnknownModel(String),
    #[error("settings store: {0}")]
    SettingsStore(#[from] settings::SettingsStoreError),
}

#[cfg(desktop)]
struct FirstRunModelReadiness {
    model_ready: bool,
    model_readiness_error: Option<String>,
    required_models: Vec<settings::FirstRunModelStatus>,
    asr_candidates: Vec<settings::FirstRunAsrCandidate>,
    recommended_asr_model_id: Option<String>,
    selected_asr_model_id: Option<String>,
}

#[cfg(desktop)]
fn first_run_model_readiness(registry_path: &Path, models_dir: &Path) -> FirstRunModelReadiness {
    let registry = match models::ModelRegistry::load(registry_path) {
        Ok(registry) => registry,
        Err(err) => {
            return FirstRunModelReadiness {
                model_ready: false,
                model_readiness_error: Some(format!("Model registry unavailable: {err}")),
                required_models: Vec::new(),
                asr_candidates: Vec::new(),
                recommended_asr_model_id: None,
                selected_asr_model_id: None,
            };
        }
    };
    let platform_tag = first_run_platform_tag();
    let recommended_asr = first_run_asr_recommendation(&registry, platform_tag);
    let recommended_asr_model_id = recommended_asr.map(|model| model.id.clone());
    let selected_asr_model_id = recommended_asr_model_id.clone();

    let required_models = registry
        .verify_required_first_run_models(models_dir)
        .into_iter()
        .map(|(model, status)| first_run_model_status(model, status, models_dir))
        .collect::<Vec<_>>();
    let asr_candidates = registry
        .recommended_for(models::ModelTask::Asr)
        .into_iter()
        .map(|model| {
            first_run_asr_candidate(
                model,
                model.verify_artifact(models_dir),
                models_dir,
                recommended_asr_model_id.as_deref(),
                platform_tag,
            )
        })
        .collect::<Vec<_>>();

    let model_ready = !required_models.is_empty()
        && required_models
            .iter()
            .all(|model| model.state == settings::FirstRunModelState::Ready);
    let model_readiness_error = model_readiness_error(&required_models);

    FirstRunModelReadiness {
        model_ready,
        model_readiness_error,
        required_models,
        asr_candidates,
        recommended_asr_model_id,
        selected_asr_model_id,
    }
}

#[cfg(desktop)]
fn first_run_model_status(
    model: &models::ModelEntry,
    status: Result<models::ModelArtifactStatus, models::ModelRegistryError>,
    models_dir: &Path,
) -> settings::FirstRunModelStatus {
    let download_plan = model.download_plan(models_dir);
    let (state, detail) = match status {
        Ok(models::ModelArtifactStatus::Ready { size_bytes, .. }) => (
            settings::FirstRunModelState::Ready,
            format!("Verified artifact, {} MB on disk", bytes_to_mb(size_bytes)),
        ),
        Ok(models::ModelArtifactStatus::Missing { .. }) => match &download_plan {
            Ok(_) => (
                settings::FirstRunModelState::Missing,
                "Download required before first dictation".to_string(),
            ),
            Err(err) => (
                settings::FirstRunModelState::Blocked,
                format!("Download unavailable: {err}"),
            ),
        },
        Err(models::ModelRegistryError::PlaceholderChecksum { .. }) => (
            settings::FirstRunModelState::Blocked,
            "Registry checksum pending".to_string(),
        ),
        Err(models::ModelRegistryError::ChecksumMismatch { quarantined_to, .. }) => (
            settings::FirstRunModelState::Blocked,
            format!(
                "Checksum mismatch; quarantined at {}",
                quarantined_to.display()
            ),
        ),
        Err(err) => (
            settings::FirstRunModelState::Blocked,
            format!("Registry error: {err}"),
        ),
    };

    settings::FirstRunModelStatus {
        id: model.id.clone(),
        task: model_task_label(model.task).to_string(),
        lane: model.lane.clone(),
        runtime: model.runtime.clone(),
        file: model.file.clone(),
        state,
        detail,
        download_available: download_plan.is_ok(),
        download_size_mb: download_plan.as_ref().ok().map(|plan| plan.size_mb),
        download_source_count: download_plan
            .as_ref()
            .ok()
            .map(|plan| plan.sources.len().min(usize::from(u16::MAX)) as u16)
            .unwrap_or(0),
        license: model.license.clone(),
        license_review_required: model.license_review_required,
    }
}

#[cfg(desktop)]
fn first_run_asr_candidate(
    model: &models::ModelEntry,
    status: Result<models::ModelArtifactStatus, models::ModelRegistryError>,
    models_dir: &Path,
    recommended_asr_model_id: Option<&str>,
    platform_tag: &str,
) -> settings::FirstRunAsrCandidate {
    let base = first_run_model_status(model, status, models_dir);
    let selected = recommended_asr_model_id == Some(model.id.as_str());

    settings::FirstRunAsrCandidate {
        id: model.id.clone(),
        lane: model.lane.clone(),
        runtime: model.runtime.clone(),
        size_mb: model.size_mb,
        min_hw: model.min_hw.clone(),
        state: base.state,
        detail: base.detail,
        download_available: base.download_available,
        download_size_mb: base.download_size_mb,
        download_source_count: base.download_source_count,
        selected,
        recommendation: selected.then(|| recommendation_reason(model, platform_tag).to_string()),
        license: model.license.clone(),
        license_review_required: model.license_review_required,
    }
}

#[cfg(desktop)]
fn model_readiness_error(models: &[settings::FirstRunModelStatus]) -> Option<String> {
    if models.is_empty() {
        return Some("No required first-run models are registered".to_string());
    }

    if models
        .iter()
        .any(|model| model.state == settings::FirstRunModelState::Blocked)
    {
        return Some(
            "Model registry needs verified metadata, checksums, or artifact repair".to_string(),
        );
    }

    None
}

#[cfg(desktop)]
fn first_run_asr_recommendation<'a>(
    registry: &'a models::ModelRegistry,
    platform_tag: &str,
) -> Option<&'a models::ModelEntry> {
    let candidates = registry.recommended_for(models::ModelTask::Asr);
    candidates
        .iter()
        .copied()
        .find(|model| model.default_for.iter().any(|tag| tag == platform_tag))
        .or_else(|| {
            candidates.iter().copied().find(|model| {
                model.lane.as_deref() == Some("cpu") || model.min_hw.eq_ignore_ascii_case("any")
            })
        })
        .or_else(|| candidates.first().copied())
}

#[cfg(desktop)]
fn recommendation_reason(model: &models::ModelEntry, platform_tag: &str) -> &'static str {
    if model.default_for.iter().any(|tag| tag == platform_tag) {
        "Recommended for this OS lane"
    } else if model.lane.as_deref() == Some("cpu") || model.min_hw.eq_ignore_ascii_case("any") {
        "CPU-safe first-run default"
    } else {
        "Recommended local ASR option"
    }
}

#[cfg(desktop)]
fn first_run_platform_tag() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "desktop"
    }
}

#[cfg(desktop)]
fn bytes_to_mb(size_bytes: u64) -> u64 {
    size_bytes.div_ceil(1024 * 1024)
}

#[cfg(desktop)]
fn model_task_label(task: models::ModelTask) -> &'static str {
    match task {
        models::ModelTask::Asr => "ASR",
        models::ModelTask::Vad => "VAD",
        models::ModelTask::Cleanup => "Cleanup",
        models::ModelTask::Prediction => "Prediction",
    }
}

#[cfg(desktop)]
struct HotkeyRuntime {
    coordinator: hotkeys::CaptureCoordinator,
    recorder: audio::WalCaptureRuntime,
    history: history::HistoryStore,
    target_resolver: Box<dyn profiles::ResolveSessionTarget + Send>,
    active_target: Option<profiles::SessionTarget>,
    processor: Box<dyn pipeline::CaptureProcessor + Send>,
    injector: Box<dyn inject::TextInjector + Send>,
    unknown_field_policy: inject::UnknownFieldPolicy,
    prefer_clipboard: bool,
}

#[cfg(desktop)]
#[derive(Debug, thiserror::Error)]
enum HotkeyRuntimeError {
    #[error("audio: {0}")]
    Audio(#[from] audio::CaptureRuntimeError),
    #[error("history: {0}")]
    History(#[from] history::HistoryError),
    #[error("pipeline: {0}")]
    Pipeline(#[from] pipeline::PipelineError),
}

#[cfg(desktop)]
impl HotkeyRuntime {
    fn new(
        app_data_dir: impl Into<PathBuf>,
        first_run: &settings::FirstRunStatus,
    ) -> Result<Self, HotkeyRuntimeError> {
        let app_data_dir = app_data_dir.into();
        let recorder = audio::WalCaptureRuntime::new(app_data_dir.clone());
        let history = history::HistoryStore::open(&app_data_dir)?;
        let asr_context = selected_asr_runtime_context(first_run);
        Ok(Self::with_recorder(
            recorder,
            history,
            Box::new(profiles::platform_target_resolver()),
            Box::new(pipeline::default_runtime_pipeline(
                asr_context.model_id.as_deref(),
                asr_context.lane.as_deref(),
            )),
            Box::new(inject::platform_injector()),
        ))
    }

    #[cfg(test)]
    fn new_wal_only(app_data_dir: impl Into<PathBuf>) -> Result<Self, HotkeyRuntimeError> {
        let app_data_dir = app_data_dir.into();
        let recorder = audio::WalCaptureRuntime::new_wal_only(app_data_dir.clone());
        let history = history::HistoryStore::open(&app_data_dir)?;
        Ok(Self::with_recorder(
            recorder,
            history,
            Box::new(profiles::platform_target_resolver()),
            Box::new(pipeline::default_runtime_pipeline(None, None)),
            Box::new(inject::platform_injector()),
        ))
    }

    fn with_recorder(
        recorder: audio::WalCaptureRuntime,
        history: history::HistoryStore,
        target_resolver: Box<dyn profiles::ResolveSessionTarget + Send>,
        processor: Box<dyn pipeline::CaptureProcessor + Send>,
        injector: Box<dyn inject::TextInjector + Send>,
    ) -> Self {
        Self {
            coordinator: hotkeys::CaptureCoordinator::new(
                hotkeys::HotkeyMode::PushToTalk,
                hotkeys::CaptureConfig::default(),
            ),
            recorder,
            history,
            target_resolver,
            active_target: None,
            processor,
            injector,
            unknown_field_policy: inject::UnknownFieldPolicy::default(),
            prefer_clipboard: false,
        }
    }

    #[cfg(test)]
    fn with_target_resolver(
        mut self,
        target_resolver: Box<dyn profiles::ResolveSessionTarget + Send>,
    ) -> Self {
        self.target_resolver = target_resolver;
        self
    }

    #[cfg(test)]
    fn with_processor(mut self, processor: Box<dyn pipeline::CaptureProcessor + Send>) -> Self {
        self.processor = processor;
        self
    }

    #[cfg(test)]
    fn with_injector(mut self, injector: Box<dyn inject::TextInjector + Send>) -> Self {
        self.injector = injector;
        self
    }

    fn handle_signal(
        &mut self,
        signal: hotkeys::Signal,
    ) -> Result<Option<u64>, HotkeyRuntimeError> {
        let was_finalizing = matches!(
            self.coordinator.state(),
            hotkeys::CaptureState::Finalizing { .. }
        );
        let at_ms = signal_at_ms(signal);
        let action = self.coordinator.step(signal);

        match action {
            hotkeys::Action::None => {}
            hotkeys::Action::StartCapture => {
                let id = match self.recorder.start_capture(at_ms) {
                    Ok(id) => id,
                    Err(err) => {
                        self.coordinator.reset();
                        return Err(err.into());
                    }
                };
                let target = self.target_resolver.resolve_session_target();
                let started = events::SessionEvent::Started {
                    id,
                    target_app: target.app.clone(),
                    at_ms,
                };
                if let Err(err) = self.history.record_event(&started) {
                    let _ = self.recorder.discard_capture(at_ms);
                    self.coordinator.reset();
                    return Err(err.into());
                }
                self.active_target = Some(target.clone());
                println!(
                    "Kaydence capture started: id={:?} target_app={} profile={} sessions_dir={}",
                    id,
                    target.app.name,
                    target.profile.id,
                    self.recorder.sessions_dir().display()
                );
            }
            hotkeys::Action::FinalizeCapture => {
                let summary = self.recorder.finalize_capture(at_ms)?;
                let bound_target = self.active_target.take();
                if let Some(target) = &bound_target {
                    self.processor.set_cleanup_dial(target.profile.cleanup_dial);
                }
                let events = self.processor.process_capture(&summary)?;
                let committed = pipeline::committed_text(&events);
                self.history.record_events(&events)?;
                let injection_event = committed.map(|committed| match &bound_target {
                    Some(bound) => {
                        let current = self.target_resolver.resolve_session_target();
                        match inject::verify_focus_binding(
                            &focus_target(bound),
                            &focus_target(&current),
                        ) {
                            Ok(()) => inject::inject_committed_text(
                                self.injector.as_mut(),
                                committed.id,
                                &committed.text,
                                self.unknown_field_policy,
                                self.prefer_clipboard,
                            ),
                            Err(reason) => events::SessionEvent::Held {
                                id: committed.id,
                                reason,
                            },
                        }
                    }
                    None => events::SessionEvent::Held {
                        id: committed.id,
                        reason: events::HoldReason::FocusChanged,
                    },
                });
                if let Some(event) = injection_event {
                    self.history.record_event(&event)?;
                    println!("Kaydence injection outcome: event={event:?}");
                }
                println!(
                    "Kaydence capture processed: id={:?} events={} samples={} started_ms={} finalized_ms={}",
                    summary.id,
                    events.len(),
                    summary.samples_written,
                    summary.started_ms,
                    summary.finalized_ms
                );
            }
            hotkeys::Action::DiscardCapture => {
                let discarded = self.recorder.discard_capture(at_ms)?;
                self.active_target = None;
                self.history.delete_session(discarded.id)?;
                println!(
                    "Kaydence capture discarded: id={:?} path={} started_ms={} discarded_ms={} removed={}",
                    discarded.id,
                    discarded.wal_path.display(),
                    discarded.started_ms,
                    discarded.discarded_ms,
                    discarded.removed
                );
            }
        }

        if !was_finalizing {
            if let hotkeys::CaptureState::Finalizing { ends_ms, .. } = self.coordinator.state() {
                return Ok(Some(ends_ms));
            }
        }
        Ok(None)
    }
}

#[cfg(desktop)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct AsrRuntimeContext {
    model_id: Option<String>,
    lane: Option<String>,
}

#[cfg(desktop)]
fn selected_asr_runtime_context(first_run: &settings::FirstRunStatus) -> AsrRuntimeContext {
    let model_id = first_run.selected_asr_model_id.clone();
    let lane = model_id.as_deref().and_then(|model_id| {
        first_run
            .asr_candidates
            .iter()
            .find(|candidate| candidate.id == model_id)
            .and_then(|candidate| candidate.lane.clone())
    });

    AsrRuntimeContext { model_id, lane }
}

#[cfg(desktop)]
fn focus_target(target: &profiles::SessionTarget) -> inject::FocusTarget {
    match target.source {
        profiles::TargetSource::Detected => inject::FocusTarget::detected(target.app.clone()),
        profiles::TargetSource::Unknown => inject::FocusTarget::unknown(target.app.clone()),
    }
}

#[cfg(desktop)]
fn signal_at_ms(signal: hotkeys::Signal) -> u64 {
    match signal {
        hotkeys::Signal::Press { at_ms }
        | hotkeys::Signal::Release { at_ms }
        | hotkeys::Signal::Tick { at_ms } => at_ms,
    }
}

#[cfg(desktop)]
fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

#[cfg(desktop)]
fn schedule_tail_tick(runtime: &Arc<Mutex<HotkeyRuntime>>, started: Instant, ends_ms: u64) {
    let runtime = Arc::clone(runtime);
    std::thread::spawn(move || {
        let sleep_ms = ends_ms.saturating_sub(elapsed_ms(started));
        if sleep_ms > 0 {
            std::thread::sleep(Duration::from_millis(sleep_ms));
        }
        let at_ms = elapsed_ms(started);
        match runtime.lock() {
            Ok(mut runtime) => {
                if let Err(err) = runtime.handle_signal(hotkeys::Signal::Tick { at_ms }) {
                    eprintln!("Kaydence hotkey tail tick failed: {err}");
                }
            }
            Err(_) => {
                eprintln!("Kaydence hotkey runtime lock poisoned during tail tick");
            }
        }
    });
}

#[cfg(desktop)]
fn install_global_hotkey(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use hotkeys::Signal;
    use tauri::Manager;
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};

    let shortcut = Shortcut::new(None, Code::AltRight);
    let app_data_dir = app.path().app_data_dir()?;
    let first_run = app.state::<RuntimeSnapshot>().snapshot().settings.first_run;
    let runtime = Arc::new(Mutex::new(HotkeyRuntime::new(app_data_dir, &first_run)?));
    let started = Instant::now();
    let handler_runtime = Arc::clone(&runtime);

    app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |_app, observed, event| {
                if observed != &shortcut {
                    return;
                }

                let at_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                let signal = match event.state() {
                    ShortcutState::Pressed => Signal::Press { at_ms },
                    ShortcutState::Released => Signal::Release { at_ms },
                };

                let tail_wake_ms = match handler_runtime.lock() {
                    Ok(mut runtime) => match runtime.handle_signal(signal) {
                        Ok(tail_wake_ms) => tail_wake_ms,
                        Err(err) => {
                            eprintln!("Kaydence hotkey runtime failed: {err}");
                            None
                        }
                    },
                    Err(_) => {
                        eprintln!("Kaydence hotkey runtime lock poisoned");
                        None
                    }
                };

                if let Some(ends_ms) = tail_wake_ms {
                    schedule_tail_tick(&handler_runtime, started, ends_ms);
                }
            })
            .build(),
    )?;

    app.global_shortcut().register(shortcut)?;
    app.state::<RuntimeSnapshot>().mark_hotkey_registered();
    Ok(())
}

/// Run the Kaydence desktop app. Called by the thin `main.rs` binary.
///
/// Run the Tauri shell plus the current hotkey/audio/pipeline runtime.
pub fn run() {
    tauri::Builder::default()
        .manage(RuntimeSnapshot::default())
        .invoke_handler(tauri::generate_handler![
            app_snapshot,
            select_asr_model,
            refresh_model_readiness,
            recent_history,
            delete_history_session,
            export_history_session,
            play_history_audio,
            purge_history
        ])
        .setup(|app| {
            #[cfg(desktop)]
            {
                use tauri::Manager;

                let snapshot = app.state::<RuntimeSnapshot>();
                match app.path().app_data_dir() {
                    Ok(app_data_dir) => {
                        if let Err(err) = snapshot.refresh_models_from_app_data(&app_data_dir) {
                            eprintln!("Kaydence settings load failed: {err}");
                        }
                    }
                    Err(err) => snapshot.mark_model_readiness_failed(format!(
                        "App data directory unavailable: {err}"
                    )),
                }

                if let Err(err) = install_global_hotkey(app) {
                    app.state::<RuntimeSnapshot>()
                        .mark_hotkey_registration_failed(err.to_string());
                    eprintln!("Kaydence global hotkey disabled: {err}");
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Kaydence");
}

#[cfg(all(test, desktop))]
mod tests {
    use super::*;
    use crate::events::{CleanupDial, HoldReason, InjectMethod};
    use sha2::{Digest, Sha256};
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    fn tmp() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kaydence-hotkey-runtime-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn detected_app() -> events::AppRef {
        events::AppRef {
            id: "com.example.editor".to_string(),
            name: "Example Editor".to_string(),
        }
    }

    fn other_app() -> events::AppRef {
        events::AppRef {
            id: "com.example.mail".to_string(),
            name: "Example Mail".to_string(),
        }
    }

    fn session_target(app: events::AppRef, cleanup_dial: CleanupDial) -> profiles::SessionTarget {
        profiles::SessionTarget {
            app: app.clone(),
            profile: profiles::AppProfile::user_edited(
                "test-profile",
                "Test Profile",
                vec![app.id.clone()],
                cleanup_dial,
            ),
            source: profiles::TargetSource::Detected,
        }
    }

    struct QueueTargetResolver {
        targets: VecDeque<profiles::SessionTarget>,
        last: profiles::SessionTarget,
    }

    impl QueueTargetResolver {
        fn new(targets: Vec<profiles::SessionTarget>) -> Self {
            let last = targets
                .last()
                .expect("queue target resolver needs at least one target")
                .clone();
            Self {
                targets: targets.into(),
                last,
            }
        }
    }

    impl profiles::ResolveSessionTarget for QueueTargetResolver {
        fn resolve_session_target(&mut self) -> profiles::SessionTarget {
            if let Some(target) = self.targets.pop_front() {
                self.last = target.clone();
                target
            } else {
                self.last.clone()
            }
        }
    }

    struct ScriptedProcessor {
        cleanup_dial: CleanupDial,
        seen_dials: Arc<Mutex<Vec<CleanupDial>>>,
    }

    impl ScriptedProcessor {
        fn new(seen_dials: Arc<Mutex<Vec<CleanupDial>>>) -> Self {
            Self {
                cleanup_dial: CleanupDial::Light,
                seen_dials,
            }
        }
    }

    impl pipeline::CaptureProcessor for ScriptedProcessor {
        fn set_cleanup_dial(&mut self, cleanup_dial: CleanupDial) {
            self.cleanup_dial = cleanup_dial;
            self.seen_dials.lock().unwrap().push(cleanup_dial);
        }

        fn process_capture(
            &mut self,
            summary: &audio::CaptureSessionSummary,
        ) -> Result<Vec<events::SessionEvent>, pipeline::PipelineError> {
            Ok(vec![
                summary.audio_persisted_event(),
                events::SessionEvent::RawFinal {
                    id: summary.id,
                    text: "um hello captain".to_string(),
                },
                events::SessionEvent::CleanFinal {
                    id: summary.id,
                    text: "Hello captain.".to_string(),
                    dial: self.cleanup_dial,
                },
            ])
        }
    }

    struct TestInjector {
        caps: inject::InjectorCaps,
        field: inject::FieldKind,
        delivered: Arc<Mutex<Vec<String>>>,
    }

    impl TestInjector {
        fn no_target() -> Self {
            Self {
                caps: inject::InjectorCaps {
                    native_text_insert: false,
                    keystroke: inject::KeystrokeChannel::None,
                    clipboard: false,
                },
                field: inject::FieldKind::NoTarget,
                delivered: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn native(delivered: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                caps: inject::InjectorCaps {
                    native_text_insert: true,
                    keystroke: inject::KeystrokeChannel::None,
                    clipboard: false,
                },
                field: inject::FieldKind::Editable,
                delivered,
            }
        }
    }

    impl inject::TextInjector for TestInjector {
        fn caps(&self) -> inject::InjectorCaps {
            self.caps
        }

        fn focused_field(&self) -> inject::FieldKind {
            self.field
        }

        fn insert_native(&mut self, text: &str) -> Result<(), inject::InjectError> {
            self.delivered.lock().unwrap().push(text.to_string());
            Ok(())
        }

        fn synth_text(&mut self, text: &str) -> Result<(), inject::InjectError> {
            self.delivered.lock().unwrap().push(text.to_string());
            Ok(())
        }
    }

    fn matching_profiles(cleanup_dial: CleanupDial) -> profiles::ProfileStore {
        let app = detected_app();
        profiles::ProfileStore::with_profiles(vec![profiles::AppProfile::user_edited(
            "example-editor",
            "Example Editor",
            vec![app.id],
            cleanup_dial,
        )])
    }

    fn sha256_for(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn first_run_registry_json(asr_hash: &str, vad_hash: &str) -> String {
        format!(
            r#"{{
              "schema_version": 1,
              "models": [
                {{
                  "id": "fixture-asr",
                  "task": "asr",
                  "lane": "cpu",
                  "runtime": "onnxruntime",
                  "file": "fixture-asr.onnx",
                  "sha256": "{asr_hash}",
                  "size_mb": 1,
                  "license": "Apache-2.0",
                  "min_hw": "any",
                  "recommended": true,
                  "sources": ["https\u003A//models.example.test/fixture-asr.onnx"]
                }},
                {{
                  "id": "fixture-vad",
                  "task": "vad",
                  "runtime": "onnxruntime",
                  "file": "fixture-vad.onnx",
                  "sha256": "{vad_hash}",
                  "size_mb": 1,
                  "license": "MIT",
                  "min_hw": "any",
                  "recommended": true,
                  "sources": ["https\u003A//models.example.test/fixture-vad.onnx"]
                }}
              ]
            }}"#
        )
    }

    fn selectable_asr_registry_json() -> String {
        r#"{
          "schema_version": 1,
          "models": [
            {
              "id": "fixture-asr",
              "task": "asr",
              "lane": "cpu",
              "runtime": "onnxruntime",
              "file": "fixture-asr.onnx",
              "sha256": "TODO",
              "size_mb": 1,
              "license": "Apache-2.0",
              "min_hw": "any",
              "recommended": true
            },
            {
              "id": "fixture-gpu",
              "task": "asr",
              "lane": "gpu",
              "runtime": "whisper.cpp",
              "file": "fixture-gpu.bin",
              "sha256": "TODO",
              "size_mb": 2,
              "license": "MIT",
              "min_hw": "metal_or_dgpu",
              "recommended": true
            },
            {
              "id": "fixture-vad",
              "task": "vad",
              "runtime": "onnxruntime",
              "file": "fixture-vad.onnx",
              "sha256": "TODO",
              "size_mb": 1,
              "license": "MIT",
              "min_hw": "any",
              "recommended": true
            }
          ]
        }"#
        .to_string()
    }

    fn write_first_run_registry(
        app_data: &std::path::Path,
        asr_hash: &str,
        vad_hash: &str,
    ) -> std::path::PathBuf {
        let registry_dir = app_data.join("registry");
        std::fs::create_dir_all(&registry_dir).unwrap();
        let registry_path = registry_dir.join("registry.json");
        std::fs::write(&registry_path, first_run_registry_json(asr_hash, vad_hash)).unwrap();
        registry_path
    }

    fn write_selectable_asr_registry(app_data: &std::path::Path) -> std::path::PathBuf {
        let registry_dir = app_data.join("registry");
        std::fs::create_dir_all(&registry_dir).unwrap();
        let registry_path = registry_dir.join("registry.json");
        std::fs::write(&registry_path, selectable_asr_registry_json()).unwrap();
        registry_path
    }

    #[test]
    fn runtime_snapshot_reflects_hotkey_registration_success() {
        let state = RuntimeSnapshot::default();

        assert!(!state.snapshot().settings.first_run.hotkey_registered);
        state.mark_hotkey_registered();

        let snapshot = state.snapshot();
        assert!(snapshot.settings.first_run.hotkey_registered);
        assert_eq!(snapshot.settings.first_run.hotkey_registration_error, None);
    }

    #[test]
    fn runtime_snapshot_records_hotkey_registration_failure() {
        let state = RuntimeSnapshot::default();
        state.mark_hotkey_registered();
        state.mark_hotkey_registration_failed("shortcut already registered".to_string());

        let snapshot = state.snapshot();
        assert!(!snapshot.settings.first_run.hotkey_registered);
        assert_eq!(
            snapshot.settings.first_run.hotkey_registration_error,
            Some("shortcut already registered".to_string())
        );
    }

    #[test]
    fn runtime_snapshot_blocks_placeholder_model_checksums() {
        let app_data = tmp();
        let registry_path = write_first_run_registry(&app_data, "TODO", "TODO");
        let state = RuntimeSnapshot::default();

        state.refresh_model_readiness(&registry_path, &app_data.join("models"));

        let first_run = state.snapshot().settings.first_run;
        assert!(!first_run.model_ready);
        assert_eq!(
            first_run.model_readiness_error,
            Some(
                "Model registry needs verified metadata, checksums, or artifact repair".to_string()
            )
        );
        assert_eq!(first_run.required_models.len(), 2);
        assert!(first_run
            .required_models
            .iter()
            .all(|model| model.state == settings::FirstRunModelState::Blocked));
        assert_eq!(
            first_run.recommended_asr_model_id.as_deref(),
            Some("fixture-asr")
        );
        assert_eq!(
            first_run.selected_asr_model_id.as_deref(),
            Some("fixture-asr")
        );
        assert_eq!(first_run.asr_candidates.len(), 1);
        assert!(first_run.asr_candidates[0].selected);
        assert_eq!(
            first_run.asr_candidates[0].recommendation.as_deref(),
            Some("CPU-safe first-run default")
        );
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn runtime_snapshot_reports_missing_first_run_models() {
        let app_data = tmp();
        let registry_path =
            write_first_run_registry(&app_data, &sha256_for(b"asr"), &sha256_for(b"vad"));
        let state = RuntimeSnapshot::default();

        state.refresh_model_readiness(&registry_path, &app_data.join("models"));

        let first_run = state.snapshot().settings.first_run;
        assert!(!first_run.model_ready);
        assert_eq!(first_run.model_readiness_error, None);
        assert!(first_run
            .required_models
            .iter()
            .all(|model| model.state == settings::FirstRunModelState::Missing));
        assert!(first_run
            .required_models
            .iter()
            .all(|model| model.download_available
                && model.download_size_mb == Some(1)
                && model.download_source_count == 1));
        assert!(first_run
            .asr_candidates
            .iter()
            .all(|model| model.state == settings::FirstRunModelState::Missing));
        assert!(first_run
            .asr_candidates
            .iter()
            .all(|model| model.download_available
                && model.download_size_mb == Some(1)
                && model.download_source_count == 1));
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn runtime_snapshot_rechecks_models_after_artifact_install() {
        let app_data = tmp();
        let registry_path =
            write_first_run_registry(&app_data, &sha256_for(b"asr"), &sha256_for(b"vad"));
        let state = RuntimeSnapshot::default();

        let initial = state.refresh_models(&registry_path, &app_data).unwrap();

        assert!(!initial.settings.first_run.model_ready);
        assert!(initial
            .settings
            .first_run
            .required_models
            .iter()
            .all(|model| model.state == settings::FirstRunModelState::Missing));

        let models_dir = app_data.join("models");
        std::fs::create_dir_all(&models_dir).unwrap();
        std::fs::write(models_dir.join("fixture-asr.onnx"), b"asr").unwrap();
        std::fs::write(models_dir.join("fixture-vad.onnx"), b"vad").unwrap();

        let refreshed = state.refresh_models(&registry_path, &app_data).unwrap();

        assert!(refreshed.settings.first_run.model_ready);
        assert_eq!(refreshed.settings.first_run.model_readiness_error, None);
        assert!(refreshed
            .settings
            .first_run
            .required_models
            .iter()
            .all(|model| model.state == settings::FirstRunModelState::Ready));
        assert!(state.snapshot().settings.first_run.model_ready);
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn runtime_snapshot_blocks_missing_model_without_download_sources() {
        let app_data = tmp();
        let registry_dir = app_data.join("registry");
        std::fs::create_dir_all(&registry_dir).unwrap();
        let registry_path = registry_dir.join("registry.json");
        std::fs::write(
            &registry_path,
            format!(
                r#"{{
                  "schema_version": 1,
                  "models": [
                    {{
                      "id": "fixture-asr",
                      "task": "asr",
                      "lane": "cpu",
                      "runtime": "onnxruntime",
                      "file": "fixture-asr.onnx",
                      "sha256": "{}",
                      "size_mb": 1,
                      "license": "Apache-2.0",
                      "min_hw": "any",
                      "recommended": true,
                      "sources": ["TODO_primary"]
                    }}
                  ]
                }}"#,
                sha256_for(b"asr")
            ),
        )
        .unwrap();
        let state = RuntimeSnapshot::default();

        state.refresh_model_readiness(&registry_path, &app_data.join("models"));

        let first_run = state.snapshot().settings.first_run;
        assert!(!first_run.model_ready);
        assert_eq!(
            first_run.model_readiness_error,
            Some(
                "Model registry needs verified metadata, checksums, or artifact repair".to_string()
            )
        );
        assert_eq!(first_run.required_models.len(), 1);
        assert_eq!(
            first_run.required_models[0].state,
            settings::FirstRunModelState::Blocked
        );
        assert!(first_run.required_models[0]
            .detail
            .contains("Download unavailable"));
        assert!(!first_run.required_models[0].download_available);
        assert_eq!(first_run.required_models[0].download_size_mb, None);
        assert_eq!(first_run.required_models[0].download_source_count, 0);
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn runtime_snapshot_marks_verified_first_run_models_ready() {
        let app_data = tmp();
        let models_dir = app_data.join("models");
        std::fs::create_dir_all(&models_dir).unwrap();
        std::fs::write(models_dir.join("fixture-asr.onnx"), b"asr").unwrap();
        std::fs::write(models_dir.join("fixture-vad.onnx"), b"vad").unwrap();
        let registry_path =
            write_first_run_registry(&app_data, &sha256_for(b"asr"), &sha256_for(b"vad"));
        let state = RuntimeSnapshot::default();

        state.refresh_model_readiness(&registry_path, &models_dir);

        let first_run = state.snapshot().settings.first_run;
        assert!(first_run.model_ready);
        assert_eq!(first_run.model_readiness_error, None);
        assert_eq!(first_run.required_models.len(), 2);
        assert!(first_run
            .required_models
            .iter()
            .all(|model| model.state == settings::FirstRunModelState::Ready));
        assert!(first_run.model_ready);
        assert!(first_run.asr_candidates[0].selected);
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn runtime_snapshot_quarantines_mismatched_first_run_model() {
        let app_data = tmp();
        let models_dir = app_data.join("models");
        std::fs::create_dir_all(&models_dir).unwrap();
        let mismatched_path = models_dir.join("fixture-asr.onnx");
        std::fs::write(&mismatched_path, b"unexpected").unwrap();
        std::fs::write(models_dir.join("fixture-vad.onnx"), b"vad").unwrap();
        let registry_path =
            write_first_run_registry(&app_data, &sha256_for(b"expected"), &sha256_for(b"vad"));
        let state = RuntimeSnapshot::default();

        state.refresh_model_readiness(&registry_path, &models_dir);

        let first_run = state.snapshot().settings.first_run;
        let asr_status = first_run
            .required_models
            .iter()
            .find(|model| model.id == "fixture-asr")
            .unwrap();
        assert_eq!(asr_status.state, settings::FirstRunModelState::Blocked);
        assert!(asr_status.detail.contains("quarantined at"));
        assert!(!mismatched_path.exists());
        assert!(models_dir.join(models::QUARANTINE_DIR_NAME).exists());
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn os_default_asr_recommendation_beats_cpu_fallback() {
        let registry = models::ModelRegistry::from_json(
            r#"{
              "schema_version": 1,
              "models": [
                {
                  "id": "cpu-safe",
                  "task": "asr",
                  "lane": "cpu",
                  "runtime": "onnxruntime",
                  "file": "cpu.onnx",
                  "sha256": "TODO",
                  "size_mb": 1,
                  "license": "Apache-2.0",
                  "min_hw": "any",
                  "recommended": true
                },
                {
                  "id": "mac-gpu",
                  "task": "asr",
                  "lane": "gpu",
                  "runtime": "whisper.cpp",
                  "file": "gpu.bin",
                  "sha256": "TODO",
                  "size_mb": 1,
                  "license": "MIT",
                  "min_hw": "metal_or_dgpu",
                  "recommended": true,
                  "default_for": ["macos"]
                }
              ]
            }"#,
        )
        .unwrap();

        let recommended = first_run_asr_recommendation(&registry, "macos").unwrap();

        assert_eq!(recommended.id, "mac-gpu");
        assert_eq!(
            recommendation_reason(recommended, "macos"),
            "Recommended for this OS lane"
        );
    }

    #[test]
    fn selecting_asr_model_updates_snapshot_and_persists_settings() {
        let app_data = tmp();
        let registry_path = write_selectable_asr_registry(&app_data);
        let state = RuntimeSnapshot::default();
        state.set_settings_store(&app_data);
        state.refresh_model_readiness(&registry_path, &app_data.join("models"));

        let snapshot = state.select_asr_model("fixture-gpu").unwrap();

        let first_run = snapshot.settings.first_run;
        assert_eq!(
            first_run.selected_asr_model_id.as_deref(),
            Some("fixture-gpu")
        );
        assert!(first_run
            .asr_candidates
            .iter()
            .any(|candidate| candidate.id == "fixture-gpu" && candidate.selected));
        assert!(first_run
            .asr_candidates
            .iter()
            .any(|candidate| candidate.id == "fixture-asr" && !candidate.selected));
        assert_eq!(
            settings::SettingsStore::new(&app_data)
                .load()
                .unwrap()
                .selected_asr_model_id
                .as_deref(),
            Some("fixture-gpu")
        );
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn persisted_asr_selection_applies_after_model_refresh() {
        let app_data = tmp();
        let registry_path = write_selectable_asr_registry(&app_data);
        let store = settings::SettingsStore::new(&app_data);
        store
            .save(&settings::UserSettingsFile {
                selected_asr_model_id: Some("fixture-gpu".to_string()),
                ..settings::UserSettingsFile::default()
            })
            .unwrap();
        let state = RuntimeSnapshot::default();
        state.set_settings_store(&app_data);

        state.refresh_model_readiness(&registry_path, &app_data.join("models"));
        state.apply_persisted_user_settings().unwrap();

        let first_run = state.snapshot().settings.first_run;
        assert_eq!(
            first_run.selected_asr_model_id.as_deref(),
            Some("fixture-gpu")
        );
        assert!(first_run
            .asr_candidates
            .iter()
            .any(|candidate| candidate.id == "fixture-gpu" && candidate.selected));
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn selecting_unknown_asr_model_is_rejected() {
        let app_data = tmp();
        let registry_path = write_selectable_asr_registry(&app_data);
        let state = RuntimeSnapshot::default();
        state.set_settings_store(&app_data);
        state.refresh_model_readiness(&registry_path, &app_data.join("models"));

        let err = state.select_asr_model("missing-model").unwrap_err();

        assert!(matches!(err, SelectModelError::UnknownModel(model) if model == "missing-model"));
        assert_eq!(
            settings::SettingsStore::new(&app_data)
                .load()
                .unwrap()
                .selected_asr_model_id,
            None
        );
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn selected_asr_runtime_context_carries_model_and_lane() {
        let first_run = settings::FirstRunStatus {
            selected_asr_model_id: Some("fixture-gpu".to_string()),
            asr_candidates: vec![settings::FirstRunAsrCandidate {
                id: "fixture-gpu".to_string(),
                lane: Some("gpu".to_string()),
                runtime: "whisper.cpp".to_string(),
                size_mb: 2,
                min_hw: "metal_or_dgpu".to_string(),
                state: settings::FirstRunModelState::Ready,
                detail: "Verified artifact, 2 MB on disk".to_string(),
                download_available: true,
                download_size_mb: Some(2),
                download_source_count: 1,
                selected: true,
                recommendation: Some("Recommended for this OS lane".to_string()),
                license: "MIT".to_string(),
                license_review_required: false,
            }],
            ..settings::FirstRunStatus::default()
        };

        assert_eq!(
            selected_asr_runtime_context(&first_run),
            AsrRuntimeContext {
                model_id: Some("fixture-gpu".to_string()),
                lane: Some("gpu".to_string())
            }
        );
    }

    #[test]
    fn recover_history_audio_surfaces_untracked_wal_once() {
        let app_data = tmp();
        let id = events::SessionId::new(ulid::Ulid::new());
        let sessions_dir = app_data.join("sessions");
        let mut writer = audio::wal::WalWriter::create(&sessions_dir, &id.0.to_string()).unwrap();
        writer.append(&vec![0.1f32; 240]).unwrap();
        let wal_path = writer.path().to_path_buf();
        drop(writer);
        let mut store = history::HistoryStore::open(&app_data).unwrap();

        assert_eq!(recover_history_audio(&mut store, &app_data).unwrap(), 1);
        assert_eq!(recover_history_audio(&mut store, &app_data).unwrap(), 0);

        let session = store.get_session(id).unwrap().unwrap();
        assert_eq!(
            session.audio_path.as_deref(),
            Some(wal_path.to_str().unwrap())
        );
        assert_eq!(
            session.failure.as_ref().map(|failure| failure.stage),
            Some(events::Stage::Capture)
        );
        assert!(session
            .failure
            .as_ref()
            .unwrap()
            .error
            .contains("240 samples preserved"));
        assert_eq!(session.event_count, 2);
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn hotkey_runtime_processes_finalized_capture_and_holds_when_no_target() {
        let app_data = tmp();
        let seen_dials = Arc::new(Mutex::new(Vec::new()));
        let mut runtime = HotkeyRuntime::new_wal_only(&app_data)
            .unwrap()
            .with_target_resolver(Box::new(profiles::SessionTargetResolver::new(
                profiles::StaticFrontmostAppDetector::new(detected_app()),
                matching_profiles(CleanupDial::Full),
            )))
            .with_processor(Box::new(ScriptedProcessor::new(Arc::clone(&seen_dials))))
            .with_injector(Box::new(TestInjector::no_target()));

        assert_eq!(
            runtime
                .handle_signal(hotkeys::Signal::Press { at_ms: 0 })
                .unwrap(),
            None
        );
        let id = runtime.recorder.active_session_id().unwrap();
        assert_eq!(
            runtime
                .handle_signal(hotkeys::Signal::Release { at_ms: 400 })
                .unwrap(),
            Some(700)
        );
        assert_eq!(
            runtime
                .handle_signal(hotkeys::Signal::Tick { at_ms: 700 })
                .unwrap(),
            None
        );

        let path = app_data.join("sessions").join(format!("{}.wav", id.0));
        assert!(runtime.recorder.active_session_id().is_none());
        assert!(path.exists());
        assert_eq!(&std::fs::read(&path).unwrap()[0..4], b"RIFF");
        assert!(app_data.join(history::HISTORY_DB_FILE).exists());

        let session = runtime.history.get_session(id).unwrap().unwrap();
        assert_eq!(session.target_app, Some(detected_app()));
        assert_eq!(session.audio_path.as_deref(), Some(path.to_str().unwrap()));
        assert_eq!(session.raw_text.as_deref(), Some("um hello captain"));
        assert_eq!(session.clean_text.as_deref(), Some("Hello captain."));
        assert_eq!(session.cleanup_dial, Some(CleanupDial::Full));
        assert_eq!(session.held_reason, Some(HoldReason::NoTarget));
        assert_eq!(session.event_count, 5);
        assert_eq!(*seen_dials.lock().unwrap(), vec![CleanupDial::Full]);
        assert_eq!(
            runtime.history.events_for_session(id).unwrap(),
            vec![
                events::SessionEvent::Started {
                    id,
                    target_app: detected_app(),
                    at_ms: 0,
                },
                audio::CaptureSessionSummary {
                    id,
                    wal_path: path,
                    samples_written: 0,
                    dropped_input_samples: 0,
                    started_ms: 0,
                    finalized_ms: 700
                }
                .audio_persisted_event(),
                events::SessionEvent::RawFinal {
                    id,
                    text: "um hello captain".to_string(),
                },
                events::SessionEvent::CleanFinal {
                    id,
                    text: "Hello captain.".to_string(),
                    dial: CleanupDial::Full,
                },
                events::SessionEvent::Held {
                    id,
                    reason: HoldReason::NoTarget,
                },
            ]
        );
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn hotkey_runtime_injects_committed_text_when_backend_succeeds() {
        let app_data = tmp();
        let delivered = Arc::new(Mutex::new(Vec::new()));
        let seen_dials = Arc::new(Mutex::new(Vec::new()));
        let mut runtime = HotkeyRuntime::new_wal_only(&app_data)
            .unwrap()
            .with_target_resolver(Box::new(profiles::SessionTargetResolver::new(
                profiles::StaticFrontmostAppDetector::new(detected_app()),
                matching_profiles(CleanupDial::Light),
            )))
            .with_processor(Box::new(ScriptedProcessor::new(Arc::clone(&seen_dials))))
            .with_injector(Box::new(TestInjector::native(Arc::clone(&delivered))));

        runtime
            .handle_signal(hotkeys::Signal::Press { at_ms: 0 })
            .unwrap();
        let id = runtime.recorder.active_session_id().unwrap();
        runtime
            .handle_signal(hotkeys::Signal::Release { at_ms: 400 })
            .unwrap();
        runtime
            .handle_signal(hotkeys::Signal::Tick { at_ms: 700 })
            .unwrap();

        assert_eq!(*delivered.lock().unwrap(), vec!["Hello captain."]);
        let session = runtime.history.get_session(id).unwrap().unwrap();
        assert_eq!(session.injected_method, Some(InjectMethod::Native));
        assert_eq!(
            runtime.history.events_for_session(id).unwrap().last(),
            Some(&events::SessionEvent::Injected {
                id,
                method: InjectMethod::Native,
            })
        );
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn hotkey_runtime_holds_when_focus_changes_before_delivery() {
        let app_data = tmp();
        let delivered = Arc::new(Mutex::new(Vec::new()));
        let seen_dials = Arc::new(Mutex::new(Vec::new()));
        let mut runtime = HotkeyRuntime::new_wal_only(&app_data)
            .unwrap()
            .with_target_resolver(Box::new(QueueTargetResolver::new(vec![
                session_target(detected_app(), CleanupDial::Light),
                session_target(other_app(), CleanupDial::Light),
            ])))
            .with_processor(Box::new(ScriptedProcessor::new(Arc::clone(&seen_dials))))
            .with_injector(Box::new(TestInjector::native(Arc::clone(&delivered))));

        runtime
            .handle_signal(hotkeys::Signal::Press { at_ms: 0 })
            .unwrap();
        let id = runtime.recorder.active_session_id().unwrap();
        runtime
            .handle_signal(hotkeys::Signal::Release { at_ms: 400 })
            .unwrap();
        runtime
            .handle_signal(hotkeys::Signal::Tick { at_ms: 700 })
            .unwrap();

        assert!(delivered.lock().unwrap().is_empty());
        let session = runtime.history.get_session(id).unwrap().unwrap();
        assert_eq!(session.held_reason, Some(HoldReason::FocusChanged));
        assert_eq!(session.injected_method, None);
        assert_eq!(
            runtime.history.events_for_session(id).unwrap().last(),
            Some(&events::SessionEvent::Held {
                id,
                reason: HoldReason::FocusChanged,
            })
        );
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn hotkey_runtime_holds_when_target_identity_is_unknown() {
        let app_data = tmp();
        let delivered = Arc::new(Mutex::new(Vec::new()));
        let seen_dials = Arc::new(Mutex::new(Vec::new()));
        let mut runtime = HotkeyRuntime::new_wal_only(&app_data)
            .unwrap()
            .with_target_resolver(Box::new(profiles::SessionTargetResolver::new(
                profiles::StaticFrontmostAppDetector::unknown(),
                profiles::ProfileStore::default(),
            )))
            .with_processor(Box::new(ScriptedProcessor::new(Arc::clone(&seen_dials))))
            .with_injector(Box::new(TestInjector::native(Arc::clone(&delivered))));

        runtime
            .handle_signal(hotkeys::Signal::Press { at_ms: 0 })
            .unwrap();
        let id = runtime.recorder.active_session_id().unwrap();
        runtime
            .handle_signal(hotkeys::Signal::Release { at_ms: 400 })
            .unwrap();
        runtime
            .handle_signal(hotkeys::Signal::Tick { at_ms: 700 })
            .unwrap();

        assert!(delivered.lock().unwrap().is_empty());
        let session = runtime.history.get_session(id).unwrap().unwrap();
        assert_eq!(session.target_app, Some(profiles::unknown_app_ref()));
        assert_eq!(session.held_reason, Some(HoldReason::FocusChanged));
        assert_eq!(session.injected_method, None);
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn hotkey_runtime_discards_short_tap_wal() {
        let app_data = tmp();
        let mut runtime = HotkeyRuntime::new_wal_only(&app_data).unwrap();

        runtime
            .handle_signal(hotkeys::Signal::Press { at_ms: 0 })
            .unwrap();
        let id = runtime.recorder.active_session_id().unwrap();
        let path = app_data.join("sessions").join(format!("{}.wav", id.0));
        assert!(path.exists());

        assert_eq!(
            runtime
                .handle_signal(hotkeys::Signal::Release { at_ms: 100 })
                .unwrap(),
            None
        );

        assert!(runtime.recorder.active_session_id().is_none());
        assert!(!path.exists());
        assert!(runtime.history.get_session(id).unwrap().is_none());
        assert!(runtime.history.events_for_session(id).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(app_data);
    }
}
