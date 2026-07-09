//! Remember: persist session rows and the exact event stream to local SQLite.
//!
//! History is the safety net for non-negotiable #2. It consumes only
//! `SessionEvent` values, stores the append-only event log as JSON, and keeps a
//! query-friendly session summary for the HUD/history browser.

use crate::events::{
    AppRef, CleanupDial, HoldReason, InjectMethod, SessionEvent, SessionId, Stage,
};
use crate::settings::APP_NAME;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const HISTORY_DB_FILE: &str = "history.sqlite3";
pub const SCHEMA_VERSION: i64 = 1;
pub const EXPORTS_DIR: &str = "exports";

#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("history io: {0}")]
    Io(#[from] std::io::Error),
    #[error("history sqlite: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("history event json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("history clock moved before unix epoch")]
    Clock,
}

pub struct HistoryStore {
    conn: Connection,
    path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeleteSessionOutcome {
    pub deleted: bool,
    pub audio_removed: bool,
    pub audio_path: Option<String>,
    pub exports_removed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportSessionOutcome {
    pub exported: bool,
    pub json_path: Option<String>,
    pub text_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PurgeHistoryOutcome {
    pub sessions_deleted: usize,
    pub audio_files_removed: usize,
    pub export_files_removed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RetentionSweepOutcome {
    pub sessions_deleted: usize,
    pub audio_files_removed: usize,
    pub export_files_removed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HistoryAudioPlayback {
    pub asset_path: String,
    pub mime_type: String,
    pub byte_length: u64,
}

#[derive(Debug, Clone, Serialize)]
struct HistorySessionExport {
    schema_version: i64,
    exported_ms: u64,
    session: HistorySession,
    events: Vec<SessionEvent>,
}

impl HistoryStore {
    pub fn open(app_data_dir: &Path) -> Result<Self, HistoryError> {
        fs::create_dir_all(app_data_dir)?;
        Self::open_at(&app_data_dir.join(HISTORY_DB_FILE))
    }

    pub fn open_at(path: &Path) -> Result<Self, HistoryError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let store = Self {
            conn,
            path: Some(path.to_path_buf()),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self, HistoryError> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn, path: None };
        store.migrate()?;
        Ok(store)
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn schema_version(&self) -> Result<i64, HistoryError> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?)
    }

    pub fn record_events(&mut self, events: &[SessionEvent]) -> Result<(), HistoryError> {
        for event in events {
            self.record_event(event)?;
        }
        Ok(())
    }

    pub fn record_event(&mut self, event: &SessionEvent) -> Result<(), HistoryError> {
        let session_id = event.session_id();
        let now_ms = now_ms()?;
        let event_type = event_type(event);
        let event_json = serde_json::to_string(event)?;

        let tx = self.conn.transaction()?;
        ensure_session(&tx, session_id, now_ms)?;
        apply_event_to_summary(&tx, event, now_ms)?;
        let sequence = next_sequence(&tx, session_id)?;
        tx.execute(
            "INSERT INTO session_events (session_id, sequence, event_type, event_json, recorded_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                session_id_string(session_id),
                sequence,
                event_type,
                event_json,
                now_ms
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn delete_session(&mut self, session_id: SessionId) -> Result<bool, HistoryError> {
        Ok(self.conn.execute(
            "DELETE FROM sessions WHERE session_id = ?1",
            params![session_id_string(session_id)],
        )? > 0)
    }

    pub fn delete_session_and_audio(
        &mut self,
        session_id: &str,
        app_data_dir: &Path,
    ) -> Result<DeleteSessionOutcome, HistoryError> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Ok(DeleteSessionOutcome {
                deleted: false,
                audio_removed: false,
                audio_path: None,
                exports_removed: 0,
            });
        }

        let Some(audio_path) = self.audio_path_for_session(session_id)? else {
            return Ok(DeleteSessionOutcome {
                deleted: false,
                audio_removed: false,
                audio_path: None,
                exports_removed: 0,
            });
        };

        let audio_removed = remove_safe_session_audio(audio_path.as_deref(), app_data_dir)?;
        let exports_removed = remove_session_exports(session_id, app_data_dir)?;
        let deleted = self.delete_session_by_string(session_id)?;

        Ok(DeleteSessionOutcome {
            deleted,
            audio_removed,
            audio_path,
            exports_removed,
        })
    }

    pub fn export_session(
        &self,
        session_id: SessionId,
        app_data_dir: &Path,
    ) -> Result<ExportSessionOutcome, HistoryError> {
        let Some(session) = self.get_session(session_id)? else {
            return Ok(ExportSessionOutcome {
                exported: false,
                json_path: None,
                text_path: None,
            });
        };
        let events = self.events_for_session(session_id)?;
        let export = HistorySessionExport {
            schema_version: SCHEMA_VERSION,
            exported_ms: now_ms()?.max(0) as u64,
            session,
            events,
        };

        let exports_dir = app_data_dir.join(EXPORTS_DIR);
        fs::create_dir_all(&exports_dir)?;
        let session_id = session_id_string(session_id);
        let json_path = exports_dir.join(format!("{session_id}.json"));
        let text_path = exports_dir.join(format!("{session_id}.txt"));

        fs::write(&json_path, serde_json::to_string_pretty(&export)?)?;
        fs::write(&text_path, render_session_text_export(&export))?;

        Ok(ExportSessionOutcome {
            exported: true,
            json_path: Some(json_path.display().to_string()),
            text_path: Some(text_path.display().to_string()),
        })
    }

    pub fn audio_playback(
        &self,
        session_id: SessionId,
        app_data_dir: &Path,
    ) -> Result<Option<HistoryAudioPlayback>, HistoryError> {
        let Some(audio_path) = self.audio_path_for_session(&session_id_string(session_id))? else {
            return Ok(None);
        };
        let Some(audio_path) = safe_session_audio_path(audio_path.as_deref(), app_data_dir)? else {
            return Ok(None);
        };
        if audio_path
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("wav")
        {
            return Ok(None);
        }
        let byte_length = fs::metadata(&audio_path)?.len();
        Ok(Some(HistoryAudioPlayback {
            asset_path: audio_path.display().to_string(),
            mime_type: "audio/wav".to_string(),
            byte_length,
        }))
    }

    pub fn purge_all(&mut self, app_data_dir: &Path) -> Result<PurgeHistoryOutcome, HistoryError> {
        let sessions_deleted = self.session_count()?;
        let audio_files_removed = remove_owned_dir_files(app_data_dir, "sessions")?;
        let export_files_removed = remove_owned_dir_files(app_data_dir, EXPORTS_DIR)?;

        self.conn.execute("DELETE FROM sessions", [])?;

        Ok(PurgeHistoryOutcome {
            sessions_deleted,
            audio_files_removed,
            export_files_removed,
        })
    }

    pub fn sweep_retention(
        &mut self,
        retention_days: u16,
        app_data_dir: &Path,
    ) -> Result<RetentionSweepOutcome, HistoryError> {
        let now_ms = now_ms()?.max(0) as u64;
        let retention_ms = Duration::from_secs(u64::from(retention_days.max(1)) * 24 * 60 * 60)
            .as_millis()
            .min(u128::from(u64::MAX)) as u64;
        self.sweep_retention_before(now_ms.saturating_sub(retention_ms), app_data_dir)
    }

    pub fn sweep_retention_before(
        &mut self,
        cutoff_ms: u64,
        app_data_dir: &Path,
    ) -> Result<RetentionSweepOutcome, HistoryError> {
        let expired = self.expired_sessions(cutoff_ms)?;
        let mut audio_files_removed = 0usize;
        let mut export_files_removed = 0usize;
        let mut sessions_deleted = 0usize;

        for session in expired {
            if remove_safe_session_audio(session.audio_path.as_deref(), app_data_dir)? {
                audio_files_removed += 1;
            }
            export_files_removed += remove_session_exports(&session.id, app_data_dir)?;
            if self.delete_session_by_string(&session.id)? {
                sessions_deleted += 1;
            }
        }

        audio_files_removed += remove_expired_owned_dir_files(app_data_dir, "sessions", cutoff_ms)?;
        export_files_removed +=
            remove_expired_owned_dir_files(app_data_dir, EXPORTS_DIR, cutoff_ms)?;

        Ok(RetentionSweepOutcome {
            sessions_deleted,
            audio_files_removed,
            export_files_removed,
        })
    }

    pub fn record_recovered_audio(
        &mut self,
        session_id: SessionId,
        wal_path: &Path,
        samples: u64,
    ) -> Result<bool, HistoryError> {
        if self
            .audio_path_for_session(&session_id_string(session_id))?
            .flatten()
            .is_some()
        {
            return Ok(false);
        }

        self.record_events(&[
            SessionEvent::AudioPersisted {
                id: session_id,
                wal_path: wal_path.display().to_string(),
            },
            SessionEvent::Failed {
                id: session_id,
                stage: Stage::Capture,
                error: format!(
                    "Recovered audio after app exit before transcription; {samples} samples preserved on disk."
                ),
            },
        ])?;
        Ok(true)
    }

    pub fn session_has_audio(&self, session_id: SessionId) -> Result<bool, HistoryError> {
        Ok(self
            .audio_path_for_session(&session_id_string(session_id))?
            .flatten()
            .is_some())
    }

    pub fn get_session(
        &self,
        session_id: SessionId,
    ) -> Result<Option<HistorySession>, HistoryError> {
        let row = self
            .conn
            .query_row(
                "SELECT s.session_id,
                        s.started_ms,
                        s.target_app_id,
                        s.target_app_name,
                        s.audio_path,
                        s.raw_text,
                        s.clean_text,
                        s.cleanup_dial_json,
                        s.injected_method_json,
                        s.held_reason_json,
                        s.failed_stage_json,
                        s.failed_error,
                        COUNT(e.id) AS event_count
                   FROM sessions s
              LEFT JOIN session_events e ON e.session_id = s.session_id
                  WHERE s.session_id = ?1
               GROUP BY s.session_id",
                params![session_id_string(session_id)],
                RawSessionRow::from_row,
            )
            .optional()?;
        row.map(HistorySession::try_from).transpose()
    }

    pub fn list_recent(&self, limit: usize) -> Result<Vec<HistorySession>, HistoryError> {
        let mut stmt = self.conn.prepare(
            "SELECT s.session_id,
                    s.started_ms,
                    s.target_app_id,
                    s.target_app_name,
                    s.audio_path,
                    s.raw_text,
                    s.clean_text,
                    s.cleanup_dial_json,
                    s.injected_method_json,
                    s.held_reason_json,
                    s.failed_stage_json,
                    s.failed_error,
                    COUNT(e.id) AS event_count
               FROM sessions s
          LEFT JOIN session_events e ON e.session_id = s.session_id
           GROUP BY s.session_id
           ORDER BY s.updated_ms DESC, s.session_id DESC
              LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], RawSessionRow::from_row)?;
        rows.map(|row| HistorySession::try_from(row?)).collect()
    }

    pub fn events_for_session(
        &self,
        session_id: SessionId,
    ) -> Result<Vec<SessionEvent>, HistoryError> {
        let mut stmt = self.conn.prepare(
            "SELECT event_json
               FROM session_events
              WHERE session_id = ?1
           ORDER BY sequence ASC",
        )?;
        let rows = stmt.query_map(params![session_id_string(session_id)], |row| {
            row.get::<_, String>(0)
        })?;

        rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
    }

    fn audio_path_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<Option<String>>, HistoryError> {
        Ok(self
            .conn
            .query_row(
                "SELECT audio_path FROM sessions WHERE session_id = ?1",
                params![session_id],
                |row| row.get(0),
            )
            .optional()?)
    }

    fn delete_session_by_string(&mut self, session_id: &str) -> Result<bool, HistoryError> {
        Ok(self.conn.execute(
            "DELETE FROM sessions WHERE session_id = ?1",
            params![session_id],
        )? > 0)
    }

    fn session_count(&self) -> Result<usize, HistoryError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))?;
        Ok(count.max(0) as usize)
    }

    fn expired_sessions(&self, cutoff_ms: u64) -> Result<Vec<ExpiredSession>, HistoryError> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id, audio_path
               FROM sessions
              WHERE COALESCE(started_ms, updated_ms, created_ms, 0) < ?1
           ORDER BY session_id ASC",
        )?;
        let rows = stmt.query_map(params![sqlite_ms(cutoff_ms)], |row| {
            Ok(ExpiredSession {
                id: row.get(0)?,
                audio_path: row.get(1)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(HistoryError::from)
    }

    fn migrate(&self) -> Result<(), HistoryError> {
        self.conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS history_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sessions (
                session_id TEXT PRIMARY KEY,
                started_ms INTEGER,
                target_app_id TEXT,
                target_app_name TEXT,
                audio_path TEXT,
                raw_text TEXT,
                clean_text TEXT,
                cleanup_dial_json TEXT,
                injected_method_json TEXT,
                held_reason_json TEXT,
                failed_stage_json TEXT,
                failed_error TEXT,
                created_ms INTEGER NOT NULL,
                updated_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS session_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                sequence INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                event_json TEXT NOT NULL,
                recorded_ms INTEGER NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE,
                UNIQUE (session_id, sequence)
            );

            CREATE INDEX IF NOT EXISTS idx_session_events_session
                ON session_events(session_id, sequence);
            CREATE INDEX IF NOT EXISTS idx_sessions_updated
                ON sessions(updated_ms DESC);

            PRAGMA user_version = 1;
            ",
        )?;
        self.conn.execute(
            "INSERT OR IGNORE INTO history_migrations (version, name, applied_ms)
             VALUES (?1, ?2, ?3)",
            params![SCHEMA_VERSION, "initial_history_schema", now_ms()?],
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HistorySession {
    pub id: String,
    pub started_ms: Option<u64>,
    pub target_app: Option<AppRef>,
    pub audio_path: Option<String>,
    pub raw_text: Option<String>,
    pub clean_text: Option<String>,
    pub cleanup_dial: Option<CleanupDial>,
    pub injected_method: Option<InjectMethod>,
    pub held_reason: Option<HoldReason>,
    pub failure: Option<HistoryFailure>,
    pub event_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HistoryFailure {
    pub stage: Stage,
    pub error: String,
}

struct RawSessionRow {
    id: String,
    started_ms: Option<i64>,
    target_app_id: Option<String>,
    target_app_name: Option<String>,
    audio_path: Option<String>,
    raw_text: Option<String>,
    clean_text: Option<String>,
    cleanup_dial_json: Option<String>,
    injected_method_json: Option<String>,
    held_reason_json: Option<String>,
    failed_stage_json: Option<String>,
    failed_error: Option<String>,
    event_count: i64,
}

struct ExpiredSession {
    id: String,
    audio_path: Option<String>,
}

impl RawSessionRow {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            started_ms: row.get(1)?,
            target_app_id: row.get(2)?,
            target_app_name: row.get(3)?,
            audio_path: row.get(4)?,
            raw_text: row.get(5)?,
            clean_text: row.get(6)?,
            cleanup_dial_json: row.get(7)?,
            injected_method_json: row.get(8)?,
            held_reason_json: row.get(9)?,
            failed_stage_json: row.get(10)?,
            failed_error: row.get(11)?,
            event_count: row.get(12)?,
        })
    }
}

impl TryFrom<RawSessionRow> for HistorySession {
    type Error = HistoryError;

    fn try_from(row: RawSessionRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            started_ms: row.started_ms.map(|value| value.max(0) as u64),
            target_app: match (row.target_app_id, row.target_app_name) {
                (Some(id), Some(name)) => Some(AppRef { id, name }),
                _ => None,
            },
            audio_path: row.audio_path,
            raw_text: row.raw_text,
            clean_text: row.clean_text,
            cleanup_dial: decode_optional_json(row.cleanup_dial_json)?,
            injected_method: decode_optional_json(row.injected_method_json)?,
            held_reason: decode_optional_json(row.held_reason_json)?,
            failure: match (
                decode_optional_json(row.failed_stage_json)?,
                row.failed_error,
            ) {
                (Some(stage), Some(error)) => Some(HistoryFailure { stage, error }),
                _ => None,
            },
            event_count: row.event_count.max(0) as usize,
        })
    }
}

