use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Days, Local, NaiveDate, Utc};

use crate::model::{SessionRecord, SessionStatus};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FocusCompletion {
    pub started_at: DateTime<Utc>,
    pub planned_seconds: i64,
    pub actual_seconds: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DayTotal {
    pub date: NaiveDate,
    pub minutes: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Statistics {
    pub completed_focus_sessions: u32,
    pub completed_focus_minutes: u32,
    pub daily_goal_minutes: u32,
    pub goal_percent: u16,
    pub remaining_goal_minutes: u32,
    pub current_streak: u32,
    pub longest_streak: u32,
    pub total_focus_sessions: u32,
    pub total_focus_minutes: u32,
    pub last_seven_days: Vec<DayTotal>,
}

impl Statistics {
    pub fn calculate(records: &[SessionRecord], today: NaiveDate, daily_goal_minutes: u32) -> Self {
        let completions = records
            .iter()
            .filter(|record| {
                record.session_type.is_focus()
                    && record.status == SessionStatus::Completed
                    && record.actual_seconds >= record.planned_seconds
            })
            .map(|record| FocusCompletion {
                started_at: record.started_at,
                planned_seconds: record.planned_seconds,
                actual_seconds: record.actual_seconds,
            })
            .collect::<Vec<_>>();
        Self::calculate_focus(&completions, today, daily_goal_minutes)
    }

    pub fn calculate_focus(
        records: &[FocusCompletion],
        today: NaiveDate,
        daily_goal_minutes: u32,
    ) -> Self {
        let mut by_day = BTreeMap::<NaiveDate, (u32, u32)>::new();
        let mut total_sessions = 0_u32;
        let mut total_minutes = 0_u32;
        for record in records {
            if record.actual_seconds < record.planned_seconds {
                continue;
            }
            let date = record.started_at.with_timezone(&Local).date_naive();
            let minutes = (record.actual_seconds / 60).max(0) as u32;
            let entry = by_day.entry(date).or_default();
            entry.0 = entry.0.saturating_add(1);
            entry.1 = entry.1.saturating_add(minutes);
            total_sessions = total_sessions.saturating_add(1);
            total_minutes = total_minutes.saturating_add(minutes);
        }
        let (today_sessions, today_minutes) = by_day.get(&today).copied().unwrap_or_default();
        let goal_percent = if daily_goal_minutes == 0 {
            0
        } else {
            ((today_minutes as f64 / daily_goal_minutes as f64) * 100.0).round().clamp(0.0, 100.0)
                as u16
        };
        let remaining_goal_minutes = daily_goal_minutes.saturating_sub(today_minutes);
        let goal_days: BTreeSet<NaiveDate> = by_day
            .iter()
            .filter_map(|(date, (_, minutes))| (*minutes >= daily_goal_minutes).then_some(*date))
            .collect();
        let current_streak = streak_ending_at(&goal_days, today);
        let longest_streak = longest_streak(&goal_days);
        let last_seven_days = (0..7)
            .rev()
            .filter_map(|offset| today.checked_sub_days(Days::new(offset)))
            .map(|date| DayTotal {
                date,
                minutes: by_day.get(&date).map_or(0, |(_, minutes)| *minutes),
            })
            .collect();

        Self {
            completed_focus_sessions: today_sessions,
            completed_focus_minutes: today_minutes,
            daily_goal_minutes,
            goal_percent,
            remaining_goal_minutes,
            current_streak,
            longest_streak,
            total_focus_sessions: total_sessions,
            total_focus_minutes: total_minutes,
            last_seven_days,
        }
    }

    pub fn empty(today: NaiveDate, daily_goal_minutes: u32) -> Self {
        Self::calculate(&[], today, daily_goal_minutes)
    }
}

fn streak_ending_at(goal_days: &BTreeSet<NaiveDate>, today: NaiveDate) -> u32 {
    let start = if goal_days.contains(&today) { today } else { today.pred_opt().unwrap_or(today) };
    let mut streak = 0;
    let mut date = start;
    while goal_days.contains(&date) {
        streak += 1;
        let Some(previous) = date.pred_opt() else { break };
        date = previous;
    }
    streak
}

fn longest_streak(goal_days: &BTreeSet<NaiveDate>) -> u32 {
    let mut longest = 0;
    let mut current = 0;
    let mut previous: Option<NaiveDate> = None;
    for date in goal_days {
        if previous.is_some_and(|previous_date| *date == previous_date.succ_opt().unwrap_or(*date))
        {
            current += 1;
        } else {
            current = 1;
        }
        longest = longest.max(current);
        previous = Some(*date);
    }
    longest
}

pub fn local_today() -> NaiveDate {
    Local::now().date_naive()
}

pub fn format_local_time(timestamp: DateTime<Utc>) -> String {
    timestamp.with_timezone(&Local).format("%H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{SessionRecord, SessionStatus, SessionType};

    fn record(
        day: NaiveDate,
        minutes: i64,
        status: SessionStatus,
        kind: SessionType,
    ) -> SessionRecord {
        let started_at = DateTime::<Utc>::from_naive_utc_and_offset(
            day.and_hms_opt(12, 0, 0).expect("time"),
            Utc,
        );
        SessionRecord {
            id: format!("{day}-{minutes}"),
            session_type: kind,
            started_at,
            ended_at: Some(started_at + chrono::Duration::minutes(minutes)),
            planned_seconds: 60 * minutes,
            actual_seconds: 60 * minutes,
            status,
        }
    }

    #[test]
    fn statistics_exclude_breaks_skips_and_partial_focus() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 18).expect("date");
        let mut complete = record(today, 25, SessionStatus::Completed, SessionType::Focus);
        complete.id = "complete".to_owned();
        let mut partial = complete.clone();
        partial.id = "partial".to_owned();
        partial.actual_seconds = 20 * 60;
        let stats = Statistics::calculate(
            &[
                complete,
                partial,
                record(today, 5, SessionStatus::Completed, SessionType::ShortBreak),
                record(today, 25, SessionStatus::Skipped, SessionType::Focus),
            ],
            today,
            50,
        );
        assert_eq!(stats.completed_focus_sessions, 1);
        assert_eq!(stats.completed_focus_minutes, 25);
        assert_eq!(stats.remaining_goal_minutes, 25);
    }

    #[test]
    fn streak_uses_goal_days_and_ignores_an_open_today() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 18).expect("date");
        let records = vec![
            record(
                today.pred_opt().expect("yesterday"),
                60,
                SessionStatus::Completed,
                SessionType::Focus,
            ),
            record(today - Days::new(2), 60, SessionStatus::Completed, SessionType::Focus),
            record(today - Days::new(3), 60, SessionStatus::Completed, SessionType::Focus),
        ];
        let stats = Statistics::calculate(&records, today, 60);
        assert_eq!(stats.current_streak, 3);
        assert_eq!(stats.longest_streak, 3);
    }

    #[test]
    fn seven_day_summary_has_stable_order() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 18).expect("date");
        let stats = Statistics::empty(today, 100);
        assert_eq!(stats.last_seven_days.len(), 7);
        assert_eq!(stats.last_seven_days.first().map(|day| day.date), Some(today - Days::new(6)));
        assert_eq!(stats.last_seven_days.last().map(|day| day.date), Some(today));
    }

    #[test]
    fn goal_progress_is_percentage_clamped_and_remaining_time_saturates() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 18).expect("date");
        let half = record(today, 30, SessionStatus::Completed, SessionType::Focus);
        let stats = Statistics::calculate(&[half], today, 60);
        assert_eq!(stats.goal_percent, 50);
        assert_eq!(stats.remaining_goal_minutes, 30);

        let over = record(today, 90, SessionStatus::Completed, SessionType::Focus);
        let stats = Statistics::calculate(&[over], today, 60);
        assert_eq!(stats.goal_percent, 100);
        assert_eq!(stats.remaining_goal_minutes, 0);

        let stats = Statistics::empty(today, 0);
        assert_eq!(stats.goal_percent, 0);
        assert_eq!(stats.remaining_goal_minutes, 0);
    }

    #[test]
    fn a_gap_breaks_current_streak_but_not_historical_longest_streak() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 18).expect("date");
        let records = vec![
            record(today, 60, SessionStatus::Completed, SessionType::Focus),
            record(today - Days::new(2), 60, SessionStatus::Completed, SessionType::Focus),
            record(today - Days::new(3), 60, SessionStatus::Completed, SessionType::Focus),
            record(today - Days::new(4), 60, SessionStatus::Completed, SessionType::Focus),
        ];
        let stats = Statistics::calculate(&records, today, 60);
        assert_eq!(stats.current_streak, 1);
        assert_eq!(stats.longest_streak, 3);
    }
}
