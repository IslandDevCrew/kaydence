//! Configure: single source of config truth; typed; hot-reload.
//!
//! P1 starts with an in-memory, serializable settings snapshot that captures the
//! first-run defaults the UI and platform modules need. Disk persistence and
//! hot-reload land when the settings screen becomes editable; until then this
//! module still owns the types, defaults, and validation rules.

use crate::events::CleanupDial;
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Current settings-file schema version.
pub const SCHEMA_VERSION: u16 = 1;
pub const SETTINGS_FILE_NAME: &str = "settings.json";
pub const FIRST_RUN_SETUP_TARGET_MS: u64 = 60_000;
pub const FIRST_RUN_PROOF_PLAN_SCHEMA_VERSION: u16 = 1;
/// The product brand string. Never hardcode "Kaydence" anywhere else.
pub const APP_NAME: &str = "Kaydence";
pub const DEFAULT_HOTKEY_BINDING: &str = "RightAlt";

/// Reverse-DNS application identifier (matches tauri.conf.json `identifier`).
pub const APP_IDENTIFIER: &str = "io.kaydence.app";

/// A UI/backend snapshot exposed to the frontend by Tauri command. Frontend
/// renders this; Rust keeps the canonical settings truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSnapshot {
    pub app_name: String,
    pub app_identifier: String,
    pub settings: AppSettings,
}

impl Default for AppSnapshot {
    fn default() -> Self {
        Self {
            app_name: APP_NAME.into(),
            app_identifier: APP_IDENTIFIER.into(),
            settings: AppSettings::default(),
        }
    }
}

/// Human-readable JSON settings. Keep secrets out; BYOK keys live in the OS
/// keychain when that feature lands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    pub schema_version: u16,
    pub hotkey: HotkeySettings,
    pub capture: CaptureSettings,
    pub engine: EngineSettings,
    pub cleanup: CleanupSettings,
    pub injection: InjectionSettings,
    pub privacy: PrivacySettings,
    pub first_run: FirstRunStatus,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            hotkey: HotkeySettings::default(),
            capture: CaptureSettings::default(),
            engine: EngineSettings::default(),
            cleanup: CleanupSettings::default(),
            injection: InjectionSettings::default(),
            privacy: PrivacySettings::default(),
            first_run: FirstRunStatus::default(),
        }
    }
}

impl AppSettings {
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(SettingsError::UnsupportedSchema {
                expected: SCHEMA_VERSION,
                got: self.schema_version,
            });
        }
        self.hotkey.validate()?;
        self.capture.validate()?;
        self.privacy.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserSettingsFile {
    pub schema_version: u16,
    #[serde(default)]
    pub selected_asr_model_id: Option<String>,
    #[serde(default)]
    pub hotkey_mode: Option<HotkeyModeSetting>,
    #[serde(default)]
    pub hotkey_primary_binding: Option<String>,
    #[serde(default)]
    pub cleanup_default_dial: Option<CleanupDial>,
    #[serde(default)]
    pub first_run_started_at_ms: Option<u64>,
    #[serde(default)]
    pub first_dictation_completed: bool,
    #[serde(default)]
    pub first_dictation_completed_at_ms: Option<u64>,
}

impl Default for UserSettingsFile {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            selected_asr_model_id: None,
            hotkey_mode: None,
            hotkey_primary_binding: None,
            cleanup_default_dial: None,
            first_run_started_at_ms: None,
            first_dictation_completed: false,
            first_dictation_completed_at_ms: None,
        }
    }
}