fn ensure_session(
    conn: &rusqlite::Transaction<'_>,
    id: SessionId,
    now_ms: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO sessions (session_id, created_ms, updated_ms)
         VALUES (?1, ?2, ?2)",
        params![session_id_string(id), now_ms],
    )?;
    Ok(())
}

fn apply_event_to_summary(
    conn: &rusqlite::Transaction<'_>,
    event: &SessionEvent,
    now_ms: i64,
) -> Result<(), HistoryError> {
    match event {
        SessionEvent::Started {
            id,
            target_app,
            at_ms,
        } => {
            conn.execute(
                "UPDATE sessions
                    SET started_ms = ?2,
                        target_app_id = ?3,
                        target_app_name = ?4,
                        updated_ms = ?5
                  WHERE session_id = ?1",
                params![
                    session_id_string(*id),
                    sqlite_ms(*at_ms),
                    target_app.id,
                    target_app.name,
                    now_ms
                ],
            )?;
        }
        SessionEvent::AudioPersisted { id, wal_path } => {
            conn.execute(
                "UPDATE sessions
                    SET audio_path = ?2,
                        updated_ms = ?3
                  WHERE session_id = ?1",
                params![session_id_string(*id), wal_path, now_ms],
            )?;
        }
        SessionEvent::RawFinal { id, text } => {
            append_summary_text(conn, *id, "raw_text", text, now_ms)?;
        }
        SessionEvent::CleanFinal { id, text, dial } => {
            append_summary_text(conn, *id, "clean_text", text, now_ms)?;
            conn.execute(
                "UPDATE sessions
                    SET cleanup_dial_json = ?2,
                        updated_ms = ?3
                  WHERE session_id = ?1",
                params![session_id_string(*id), serde_json::to_string(dial)?, now_ms],
            )?;
        }
        SessionEvent::Injected { id, method } => {
            conn.execute(
                "UPDATE sessions
                    SET injected_method_json = ?2,
                        held_reason_json = NULL,
                        failed_stage_json = NULL,
                        failed_error = NULL,
                        updated_ms = ?3
                  WHERE session_id = ?1",
                params![
                    session_id_string(*id),
                    serde_json::to_string(method)?,
                    now_ms
                ],
            )?;
        }
        SessionEvent::Held { id, reason } => {
            conn.execute(
                "UPDATE sessions
                    SET held_reason_json = ?2,
                        updated_ms = ?3
                  WHERE session_id = ?1",
                params![
                    session_id_string(*id),
                    serde_json::to_string(reason)?,
                    now_ms
                ],
            )?;
        }
        SessionEvent::Failed { id, stage, error } => {
            conn.execute(
                "UPDATE sessions
                    SET failed_stage_json = ?2,
                        failed_error = ?3,
                        updated_ms = ?4
                  WHERE session_id = ?1",
                params![
                    session_id_string(*id),
                    serde_json::to_string(stage)?,
                    error,
                    now_ms
                ],
            )?;
        }
        SessionEvent::Partial { id, .. }
        | SessionEvent::PredictionOffered { id, .. }
        | SessionEvent::PredictionMerged { id, .. }
        | SessionEvent::PredictionDismissed { id, .. }
        | SessionEvent::PredictionStale { id } => {
            conn.execute(
                "UPDATE sessions
                    SET updated_ms = ?2
                  WHERE session_id = ?1",
                params![session_id_string(*id), now_ms],
            )?;
        }
    }
    Ok(())
}

