use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::model::{
    ActiveSessionSnapshot, ActiveStatus, SessionRecord, SessionStatus, SessionType,
};
use crate::statistics::FocusCompletion;

pub struct Storage {
    connection: Connection,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("could not create {}", parent.display()))?;
        }
        let connection = Connection::open(path)
            .with_context(|| format!("could not open database {}", path.display()))?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .context("could not enable SQLite foreign keys")?;
        migrate(&connection).context("could not migrate database")?;
        Ok(Self { connection })
    }

    pub fn insert_session(&self, record: &SessionRecord) -> Result<bool> {
        let changed = self.connection.execute(
            "INSERT INTO sessions
             (id, session_type, started_at, ended_at, planned_seconds, actual_seconds, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO NOTHING",
            params![
                record.id,
                encode_session_type(record.session_type),
                record.started_at.timestamp(),
                record.ended_at.map(|value| value.timestamp()),
                record.planned_seconds,
                record.actual_seconds,
                encode_status(record.status),
            ],
        )?;
        Ok(changed == 1)
    }

    pub fn recent_sessions(&self, limit: u32) -> Result<Vec<SessionRecord>> {
        let mut statement = self.connection.prepare(
            "SELECT id, session_type, started_at, ended_at, planned_seconds, actual_seconds, status
             FROM sessions ORDER BY started_at DESC LIMIT ?1",
        )?;
        let rows = statement.query_map([i64::from(limit.min(200))], decode_record)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().context("could not read recent sessions")
    }

    pub fn completed_focus_since(&self, timestamp: i64) -> Result<Vec<FocusCompletion>> {
        let mut statement = self.connection.prepare(
            "SELECT started_at, planned_seconds, actual_seconds
             FROM sessions
             WHERE started_at >= ?1
               AND session_type = 'focus'
               AND status = 'completed'
               AND actual_seconds >= planned_seconds
             ORDER BY started_at ASC",
        )?;
        let rows = statement.query_map([timestamp], decode_focus_completion)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("could not read completed focus sessions")
    }

    pub fn records_since(&self, timestamp: i64) -> Result<Vec<SessionRecord>> {
        let mut statement = self.connection.prepare(
            "SELECT id, session_type, started_at, ended_at, planned_seconds, actual_seconds, status
             FROM sessions WHERE started_at >= ?1 ORDER BY started_at ASC",
        )?;
        let rows = statement.query_map([timestamp], decode_record)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().context("could not read statistics records")
    }

    pub fn save_active(&self, snapshot: &ActiveSessionSnapshot) -> Result<()> {
        let transaction = self.connection.unchecked_transaction()?;
        save_active_in_transaction(&transaction, snapshot)?;
        transaction.commit().context("could not commit active timer")
    }

    pub fn load_active(&self) -> Result<Option<ActiveSessionSnapshot>> {
        self.connection
            .query_row(
                "SELECT session_id, session_type, state, started_at, planned_seconds, elapsed_seconds, running_since
                 FROM active_timer WHERE slot = 1",
                [],
                decode_active,
            )
            .optional()
            .context("could not read active timer")
    }

    pub fn clear_active(&self) -> Result<()> {
        self.connection.execute("DELETE FROM active_timer WHERE slot = 1", [])?;
        Ok(())
    }

    pub fn finish_session(&self, record: &SessionRecord) -> Result<bool> {
        let transaction = self.connection.unchecked_transaction()?;
        let changed = insert_session_in_transaction(&transaction, record)?;
        transaction.execute("DELETE FROM active_timer WHERE slot = 1", [])?;
        transaction.commit().context("could not commit completed session")?;
        Ok(changed)
    }

    pub fn connection_for_test(&self) -> &Connection {
        &self.connection
    }
}

