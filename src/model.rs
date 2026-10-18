use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionType {
    Focus,
    ShortBreak,
    LongBreak,
}

impl SessionType {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Focus => "FOCUS",
            Self::ShortBreak => "SHORT BREAK",
            Self::LongBreak => "LONG BREAK",
        }
    }

    pub const fn is_focus(self) -> bool {
        matches!(self, Self::Focus)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Completed,
    Skipped,
    Reset,
    Interrupted,
}

impl SessionStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Skipped => "skipped",
            Self::Reset => "reset",
            Self::Interrupted => "interrupted",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionRecord {
    pub id: String,
    pub session_type: SessionType,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub planned_seconds: i64,
    pub actual_seconds: i64,
    pub status: SessionStatus,
}

impl SessionRecord {
    pub fn local_start(&self) -> DateTime<Local> {
        self.started_at.with_timezone(&Local)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveSessionSnapshot {
    pub id: String,
    pub session_type: SessionType,
    pub status: ActiveStatus,
    pub started_at: DateTime<Utc>,
    pub planned_seconds: i64,
    pub elapsed_seconds: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActiveStatus {
    Running,
    Paused,
}