fn append_summary_text(
    conn: &rusqlite::Transaction<'_>,
    id: SessionId,
    column: &'static str,
    text: &str,
    now_ms: i64,
) -> rusqlite::Result<()> {
    let sql = format!(
        "UPDATE sessions
            SET {column} = CASE
                    WHEN {column} IS NULL OR {column} = '' THEN ?2
                    ELSE {column} || ' ' || ?2
                END,
                updated_ms = ?3
          WHERE session_id = ?1"
    );
    conn.execute(&sql, params![session_id_string(id), text, now_ms])?;
    Ok(())
}

fn next_sequence(conn: &rusqlite::Transaction<'_>, id: SessionId) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COALESCE(MAX(sequence) + 1, 0)
           FROM session_events
          WHERE session_id = ?1",
        params![session_id_string(id)],
        |row| row.get(0),
    )
}

fn event_type(event: &SessionEvent) -> &'static str {
    match event {
        SessionEvent::Started { .. } => "started",
        SessionEvent::AudioPersisted { .. } => "audio_persisted",
        SessionEvent::Partial { .. } => "partial",
        SessionEvent::RawFinal { .. } => "raw_final",
        SessionEvent::CleanFinal { .. } => "clean_final",
        SessionEvent::Injected { .. } => "injected",
        SessionEvent::Held { .. } => "held",
        SessionEvent::Failed { .. } => "failed",
        SessionEvent::PredictionOffered { .. } => "prediction_offered",
        SessionEvent::PredictionMerged { .. } => "prediction_merged",
        SessionEvent::PredictionDismissed { .. } => "prediction_dismissed",
        SessionEvent::PredictionStale { .. } => "prediction_stale",
    }
}