fn migrate(connection: &Connection) -> Result<()> {
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version == 0 {
        let transaction = connection.unchecked_transaction()?;
        transaction.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY NOT NULL,
                session_type TEXT NOT NULL CHECK (session_type IN ('focus', 'short_break', 'long_break')),
                started_at INTEGER NOT NULL,
                ended_at INTEGER,
                planned_seconds INTEGER NOT NULL CHECK (planned_seconds > 0),
                actual_seconds INTEGER NOT NULL CHECK (actual_seconds >= 0),
                status TEXT NOT NULL CHECK (status IN ('completed', 'skipped', 'reset', 'interrupted')),
                created_at INTEGER NOT NULL DEFAULT (unixepoch())
            );
            CREATE INDEX IF NOT EXISTS sessions_started_at_idx ON sessions (started_at DESC);
            CREATE INDEX IF NOT EXISTS sessions_completed_focus_idx
                ON sessions (started_at)
                WHERE session_type = 'focus'
                  AND status = 'completed'
                  AND actual_seconds >= planned_seconds;
            CREATE TABLE IF NOT EXISTS active_timer (
                slot INTEGER PRIMARY KEY CHECK (slot = 1),
                session_id TEXT NOT NULL UNIQUE,
                session_type TEXT NOT NULL CHECK (session_type IN ('focus', 'short_break', 'long_break')),
                state TEXT NOT NULL CHECK (state IN ('running', 'paused')),
                started_at INTEGER NOT NULL,
                planned_seconds INTEGER NOT NULL CHECK (planned_seconds > 0),
                elapsed_seconds INTEGER NOT NULL CHECK (elapsed_seconds >= 0),
                running_since INTEGER
             );
             PRAGMA user_version = 3;",
        )?;
        transaction.commit().context("could not commit initial schema")?;
    } else if version == 1 {
        let transaction = connection.unchecked_transaction()?;
        transaction.execute("ALTER TABLE active_timer ADD COLUMN running_since INTEGER", [])?;
        transaction.execute_batch(
            "CREATE INDEX IF NOT EXISTS sessions_completed_focus_idx
                ON sessions (started_at)
                WHERE session_type = 'focus'
                  AND status = 'completed'
                  AND actual_seconds >= planned_seconds;
             PRAGMA user_version = 3;",
        )?;
        transaction.commit().context("could not migrate active timer schema")?;
    } else if version == 2 {
        let transaction = connection.unchecked_transaction()?;
        transaction.execute_batch(
            "CREATE INDEX IF NOT EXISTS sessions_completed_focus_idx
                ON sessions (started_at)
                WHERE session_type = 'focus'
                  AND status = 'completed'
                  AND actual_seconds >= planned_seconds;
             PRAGMA user_version = 3;",
        )?;
        transaction.commit().context("could not migrate statistics index")?;
    } else if version != 3 {
        bail!("unsupported database schema version {version}");
    }
    Ok(())
}

fn insert_session_in_transaction(
    transaction: &Transaction<'_>,
    record: &SessionRecord,
) -> Result<bool> {
    let changed = transaction.execute(
        "INSERT INTO sessions
         (id, session_type, started_at, ended_at, planned_seconds, actual_seconds, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(id) DO NOTHING",
        params![
            record.id,
            encode_session_type(record.session_type),
            record.started_at.timestamp(),
            record.ended_at.map(|value| value.timestamp()),
            record.planned_seconds,
            record.actual_seconds,
            encode_status(record.status),
        ],
    )?;
    Ok(changed == 1)
}

fn save_active_in_transaction(
    transaction: &Transaction<'_>,
    snapshot: &ActiveSessionSnapshot,
) -> Result<()> {
    transaction.execute("DELETE FROM active_timer WHERE slot = 1", [])?;
    transaction.execute(
        "INSERT INTO active_timer
         (slot, session_id, session_type, state, started_at, planned_seconds, elapsed_seconds, running_since)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            snapshot.id,
            encode_session_type(snapshot.session_type),
            encode_active_status(snapshot.status),
            snapshot.started_at.timestamp(),
            snapshot.planned_seconds,
            snapshot.elapsed_seconds,
            snapshot.running_since.map(|value| value.timestamp()),
        ],
    )?;
    Ok(())
}

fn decode_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionRecord> {
    let started_at = timestamp(row.get(2)?)?;
    let ended_at = row.get::<_, Option<i64>>(3)?.map(timestamp).transpose()?;
    Ok(SessionRecord {
        id: row.get(0)?,
        session_type: decode_session_type(&row.get::<_, String>(1)?)?,
        started_at,
        ended_at,
        planned_seconds: row.get(4)?,
        actual_seconds: row.get(5)?,
        status: decode_status(&row.get::<_, String>(6)?)?,
    })
}

