//! Configure: single source of config truth; typed; hot-reload.
//!
//! P1 starts with an in-memory, serializable settings snapshot that captures the
//! first-run defaults the UI and platform modules need. Disk persistence and
//! hot-reload land when the settings screen becomes editable; until then this
//! module still owns the types, defaults, and validation rules.

use crate::events::CleanupDial;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Current settings-file schema version.
pub const SCHEMA_VERSION: u16 = 1;
/// The product brand string. Never hardcode "Kaydence" anywhere else.
pub const APP_NAME: &str = "Kaydence";

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
pub struct HotkeySettings {
    pub mode: HotkeyModeSetting,
    pub primary_binding: String,
    pub secondary_dial_override_binding: String,
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            mode: HotkeyModeSetting::PushToTalk,
            primary_binding: "RightAlt".into(),
            secondary_dial_override_binding: "Shift+RightAlt".into(),
        }
    }
}

impl HotkeySettings {
    fn validate(&self) -> Result<(), SettingsError> {
        if self.primary_binding.trim().is_empty() {
            return Err(SettingsError::EmptyHotkeyBinding);
        }
        if self.secondary_dial_override_binding.trim().is_empty() {
            return Err(SettingsError::EmptyHotkeyBinding);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HotkeyModeSetting {
    PushToTalk,
    Toggle,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FirstRunStatus {
    pub model_ready: bool,
    pub model_readiness_error: Option<String>,
    pub required_models: Vec<FirstRunModelStatus>,
    pub microphone_permission_ready: bool,
    pub input_permission_ready: bool,
    pub hotkey_registered: bool,
    pub hotkey_registration_error: Option<String>,
    pub first_dictation_completed: bool,
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
pub struct FirstRunModelStatus {
    pub id: String,
    pub task: String,
    pub lane: Option<String>,
    pub runtime: String,
    pub file: String,
    pub state: FirstRunModelState,
    pub detail: String,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_product_thesis() {
        let settings = AppSettings::default();
        assert_eq!(settings.schema_version, SCHEMA_VERSION);
        assert_eq!(settings.hotkey.mode, HotkeyModeSetting::PushToTalk);
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
        assert_eq!(
            serde_json::to_string(&status).unwrap(),
            "{\"model_ready\":false,\"model_readiness_error\":null,\"required_models\":[],\"microphone_permission_ready\":false,\"input_permission_ready\":false,\"hotkey_registered\":false,\"hotkey_registration_error\":\"shortcut already registered\",\"first_dictation_completed\":false}"
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
            license: "Apache-2.0".to_string(),
            license_review_required: false,
        };

        let json = serde_json::to_string(&status).unwrap();

        assert!(json.contains("\"state\":\"blocked\""));
        assert!(json.contains("\"id\":\"parakeet-v3\""));
    }
}
