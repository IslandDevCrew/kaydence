//! Remember: persist session rows and the exact event stream to local SQLite.
//!
//! History is the safety net for non-negotiable #2. It consumes only
//! `SessionEvent` values, stores the append-only event log as JSON, and keeps a
//! query-friendly session summary for the HUD/history browser.

use crate::events::{
    AppRef, CleanupDial, HoldReason, InjectMethod, SessionEvent, SessionId, Stage,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const HISTORY_DB_FILE: &str = "history.sqlite3";
pub const SCHEMA_VERSION: i64 = 1;

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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
}