fn decode_focus_completion(row: &rusqlite::Row<'_>) -> rusqlite::Result<FocusCompletion> {
    Ok(FocusCompletion {
        started_at: timestamp(row.get(0)?)?,
        planned_seconds: row.get(1)?,
        actual_seconds: row.get(2)?,
    })
}

fn decode_active(row: &rusqlite::Row<'_>) -> rusqlite::Result<ActiveSessionSnapshot> {
    Ok(ActiveSessionSnapshot {
        id: row.get(0)?,
        session_type: decode_session_type(&row.get::<_, String>(1)?)?,
        status: decode_active_status(&row.get::<_, String>(2)?)?,
        started_at: timestamp(row.get(3)?)?,
        planned_seconds: row.get(4)?,
        elapsed_seconds: row.get(5)?,
        running_since: row.get::<_, Option<i64>>(6)?.map(timestamp).transpose()?,
    })
}

fn timestamp(value: i64) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::<Utc>::from_timestamp(value, 0).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            8,
            rusqlite::types::Type::Integer,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "timestamp is outside chrono range",
            )),
        )
    })
}

fn encode_session_type(value: SessionType) -> &'static str {
    match value {
        SessionType::Focus => "focus",
        SessionType::ShortBreak => "short_break",
        SessionType::LongBreak => "long_break",
    }
}

fn decode_session_type(value: &str) -> rusqlite::Result<SessionType> {
    match value {
        "focus" => Ok(SessionType::Focus),
        "short_break" => Ok(SessionType::ShortBreak),
        "long_break" => Ok(SessionType::LongBreak),
        _ => Err(rusqlite::Error::InvalidColumnType(
            1,
            "session_type".to_owned(),
            rusqlite::types::Type::Text,
        )),
    }
}

fn encode_status(value: SessionStatus) -> &'static str {
    value.label()
}

fn decode_status(value: &str) -> rusqlite::Result<SessionStatus> {
    match value {
        "completed" => Ok(SessionStatus::Completed),
        "skipped" => Ok(SessionStatus::Skipped),
        "reset" => Ok(SessionStatus::Reset),
        "interrupted" => Ok(SessionStatus::Interrupted),
        _ => Err(rusqlite::Error::InvalidColumnType(
            6,
            "status".to_owned(),
            rusqlite::types::Type::Text,
        )),
    }
}

fn encode_active_status(value: ActiveStatus) -> &'static str {
    match value {
        ActiveStatus::Running => "running",
        ActiveStatus::Paused => "paused",
    }
}