impl UserSettingsFile {
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(SettingsError::UnsupportedSchema {
                expected: SCHEMA_VERSION,
                got: self.schema_version,
            });
        }
        if self
            .selected_asr_model_id
            .as_deref()
            .is_some_and(|id| id.trim().is_empty())
        {
            return Err(SettingsError::EmptyModelSelection);
        }
        if let Some(binding) = self.hotkey_primary_binding.as_deref() {
            validate_hotkey_binding(binding)?;
        }
        if let (Some(started), Some(completed)) = (
            self.first_run_started_at_ms,
            self.first_dictation_completed_at_ms,
        ) {
            if completed < started {
                return Err(SettingsError::InvalidFirstRunTiming(
                    "completion precedes start",
                ));
            }
        }
        if self.first_dictation_completed_at_ms.is_some() && !self.first_dictation_completed {
            return Err(SettingsError::InvalidFirstRunTiming(
                "completion timestamp requires completed=true",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn new(app_data_dir: impl AsRef<Path>) -> Self {
        Self {
            path: app_data_dir.as_ref().join(SETTINGS_FILE_NAME),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<UserSettingsFile, SettingsStoreError> {
        match fs::read_to_string(&self.path) {
            Ok(json) => {
                let settings: UserSettingsFile = serde_json::from_str(&json)?;
                settings.validate()?;
                Ok(settings)
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(UserSettingsFile::default()),
            Err(err) => Err(err.into()),
        }
    }

    pub fn save(&self, settings: &UserSettingsFile) -> Result<(), SettingsStoreError> {
        settings.validate()?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(settings)?;
        fs::write(&tmp, json)?;
        fs::rename(tmp, &self.path)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeySettings {
    pub mode: HotkeyModeSetting,
    pub primary_binding: String,
    pub primary_binding_options: Vec<HotkeyBindingOption>,
    pub secondary_dial_override_binding: String,
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            mode: HotkeyModeSetting::PushToTalk,
            primary_binding: DEFAULT_HOTKEY_BINDING.into(),
            primary_binding_options: hotkey_binding_options(),
            secondary_dial_override_binding: "Shift+RightAlt".into(),
        }
    }
}

impl HotkeySettings {
    fn validate(&self) -> Result<(), SettingsError> {
        if self.primary_binding.trim().is_empty() {
            return Err(SettingsError::EmptyHotkeyBinding);
        }
        validate_hotkey_binding(&self.primary_binding)?;
        if self.secondary_dial_override_binding.trim().is_empty() {
            return Err(SettingsError::EmptyHotkeyBinding);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeyBindingOption {
    pub id: String,
    pub label: String,
    pub detail: String,
}

pub fn hotkey_binding_options() -> Vec<HotkeyBindingOption> {
    [
        (
            DEFAULT_HOTKEY_BINDING,
            "Right Alt / Option",
            "Default hold key, best when the OS accepts the right-side modifier.",
        ),
        (
            "F13",
            "F13",
            "Dedicated function-key fallback for extended keyboards.",
        ),
        (
            "F14",
            "F14",
            "Second dedicated function-key fallback for extended keyboards.",
        ),
        (
            "Control+Space",
            "Control Space",
            "Chord fallback for compact keyboards without F13/F14.",
        ),
        (
            "Shift+F13",
            "Shift F13",
            "Conflict-escape chord when a plain function key is already taken.",
        ),
    ]
    .into_iter()
    .map(|(id, label, detail)| HotkeyBindingOption {
        id: id.to_string(),
        label: label.to_string(),
        detail: detail.to_string(),
    })
    .collect()
}

pub fn normalize_hotkey_binding(value: &str) -> Result<String, SettingsError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(SettingsError::EmptyHotkeyBinding);
    }

    let compact = trimmed
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '-' && *ch != '_')
        .flat_map(char::to_lowercase)
        .collect::<String>();

    let normalized = match compact.as_str() {
        "rightalt" | "altright" | "rightoption" | "optionright" => DEFAULT_HOTKEY_BINDING,
        "f13" => "F13",
        "f14" => "F14",
        "control+space" | "ctrl+space" | "controlspace" | "ctrlspace" => "Control+Space",
        "shift+f13" | "shiftf13" => "Shift+F13",
        _ => return Err(SettingsError::InvalidHotkeyBinding(trimmed.to_string())),
    };

    Ok(normalized.to_string())
}

pub fn validate_hotkey_binding(value: &str) -> Result<(), SettingsError> {
    normalize_hotkey_binding(value).map(|_| ())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HotkeyModeSetting {
    PushToTalk,
    Toggle,
}

impl HotkeyModeSetting {
    pub fn parse(value: &str) -> Result<Self, SettingsError> {
        match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "push_to_talk" | "ptt" => Ok(Self::PushToTalk),
            "toggle" => Ok(Self::Toggle),
            other => Err(SettingsError::InvalidHotkeyMode(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureSettings {
    pub min_capture_ms: u64,
    pub tail_buffer_ms: u64,
    pub debounce_ms: u64,
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self {
            min_capture_ms: 250,
            tail_buffer_ms: 300,
            debounce_ms: 30,
        }
    }
}

impl CaptureSettings {
    fn validate(&self) -> Result<(), SettingsError> {
        if self.min_capture_ms == 0 {
            return Err(SettingsError::InvalidCaptureWindow("min_capture_ms"));
        }
        if self.tail_buffer_ms == 0 {
            return Err(SettingsError::InvalidCaptureWindow("tail_buffer_ms"));
        }
        if self.debounce_ms == 0 {
            return Err(SettingsError::InvalidCaptureWindow("debounce_ms"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalAsrEngine {
    ParakeetCpu,
    WhisperGpu,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineSettings {
    pub default_local_asr: LocalAsrEngine,
    pub auto_recommend_by_hardware: bool,
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            default_local_asr: LocalAsrEngine::ParakeetCpu,
            auto_recommend_by_hardware: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupSettings {
    pub default_dial: CleanupDial,
    pub full_requires_explicit_opt_in: bool,
}

impl Default for CleanupSettings {
    fn default() -> Self {
        Self {
            default_dial: CleanupDial::Light,
            full_requires_explicit_opt_in: true,
        }
    }
}

pub fn parse_cleanup_dial(dial: &str) -> Result<CleanupDial, SettingsError> {
    let compact = dial
        .trim()
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '-' && *ch != '_')
        .flat_map(char::to_lowercase)
        .collect::<String>();

    match compact.as_str() {
        "raw" => Ok(CleanupDial::Raw),
        "light" => Ok(CleanupDial::Light),
        "full" => Ok(CleanupDial::Full),
        _ => Err(SettingsError::InvalidCleanupDial(dial.trim().to_string())),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InjectionSettings {
    pub unknown_focus_policy: UnknownFocusPolicy,
    pub prefer_clipboard_fallback: bool,
}

impl Default for InjectionSettings {
    fn default() -> Self {
        Self {
            unknown_focus_policy: UnknownFocusPolicy::WarnAndAllow,
            prefer_clipboard_fallback: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnknownFocusPolicy {
    /// Default for opaque Wayland clients: allow but mark unverified in the UI.
    WarnAndAllow,
    /// Maximum privacy mode: refuse when secure-field status cannot be proven.
    Refuse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacySettings {
    pub history_retention_days: u16,
    pub local_context_enabled: bool,
    pub local_ocr_enabled: bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            history_retention_days: 30,
            local_context_enabled: false,
            local_ocr_enabled: false,
        }
    }
}

impl PrivacySettings {
    fn validate(&self) -> Result<(), SettingsError> {
        if !(1..=365).contains(&self.history_retention_days) {
            return Err(SettingsError::InvalidRetentionDays(
                self.history_retention_days,
            ));
        }
        if self.local_ocr_enabled && !self.local_context_enabled {
            return Err(SettingsError::OcrRequiresContext);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstRunStatus {
    pub model_ready: bool,
    pub model_readiness_error: Option<String>,
    pub required_models: Vec<FirstRunModelStatus>,
    pub asr_candidates: Vec<FirstRunAsrCandidate>,
    pub recommended_asr_model_id: Option<String>,
    pub selected_asr_model_id: Option<String>,
    pub asr_runtime: FirstRunAsrRuntimeStatus,
    pub next_step: FirstRunNextStep,
    pub permission_requirements: Vec<FirstRunPermissionRequirement>,
    pub microphone_permission_ready: bool,
    pub input_permission_ready: bool,
    pub hotkey_registered: bool,
    pub hotkey_registration_error: Option<String>,
    pub setup_timing: FirstRunSetupTiming,
    pub first_dictation_completed: bool,
}

impl Default for FirstRunStatus {
    fn default() -> Self {
        let mut status = Self {
            model_ready: false,
            model_readiness_error: None,
            required_models: Vec::new(),
            asr_candidates: Vec::new(),
            recommended_asr_model_id: None,
            selected_asr_model_id: None,
            asr_runtime: FirstRunAsrRuntimeStatus::default(),
            next_step: FirstRunNextStep::default(),
            permission_requirements: first_run_permission_requirements(),
            microphone_permission_ready: false,
            input_permission_ready: false,
            hotkey_registered: false,
            hotkey_registration_error: None,
            setup_timing: FirstRunSetupTiming::default(),
            first_dictation_completed: false,
        };
        status.recompute_next_step();
        status
    }
}

impl FirstRunStatus {
    pub fn ready_to_dictate(&self) -> bool {
        self.model_ready
            && self.permission_requirements_ready()
            && self.microphone_permission_ready
            && self.input_permission_ready
            && self.hotkey_registered
    }

    pub fn permission_requirements_ready(&self) -> bool {
        !self.permission_requirements.is_empty()
            && self
                .permission_requirements
                .iter()
                .all(|requirement| requirement.state == FirstRunPermissionState::Ready)
    }

    pub fn recompute_next_step(&mut self) {
        self.next_step = FirstRunNextStep::from_status(self);
    }
}

pub fn sync_first_run_permission_requirements(status: &mut FirstRunStatus) {
    for requirement in &mut status.permission_requirements {
        match requirement.id.as_str() {
            "microphone" if status.microphone_permission_ready => {
                requirement.state = FirstRunPermissionState::Ready;
                requirement.detail =
                    "Runtime proof observed: capture produced persisted local audio.".to_string();
                requirement.action =
                    "No action needed; microphone proof is recorded for this runtime.".to_string();
                requirement.action_label = "Review proof".to_string();
            }
            "input_monitoring" if status.input_permission_ready => {
                requirement.state = FirstRunPermissionState::Ready;
                requirement.detail =
                    "Runtime proof observed: the selected hotkey reached Kaydence from the OS event stream."
                        .to_string();
                requirement.action =
                    "No action needed; hotkey event proof is recorded for this runtime."
                        .to_string();
                requirement.action_label = "Review proof".to_string();
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstRunSetupTiming {
    pub started_at_ms: Option<u64>,
    pub completed_at_ms: Option<u64>,
    pub elapsed_ms: Option<u64>,
    pub target_ms: u64,
    pub within_target: Option<bool>,
}

impl Default for FirstRunSetupTiming {
    fn default() -> Self {
        Self::from_parts(None, None)
    }
}

impl FirstRunSetupTiming {
    pub fn from_user_settings(settings: &UserSettingsFile) -> Self {
        Self::from_parts(
            settings.first_run_started_at_ms,
            settings.first_dictation_completed_at_ms,
        )
    }

    pub fn from_parts(started_at_ms: Option<u64>, completed_at_ms: Option<u64>) -> Self {
        let elapsed_ms = started_at_ms
            .zip(completed_at_ms)
            .and_then(|(started, completed)| completed.checked_sub(started));
        let within_target = elapsed_ms.map(|elapsed| elapsed <= FIRST_RUN_SETUP_TARGET_MS);

        Self {
            started_at_ms,
            completed_at_ms,
            elapsed_ms,
            target_ms: FIRST_RUN_SETUP_TARGET_MS,
            within_target,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstRunPermissionRequirement {
    pub id: String,
    pub label: String,
    pub state: FirstRunPermissionState,
    pub detail: String,
    pub action: String,
    pub action_label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirstRunPermissionState {
    Ready,
    NeedsHardware,
    NeedsReview,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstRunPermissionActionOutcome {
    pub requirement_id: String,
    pub label: String,
    pub state: FirstRunPermissionState,
    pub action_label: String,
    pub settings_target: Option<String>,
    pub settings_open_label: Option<String>,
    pub manual_step: String,
    pub proof_requirement: String,
    pub proof_command: Option<String>,
    pub expected_evidence: String,
    pub ready_boundary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstRunPermissionSettingsOpenOutcome {
    pub requirement_id: String,
    pub label: String,
    pub opened: bool,
    pub settings_target: Option<String>,
    pub manual_step: String,
    pub proof_requirement: String,
    pub expected_evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstRunNextStep {
    pub kind: FirstRunNextStepKind,
    pub target_id: Option<String>,
    pub title: String,
    pub detail: String,
    pub action_label: String,
    pub proof_requirement: String,
}

impl Default for FirstRunNextStep {
    fn default() -> Self {
        Self {
            kind: FirstRunNextStepKind::Setup,
            target_id: None,
            title: "Resolve setup".to_string(),
            detail: "Kaydence is still gathering first-run readiness state.".to_string(),
            action_label: "Resolve setup".to_string(),
            proof_requirement: "Refresh setup state before claiming readiness.".to_string(),
        }
    }
}

impl FirstRunNextStep {
    pub fn from_status(status: &FirstRunStatus) -> Self {
        if let Some(model) = status
            .required_models
            .iter()
            .find(|model| model.state == FirstRunModelState::Blocked)
        {
            return Self {
                kind: FirstRunNextStepKind::ModelMetadata,
                target_id: Some(model.id.clone()),
                title: "Review model metadata".to_string(),
                detail: format!("{} is blocked: {}", model.id, model.detail),
                action_label: "Review registry".to_string(),
                proof_requirement:
                    "Record reviewed sha256 and HTTPS source metadata before enabling install or download."
                        .to_string(),
            };
        }

        if let Some(model) = status
            .required_models
            .iter()
            .find(|model| model.state == FirstRunModelState::Missing)
        {
            return Self {
                kind: FirstRunNextStepKind::ModelInstall,
                target_id: Some(model.id.clone()),
                title: "Install reviewed model".to_string(),
                detail: format!(
                    "{} needs a verified local artifact before first dictation.",
                    model.id
                ),
                action_label: "Install artifact".to_string(),
                proof_requirement:
                    "Import a reviewed artifact and verify its sha256 under app-data models/."
                        .to_string(),
            };
        }

        if !status.model_ready {
            return Self {
                kind: FirstRunNextStepKind::ModelMetadata,
                target_id: None,
                title: "Resolve model readiness".to_string(),
                detail: status
                    .model_readiness_error
                    .clone()
                    .unwrap_or_else(|| "Model readiness has not been proven yet.".to_string()),
                action_label: "Review models".to_string(),
                proof_requirement: "Refresh model readiness with verified ASR and VAD artifacts."
                    .to_string(),
            };
        }

        if let Some(requirement) = status
            .permission_requirements
            .iter()
            .find(|requirement| requirement.state != FirstRunPermissionState::Ready)
        {
            return Self {
                kind: FirstRunNextStepKind::Permission,
                target_id: Some(requirement.id.clone()),
                title: format!("Grant {}", requirement.label),
                detail: requirement.detail.clone(),
                action_label: requirement.action_label.clone(),
                proof_requirement: first_run_permission_proof(&requirement.id).to_string(),
            };
        }

        if !status.hotkey_registered {
            return Self {
                kind: FirstRunNextStepKind::Hotkey,
                target_id: None,
                title: "Recover hotkey registration".to_string(),
                detail: status
                    .hotkey_registration_error
                    .clone()
                    .unwrap_or_else(|| "Register the selected global hotkey.".to_string()),
                action_label: "Choose hotkey".to_string(),
                proof_requirement:
                    "Register and trigger the selected global hotkey on this OS build.".to_string(),
            };
        }

        if !status.first_dictation_completed {
            return Self {
                kind: FirstRunNextStepKind::Dictation,
                target_id: None,
                title: "Run first dictation".to_string(),
                detail: "Hold the registered hotkey, speak a short phrase, and inject it into a normal text field.".to_string(),
                action_label: "Start dictation proof".to_string(),
                proof_requirement:
                    "A real Injected event must persist first_dictation_completed and setup timing."
                        .to_string(),
            };
        }

        Self {
            kind: FirstRunNextStepKind::Complete,
            target_id: None,
            title: "First run complete".to_string(),
            detail: "Setup has a persisted first-dictation proof.".to_string(),
            action_label: "Open cockpit".to_string(),
            proof_requirement:
                "settings.json contains first_dictation_completed=true and completion timing."
                    .to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirstRunNextStepKind {
    Setup,
    ModelMetadata,
    ModelInstall,
    Permission,
    Hotkey,
    Dictation,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstRunProofPlan {
    pub schema_version: u16,
    pub generated_at_ms: u64,
    pub app_name: String,
    pub app_identifier: String,
    pub os_lane: String,
    pub ready_to_dictate: bool,
    pub first_dictation_completed: bool,
    pub next_step: FirstRunNextStep,
    pub setup_timing: FirstRunSetupTiming,
    pub model_ready: bool,
    pub model_readiness_error: Option<String>,
    pub selected_asr_model_id: Option<String>,
    pub asr_runtime: FirstRunAsrRuntimeStatus,
    pub required_models: Vec<FirstRunModelStatus>,
    pub asr_candidates: Vec<FirstRunAsrCandidate>,
    pub permission_requirements: Vec<FirstRunPermissionRequirement>,
    pub hotkey_registered: bool,
    pub hotkey_registration_error: Option<String>,
    pub proof_items: Vec<FirstRunProofItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstRunProofItem {
    pub id: String,
    pub label: String,
    pub state: FirstRunProofItemState,
    pub target_id: Option<String>,
    pub detail: String,
    pub operator_action: String,
    pub proof_requirement: String,
    pub proof_command: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirstRunProofItemState {
    Ready,
    Pending,
    Missing,
    NeedsHardware,
    NeedsReview,
    Blocked,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstRunProofExportOutcome {
    pub exported: bool,
    pub json_path: Option<String>,
    pub item_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstRunModelDownloadPreflight {
    pub model_id: String,
    pub task: String,
    pub lane: Option<String>,
    pub runtime: String,
    pub file: String,
    pub state: FirstRunModelState,
    pub detail: String,
    pub available: bool,
    pub destination_path: Option<String>,
    pub expected_sha256: Option<String>,
    pub size_mb: Option<u64>,
    pub source_count: u16,
    pub sources: Vec<String>,
    pub license: String,
    pub license_review_required: bool,
    pub blocked_reason: Option<String>,
    pub operator_action: String,
    pub proof_requirement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstRunAsrRuntimeStatus {
    pub state: FirstRunAsrRuntimeState,
    pub selected_model_id: Option<String>,
    pub lane: Option<String>,
    pub runtime: Option<String>,
    pub artifact_path: Option<String>,
    pub artifact_size_bytes: Option<u64>,
    pub adapter_ready: bool,
    pub detail: String,
    pub proof_requirement: String,
}

impl Default for FirstRunAsrRuntimeStatus {
    fn default() -> Self {
        Self {
            state: FirstRunAsrRuntimeState::Pending,
            selected_model_id: None,
            lane: None,
            runtime: None,
            artifact_path: None,
            artifact_size_bytes: None,
            adapter_ready: false,
            detail: "Select and verify a local ASR model before runtime load.".to_string(),
            proof_requirement:
                "A real ASR adapter must load a verified artifact and emit transcript events before first dictation can be claimed."
                    .to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirstRunAsrRuntimeState {
    Pending,
    Blocked,
    VerifiedArtifact,
}

pub fn first_run_proof_plan(snapshot: &AppSnapshot, generated_at_ms: u64) -> FirstRunProofPlan {
    let first_run = &snapshot.settings.first_run;
    let mut proof_items = Vec::new();
    proof_items.push(first_run_next_step_proof_item(&first_run.next_step));

    proof_items.extend(
        first_run
            .required_models
            .iter()
            .map(first_run_model_proof_item),
    );
    proof_items.extend(
        first_run
            .permission_requirements
            .iter()
            .map(first_run_permission_proof_item),
    );
    proof_items.push(first_run_hotkey_proof_item(first_run));
    proof_items.push(first_run_dictation_proof_item(first_run));

    FirstRunProofPlan {
        schema_version: FIRST_RUN_PROOF_PLAN_SCHEMA_VERSION,
        generated_at_ms,
        app_name: snapshot.app_name.clone(),
        app_identifier: snapshot.app_identifier.clone(),
        os_lane: first_run_os_lane().to_string(),
        ready_to_dictate: first_run.ready_to_dictate(),
        first_dictation_completed: first_run.first_dictation_completed,
        next_step: first_run.next_step.clone(),
        setup_timing: first_run.setup_timing.clone(),
        model_ready: first_run.model_ready,
        model_readiness_error: first_run.model_readiness_error.clone(),
        selected_asr_model_id: first_run.selected_asr_model_id.clone(),
        asr_runtime: first_run.asr_runtime.clone(),
        required_models: first_run.required_models.clone(),
        asr_candidates: first_run.asr_candidates.clone(),
        permission_requirements: first_run.permission_requirements.clone(),
        hotkey_registered: first_run.hotkey_registered,
        hotkey_registration_error: first_run.hotkey_registration_error.clone(),
        proof_items,
    }
}

fn first_run_next_step_proof_item(next_step: &FirstRunNextStep) -> FirstRunProofItem {
    FirstRunProofItem {
        id: "next_step".to_string(),
        label: "Next setup step".to_string(),
        state: if next_step.kind == FirstRunNextStepKind::Complete {
            FirstRunProofItemState::Complete
        } else {
            FirstRunProofItemState::Pending
        },
        target_id: next_step.target_id.clone(),
        detail: next_step.detail.clone(),
        operator_action: next_step.action_label.clone(),
        proof_requirement: next_step.proof_requirement.clone(),
        proof_command: None,
    }
}

fn first_run_model_proof_item(model: &FirstRunModelStatus) -> FirstRunProofItem {
    FirstRunProofItem {
        id: format!("model:{}", model.id),
        label: format!("{} model {}", model.task, model.id),
        state: match model.state {
            FirstRunModelState::Ready => FirstRunProofItemState::Ready,
            FirstRunModelState::Missing => FirstRunProofItemState::Missing,
            FirstRunModelState::Blocked => FirstRunProofItemState::Blocked,
        },
        target_id: Some(model.id.clone()),
        detail: model.detail.clone(),
        operator_action: if model.state == FirstRunModelState::Ready {
            "Keep the verified app-data artifact in place.".to_string()
        } else {
            "Install a reviewed local artifact through the Kaydence model picker.".to_string()
        },
        proof_requirement: format!(
            "Model artifact must verify against registry sha256 before {} can be ready.",
            model.id
        ),
        proof_command: None,
    }
}

fn first_run_permission_proof_item(
    requirement: &FirstRunPermissionRequirement,
) -> FirstRunProofItem {
    FirstRunProofItem {
        id: format!("permission:{}", requirement.id),
        label: requirement.label.clone(),
        state: match requirement.state {
            FirstRunPermissionState::Ready => FirstRunProofItemState::Ready,
            FirstRunPermissionState::NeedsHardware => FirstRunProofItemState::NeedsHardware,
            FirstRunPermissionState::NeedsReview => FirstRunProofItemState::NeedsReview,
            FirstRunPermissionState::Blocked => FirstRunProofItemState::Blocked,
        },
        target_id: Some(requirement.id.clone()),
        detail: requirement.detail.clone(),
        operator_action: requirement.action.clone(),
        proof_requirement: first_run_permission_proof(&requirement.id).to_string(),
        proof_command: first_run_permission_proof_command(&requirement.id).map(str::to_string),
    }
}

fn first_run_hotkey_proof_item(first_run: &FirstRunStatus) -> FirstRunProofItem {
    FirstRunProofItem {
        id: "hotkey".to_string(),
        label: "Global hotkey".to_string(),
        state: if first_run.hotkey_registered {
            FirstRunProofItemState::Ready
        } else if first_run.hotkey_registration_error.is_some() {
            FirstRunProofItemState::Blocked
        } else {
            FirstRunProofItemState::Pending
        },
        target_id: None,
        detail: first_run
            .hotkey_registration_error
            .clone()
            .unwrap_or_else(|| "Register and trigger the selected global hotkey.".to_string()),
        operator_action: "Choose an allowlisted hotkey and prove it fires on this OS.".to_string(),
        proof_requirement:
            "Hotkey registration must succeed and a real trigger must reach the runtime."
                .to_string(),
        proof_command: None,
    }
}

fn first_run_dictation_proof_item(first_run: &FirstRunStatus) -> FirstRunProofItem {
    FirstRunProofItem {
        id: "first_dictation".to_string(),
        label: "First dictation".to_string(),
        state: if first_run.first_dictation_completed {
            FirstRunProofItemState::Complete
        } else {
            FirstRunProofItemState::Pending
        },
        target_id: None,
        detail: if first_run.first_dictation_completed {
            "A real Injected event completed first run.".to_string()
        } else {
            "No persisted Injected event has completed first run yet.".to_string()
        },
        operator_action:
            "Speak a short phrase through the registered hotkey into a normal editable field."
                .to_string(),
        proof_requirement:
            "History must contain an Injected event and settings.json must record completion timing."
                .to_string(),
        proof_command: None,
    }
}

fn first_run_os_lane() -> &'static str {
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

pub fn first_run_permission_requirements() -> Vec<FirstRunPermissionRequirement> {
    platform_permission_specs()
        .into_iter()
        .map(
            |(id, label, state, detail, action, action_label)| FirstRunPermissionRequirement {
                id: id.to_string(),
                label: label.to_string(),
                state,
                detail: detail.to_string(),
                action: action.to_string(),
                action_label: action_label.to_string(),
            },
        )
        .collect()
}

pub fn first_run_permission_action(
    requirement_id: &str,
) -> Result<FirstRunPermissionActionOutcome, SettingsError> {
    let requirement = first_run_permission_requirements()
        .into_iter()
        .find(|requirement| requirement.id == requirement_id)
        .ok_or_else(|| SettingsError::UnknownPermissionRequirement(requirement_id.to_string()))?;

    Ok(FirstRunPermissionActionOutcome {
        requirement_id: requirement.id,
        label: requirement.label,
        state: requirement.state,
        action_label: requirement.action_label,
        settings_target: first_run_permission_settings_target(requirement_id).map(str::to_string),
        settings_open_label: first_run_permission_settings_open_label(requirement_id)
            .map(str::to_string),
        manual_step: requirement.action,
        proof_requirement: first_run_permission_proof(requirement_id).to_string(),
        proof_command: first_run_permission_proof_command(requirement_id).map(str::to_string),
        expected_evidence: first_run_permission_expected_evidence(requirement_id).to_string(),
        ready_boundary: first_run_permission_ready_boundary(requirement_id).to_string(),
    })
}

pub fn first_run_permission_settings_open_outcome(
    requirement_id: &str,
    opened: bool,
) -> Result<FirstRunPermissionSettingsOpenOutcome, SettingsError> {
    let action = first_run_permission_action(requirement_id)?;
    Ok(FirstRunPermissionSettingsOpenOutcome {
        requirement_id: action.requirement_id,
        label: action.label,
        opened,
        settings_target: action.settings_target,
        manual_step: action.manual_step,
        proof_requirement: action.proof_requirement,
        expected_evidence: action.expected_evidence,
    })
}

pub fn first_run_permission_settings_target(requirement_id: &str) -> Option<&'static str> {
    if cfg!(target_os = "macos") {
        match requirement_id {
            "microphone" => {
                Some("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
            }
            "accessibility" => Some(
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
            ),
            "input_monitoring" => {
                Some("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
            }
            _ => None,
        }
    } else if cfg!(target_os = "windows") {
        match requirement_id {
            "microphone" => Some("ms-settings:privacy-microphone"),
            _ => None,
        }
    } else {
        None
    }
}

fn first_run_permission_settings_open_label(requirement_id: &str) -> Option<&'static str> {
    first_run_permission_settings_target(requirement_id).map(|_| {
        if cfg!(target_os = "macos") {
            "Open System Settings"
        } else if cfg!(target_os = "windows") {
            "Open Windows Settings"
        } else {
            "Open Settings"
        }
    })
}

fn first_run_permission_proof(requirement_id: &str) -> &'static str {
    match requirement_id {
        "microphone" => {
            "Run the first-dictation journey and verify capture produces persisted audio."
        }
        "accessibility" => {
            "Validate native insertion plus secure-field refusal on the current macOS build."
        }
        "input_monitoring" => {
            "Register and trigger the selected global hotkey on the current macOS build."
        }
        "uia_focus" => {
            "Run the Windows UIA focus self-test and confirm password fields are refused."
        }
        "sendinput" => {
            "Run the Windows fallback injection proof against a non-native editable field."
        }
        "accessibility_bus" => {
            "Run `cargo run --manifest-path apps/desktop/src-tauri/Cargo.toml --bin atspi-selftest` in the Linux desktop session."
        }
        "uinput" => "Run the Linux human-focus proof: normal field types, password field refuses.",
        _ => "Record OS-specific setup evidence before marking the requirement ready.",
    }
}

fn first_run_permission_proof_command(requirement_id: &str) -> Option<&'static str> {
    match requirement_id {
        "uia_focus" => Some(
            "cargo run --manifest-path apps/desktop/src-tauri/Cargo.toml --bin uia-selftest -- --probe 5",
        ),
        "sendinput" => Some(
            "cargo run --manifest-path apps/desktop/src-tauri/Cargo.toml --bin uia-selftest -- --synth 5 \"Kaydence SendInput proof\"",
        ),
        "accessibility_bus" => Some(
            "cargo run --manifest-path apps/desktop/src-tauri/Cargo.toml --bin atspi-selftest",
        ),
        "uinput" => Some(
            "cargo run --manifest-path apps/desktop/src-tauri/Cargo.toml --bin atspi-selftest -- --type \"Kaydence uinput proof\"",
        ),
        _ => None,
    }
}

fn first_run_permission_expected_evidence(requirement_id: &str) -> &'static str {
    match requirement_id {
        "microphone" => {
            "A real capture writes non-empty app-data audio and the first-dictation journey can proceed."
        }
        "accessibility" => {
            "A normal field accepts native insertion and a secure field produces a Held{SecureField} outcome."
        }
        "input_monitoring" => {
            "The selected global hotkey reaches the runtime from the OS event stream while Kaydence is backgrounded."
        }
        "uia_focus" => {
            "The UIA proof reports editable focus metadata and refuses password/secure fields on the Windows build."
        }
        "sendinput" => {
            "The SendInput proof types the requested Unicode text into a normal field after secure-field gating."
        }
        "accessibility_bus" => {
            "The AT-SPI selftest connects to the desktop bus and can inspect focused accessible objects."
        }
        "uinput" => {
            "The Linux human-focus proof types into a normal field and refuses a password field before injection."
        }
        _ => "A platform-specific proof artifact records the checked OS requirement and expected user-visible outcome.",
    }
}

fn first_run_permission_ready_boundary(requirement_id: &str) -> &'static str {
    match requirement_id {
        "microphone" => {
            "Do not mark microphone ready until capture has produced persisted audio on the current OS build."
        }
        "accessibility" => {
            "Do not mark Accessibility ready until native insertion and secure-field refusal are both observed."
        }
        "input_monitoring" => {
            "Do not mark Input Monitoring ready until a real global hotkey trigger reaches the runtime."
        }
        "uia_focus" => {
            "Do not mark UIA focus ready until the Windows selftest proves focus inspection and secure refusal."
        }
        "sendinput" => {
            "Do not mark SendInput ready until fallback typing is proven after the secure-field gate."
        }
        "accessibility_bus" => {
            "Do not mark AT-SPI ready until the desktop bus proof runs in the target Linux session."
        }
        "uinput" => {
            "Do not mark uinput ready until operator-in-the-loop focus proof passes on the target Linux desktop."
        }
        _ => "Do not mark this requirement ready until an OS-specific proof artifact exists.",
    }
}

fn platform_permission_specs() -> Vec<(
    &'static str,
    &'static str,
    FirstRunPermissionState,
    &'static str,
    &'static str,
    &'static str,
)> {
    if cfg!(target_os = "macos") {
        vec![
            (
                "microphone",
                "Microphone",
                FirstRunPermissionState::NeedsHardware,
                "Required before local capture can produce speech audio.",
                "Grant Kaydence access in System Settings -> Privacy & Security -> Microphone.",
                "Show microphone step",
            ),
            (
                "accessibility",
                "Accessibility",
                FirstRunPermissionState::NeedsHardware,
                "Required for native insertion plus focus and secure-field checks.",
                "Enable Kaydence in System Settings -> Privacy & Security -> Accessibility.",
                "Show accessibility step",
            ),
            (
                "input_monitoring",
                "Input Monitoring",
                FirstRunPermissionState::NeedsHardware,
                "Required for the global hotkey monitoring path on macOS.",
                "Enable Kaydence in System Settings -> Privacy & Security -> Input Monitoring.",
                "Show input step",
            ),
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            (
                "microphone",
                "Microphone",
                FirstRunPermissionState::NeedsHardware,
                "Required before local capture can produce speech audio.",
                "Grant microphone access in Windows Privacy & security settings.",
                "Show microphone step",
            ),
            (
                "uia_focus",
                "UI Automation focus access",
                FirstRunPermissionState::NeedsReview,
                "Required to detect secure fields and use the native insertion path.",
                "Validate with the Windows UIA self-test before marking ready.",
                "Show UIA proof",
            ),
            (
                "sendinput",
                "Keyboard injection fallback",
                FirstRunPermissionState::NeedsReview,
                "Used only after secure-field and focus checks allow fallback typing.",
                "Validate SendInput fallback on the target Windows build.",
                "Show fallback proof",
            ),
        ]
    } else if cfg!(target_os = "linux") {
        vec![
            (
                "microphone",
                "Microphone",
                FirstRunPermissionState::NeedsHardware,
                "Required before local capture can produce speech audio.",
                "Confirm PipeWire or PulseAudio input access in the desktop session.",
                "Show microphone step",
            ),
            (
                "accessibility_bus",
                "AT-SPI accessibility bus",
                FirstRunPermissionState::NeedsReview,
                "Required to identify focus and refuse secure fields before injection.",
                "Run cargo run --manifest-path apps/desktop/src-tauri/Cargo.toml --bin atspi-selftest on the GNOME VM.",
                "Show AT-SPI proof",
            ),
            (
                "uinput",
                "uinput keyboard path",
                FirstRunPermissionState::NeedsHardware,
                "Required for the current Linux keystroke injection path.",
                "Confirm the user session can access /dev/uinput or the future portal/libei path.",
                "Show uinput proof",
            ),
        ]
    } else {
        vec![(
            "platform_review",
            "Platform permission review",
            FirstRunPermissionState::NeedsReview,
            "This operating system has no locked Kaydence first-run permission contract yet.",
            "Add an OS-specific permission checklist before marking setup ready.",
            "Show review step",
        )]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstRunModelStatus {
    pub id: String,
    pub task: String,
    pub lane: Option<String>,
    pub runtime: String,
    pub file: String,
    pub state: FirstRunModelState,
    pub detail: String,
    pub download_available: bool,
    pub download_size_mb: Option<u64>,
    pub download_source_count: u16,
    pub license: String,
    pub license_review_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstRunAsrCandidate {
    pub id: String,
    pub lane: Option<String>,
    pub runtime: String,
    pub size_mb: u64,
    pub min_hw: String,
    pub state: FirstRunModelState,
    pub detail: String,
    pub download_available: bool,
    pub download_size_mb: Option<u64>,
    pub download_source_count: u16,
    pub selected: bool,
    pub recommendation: Option<String>,
    pub license: String,
    pub license_review_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirstRunModelState {
    Ready,
    Missing,
    Blocked,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SettingsError {
    #[error("unsupported settings schema version: expected {expected}, got {got}")]
    UnsupportedSchema { expected: u16, got: u16 },
    #[error("hotkey binding cannot be empty")]
    EmptyHotkeyBinding,
    #[error("invalid capture window: {0} must be greater than zero")]
    InvalidCaptureWindow(&'static str),
    #[error("history retention must be between 1 and 365 days, got {0}")]
    InvalidRetentionDays(u16),
    #[error("local OCR requires local context reading to be enabled")]
    OcrRequiresContext,
    #[error("selected ASR model id cannot be empty")]
    EmptyModelSelection,
    #[error("invalid hotkey mode: {0}")]
    InvalidHotkeyMode(String),
    #[error("unsupported hotkey binding: {0}")]
    InvalidHotkeyBinding(String),
    #[error("invalid cleanup dial: {0}")]
    InvalidCleanupDial(String),
    #[error("unknown first-run permission requirement: {0}")]
    UnknownPermissionRequirement(String),
    #[error("invalid first-run timing: {0}")]
    InvalidFirstRunTiming(&'static str),
}

#[derive(Debug, Error)]
pub enum SettingsStoreError {
    #[error("settings io: {0}")]
    Io(#[from] io::Error),
    #[error("settings json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("settings validation: {0}")]
    Validation(#[from] SettingsError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kaydence-settings-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn defaults_match_the_product_thesis() {
        let settings = AppSettings::default();
        assert_eq!(settings.schema_version, SCHEMA_VERSION);
        assert_eq!(settings.hotkey.mode, HotkeyModeSetting::PushToTalk);
        assert_eq!(settings.hotkey.primary_binding, DEFAULT_HOTKEY_BINDING);
        assert!(settings
            .hotkey
            .primary_binding_options
            .iter()
            .any(|option| option.id == DEFAULT_HOTKEY_BINDING));
        assert_eq!(settings.capture.min_capture_ms, 250);
        assert_eq!(settings.capture.tail_buffer_ms, 300);
        assert_eq!(settings.capture.debounce_ms, 30);
        assert_eq!(
            settings.engine.default_local_asr,
            LocalAsrEngine::ParakeetCpu
        );
        assert_eq!(settings.cleanup.default_dial, CleanupDial::Light);
        assert!(settings.cleanup.full_requires_explicit_opt_in);
        assert_eq!(settings.privacy.history_retention_days, 30);
        assert!(!settings.privacy.local_context_enabled);
    }

    #[test]
    fn settings_snapshot_serializes_for_the_frontend() {
        let snapshot = AppSnapshot::default();
        let json = serde_json::to_string(&snapshot).expect("snapshot serializes");
        assert!(json.contains(APP_NAME));
        assert!(json.contains("\"asr_runtime\""));
        let decoded: AppSnapshot = serde_json::from_str(&json).expect("snapshot decodes");
        assert_eq!(decoded, snapshot);
    }

    #[test]
    fn validation_rejects_bad_schema_and_empty_hotkey() {
        let mut settings = AppSettings {
            schema_version: 99,
            ..AppSettings::default()
        };
        assert!(matches!(
            settings.validate(),
            Err(SettingsError::UnsupportedSchema { .. })
        ));

        settings = AppSettings::default();
        settings.hotkey.primary_binding.clear();
        assert_eq!(settings.validate(), Err(SettingsError::EmptyHotkeyBinding));

        settings = AppSettings::default();
        settings.hotkey.primary_binding = "CapsLock".to_string();
        assert_eq!(
            settings.validate(),
            Err(SettingsError::InvalidHotkeyBinding("CapsLock".to_string()))
        );
    }

    #[test]
    fn validation_rejects_invalid_privacy_shape() {
        let mut settings = AppSettings::default();
        settings.privacy.history_retention_days = 0;
        assert_eq!(
            settings.validate(),
            Err(SettingsError::InvalidRetentionDays(0))
        );

        settings = AppSettings::default();
        settings.privacy.local_ocr_enabled = true;
        assert_eq!(settings.validate(), Err(SettingsError::OcrRequiresContext));
    }

    #[test]
    fn settings_store_defaults_when_file_is_missing() {
        let dir = tmp();
        let store = SettingsStore::new(&dir);

        let settings = store.load().unwrap();

        assert_eq!(settings, UserSettingsFile::default());
        assert_eq!(store.path(), dir.join(SETTINGS_FILE_NAME));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn settings_store_saves_selected_asr_model() {
        let dir = tmp();
        let store = SettingsStore::new(&dir);
        let settings = UserSettingsFile {
            selected_asr_model_id: Some("whisper-large-v3-turbo".to_string()),
            ..UserSettingsFile::default()
        };

        store.save(&settings).unwrap();

        assert_eq!(store.load().unwrap(), settings);
        assert!(fs::read_to_string(store.path())
            .unwrap()
            .contains("whisper-large-v3-turbo"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn settings_store_saves_hotkey_mode() {
        let dir = tmp();
        let store = SettingsStore::new(&dir);
        let settings = UserSettingsFile {
            hotkey_mode: Some(HotkeyModeSetting::Toggle),
            ..UserSettingsFile::default()
        };

        store.save(&settings).unwrap();

        assert_eq!(store.load().unwrap(), settings);
        assert!(fs::read_to_string(store.path())
            .unwrap()
            .contains("\"hotkey_mode\": \"toggle\""));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn settings_store_saves_hotkey_binding() {
        let dir = tmp();
        let store = SettingsStore::new(&dir);
        let settings = UserSettingsFile {
            hotkey_primary_binding: Some("F13".to_string()),
            ..UserSettingsFile::default()
        };

        store.save(&settings).unwrap();

        assert_eq!(store.load().unwrap(), settings);
        assert!(fs::read_to_string(store.path())
            .unwrap()
            .contains("\"hotkey_primary_binding\": \"F13\""));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn settings_store_saves_cleanup_default_dial() {
        let dir = tmp();
        let store = SettingsStore::new(&dir);
        let settings = UserSettingsFile {
            cleanup_default_dial: Some(CleanupDial::Raw),
            ..UserSettingsFile::default()
        };

        store.save(&settings).unwrap();

        assert_eq!(store.load().unwrap(), settings);
        assert!(fs::read_to_string(store.path())
            .unwrap()
            .contains("\"cleanup_default_dial\": \"raw\""));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cleanup_dial_parser_accepts_frontend_values() {
        assert_eq!(parse_cleanup_dial("raw").unwrap(), CleanupDial::Raw);
        assert_eq!(parse_cleanup_dial("Light").unwrap(), CleanupDial::Light);
        assert_eq!(parse_cleanup_dial("full").unwrap(), CleanupDial::Full);
        assert!(matches!(
            parse_cleanup_dial("author"),
            Err(SettingsError::InvalidCleanupDial(value)) if value == "author"
        ));
    }

    #[test]
    fn settings_store_saves_first_dictation_completion() {
        let dir = tmp();
        let store = SettingsStore::new(&dir);
        let settings = UserSettingsFile {
            first_run_started_at_ms: Some(1_000),
            first_dictation_completed: true,
            first_dictation_completed_at_ms: Some(60_000),
            ..UserSettingsFile::default()
        };

        store.save(&settings).unwrap();

        assert_eq!(store.load().unwrap(), settings);
        assert!(fs::read_to_string(store.path())
            .unwrap()
            .contains("\"first_dictation_completed\": true"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn first_run_setup_timing_calculates_elapsed_and_target() {
        let timing = FirstRunSetupTiming::from_parts(Some(1_000), Some(60_500));

        assert_eq!(timing.elapsed_ms, Some(59_500));
        assert_eq!(timing.target_ms, FIRST_RUN_SETUP_TARGET_MS);
        assert_eq!(timing.within_target, Some(true));

        let slow = FirstRunSetupTiming::from_parts(Some(1_000), Some(62_000));
        assert_eq!(slow.elapsed_ms, Some(61_000));
        assert_eq!(slow.within_target, Some(false));

        let missing_start = FirstRunSetupTiming::from_parts(None, Some(60_500));
        assert_eq!(missing_start.elapsed_ms, None);
        assert_eq!(missing_start.within_target, None);
    }

    #[test]
    fn validation_rejects_invalid_first_run_timing() {
        let mut settings = UserSettingsFile {
            first_run_started_at_ms: Some(10_000),
            first_dictation_completed: true,
            first_dictation_completed_at_ms: Some(1_000),
            ..UserSettingsFile::default()
        };
        assert!(matches!(
            settings.validate(),
            Err(SettingsError::InvalidFirstRunTiming(
                "completion precedes start"
            ))
        ));

        settings = UserSettingsFile {
            first_dictation_completed_at_ms: Some(1_000),
            ..UserSettingsFile::default()
        };
        assert!(matches!(
            settings.validate(),
            Err(SettingsError::InvalidFirstRunTiming(
                "completion timestamp requires completed=true"
            ))
        ));
    }

    #[test]
    fn hotkey_mode_parser_accepts_ui_values() {
        assert_eq!(
            HotkeyModeSetting::parse("push_to_talk").unwrap(),
            HotkeyModeSetting::PushToTalk
        );
        assert_eq!(
            HotkeyModeSetting::parse("push-to-talk").unwrap(),
            HotkeyModeSetting::PushToTalk
        );
        assert_eq!(
            HotkeyModeSetting::parse("toggle").unwrap(),
            HotkeyModeSetting::Toggle
        );
        assert!(matches!(
            HotkeyModeSetting::parse("hold"),
            Err(SettingsError::InvalidHotkeyMode(mode)) if mode == "hold"
        ));
    }

    #[test]
    fn hotkey_binding_parser_accepts_recommended_values() {
        assert_eq!(
            normalize_hotkey_binding("RightAlt").unwrap(),
            DEFAULT_HOTKEY_BINDING
        );
        assert_eq!(
            normalize_hotkey_binding("right-option").unwrap(),
            DEFAULT_HOTKEY_BINDING
        );
        assert_eq!(normalize_hotkey_binding("f13").unwrap(), "F13");
        assert_eq!(
            normalize_hotkey_binding("Ctrl + Space").unwrap(),
            "Control+Space"
        );
        assert_eq!(normalize_hotkey_binding("shift f13").unwrap(), "Shift+F13");
        assert!(matches!(
            normalize_hotkey_binding("CapsLock"),
            Err(SettingsError::InvalidHotkeyBinding(binding)) if binding == "CapsLock"
        ));
    }

    #[test]
    fn settings_store_rejects_empty_model_selection() {
        let dir = tmp();
        let store = SettingsStore::new(&dir);
        fs::write(
            store.path(),
            r#"{"schema_version":1,"selected_asr_model_id":""}"#,
        )
        .unwrap();

        let err = store.load().unwrap_err();

        assert!(matches!(
            err,
            SettingsStoreError::Validation(SettingsError::EmptyModelSelection)
        ));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn first_run_ready_requires_every_runtime_prerequisite() {
        let mut status = FirstRunStatus::default();
        assert!(!status.ready_to_dictate());
        status.model_ready = true;
        status.microphone_permission_ready = true;
        status.input_permission_ready = true;
        status.hotkey_registered = true;
        assert!(!status.ready_to_dictate());
        for requirement in &mut status.permission_requirements {
            requirement.state = FirstRunPermissionState::Ready;
        }
        assert!(status.ready_to_dictate());
    }

    #[test]
    fn first_run_ready_requires_permission_rows_not_only_coarse_flags() {
        let status = FirstRunStatus {
            model_ready: true,
            microphone_permission_ready: true,
            input_permission_ready: true,
            hotkey_registered: true,
            permission_requirements: vec![FirstRunPermissionRequirement {
                id: "uia_focus".to_string(),
                label: "UI Automation".to_string(),
                state: FirstRunPermissionState::NeedsReview,
                detail: "Windows focus proof is still required.".to_string(),
                action: "Run the UIA proof.".to_string(),
                action_label: "Show UIA step".to_string(),
            }],
            ..FirstRunStatus::default()
        };

        assert!(!status.permission_requirements_ready());
        assert!(!status.ready_to_dictate());
    }

    #[test]
    fn first_run_next_step_prioritizes_blocked_model_metadata() {
        let mut status = FirstRunStatus {
            model_ready: false,
            required_models: vec![FirstRunModelStatus {
                id: "parakeet-v3".to_string(),
                task: "ASR".to_string(),
                lane: Some("cpu".to_string()),
                runtime: "onnxruntime".to_string(),
                file: "parakeet-v3-int8.onnx".to_string(),
                state: FirstRunModelState::Blocked,
                detail: "Registry checksum pending".to_string(),
                download_available: false,
                download_size_mb: None,
                download_source_count: 0,
                license: "Apache-2.0".to_string(),
                license_review_required: false,
            }],
            ..FirstRunStatus::default()
        };
        status.recompute_next_step();

        assert_eq!(status.next_step.kind, FirstRunNextStepKind::ModelMetadata);
        assert_eq!(status.next_step.target_id.as_deref(), Some("parakeet-v3"));
        assert!(status.next_step.proof_requirement.contains("sha256"));
    }

    #[test]
    fn first_run_next_step_points_missing_models_to_reviewed_install() {
        let mut status = FirstRunStatus {
            model_ready: false,
            required_models: vec![FirstRunModelStatus {
                id: "silero-vad".to_string(),
                task: "VAD".to_string(),
                lane: None,
                runtime: "onnxruntime".to_string(),
                file: "silero_vad.onnx".to_string(),
                state: FirstRunModelState::Missing,
                detail: "Download required before first dictation".to_string(),
                download_available: true,
                download_size_mb: Some(2),
                download_source_count: 1,
                license: "MIT".to_string(),
                license_review_required: false,
            }],
            ..FirstRunStatus::default()
        };
        status.recompute_next_step();

        assert_eq!(status.next_step.kind, FirstRunNextStepKind::ModelInstall);
        assert_eq!(status.next_step.target_id.as_deref(), Some("silero-vad"));
        assert_eq!(status.next_step.action_label, "Install artifact");
    }

    #[test]
    fn first_run_next_step_orders_permissions_hotkey_dictation_and_completion() {
        let mut status = FirstRunStatus {
            model_ready: true,
            hotkey_registered: false,
            ..FirstRunStatus::default()
        };
        status.recompute_next_step();
        assert_eq!(status.next_step.kind, FirstRunNextStepKind::Permission);

        for requirement in &mut status.permission_requirements {
            requirement.state = FirstRunPermissionState::Ready;
        }
        status.microphone_permission_ready = true;
        status.input_permission_ready = true;
        status.recompute_next_step();
        assert_eq!(status.next_step.kind, FirstRunNextStepKind::Hotkey);

        status.hotkey_registered = true;
        status.recompute_next_step();
        assert_eq!(status.next_step.kind, FirstRunNextStepKind::Dictation);

        status.first_dictation_completed = true;
        status.recompute_next_step();
        assert_eq!(status.next_step.kind, FirstRunNextStepKind::Complete);
    }

    #[test]
    fn first_run_status_carries_hotkey_registration_errors() {
        let status = FirstRunStatus {
            hotkey_registration_error: Some("shortcut already registered".to_string()),
            ..FirstRunStatus::default()
        };

        assert!(!status.ready_to_dictate());
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(
            json["hotkey_registration_error"],
            "shortcut already registered"
        );
        assert_eq!(json["hotkey_registered"], false);
        assert!(json["permission_requirements"]
            .as_array()
            .is_some_and(|requirements| !requirements.is_empty()));
        assert_eq!(json["next_step"]["kind"], "model_metadata");
    }

    #[test]
    fn first_run_permission_requirements_match_the_platform_contract() {
        let requirements = first_run_permission_requirements();
        let ids = requirements
            .iter()
            .map(|requirement| requirement.id.as_str())
            .collect::<Vec<_>>();

        if cfg!(target_os = "macos") {
            assert_eq!(ids, ["microphone", "accessibility", "input_monitoring"]);
        } else if cfg!(target_os = "windows") {
            assert_eq!(ids, ["microphone", "uia_focus", "sendinput"]);
        } else if cfg!(target_os = "linux") {
            assert_eq!(ids, ["microphone", "accessibility_bus", "uinput"]);
        } else {
            assert_eq!(ids, ["platform_review"]);
        }

        assert!(requirements
            .iter()
            .all(|requirement| !matches!(requirement.state, FirstRunPermissionState::Ready)));
        assert!(requirements
            .iter()
            .all(|requirement| !requirement.action_label.trim().is_empty()));
    }

    #[test]
    fn first_run_permission_action_returns_manual_proof_boundary() {
        let requirement = first_run_permission_requirements()
            .into_iter()
            .next()
            .expect("platform contract has at least one requirement");
        let outcome = first_run_permission_action(&requirement.id).unwrap();

        assert_eq!(outcome.requirement_id, requirement.id);
        assert_eq!(outcome.label, requirement.label);
        assert_eq!(outcome.state, requirement.state);
        assert_eq!(outcome.action_label, requirement.action_label);
        assert_eq!(
            outcome.settings_target.as_deref(),
            first_run_permission_settings_target(&requirement.id)
        );
        assert!(!outcome.manual_step.trim().is_empty());
        assert!(!outcome.proof_requirement.trim().is_empty());
        assert_eq!(
            outcome.proof_command.as_deref(),
            first_run_permission_proof_command(&requirement.id)
        );
        assert_eq!(
            outcome.expected_evidence,
            first_run_permission_expected_evidence(&requirement.id)
        );
        assert_eq!(
            outcome.ready_boundary,
            first_run_permission_ready_boundary(&requirement.id)
        );
    }

    #[test]
    fn first_run_permission_settings_open_outcome_does_not_mark_ready() {
        let requirement = first_run_permission_requirements()
            .into_iter()
            .next()
            .expect("platform contract has at least one requirement");
        let outcome = first_run_permission_settings_open_outcome(&requirement.id, true).unwrap();

        assert_eq!(outcome.requirement_id, requirement.id);
        assert!(outcome.opened);
        assert_eq!(
            outcome.settings_target.as_deref(),
            first_run_permission_settings_target(&requirement.id)
        );
        assert!(!outcome.proof_requirement.trim().is_empty());
        assert!(!outcome.expected_evidence.trim().is_empty());
    }

    #[test]
    fn first_run_permission_action_rejects_unknown_ids() {
        assert_eq!(
            first_run_permission_action("not-a-real-permission"),
            Err(SettingsError::UnknownPermissionRequirement(
                "not-a-real-permission".to_string()
            ))
        );
    }

    #[test]
    fn first_run_permission_requirements_sync_runtime_evidence() {
        let mut status = FirstRunStatus {
            microphone_permission_ready: true,
            input_permission_ready: true,
            ..FirstRunStatus::default()
        };

        sync_first_run_permission_requirements(&mut status);

        let microphone = status
            .permission_requirements
            .iter()
            .find(|requirement| requirement.id == "microphone")
            .expect("microphone requirement exists");
        assert_eq!(microphone.state, FirstRunPermissionState::Ready);
        assert!(microphone.detail.contains("persisted local audio"));

        if cfg!(target_os = "macos") {
            let input_monitoring = status
                .permission_requirements
                .iter()
                .find(|requirement| requirement.id == "input_monitoring")
                .expect("macOS input monitoring requirement exists");
            assert_eq!(input_monitoring.state, FirstRunPermissionState::Ready);
            assert!(input_monitoring.detail.contains("OS event stream"));
        }
    }

    #[test]
    fn first_run_proof_plan_exports_backend_setup_truth() {
        let mut snapshot = AppSnapshot::default();
        snapshot.settings.first_run = FirstRunStatus {
            model_ready: true,
            required_models: vec![FirstRunModelStatus {
                id: "fixture-asr".to_string(),
                task: "ASR".to_string(),
                lane: Some("cpu".to_string()),
                runtime: "onnxruntime".to_string(),
                file: "fixture-asr.onnx".to_string(),
                state: FirstRunModelState::Ready,
                detail: "Verified artifact, 1 MB on disk".to_string(),
                download_available: true,
                download_size_mb: Some(1),
                download_source_count: 1,
                license: "Apache-2.0".to_string(),
                license_review_required: false,
            }],
            selected_asr_model_id: Some("fixture-asr".to_string()),
            permission_requirements: vec![FirstRunPermissionRequirement {
                id: "uia_focus".to_string(),
                label: "UI Automation focus access".to_string(),
                state: FirstRunPermissionState::NeedsReview,
                detail: "Validate focus and secure-field refusal.".to_string(),
                action: "Run the Windows UIA self-test.".to_string(),
                action_label: "Show UIA proof".to_string(),
            }],
            hotkey_registered: true,
            setup_timing: FirstRunSetupTiming::from_parts(Some(1_000), None),
            ..FirstRunStatus::default()
        };
        snapshot.settings.first_run.recompute_next_step();

        let plan = first_run_proof_plan(&snapshot, 12_345);

        assert_eq!(plan.schema_version, FIRST_RUN_PROOF_PLAN_SCHEMA_VERSION);
        assert_eq!(plan.generated_at_ms, 12_345);
        assert_eq!(plan.app_name, APP_NAME);
        assert_eq!(plan.selected_asr_model_id.as_deref(), Some("fixture-asr"));
        assert_eq!(plan.next_step.kind, FirstRunNextStepKind::Permission);
        assert!(!plan.ready_to_dictate);
        assert!(plan.proof_items.iter().any(|item| item.id == "next_step"
            && item.state == FirstRunProofItemState::Pending
            && item.target_id.as_deref() == Some("uia_focus")));
        assert!(plan
            .proof_items
            .iter()
            .any(|item| item.id == "model:fixture-asr"
                && item.state == FirstRunProofItemState::Ready));
        assert!(plan
            .proof_items
            .iter()
            .any(|item| item.id == "permission:uia_focus"
                && item.state == FirstRunProofItemState::NeedsReview
                && item
                    .proof_command
                    .as_deref()
                    .is_some_and(|command| command.contains("uia-selftest"))));
        assert!(plan
            .proof_items
            .iter()
            .any(|item| item.id == "first_dictation"
                && item.state == FirstRunProofItemState::Pending));
    }

    #[test]
    fn first_run_model_status_serializes_for_setup_ui() {
        let status = FirstRunModelStatus {
            id: "parakeet-v3".to_string(),
            task: "ASR".to_string(),
            lane: Some("cpu".to_string()),
            runtime: "onnxruntime".to_string(),
            file: "parakeet-v3-int8.onnx".to_string(),
            state: FirstRunModelState::Blocked,
            detail: "Registry checksum pending".to_string(),
            download_available: false,
            download_size_mb: None,
            download_source_count: 0,
            license: "Apache-2.0".to_string(),
            license_review_required: false,
        };

        let json = serde_json::to_string(&status).unwrap();

        assert!(json.contains("\"state\":\"blocked\""));
        assert!(json.contains("\"id\":\"parakeet-v3\""));
    }

    #[test]
    fn first_run_asr_candidate_serializes_selected_recommendation() {
        let candidate = FirstRunAsrCandidate {
            id: "parakeet-v3".to_string(),
            lane: Some("cpu".to_string()),
            runtime: "onnxruntime".to_string(),
            size_mb: 600,
            min_hw: "any".to_string(),
            state: FirstRunModelState::Missing,
            detail: "Download required before first dictation".to_string(),
            download_available: true,
            download_size_mb: Some(600),
            download_source_count: 2,
            selected: true,
            recommendation: Some("CPU-safe first-run default".to_string()),
            license: "Apache-2.0".to_string(),
            license_review_required: false,
        };

        let json = serde_json::to_string(&candidate).unwrap();

        assert!(json.contains("\"selected\":true"));
        assert!(json.contains("CPU-safe first-run default"));
    }

    #[test]
    fn first_run_asr_runtime_status_serializes_boundary() {
        let status = FirstRunAsrRuntimeStatus {
            state: FirstRunAsrRuntimeState::VerifiedArtifact,
            selected_model_id: Some("parakeet-v3".to_string()),
            lane: Some("cpu".to_string()),
            runtime: Some("onnxruntime".to_string()),
            artifact_path: Some("/tmp/kaydence/parakeet.onnx".to_string()),
            artifact_size_bytes: Some(1024),
            adapter_ready: false,
            detail: "Verified artifact is ready; adapter pending.".to_string(),
            proof_requirement: "Real adapter must emit transcript events.".to_string(),
        };

        let json = serde_json::to_string(&status).unwrap();

        assert!(json.contains("\"state\":\"verified_artifact\""));
        assert!(json.contains("\"adapter_ready\":false"));
        assert!(json.contains("parakeet-v3"));
    }
}
