use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::model::{
    ActiveSessionSnapshot, ActiveStatus, SessionRecord, SessionStatus, SessionType,
};

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

    pub fn records_since(&self, timestamp: i64) -> Result<Vec<SessionRecord>> {
        let mut statement = self.connection.prepare(
            "SELECT id, session_type, started_at, ended_at, planned_seconds, actual_seconds, status
             FROM sessions WHERE started_at >= ?1 ORDER BY started_at ASC LIMIT 20000",
        )?;
        let rows = statement.query_map([timestamp], decode_record)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().context("could not read statistics records")
    }

    pub fn count_completed_focus(&self) -> Result<u32> {
        let count: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM sessions
             WHERE session_type = 'focus' AND status = 'completed' AND actual_seconds >= planned_seconds",
            [],
            |row| row.get(0),
        )?;
        Ok(count.max(0) as u32)
    }

    pub fn save_active(&self, snapshot: &ActiveSessionSnapshot) -> Result<()> {
        let transaction = self.connection.unchecked_transaction()?;
        save_active_in_transaction(&transaction, snapshot)?;
        transaction.commit().context("could not commit active timer")
    }

    pub fn load_active(&self) -> Result<Option<ActiveSessionSnapshot>> {
        self.connection
            .query_row(
                "SELECT session_id, session_type, state, started_at, planned_seconds, elapsed_seconds
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
            CREATE TABLE IF NOT EXISTS active_timer (
                slot INTEGER PRIMARY KEY CHECK (slot = 1),
                session_id TEXT NOT NULL UNIQUE,
                session_type TEXT NOT NULL CHECK (session_type IN ('focus', 'short_break', 'long_break')),
                state TEXT NOT NULL CHECK (state IN ('running', 'paused')),
                started_at INTEGER NOT NULL,
                planned_seconds INTEGER NOT NULL CHECK (planned_seconds > 0),
                elapsed_seconds INTEGER NOT NULL CHECK (elapsed_seconds >= 0)
            );
            PRAGMA user_version = 1;",
        )?;
        transaction.commit().context("could not commit initial schema")?;
    } else if version != 1 {
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
         (slot, session_id, session_type, state, started_at, planned_seconds, elapsed_seconds)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            snapshot.id,
            encode_session_type(snapshot.session_type),
            encode_active_status(snapshot.status),
            snapshot.started_at.timestamp(),
            snapshot.planned_seconds,
            snapshot.elapsed_seconds,
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

fn decode_active(row: &rusqlite::Row<'_>) -> rusqlite::Result<ActiveSessionSnapshot> {
    Ok(ActiveSessionSnapshot {
        id: row.get(0)?,
        session_type: decode_session_type(&row.get::<_, String>(1)?)?,
        status: decode_active_status(&row.get::<_, String>(2)?)?,
        started_at: timestamp(row.get(3)?)?,
        planned_seconds: row.get(4)?,
        elapsed_seconds: row.get(5)?,
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
        SessionRecord {
            id: "one".to_owned(),
            session_type: SessionType::Focus,
            started_at,
            ended_at: Some(started_at + chrono::Duration::minutes(25)),
            planned_seconds: 1_500,
            actual_seconds: 1_500,
            status: SessionStatus::Completed,
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
            planned_seconds: 1_500,
            elapsed_seconds: 30,
        };
        storage.save_active(&snapshot).expect("save");
        assert_eq!(storage.load_active().expect("load"), Some(snapshot));
        storage.clear_active().expect("clear");
        assert!(storage.load_active().expect("load").is_none());
    }
}
