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
    pub first_dictation_completed: bool,
}

impl Default for UserSettingsFile {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            selected_asr_model_id: None,
            hotkey_mode: None,
            hotkey_primary_binding: None,
            first_dictation_completed: false,
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
    pub permission_requirements: Vec<FirstRunPermissionRequirement>,
    pub microphone_permission_ready: bool,
    pub input_permission_ready: bool,
    pub hotkey_registered: bool,
    pub hotkey_registration_error: Option<String>,
    pub first_dictation_completed: bool,
}

impl Default for FirstRunStatus {
    fn default() -> Self {
        Self {
            model_ready: false,
            model_readiness_error: None,
            required_models: Vec::new(),
            asr_candidates: Vec::new(),
            recommended_asr_model_id: None,
            selected_asr_model_id: None,
            permission_requirements: first_run_permission_requirements(),
            microphone_permission_ready: false,
            input_permission_ready: false,
            hotkey_registered: false,
            hotkey_registration_error: None,
            first_dictation_completed: false,
        }
    }
}

impl FirstRunStatus {
    pub fn ready_to_dictate(&self) -> bool {
        self.model_ready
            && self.microphone_permission_ready
            && self.input_permission_ready
            && self.hotkey_registered
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
    pub manual_step: String,
    pub proof_requirement: String,
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
        manual_step: requirement.action,
        proof_requirement: first_run_permission_proof(requirement_id).to_string(),
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
            "Run `cargo run --bin atspi-selftest -- focus-track` in the Linux desktop session."
        }
        "uinput" => "Run the Linux human-focus proof: normal field types, password field refuses.",
        _ => "Record OS-specific setup evidence before marking the requirement ready.",
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
                "Run cargo run --bin atspi-selftest -- focus-track on the GNOME VM.",
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
    #[error("unknown first-run permission requirement: {0}")]
    UnknownPermissionRequirement(String),
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
    fn settings_store_saves_first_dictation_completion() {
        let dir = tmp();
        let store = SettingsStore::new(&dir);
        let settings = UserSettingsFile {
            first_dictation_completed: true,
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
        assert!(status.ready_to_dictate());
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
        assert!(!outcome.manual_step.trim().is_empty());
        assert!(!outcome.proof_requirement.trim().is_empty());
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
}