fn session_id_string(id: SessionId) -> String {
    id.0.to_string()
}

fn remove_safe_session_audio(
    audio_path: Option<&str>,
    app_data_dir: &Path,
) -> Result<bool, HistoryError> {
    let Some(audio_path) = safe_session_audio_path(audio_path, app_data_dir)? else {
        return Ok(false);
    };
    fs::remove_file(audio_path)?;
    Ok(true)
}

fn safe_session_audio_path(
    audio_path: Option<&str>,
    app_data_dir: &Path,
) -> Result<Option<PathBuf>, HistoryError> {
    let Some(audio_path) = audio_path else {
        return Ok(None);
    };
    let audio_path = PathBuf::from(audio_path);
    if !audio_path.is_absolute() || !audio_path.exists() {
        return Ok(None);
    }

    let app_data_dir = fs::canonicalize(app_data_dir)?;
    let sessions_dir = match fs::canonicalize(app_data_dir.join("sessions")) {
        Ok(path) => path,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    if !sessions_dir.starts_with(&app_data_dir) {
        return Ok(None);
    }

    let audio_path = fs::canonicalize(audio_path)?;
    if !audio_path.starts_with(&sessions_dir) || !audio_path.is_file() {
        return Ok(None);
    }
    Ok(Some(audio_path))
}

fn remove_session_exports(session_id: &str, app_data_dir: &Path) -> Result<usize, HistoryError> {
    if ulid::Ulid::from_string(session_id).is_err() {
        return Ok(0);
    }

    let Some(exports_dir) = canonical_owned_child_dir(app_data_dir, EXPORTS_DIR)? else {
        return Ok(0);
    };

    let mut removed = 0usize;
    for extension in ["json", "txt"] {
        let path = exports_dir.join(format!("{session_id}.{extension}"));
        if remove_owned_file(&path)? {
            removed += 1;
        }
    }
    Ok(removed)
}

fn remove_owned_dir_files(app_data_dir: &Path, child: &str) -> Result<usize, HistoryError> {
    remove_owned_dir_files_matching(app_data_dir, child, |_| Ok(true))
}

fn remove_expired_owned_dir_files(
    app_data_dir: &Path,
    child: &str,
    cutoff_ms: u64,
) -> Result<usize, HistoryError> {
    remove_owned_dir_files_matching(app_data_dir, child, |path| {
        Ok(file_modified_ms(path)?.is_some_and(|modified_ms| modified_ms < cutoff_ms))
    })
}

fn remove_owned_dir_files_matching(
    app_data_dir: &Path,
    child: &str,
    should_remove: impl Fn(&Path) -> Result<bool, HistoryError>,
) -> Result<usize, HistoryError> {
    let Some(dir) = canonical_owned_child_dir(app_data_dir, child)? else {
        return Ok(0);
    };

    let mut removed = 0usize;
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if should_remove(&path)? && remove_owned_file(&path)? {
            removed += 1;
        }
    }
    Ok(removed)
}