fn decode_active_status(value: &str) -> rusqlite::Result<ActiveStatus> {
    match value {
        "running" => Ok(ActiveStatus::Running),
        "paused" => Ok(ActiveStatus::Paused),
        _ => Err(rusqlite::Error::InvalidColumnType(
            2,
            "state".to_owned(),
            rusqlite::types::Type::Text,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn focus_record() -> SessionRecord {
        let started_at =
            DateTime::<Utc>::from_timestamp(Utc::now().timestamp(), 0).expect("current timestamp");
        record_with("one", started_at, SessionType::Focus, SessionStatus::Completed, 1_500, 1_500)
    }

    fn record_with(
        id: &str,
        started_at: DateTime<Utc>,
        session_type: SessionType,
        status: SessionStatus,
        planned_seconds: i64,
        actual_seconds: i64,
    ) -> SessionRecord {
        SessionRecord {
            id: id.to_owned(),
            session_type,
            started_at,
            ended_at: Some(started_at + chrono::Duration::seconds(actual_seconds)),
            planned_seconds,
            actual_seconds,
            status,
        }
    }

    #[test]
    fn creates_schema_and_round_trips_sessions() {
        let dir = tempdir().expect("tempdir");
        let storage = Storage::open(&dir.path().join("tickt.db")).expect("open");
        let record = focus_record();
        assert!(storage.insert_session(&record).expect("insert"));
        assert!(!storage.insert_session(&record).expect("duplicate"));
        let recent = storage.recent_sessions(10).expect("recent");
        assert_eq!(recent, vec![record]);
    }

    #[test]
    fn active_state_is_replaced_and_cleared_transactionally() {
        let dir = tempdir().expect("tempdir");
        let storage = Storage::open(&dir.path().join("tickt.db")).expect("open");
        let snapshot = ActiveSessionSnapshot {
            id: "active".to_owned(),
            session_type: SessionType::Focus,
            status: ActiveStatus::Paused,
            started_at: DateTime::<Utc>::from_timestamp(Utc::now().timestamp(), 0)
                .expect("current timestamp"),
            running_since: None,
            planned_seconds: 1_500,
            elapsed_seconds: 30,
        };
        storage.save_active(&snapshot).expect("save");
        assert_eq!(storage.load_active().expect("load"), Some(snapshot));
        storage.clear_active().expect("clear");
        assert!(storage.load_active().expect("load").is_none());
    }

    #[test]
    fn history_queries_are_ordered_bounded_and_filtered() {
        let dir = tempdir().expect("tempdir");
        let storage = Storage::open(&dir.path().join("tickt.db")).expect("open");
        let base = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).expect("timestamp");
        let records = [
            record_with("old", base, SessionType::Focus, SessionStatus::Completed, 60, 60),
            record_with(
                "middle",
                base + chrono::Duration::seconds(60),
                SessionType::ShortBreak,
                SessionStatus::Completed,
                60,
                60,
            ),
            record_with(
                "new",
                base + chrono::Duration::seconds(120),
                SessionType::Focus,
                SessionStatus::Skipped,
                60,
                20,
            ),
        ];
        for record in &records {
            assert!(storage.insert_session(record).expect("insert"));
        }
        let recent = storage.recent_sessions(2).expect("recent");
        assert_eq!(
            recent.iter().map(|record| record.id.as_str()).collect::<Vec<_>>(),
            ["new", "middle"]
        );
        let since = storage.records_since(base.timestamp() + 60).expect("since");
        assert_eq!(
            since.iter().map(|record| record.id.as_str()).collect::<Vec<_>>(),
            ["middle", "new"]
        );
        assert_eq!(storage.completed_focus_since(0).expect("count").len(), 1);
    }

    #[test]
    fn finish_session_is_idempotent_and_clears_active_state() {
        let dir = tempdir().expect("tempdir");
        let storage = Storage::open(&dir.path().join("tickt.db")).expect("open");
        let snapshot = ActiveSessionSnapshot {
            id: "active".to_owned(),
            session_type: SessionType::Focus,
            status: ActiveStatus::Running,
            started_at: Utc::now(),
            running_since: Some(Utc::now()),
            planned_seconds: 60,
            elapsed_seconds: 60,
        };
        let record = record_with(
            "finished",
            snapshot.started_at,
            SessionType::Focus,
            SessionStatus::Completed,
            60,
            60,
        );
        storage.save_active(&snapshot).expect("active");
        assert!(storage.finish_session(&record).expect("finish"));
        assert!(!storage.finish_session(&record).expect("duplicate finish"));
        assert!(storage.load_active().expect("active load").is_none());
        assert_eq!(storage.recent_sessions(10).expect("history").len(), 1);
    }

    #[test]
    fn migrates_v1_active_timer_schema_and_rejects_unknown_versions() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("v1.db");
        let connection = Connection::open(&path).expect("open v1");
        connection
            .execute_batch(
                "CREATE TABLE sessions (
                    id TEXT PRIMARY KEY NOT NULL,
                    session_type TEXT NOT NULL,
                    started_at INTEGER NOT NULL,
                    ended_at INTEGER,
                    planned_seconds INTEGER NOT NULL,
                    actual_seconds INTEGER NOT NULL,
                    status TEXT NOT NULL
                );
                CREATE TABLE active_timer (
                    slot INTEGER PRIMARY KEY,
                    session_id TEXT NOT NULL,
                    session_type TEXT NOT NULL,
                    state TEXT NOT NULL,
                    started_at INTEGER NOT NULL,
                    planned_seconds INTEGER NOT NULL,
                    elapsed_seconds INTEGER NOT NULL
                );
                PRAGMA user_version = 1;",
            )
            .expect("create v1");
        drop(connection);
        let storage = Storage::open(&path).expect("migrate v1");
        let version: i64 = storage
            .connection_for_test()
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("version");
        assert_eq!(version, 3);

        let unknown = dir.path().join("unknown.db");
        let connection = Connection::open(&unknown).expect("open unknown");
        connection.execute_batch("PRAGMA user_version = 99;").expect("set version");
        drop(connection);
        assert!(Storage::open(&unknown).is_err());
    }
}
