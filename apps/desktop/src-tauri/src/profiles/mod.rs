//! Target binding and per-app profile resolution.
//!
//! This module owns the session-start target decision. Platform frontmost-app
//! adapters plug into the detector trait; until those adapters are validated,
//! the runtime uses an explicit unknown-app fallback rather than guessing from
//! window titles or content.

use crate::events::{AppRef, CleanupDial};

pub const DEFAULT_PROFILE_ID: &str = "default";
pub const UNKNOWN_APP_ID: &str = "unknown";
pub const UNKNOWN_APP_NAME: &str = "Unknown app";

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ProfileError {
    #[error("shipped profile {profile_id} cannot select Full cleanup")]
    ShippedProfileCannotSelectFull { profile_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppProfile {
    pub id: String,
    pub display_name: String,
    pub app_ids: Vec<String>,
    pub cleanup_dial: CleanupDial,
    pub user_edited: bool,
}

impl AppProfile {
    pub fn default_profile() -> Self {
        Self {
            id: DEFAULT_PROFILE_ID.to_string(),
            display_name: "Default".to_string(),
            app_ids: Vec::new(),
            cleanup_dial: CleanupDial::Light,
            user_edited: false,
        }
    }

    pub fn shipped(
        id: impl Into<String>,
        display_name: impl Into<String>,
        app_ids: Vec<String>,
        cleanup_dial: CleanupDial,
    ) -> Result<Self, ProfileError> {
        let id = id.into();
        if cleanup_dial == CleanupDial::Full {
            return Err(ProfileError::ShippedProfileCannotSelectFull { profile_id: id });
        }

        Ok(Self {
            id,
            display_name: display_name.into(),
            app_ids,
            cleanup_dial,
            user_edited: false,
        })
    }

    pub fn user_edited(
        id: impl Into<String>,
        display_name: impl Into<String>,
        app_ids: Vec<String>,
        cleanup_dial: CleanupDial,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            app_ids,
            cleanup_dial,
            user_edited: true,
        }
    }

    fn matches_app(&self, app: &AppRef) -> bool {
        self.app_ids.iter().any(|candidate| candidate == &app.id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileStore {
    default_profile: AppProfile,
    profiles: Vec<AppProfile>,
}

impl Default for ProfileStore {
    fn default() -> Self {
        Self {
            default_profile: AppProfile::default_profile(),
            profiles: Vec::new(),
        }
    }
}

impl ProfileStore {
    pub fn with_profiles(profiles: Vec<AppProfile>) -> Self {
        Self {
            default_profile: AppProfile::default_profile(),
            profiles,
        }
    }

    pub fn default_profile(&self) -> AppProfile {
        self.default_profile.clone()
    }

    pub fn resolve(&self, app: &AppRef) -> AppProfile {
        self.profiles
            .iter()
            .find(|profile| profile.matches_app(app))
            .cloned()
            .unwrap_or_else(|| self.default_profile.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetSource {
    Detected,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionTarget {
    pub app: AppRef,
    pub profile: AppProfile,
    pub source: TargetSource,
}

pub trait FrontmostAppDetector {
    fn frontmost_app(&mut self) -> Option<AppRef>;
}

pub trait ResolveSessionTarget {
    fn resolve_session_target(&mut self) -> SessionTarget;
}

#[derive(Debug, Default)]
pub struct UnknownFrontmostAppDetector;

impl FrontmostAppDetector for UnknownFrontmostAppDetector {
    fn frontmost_app(&mut self) -> Option<AppRef> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct StaticFrontmostAppDetector {
    app: Option<AppRef>,
}

impl StaticFrontmostAppDetector {
    pub fn new(app: AppRef) -> Self {
        Self { app: Some(app) }
    }

    pub fn unknown() -> Self {
        Self { app: None }
    }
}

impl FrontmostAppDetector for StaticFrontmostAppDetector {
    fn frontmost_app(&mut self) -> Option<AppRef> {
        self.app.clone()
    }
}

pub struct SessionTargetResolver<D> {
    detector: D,
    profiles: ProfileStore,
}

impl<D> SessionTargetResolver<D>
where
    D: FrontmostAppDetector,
{
    pub fn new(detector: D, profiles: ProfileStore) -> Self {
        Self { detector, profiles }
    }
}

impl<D> ResolveSessionTarget for SessionTargetResolver<D>
where
    D: FrontmostAppDetector,
{
    fn resolve_session_target(&mut self) -> SessionTarget {
        match self.detector.frontmost_app() {
            Some(app) => {
                let profile = self.profiles.resolve(&app);
                SessionTarget {
                    app,
                    profile,
                    source: TargetSource::Detected,
                }
            }
            None => {
                let app = unknown_app_ref();
                let profile = self.profiles.default_profile();
                SessionTarget {
                    app,
                    profile,
                    source: TargetSource::Unknown,
                }
            }
        }
    }
}

pub fn platform_target_resolver() -> SessionTargetResolver<UnknownFrontmostAppDetector> {
    SessionTargetResolver::new(UnknownFrontmostAppDetector, ProfileStore::default())
}

pub fn unknown_app_ref() -> AppRef {
    AppRef {
        id: UNKNOWN_APP_ID.to_string(),
        name: UNKNOWN_APP_NAME.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(id: &str, name: &str) -> AppRef {
        AppRef {
            id: id.to_string(),
            name: name.to_string(),
        }
    }

    #[test]
    fn unknown_target_uses_default_profile_without_guessing() {
        let mut resolver = SessionTargetResolver::new(
            StaticFrontmostAppDetector::unknown(),
            ProfileStore::default(),
        );

        let target = resolver.resolve_session_target();

        assert_eq!(target.app, unknown_app_ref());
        assert_eq!(target.profile.id, DEFAULT_PROFILE_ID);
        assert_eq!(target.profile.cleanup_dial, CleanupDial::Light);
        assert_eq!(target.source, TargetSource::Unknown);
    }

    #[test]
    fn exact_app_id_profile_wins_over_default() {
        let editor = app("com.example.editor", "Example Editor");
        let code_profile =
            AppProfile::shipped("code", "Code", vec![editor.id.clone()], CleanupDial::Light)
                .unwrap();
        let profiles = ProfileStore::with_profiles(vec![code_profile.clone()]);
        let mut resolver =
            SessionTargetResolver::new(StaticFrontmostAppDetector::new(editor.clone()), profiles);

        let target = resolver.resolve_session_target();

        assert_eq!(target.app, editor);
        assert_eq!(target.profile, code_profile);
        assert_eq!(target.source, TargetSource::Detected);
    }

    #[test]
    fn unknown_app_gets_default_when_profiles_exist() {
        let profiles = ProfileStore::with_profiles(vec![AppProfile::shipped(
            "mail",
            "Mail",
            vec!["com.example.mail".to_string()],
            CleanupDial::Light,
        )
        .unwrap()]);
        let mut resolver =
            SessionTargetResolver::new(StaticFrontmostAppDetector::unknown(), profiles);

        let target = resolver.resolve_session_target();

        assert_eq!(target.app, unknown_app_ref());
        assert_eq!(target.profile.id, DEFAULT_PROFILE_ID);
    }

    #[test]
    fn unknown_source_cannot_match_non_default_profile() {
        let profiles = ProfileStore::with_profiles(vec![AppProfile::shipped(
            "unknown-special",
            "Unknown Special",
            vec![UNKNOWN_APP_ID.to_string()],
            CleanupDial::Light,
        )
        .unwrap()]);
        let mut resolver =
            SessionTargetResolver::new(StaticFrontmostAppDetector::unknown(), profiles);

        let target = resolver.resolve_session_target();

        assert_eq!(target.app, unknown_app_ref());
        assert_eq!(target.profile.id, DEFAULT_PROFILE_ID);
        assert_eq!(target.source, TargetSource::Unknown);
    }

    #[test]
    fn shipped_profile_cannot_silently_select_full() {
        let err =
            AppProfile::shipped("too-much", "Too Much", Vec::new(), CleanupDial::Full).unwrap_err();

        assert_eq!(
            err,
            ProfileError::ShippedProfileCannotSelectFull {
                profile_id: "too-much".to_string()
            }
        );
    }

    #[test]
    fn user_edited_profile_may_select_full() {
        let profile = AppProfile::user_edited("custom", "Custom", Vec::new(), CleanupDial::Full);

        assert!(profile.user_edited);
        assert_eq!(profile.cleanup_dial, CleanupDial::Full);
    }
}
