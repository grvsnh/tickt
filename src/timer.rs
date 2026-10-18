use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::config::TimerConfig;
use crate::error::TicktError;
use crate::model::{
    ActiveSessionSnapshot, ActiveStatus, SessionRecord, SessionStatus, SessionType,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimerStatus {
    Idle,
    Running,
    Paused,
    Completed,
}

impl TimerStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Idle => "READY",
            Self::Running => "RUNNING",
            Self::Paused => "PAUSED",
            Self::Completed => "COMPLETED",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimerDisplay {
    pub session_type: SessionType,
    pub status: TimerStatus,
    pub elapsed: Duration,
    pub remaining: Duration,
    pub total: Duration,
    pub progress_percent: u16,
}

#[derive(Clone, Debug)]
struct ActiveTimer {
    id: String,
    session_type: SessionType,
    started_at: DateTime<Utc>,
    total: Duration,
    elapsed_before: Duration,
    running_since: Option<Instant>,
    running_since_wall: Option<DateTime<Utc>>,
    status: TimerStatus,
    completion_taken: bool,
}

#[derive(Clone, Debug)]
pub struct TimerEngine {
    config: TimerConfig,
    active: Option<ActiveTimer>,
    completed_focus_sessions: u32,
}

impl TimerEngine {
    pub fn new(config: TimerConfig, completed_focus_sessions: u32) -> Result<Self, TicktError> {
        if config.focus_minutes == 0
            || config.short_break_minutes == 0
            || config.long_break_minutes == 0
            || config.long_break_interval == 0
        {
            return Err(TicktError::InvalidTransition(
                "timer durations and interval must be positive".to_owned(),
            ));
        }
        Ok(Self { config, active: None, completed_focus_sessions })
    }

    pub fn status(&self) -> TimerStatus {
        self.active.as_ref().map_or(TimerStatus::Idle, |active| active.status)
    }

    pub fn session_type(&self) -> SessionType {
        self.active.as_ref().map_or(SessionType::Focus, |active| active.session_type)
    }

    pub fn display(&self, now: Instant) -> TimerDisplay {
        match &self.active {
            Some(active) => {
                let elapsed = self.elapsed_at(active, now);
                let remaining = active.total.saturating_sub(elapsed);
                let progress_percent = if active.total.is_zero() {
                    100
                } else {
                    ((elapsed.as_secs_f64() / active.total.as_secs_f64()) * 100.0)
                        .round()
                        .clamp(0.0, 100.0) as u16
                };
                TimerDisplay {
                    session_type: active.session_type,
                    status: active.status,
                    elapsed,
                    remaining,
                    total: active.total,
                    progress_percent,
                }
            }
            None => TimerDisplay {
                session_type: SessionType::Focus,
                status: TimerStatus::Idle,
                elapsed: Duration::ZERO,
                remaining: Duration::from_secs(u64::from(self.config.focus_minutes) * 60),
                total: Duration::from_secs(u64::from(self.config.focus_minutes) * 60),
                progress_percent: 0,
            },
        }
    }

    pub fn start_focus(&mut self, now: Instant, wall_now: DateTime<Utc>) -> Result<(), TicktError> {
        self.start(SessionType::Focus, now, wall_now)
    }

    pub fn start_break(
        &mut self,
        session_type: SessionType,
        now: Instant,
        wall_now: DateTime<Utc>,
    ) -> Result<(), TicktError> {
        if !matches!(session_type, SessionType::ShortBreak | SessionType::LongBreak) {
            return Err(TicktError::InvalidTransition(
                "a break must be short_break or long_break".to_owned(),
            ));
        }
        self.start(session_type, now, wall_now)
    }

    fn start(
        &mut self,
        session_type: SessionType,
        now: Instant,
        wall_now: DateTime<Utc>,
    ) -> Result<(), TicktError> {
        if self.active.is_some() {
            return Err(TicktError::InvalidTransition(
                "finish or reset the current session before starting another".to_owned(),
            ));
        }
        let minutes = match session_type {
            SessionType::Focus => self.config.focus_minutes,
            SessionType::ShortBreak => self.config.short_break_minutes,
            SessionType::LongBreak => self.config.long_break_minutes,
        };
        self.active = Some(ActiveTimer {
            id: Uuid::new_v4().to_string(),
            session_type,
            started_at: wall_now,
            total: Duration::from_secs(u64::from(minutes) * 60),
            elapsed_before: Duration::ZERO,
            running_since: Some(now),
            running_since_wall: Some(wall_now),
            status: TimerStatus::Running,
            completion_taken: false,
        });
        Ok(())
    }

    pub fn pause(&mut self, now: Instant) -> Result<(), TicktError> {
        let active = self.active.as_mut().ok_or_else(|| {
            TicktError::InvalidTransition("there is no running session to pause".to_owned())
        })?;
        if active.status != TimerStatus::Running {
            return Err(TicktError::InvalidTransition(
                "only a running session can be paused".to_owned(),
            ));
        }
        active.elapsed_before = active
            .elapsed_before
            .saturating_add(now.saturating_duration_since(active.running_since.unwrap_or(now)));
        active.elapsed_before = active.elapsed_before.min(active.total);
        active.running_since = None;
        active.running_since_wall = None;
        active.status = TimerStatus::Paused;
        Ok(())
    }

    pub fn resume(&mut self, now: Instant, wall_now: DateTime<Utc>) -> Result<(), TicktError> {
        let active = self.active.as_mut().ok_or_else(|| {
            TicktError::InvalidTransition("there is no paused session to resume".to_owned())
        })?;
        if active.status != TimerStatus::Paused {
            return Err(TicktError::InvalidTransition(
                "only a paused session can be resumed".to_owned(),
            ));
        }
        active.running_since = Some(now);
        active.running_since_wall = Some(wall_now);
        active.status = TimerStatus::Running;
        Ok(())
    }

    pub fn reset(&mut self, now: Instant) -> Result<Option<SessionRecord>, TicktError> {
        let active = self.active.take().ok_or_else(|| {
            TicktError::InvalidTransition("there is no session to reset".to_owned())
        })?;
        if active.status == TimerStatus::Completed {
            return Ok(None);
        }
        Ok(Some(self.finish_record(active, now, SessionStatus::Reset)))
    }

    pub fn interrupt(&mut self, now: Instant) -> Result<Option<SessionRecord>, TicktError> {
        let Some(active) = self.active.take() else {
            return Ok(None);
        };
        if active.status == TimerStatus::Completed {
            return Ok(None);
        }
        Ok(Some(self.finish_record(active, now, SessionStatus::Interrupted)))
    }

    pub fn skip(&mut self, now: Instant) -> Result<SessionRecord, TicktError> {
        if self.status() == TimerStatus::Completed {
            return Err(TicktError::InvalidTransition(
                "a completed session cannot be skipped".to_owned(),
            ));
        }
        let active = self.active.take().ok_or_else(|| {
            TicktError::InvalidTransition("there is no session to skip".to_owned())
        })?;
        Ok(self.finish_record(active, now, SessionStatus::Skipped))
    }

    pub fn tick(&mut self, now: Instant) -> Option<SessionRecord> {
        let should_complete = self.active.as_ref().is_some_and(|active| {
            active.status == TimerStatus::Running && self.elapsed_at(active, now) >= active.total
        });
        if !should_complete {
            return None;
        }
        let mut active = self.active.take()?;
        active.elapsed_before = active.total;
        active.running_since = None;
        active.status = TimerStatus::Completed;
        let record = self.finish_record(active.clone(), now, SessionStatus::Completed);
        active.completion_taken = true;
        self.active = Some(active);
        if record.session_type.is_focus() {
            self.completed_focus_sessions = self.completed_focus_sessions.saturating_add(1);
        }
        Some(record)
    }

    pub fn take_completed(&mut self) -> Option<SessionRecord> {
        let active = self.active.as_mut()?;
        if active.status == TimerStatus::Completed && active.completion_taken {
            active.completion_taken = false;
            return None;
        }
        None
    }

    pub fn clear_completed(&mut self) {
        if self.status() == TimerStatus::Completed {
            self.active = None;
        }
    }

    pub fn completed_focus_sessions(&self) -> u32 {
        self.completed_focus_sessions
    }

    pub fn set_completed_focus_sessions(&mut self, count: u32) {
        self.completed_focus_sessions = count;
    }

    pub fn suggested_break(&self) -> SessionType {
        if self.completed_focus_sessions > 0
            && self.completed_focus_sessions % u32::from(self.config.long_break_interval) == 0
        {
            SessionType::LongBreak
        } else {
            SessionType::ShortBreak
        }
    }

    pub fn snapshot(&self, now: Instant) -> Option<ActiveSessionSnapshot> {
        let active = self.active.as_ref()?;
        if !matches!(active.status, TimerStatus::Running | TimerStatus::Paused) {
            return None;
        }
        Some(ActiveSessionSnapshot {
            id: active.id.clone(),
            session_type: active.session_type,
            status: if active.status == TimerStatus::Running {
                ActiveStatus::Running
            } else {
                ActiveStatus::Paused
            },
            started_at: active.started_at,
            running_since: active.running_since_wall,
            planned_seconds: active.total.as_secs() as i64,
            elapsed_seconds: self.elapsed_at(active, now).as_secs() as i64,
        })
    }

    pub fn restore(
        &mut self,
        snapshot: ActiveSessionSnapshot,
        now: Instant,
        wall_now: DateTime<Utc>,
    ) -> Result<bool, TicktError> {
        if self.active.is_some() {
            return Err(TicktError::InvalidTransition(
                "cannot restore over an active timer".to_owned(),
            ));
        }
        if snapshot.planned_seconds <= 0 || snapshot.elapsed_seconds < 0 {
            return Err(TicktError::InvalidTransition(
                "recovery snapshot contains invalid durations".to_owned(),
            ));
        }
        let total = Duration::from_secs(snapshot.planned_seconds as u64);
        let saved_elapsed = Duration::from_secs(snapshot.elapsed_seconds as u64).min(total);
        let mut elapsed = saved_elapsed;
        let expired = if snapshot.status == ActiveStatus::Running {
            let running_since = snapshot.running_since.unwrap_or(snapshot.started_at);
            let wall_elapsed = (wall_now - running_since).num_seconds().max(0) as u64;
            elapsed = saved_elapsed.saturating_add(Duration::from_secs(wall_elapsed)).min(total);
            elapsed >= total
        } else {
            false
        };
        self.active = Some(ActiveTimer {
            id: snapshot.id,
            session_type: snapshot.session_type,
            started_at: snapshot.started_at,
            total,
            elapsed_before: elapsed,
            running_since: (snapshot.status == ActiveStatus::Running && !expired).then_some(now),
            running_since_wall: (snapshot.status == ActiveStatus::Running && !expired)
                .then_some(wall_now),
            status: if expired {
                TimerStatus::Paused
            } else if snapshot.status == ActiveStatus::Running {
                TimerStatus::Running
            } else {
                TimerStatus::Paused
            },
            completion_taken: false,
        });
        Ok(expired)
    }

    fn elapsed_at(&self, active: &ActiveTimer, now: Instant) -> Duration {
        let running_elapsed = active
            .running_since
            .map_or(Duration::ZERO, |started| now.saturating_duration_since(started));
        active.elapsed_before.saturating_add(running_elapsed).min(active.total)
    }

    fn finish_record(
        &self,
        active: ActiveTimer,
        now: Instant,
        status: SessionStatus,
    ) -> SessionRecord {
        let elapsed = self.elapsed_at(&active, now);
        SessionRecord {
            id: active.id,
            session_type: active.session_type,
            started_at: active.started_at,
            ended_at: Some(Utc::now()),
            planned_seconds: active.total.as_secs() as i64,
            actual_seconds: elapsed.as_secs() as i64,
            status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TimerConfig;

    fn engine() -> TimerEngine {
        TimerEngine::new(
            TimerConfig {
                focus_minutes: 1,
                short_break_minutes: 1,
                long_break_minutes: 2,
                long_break_interval: 2,
            },
            0,
        )
        .expect("valid timer")
    }

    #[test]
    fn timer_uses_elapsed_time_without_tick_drift() {
        let base = Instant::now();
        let mut timer = engine();
        timer.start_focus(base, Utc::now()).expect("start focus");
        let display = timer.display(base + Duration::from_secs(37));
        assert_eq!(display.remaining, Duration::from_secs(23));
        assert_eq!(display.progress_percent, 62);
    }

    #[test]
    fn pause_resume_and_reset_are_explicit_transitions() {
        let base = Instant::now();
        let mut timer = engine();
        timer.start_focus(base, Utc::now()).expect("start");
        timer.pause(base + Duration::from_secs(10)).expect("pause");
        assert_eq!(timer.status(), TimerStatus::Paused);
        timer.resume(base + Duration::from_secs(20), Utc::now()).expect("resume");
        let record = timer.reset(base + Duration::from_secs(25)).expect("reset");
        assert_eq!(record.expect("record").status, SessionStatus::Reset);
        assert_eq!(timer.status(), TimerStatus::Idle);
    }

    #[test]
    fn completion_is_emitted_once_and_break_schedule_is_deterministic() {
        let base = Instant::now();
        let mut timer = engine();
        timer.start_focus(base, Utc::now()).expect("start");
        let completed = timer.tick(base + Duration::from_secs(60));
        assert_eq!(completed.as_ref().map(|record| record.status), Some(SessionStatus::Completed));
        assert!(timer.tick(base + Duration::from_secs(90)).is_none());
        assert_eq!(timer.suggested_break(), SessionType::ShortBreak);
        timer.clear_completed();
        timer.start_break(SessionType::ShortBreak, base, Utc::now()).expect("break");
        assert!(timer.start_focus(base, Utc::now()).is_err());
    }

    #[test]
    fn restart_recovery_marks_elapsed_running_sessions_as_expired() {
        let mut timer = engine();
        let snapshot = ActiveSessionSnapshot {
            id: "session".to_owned(),
            session_type: SessionType::Focus,
            status: ActiveStatus::Running,
            started_at: Utc::now() - chrono::Duration::seconds(90),
            running_since: None,
            planned_seconds: 60,
            elapsed_seconds: 10,
        };
        let expired = timer.restore(snapshot, Instant::now(), Utc::now()).expect("restore");
        assert!(expired);
        assert_eq!(timer.status(), TimerStatus::Paused);
        assert_eq!(timer.display(Instant::now()).remaining, Duration::ZERO);
    }

    #[test]
    fn restart_recovery_excludes_time_spent_paused_before_resume() {
        let base_wall = Utc::now();
        let mut timer = engine();
        let snapshot = ActiveSessionSnapshot {
            id: "session".to_owned(),
            session_type: SessionType::Focus,
            status: ActiveStatus::Running,
            started_at: base_wall - chrono::Duration::seconds(900),
            running_since: Some(base_wall),
            planned_seconds: 300,
            elapsed_seconds: 60,
        };
        let now = Instant::now();
        let expired = timer.restore(snapshot, now, base_wall).expect("restore");
        assert!(!expired);
        assert_eq!(timer.display(now).elapsed, Duration::from_secs(60));
        assert_eq!(timer.display(now + Duration::from_secs(20)).elapsed, Duration::from_secs(80));
    }

    #[test]
    fn long_break_is_scheduled_at_the_configured_focus_interval() {
        let base = Instant::now();
        let mut timer = engine();

        for _ in 0..2 {
            timer.start_focus(base, Utc::now()).expect("focus starts");
            assert!(timer.tick(base + Duration::from_secs(60)).is_some());
            timer.clear_completed();
            if timer.completed_focus_sessions() == 1 {
                assert_eq!(timer.suggested_break(), SessionType::ShortBreak);
                timer
                    .start_break(SessionType::ShortBreak, base, Utc::now())
                    .expect("short break starts");
                assert!(timer.tick(base + Duration::from_secs(60)).is_some());
                assert_eq!(timer.completed_focus_sessions(), 1);
                timer.clear_completed();
            }
        }

        assert_eq!(timer.completed_focus_sessions(), 2);
        assert_eq!(timer.suggested_break(), SessionType::LongBreak);
    }

    #[test]
    fn invalid_transitions_are_rejected_without_mutating_idle_state() {
        let base = Instant::now();
        let mut timer = engine();
        assert!(timer.pause(base).is_err());
        assert!(timer.resume(base, Utc::now()).is_err());
        assert!(timer.reset(base).is_err());
        assert!(timer.skip(base).is_err());
        assert_eq!(timer.status(), TimerStatus::Idle);
    }
}