fn canonical_owned_child_dir(
    app_data_dir: &Path,
    child: &str,
) -> Result<Option<PathBuf>, HistoryError> {
    let app_data_dir = fs::canonicalize(app_data_dir)?;
    let child_dir = match fs::canonicalize(app_data_dir.join(child)) {
        Ok(path) => path,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    if child_dir.starts_with(&app_data_dir) && child_dir.is_dir() {
        Ok(Some(child_dir))
    } else {
        Ok(None)
    }
}

fn remove_owned_file(path: &Path) -> Result<bool, HistoryError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(err.into()),
    };
    let file_type = metadata.file_type();
    if !file_type.is_file() && !file_type.is_symlink() {
        return Ok(false);
    }
    fs::remove_file(path)?;
    Ok(true)
}

fn file_modified_ms(path: &Path) -> Result<Option<u64>, HistoryError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    let modified = metadata.modified()?;
    let duration = modified
        .duration_since(UNIX_EPOCH)
        .map_err(|_| HistoryError::Clock)?;
    Ok(Some(duration.as_millis().min(u128::from(u64::MAX)) as u64))
}

fn render_session_text_export(export: &HistorySessionExport) -> String {
    let session = &export.session;
    let target_app = session
        .target_app
        .as_ref()
        .map(|app| format!("{} ({})", app.name, app.id))
        .unwrap_or_else(|| "Unknown app".to_string());
    let status = session_status(session);
    let audio_path = session.audio_path.as_deref().unwrap_or("No audio path");
    let raw_text = session.raw_text.as_deref().unwrap_or("");
    let clean_text = session.clean_text.as_deref().unwrap_or("");

    let started_ms = session
        .started_ms
        .map(|value| value.to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    let event_count = export.events.len();

    format!(
        "{APP_NAME} Local History Export\n\
Session: {session_id}\n\
Exported ms: {exported_ms}\n\
Started ms: {started_ms}\n\
Target app: {target_app}\n\
Status: {status}\n\
Audio: {audio_path}\n\
Events: {event_count}\n\
\n\
Raw transcript:\n\
{raw_text}\n\
\n\
Clean transcript:\n\
{clean_text}\n",
        session_id = session.id,
        exported_ms = export.exported_ms,
    )
}

fn session_status(session: &HistorySession) -> String {
    if let Some(failure) = &session.failure {
        return format!("Failed: {:?} - {}", failure.stage, failure.error);
    }
    if let Some(reason) = &session.held_reason {
        return format!("Held: {reason:?}");
    }
    if let Some(method) = &session.injected_method {
        return format!("Injected: {method:?}");
    }
    if session.audio_path.is_some() {
        return "Audio saved".to_string();
    }
    "Started".to_string()
}

fn decode_optional_json<T>(value: Option<String>) -> Result<Option<T>, HistoryError>
where
    T: serde::de::DeserializeOwned,
{
    value
        .map(|json| serde_json::from_str(&json))
        .transpose()
        .map_err(HistoryError::from)
}

fn sqlite_ms(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}

fn now_ms() -> Result<i64, HistoryError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| HistoryError::Clock)?;
    Ok(duration.as_millis().min(i64::MAX as u128) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ulid::Ulid;

    fn sid(value: u128) -> SessionId {
        SessionId::new(Ulid::from_parts(1, value))
    }

    fn app() -> AppRef {
        AppRef {
            id: "com.example.editor".to_string(),
            name: "Example Editor".to_string(),
        }
    }

    fn temp_app_data(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kaydence-history-{label}-{}-{}",
            std::process::id(),
            Ulid::new()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn migrations_create_schema_and_version_marker() {
        let store = HistoryStore::open_in_memory().unwrap();

        assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
        assert!(store.path().is_none());
        let migrations: i64 = store
            .conn
            .query_row("SELECT COUNT(*) FROM history_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(migrations, 1);
    }

    #[test]
    fn records_normal_flow_summary_and_replays_events() {
        let id = sid(42);
        let mut store = HistoryStore::open_in_memory().unwrap();
        let events = vec![
            SessionEvent::Started {
                id,
                target_app: app(),
                at_ms: 10,
            },
            SessionEvent::AudioPersisted {
                id,
                wal_path: "/tmp/session.wav".to_string(),
            },
            SessionEvent::RawFinal {
                id,
                text: "um hello captain".to_string(),
            },
            SessionEvent::CleanFinal {
                id,
                text: "Hello captain.".to_string(),
                dial: CleanupDial::Light,
            },
            SessionEvent::Injected {
                id,
                method: InjectMethod::Native,
            },
        ];

        store.record_events(&events).unwrap();

        let session = store.get_session(id).unwrap().unwrap();
        assert_eq!(session.started_ms, Some(10));
        assert_eq!(session.target_app, Some(app()));
        assert_eq!(session.audio_path.as_deref(), Some("/tmp/session.wav"));
        assert_eq!(session.raw_text.as_deref(), Some("um hello captain"));
        assert_eq!(session.clean_text.as_deref(), Some("Hello captain."));
        assert_eq!(session.cleanup_dial, Some(CleanupDial::Light));
        assert_eq!(session.injected_method, Some(InjectMethod::Native));
        assert_eq!(session.event_count, events.len());
        assert_eq!(store.events_for_session(id).unwrap(), events);
    }

    #[test]
    fn held_and_failed_sessions_keep_available_text() {
        let id = sid(43);
        let mut store = HistoryStore::open_in_memory().unwrap();

        store
            .record_events(&[
                SessionEvent::RawFinal {
                    id,
                    text: "raw survives".to_string(),
                },
                SessionEvent::CleanFinal {
                    id,
                    text: "Raw survives.".to_string(),
                    dial: CleanupDial::Light,
                },
                SessionEvent::Held {
                    id,
                    reason: HoldReason::SecureField,
                },
                SessionEvent::Failed {
                    id,
                    stage: Stage::Inject,
                    error: "native insert unavailable".to_string(),
                },
            ])
            .unwrap();

        let session = store.get_session(id).unwrap().unwrap();
        assert_eq!(session.raw_text.as_deref(), Some("raw survives"));
        assert_eq!(session.clean_text.as_deref(), Some("Raw survives."));
        assert_eq!(session.held_reason, Some(HoldReason::SecureField));
        assert_eq!(
            session.failure,
            Some(HistoryFailure {
                stage: Stage::Inject,
                error: "native insert unavailable".to_string()
            })
        );
    }

    #[test]
    fn history_session_serializes_for_frontend() {
        let session = HistorySession {
            id: "01HX0000000000000000000000".to_string(),
            started_ms: Some(10),
            target_app: Some(app()),
            audio_path: Some("/tmp/session.wav".to_string()),
            raw_text: None,
            clean_text: None,
            cleanup_dial: None,
            injected_method: None,
            held_reason: None,
            failure: Some(HistoryFailure {
                stage: Stage::Recognize,
                error: "local ASR adapter pending".to_string(),
            }),
            event_count: 3,
        };

        let json = serde_json::to_string(&session).unwrap();

        assert!(json.contains("\"target_app\""));
        assert!(json.contains("\"stage\":\"recognize\""));
        assert!(json.contains("local ASR adapter pending"));
    }

    #[test]
    fn separated_segments_are_aggregated_without_losing_event_order() {
        let id = sid(44);
        let mut store = HistoryStore::open_in_memory().unwrap();
        let events = vec![
            SessionEvent::RawFinal {
                id,
                text: "first".to_string(),
            },
            SessionEvent::CleanFinal {
                id,
                text: "First.".to_string(),
                dial: CleanupDial::Light,
            },
            SessionEvent::RawFinal {
                id,
                text: "second".to_string(),
            },
            SessionEvent::CleanFinal {
                id,
                text: "Second.".to_string(),
                dial: CleanupDial::Light,
            },
        ];

        store.record_events(&events).unwrap();

        let session = store.get_session(id).unwrap().unwrap();
        assert_eq!(session.raw_text.as_deref(), Some("first second"));
        assert_eq!(session.clean_text.as_deref(), Some("First. Second."));
        assert_eq!(store.events_for_session(id).unwrap(), events);
    }

    #[test]
    fn list_recent_returns_updated_sessions_first() {
        let mut store = HistoryStore::open_in_memory().unwrap();
        let first = sid(45);
        let second = sid(46);

        store
            .record_event(&SessionEvent::RawFinal {
                id: first,
                text: "older".to_string(),
            })
            .unwrap();
        store
            .record_event(&SessionEvent::RawFinal {
                id: second,
                text: "newer".to_string(),
            })
            .unwrap();

        let recent = store.list_recent(1).unwrap();

        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, session_id_string(second));
    }

    #[test]
    fn delete_session_removes_summary_and_event_stream() {
        let id = sid(47);
        let mut store = HistoryStore::open_in_memory().unwrap();
        store
            .record_events(&[
                SessionEvent::RawFinal {
                    id,
                    text: "remove me".to_string(),
                },
                SessionEvent::CleanFinal {
                    id,
                    text: "Remove me.".to_string(),
                    dial: CleanupDial::Light,
                },
            ])
            .unwrap();

        assert!(store.delete_session(id).unwrap());

        assert!(store.get_session(id).unwrap().is_none());
        assert!(store.events_for_session(id).unwrap().is_empty());
        assert!(!store.delete_session(id).unwrap());
    }

    #[test]
    fn delete_session_and_audio_removes_db_row_and_wal_file() {
        let id = sid(48);
        let app_data = temp_app_data("delete-audio");
        let sessions_dir = app_data.join("sessions");
        fs::create_dir_all(&sessions_dir).unwrap();
        let audio_path = sessions_dir.join(format!("{}.wav", id.0));
        fs::write(&audio_path, b"fixture audio").unwrap();
        let mut store = HistoryStore::open(&app_data).unwrap();
        store
            .record_events(&[
                SessionEvent::AudioPersisted {
                    id,
                    wal_path: audio_path.display().to_string(),
                },
                SessionEvent::RawFinal {
                    id,
                    text: "delete this".to_string(),
                },
            ])
            .unwrap();

        let outcome = store
            .delete_session_and_audio(&session_id_string(id), &app_data)
            .unwrap();

        assert_eq!(
            outcome,
            DeleteSessionOutcome {
                deleted: true,
                audio_removed: true,
                audio_path: Some(audio_path.display().to_string()),
                exports_removed: 0,
            }
        );
        assert!(!audio_path.exists());
        assert!(store.get_session(id).unwrap().is_none());

        fs::remove_dir_all(app_data).unwrap();
    }

    #[test]
    fn delete_session_and_audio_refuses_external_audio_path() {
        let id = sid(49);
        let app_data = temp_app_data("delete-external");
        let external_dir = temp_app_data("external-audio");
        let audio_path = external_dir.join("outside.wav");
        fs::write(&audio_path, b"outside audio").unwrap();
        let mut store = HistoryStore::open(&app_data).unwrap();
        store
            .record_events(&[
                SessionEvent::AudioPersisted {
                    id,
                    wal_path: audio_path.display().to_string(),
                },
                SessionEvent::RawFinal {
                    id,
                    text: "delete row only".to_string(),
                },
            ])
            .unwrap();

        let outcome = store
            .delete_session_and_audio(&session_id_string(id), &app_data)
            .unwrap();

        assert_eq!(
            outcome,
            DeleteSessionOutcome {
                deleted: true,
                audio_removed: false,
                audio_path: Some(audio_path.display().to_string()),
                exports_removed: 0,
            }
        );
        assert!(audio_path.exists());
        assert!(store.get_session(id).unwrap().is_none());

        fs::remove_dir_all(app_data).unwrap();
        fs::remove_dir_all(external_dir).unwrap();
    }

    #[test]
    fn delete_session_and_audio_removes_session_exports() {
        let id = sid(54);
        let app_data = temp_app_data("delete-exports");
        let mut store = HistoryStore::open(&app_data).unwrap();
        store
            .record_event(&SessionEvent::RawFinal {
                id,
                text: "export then delete".to_string(),
            })
            .unwrap();
        let export = store.export_session(id, &app_data).unwrap();
        let json_path = PathBuf::from(export.json_path.unwrap());
        let text_path = PathBuf::from(export.text_path.unwrap());

        let outcome = store
            .delete_session_and_audio(&session_id_string(id), &app_data)
            .unwrap();

        assert_eq!(
            outcome,
            DeleteSessionOutcome {
                deleted: true,
                audio_removed: false,
                audio_path: None,
                exports_removed: 2,
            }
        );
        assert!(!json_path.exists());
        assert!(!text_path.exists());
        assert!(store.get_session(id).unwrap().is_none());

        fs::remove_dir_all(app_data).unwrap();
    }

    #[test]
    fn export_session_writes_json_and_text_under_app_data() {
        let id = sid(52);
        let app_data = temp_app_data("export-session");
        let sessions_dir = app_data.join("sessions");
        fs::create_dir_all(&sessions_dir).unwrap();
        let audio_path = sessions_dir.join(format!("{}.wav", id.0));
        fs::write(&audio_path, b"fixture audio").unwrap();
        let mut store = HistoryStore::open(&app_data).unwrap();
        store
            .record_events(&[
                SessionEvent::Started {
                    id,
                    target_app: app(),
                    at_ms: 99,
                },
                SessionEvent::AudioPersisted {
                    id,
                    wal_path: audio_path.display().to_string(),
                },
                SessionEvent::RawFinal {
                    id,
                    text: "um export my history".to_string(),
                },
                SessionEvent::CleanFinal {
                    id,
                    text: "Export my history.".to_string(),
                    dial: CleanupDial::Light,
                },
            ])
            .unwrap();

        let outcome = store.export_session(id, &app_data).unwrap();

        let json_path = app_data.join(EXPORTS_DIR).join(format!("{}.json", id.0));
        let text_path = app_data.join(EXPORTS_DIR).join(format!("{}.txt", id.0));
        assert_eq!(
            outcome,
            ExportSessionOutcome {
                exported: true,
                json_path: Some(json_path.display().to_string()),
                text_path: Some(text_path.display().to_string()),
            }
        );

        let exported_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(json_path).unwrap()).unwrap();
        assert_eq!(exported_json["schema_version"], SCHEMA_VERSION);
        assert_eq!(exported_json["session"]["id"], session_id_string(id));
        assert_eq!(exported_json["session"]["raw_text"], "um export my history");
        assert_eq!(exported_json["session"]["clean_text"], "Export my history.");
        assert_eq!(exported_json["events"].as_array().unwrap().len(), 4);

        let exported_text = fs::read_to_string(text_path).unwrap();
        assert!(exported_text.contains(APP_NAME));
        assert!(exported_text.contains("Example Editor"));
        assert!(exported_text.contains(audio_path.to_str().unwrap()));
        assert!(exported_text.contains("um export my history"));
        assert!(exported_text.contains("Export my history."));

        fs::remove_dir_all(app_data).unwrap();
    }

    #[test]
    fn export_session_returns_false_for_missing_session() {
        let app_data = temp_app_data("export-missing");
        let store = HistoryStore::open(&app_data).unwrap();

        let outcome = store.export_session(sid(53), &app_data).unwrap();

        assert_eq!(
            outcome,
            ExportSessionOutcome {
                exported: false,
                json_path: None,
                text_path: None,
            }
        );
        assert!(!app_data.join(EXPORTS_DIR).exists());

        fs::remove_dir_all(app_data).unwrap();
    }

    #[test]
    fn audio_playback_returns_safe_app_data_wav_asset() {
        let id = sid(58);
        let app_data = temp_app_data("playback-safe");
        let sessions_dir = app_data.join("sessions");
        fs::create_dir_all(&sessions_dir).unwrap();
        let audio_path = sessions_dir.join(format!("{}.wav", id.0));
        fs::write(&audio_path, b"fixture audio").unwrap();
        let mut store = HistoryStore::open(&app_data).unwrap();
        store
            .record_event(&SessionEvent::AudioPersisted {
                id,
                wal_path: audio_path.display().to_string(),
            })
            .unwrap();

        let playback = store.audio_playback(id, &app_data).unwrap().unwrap();

        assert_eq!(
            playback,
            HistoryAudioPlayback {
                asset_path: fs::canonicalize(&audio_path).unwrap().display().to_string(),
                mime_type: "audio/wav".to_string(),
                byte_length: b"fixture audio".len() as u64,
            }
        );

        fs::remove_dir_all(app_data).unwrap();
    }

    #[test]
    fn audio_playback_refuses_external_audio_path() {
        let id = sid(59);
        let app_data = temp_app_data("playback-external");
        let external_dir = temp_app_data("playback-external-file");
        let audio_path = external_dir.join("outside.wav");
        fs::write(&audio_path, b"outside audio").unwrap();
        let mut store = HistoryStore::open(&app_data).unwrap();
        store
            .record_event(&SessionEvent::AudioPersisted {
                id,
                wal_path: audio_path.display().to_string(),
            })
            .unwrap();

        assert_eq!(store.audio_playback(id, &app_data).unwrap(), None);

        fs::remove_dir_all(app_data).unwrap();
        fs::remove_dir_all(external_dir).unwrap();
    }

    #[test]
    fn purge_all_removes_rows_audio_exports_and_untracked_session_files() {
        let id = sid(55);
        let app_data = temp_app_data("purge-all");
        let sessions_dir = app_data.join("sessions");
        fs::create_dir_all(&sessions_dir).unwrap();
        let audio_path = sessions_dir.join(format!("{}.wav", id.0));
        let untracked_audio = sessions_dir.join("untracked.wav");
        fs::write(&audio_path, b"fixture audio").unwrap();
        fs::write(&untracked_audio, b"orphan audio").unwrap();
        let mut store = HistoryStore::open(&app_data).unwrap();
        store
            .record_events(&[
                SessionEvent::AudioPersisted {
                    id,
                    wal_path: audio_path.display().to_string(),
                },
                SessionEvent::RawFinal {
                    id,
                    text: "purge this".to_string(),
                },
            ])
            .unwrap();
        let export = store.export_session(id, &app_data).unwrap();
        let json_path = PathBuf::from(export.json_path.unwrap());
        let text_path = PathBuf::from(export.text_path.unwrap());

        let outcome = store.purge_all(&app_data).unwrap();

        assert_eq!(
            outcome,
            PurgeHistoryOutcome {
                sessions_deleted: 1,
                audio_files_removed: 2,
                export_files_removed: 2,
            }
        );
        assert!(!audio_path.exists());
        assert!(!untracked_audio.exists());
        assert!(!json_path.exists());
        assert!(!text_path.exists());
        assert!(store.list_recent(10).unwrap().is_empty());
        assert!(store.events_for_session(id).unwrap().is_empty());

        fs::remove_dir_all(app_data).unwrap();
    }

    #[test]
    fn retention_sweep_removes_only_expired_sessions_and_exports() {
        let old_id = sid(56);
        let fresh_id = sid(57);
        let app_data = temp_app_data("retention-sweep");
        let sessions_dir = app_data.join("sessions");
        fs::create_dir_all(&sessions_dir).unwrap();
        let old_audio = sessions_dir.join(format!("{}.wav", old_id.0));
        let fresh_audio = sessions_dir.join(format!("{}.wav", fresh_id.0));
        fs::write(&old_audio, b"old audio").unwrap();
        fs::write(&fresh_audio, b"fresh audio").unwrap();
        let mut store = HistoryStore::open(&app_data).unwrap();
        for (id, audio_path, text) in [
            (old_id, old_audio.as_path(), "old"),
            (fresh_id, fresh_audio.as_path(), "fresh"),
        ] {
            store
                .record_events(&[
                    SessionEvent::AudioPersisted {
                        id,
                        wal_path: audio_path.display().to_string(),
                    },
                    SessionEvent::RawFinal {
                        id,
                        text: text.to_string(),
                    },
                ])
                .unwrap();
            store.export_session(id, &app_data).unwrap();
        }
        store
            .conn
            .execute(
                "UPDATE sessions
                    SET started_ms = ?2, created_ms = ?2, updated_ms = ?2
                  WHERE session_id = ?1",
                params![session_id_string(old_id), 1_000i64],
            )
            .unwrap();
        store
            .conn
            .execute(
                "UPDATE sessions
                    SET started_ms = ?2, created_ms = ?2, updated_ms = ?2
                  WHERE session_id = ?1",
                params![session_id_string(fresh_id), 3_000i64],
            )
            .unwrap();

        let outcome = store.sweep_retention_before(2_000, &app_data).unwrap();

        assert_eq!(
            outcome,
            RetentionSweepOutcome {
                sessions_deleted: 1,
                audio_files_removed: 1,
                export_files_removed: 2,
            }
        );
        assert!(store.get_session(old_id).unwrap().is_none());
        assert!(store.get_session(fresh_id).unwrap().is_some());
        assert!(!old_audio.exists());
        assert!(fresh_audio.exists());
        assert!(!app_data
            .join(EXPORTS_DIR)
            .join(format!("{}.json", old_id.0))
            .exists());
        assert!(app_data
            .join(EXPORTS_DIR)
            .join(format!("{}.json", fresh_id.0))
            .exists());

        fs::remove_dir_all(app_data).unwrap();
    }

    #[test]
    fn record_recovered_audio_surfaces_orphan_wal_as_capture_failure() {
        let id = sid(50);
        let app_data = temp_app_data("recover-orphan");
        let audio_path = app_data.join("sessions").join(format!("{}.wav", id.0));
        fs::create_dir_all(audio_path.parent().unwrap()).unwrap();
        fs::write(&audio_path, b"fixture audio").unwrap();
        let mut store = HistoryStore::open(&app_data).unwrap();

        assert!(store.record_recovered_audio(id, &audio_path, 320).unwrap());

        let session = store.get_session(id).unwrap().unwrap();
        assert_eq!(
            session.audio_path.as_deref(),
            Some(audio_path.to_str().unwrap())
        );
        assert_eq!(
            session.failure,
            Some(HistoryFailure {
                stage: Stage::Capture,
                error: "Recovered audio after app exit before transcription; 320 samples preserved on disk."
                    .to_string(),
            })
        );
        assert_eq!(session.event_count, 2);
        assert_eq!(
            store.events_for_session(id).unwrap(),
            vec![
                SessionEvent::AudioPersisted {
                    id,
                    wal_path: audio_path.display().to_string(),
                },
                SessionEvent::Failed {
                    id,
                    stage: Stage::Capture,
                    error:
                        "Recovered audio after app exit before transcription; 320 samples preserved on disk."
                            .to_string(),
                },
            ]
        );

        fs::remove_dir_all(app_data).unwrap();
    }

    #[test]
    fn record_recovered_audio_is_idempotent_for_existing_audio_path() {
        let id = sid(51);
        let app_data = temp_app_data("recover-existing");
        let audio_path = app_data.join("sessions").join(format!("{}.wav", id.0));
        fs::create_dir_all(audio_path.parent().unwrap()).unwrap();
        fs::write(&audio_path, b"fixture audio").unwrap();
        let mut store = HistoryStore::open(&app_data).unwrap();
        store
            .record_event(&SessionEvent::Started {
                id,
                target_app: app(),
                at_ms: 12,
            })
            .unwrap();

        assert!(store.record_recovered_audio(id, &audio_path, 640).unwrap());
        assert!(!store.record_recovered_audio(id, &audio_path, 640).unwrap());

        let session = store.get_session(id).unwrap().unwrap();
        assert_eq!(session.target_app, Some(app()));
        assert_eq!(session.event_count, 3);

        fs::remove_dir_all(app_data).unwrap();
    }
}
