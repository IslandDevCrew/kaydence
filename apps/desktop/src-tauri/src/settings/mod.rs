//! Configure: single source of config truth; typed; hot-reload.
//!
//! Stub (P0-T3) — but `APP_NAME` is real: the brand string is NEVER hardcoded
//! elsewhere (root `AGENTS.md` §9). Consumers read it from here so the
//! codename->name change stays one line.
#![allow(dead_code)]

/// The product brand string. Never hardcode "Kaydence" anywhere else.
pub const APP_NAME: &str = "Kaydence";

/// Reverse-DNS application identifier (matches tauri.conf.json `identifier`).
pub const APP_IDENTIFIER: &str = "io.kaydence.app";
