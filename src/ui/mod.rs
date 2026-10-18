use std::time::Instant;

use chrono::Local;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
};

use crate::app::{App, View};
use crate::model::{SessionStatus, SessionType};
use crate::statistics::format_local_time;
use crate::theme::Theme;
use crate::timer::TimerStatus;

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let theme = app.theme();
    frame
        .render_widget(Block::default().style(Style::default().bg(theme.background)), frame.area());
    if frame.area().width < 64 || frame.area().height < 18 {
        draw_compact(frame, app, theme);
        return;
    }

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10), Constraint::Length(2)])
        .split(frame.area());
    draw_header(frame, app, root[0], theme);
    match app.view {
        View::Dashboard => draw_dashboard(frame, app, root[1], theme),
        View::History => draw_history(frame, app, root[1], theme),
        View::Statistics => draw_statistics(frame, app, root[1], theme),
        View::Settings => draw_settings(frame, app, root[1], theme),
        View::Help => draw_help(frame, app, root[1], theme),
    }
    draw_footer(frame, app, root[2], theme);

    if let Some(action) = app.confirmation {
        draw_confirmation(frame, app, action, theme);
    }
    if let Some(recovery) = &app.recovery {
        draw_recovery(frame, app, recovery.expired, theme);
    }
}

fn draw_header(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let now = Local::now();
    let mut line = Line::from(vec![
        Span::styled(" TICKT ", theme.title().bg(theme.accent).fg(theme.background)),
        Span::styled("  Make time count", Style::default().fg(theme.secondary)),
        Span::raw(" "),
        Span::styled(
            format!("{}  {}", now.format("%a %d %b"), now.format("%H:%M")),
            Style::default().fg(theme.primary),
        ),
    ]);
    let status = if app.persistence_warning {
        Span::styled("  ! storage warning", Style::default().fg(theme.warning))
    } else {
        Span::styled("  ● local", Style::default().fg(theme.success))
    };
    line.spans.push(status);
    let paragraph = Paragraph::new(line)
        .style(Style::default().bg(theme.surface))
        .block(Block::default().borders(Borders::BOTTOM).border_style(theme.border));
    frame.render_widget(paragraph, area);
}

fn draw_dashboard(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(61), Constraint::Percentage(39)])
        .split(area);
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(11), Constraint::Min(6)])
        .split(columns[0]);
    draw_timer(frame, app, left[0], theme);
    draw_recent(frame, app, left[1], theme);
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(6)])
        .split(columns[1]);
    draw_today(frame, app, right[0], theme);
    draw_next(frame, app, right[1], theme);
}

fn draw_timer(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let display = app.timer.display(Instant::now());
    let mode_color = if display.session_type.is_focus() { theme.focus } else { theme.break_color };
    let status = match display.status {
        TimerStatus::Running => Span::styled("RUNNING", Style::default().fg(theme.success)),
        TimerStatus::Paused => Span::styled("PAUSED", Style::default().fg(theme.warning)),
        TimerStatus::Completed => Span::styled("COMPLETED", Style::default().fg(theme.success)),
        TimerStatus::Idle => Span::styled("READY", Style::default().fg(theme.secondary)),
    };
    let title = Line::from(vec![
        Span::styled(
            display.session_type.label(),
            Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        status,
    ]);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border)
        .title(title)
        .style(theme.panel_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let timer_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Length(2), Constraint::Min(2)])
        .split(inner);
    let countdown = Paragraph::new(format_duration(display.remaining))
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD));
    frame.render_widget(countdown, timer_layout[0]);
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(mode_color).bg(theme.surface))
        .ratio(f64::from(display.progress_percent) / 100.0)
        .label(format!("{}%", display.progress_percent));
    frame.render_widget(gauge, timer_layout[1]);
    let hint = match display.status {
        TimerStatus::Idle => "Space start focus  ·  n next session",
        TimerStatus::Running => "Space pause  ·  s skip  ·  r reset",
        TimerStatus::Paused => "Space resume  ·  s skip  ·  r reset",
        TimerStatus::Completed => "n begin the suggested next session",
    };
    frame.render_widget(
        Paragraph::new(hint)
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.secondary)),
        timer_layout[2],
    );
}

fn draw_today(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let stats = &app.stats;
    let block = panel("TODAY'S FOCUS", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let progress = format!(
        "{} / {} min  ({}%)",
        stats.completed_focus_minutes, stats.daily_goal_minutes, stats.goal_percent
    );
    let text = vec![
        Line::from(vec![
            Span::styled(format!("{:>2}", stats.completed_focus_sessions), theme.title()),
            Span::styled(" completed sessions", Style::default().fg(theme.secondary)),
        ]),
        Line::from(vec![
            Span::styled(progress, Style::default().fg(theme.accent)),
            Span::styled(
                format!("  ·  {} min to goal", stats.remaining_goal_minutes),
                Style::default().fg(theme.secondary),
            ),
        ]),
        Line::from(Span::styled(
            format!(
                "{} day streak  ·  {} min all time",
                stats.current_streak, stats.total_focus_minutes
            ),
            Style::default().fg(theme.secondary),
        )),
    ];
    frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }), inner);
}

fn draw_recent(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let block = panel("RECENT ACTIVITY", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if app.history.is_empty() {
        frame.render_widget(
            Paragraph::new("No sessions yet.\nStart with a focused 25 minutes.")
                .style(Style::default().fg(theme.secondary))
                .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    }
    let items = app.history.iter().take(5).map(|session| {
        let color = match session.status {
            SessionStatus::Completed => theme.success,
            SessionStatus::Skipped => theme.warning,
            _ => theme.secondary,
        };
        ListItem::new(Line::from(vec![
            Span::styled(
                format!("{:<11}", short_type(session.session_type)),
                Style::default().fg(if session.session_type.is_focus() {
                    theme.focus
                } else {
                    theme.break_color
                }),
            ),
            Span::styled(
                format!("{:>3}m ", session.actual_seconds / 60),
                Style::default().fg(theme.primary),
            ),
            Span::styled(session.status.label(), Style::default().fg(color)),
            Span::styled(
                format!("  {}", format_local_time(session.started_at)),
                Style::default().fg(theme.muted),
            ),
        ]))
    });
    frame.render_widget(List::new(items), inner);
}

fn draw_next(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let block = panel("UP NEXT", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let next = match app.timer.status() {
        TimerStatus::Completed if app.timer.session_type().is_focus() => {
            format!("{}\n\nPress n to begin", app.timer.suggested_break().label())
        }
        TimerStatus::Completed => "FOCUS\n\nPress n to begin".to_owned(),
        TimerStatus::Idle => "FOCUS\n\nPress Space to start".to_owned(),
        _ => "Stay with the current session.\n\nSmall steps, done well.".to_owned(),
    };
    frame.render_widget(
        Paragraph::new(next).style(Style::default().fg(theme.secondary)).wrap(Wrap { trim: true }),
        inner,
    );
}

fn draw_history(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let block = panel("SESSION HISTORY  ·  Esc dashboard  ·  j/k scroll", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if app.history.is_empty() {
        frame.render_widget(Paragraph::new("No recorded sessions yet."), inner);
        return;
    }
    let start = app.history_scroll.min(app.history.len().saturating_sub(1));
    let items = app.history.iter().skip(start).take(inner.height as usize).map(|session| {
        let color = if session.status == SessionStatus::Completed {
            theme.success
        } else {
            theme.secondary
        };
        ListItem::new(Line::from(vec![
            Span::styled(
                format!("{:<11}", short_type(session.session_type)),
                Style::default().fg(theme.primary),
            ),
            Span::styled(
                format!("{:>4}m ", session.actual_seconds / 60),
                Style::default().fg(theme.accent),
            ),
            Span::styled(format!("{:<11}", session.status.label()), Style::default().fg(color)),
            Span::styled(format_local_time(session.started_at), Style::default().fg(theme.muted)),
        ]))
    });
    frame.render_widget(List::new(items), inner);
}

fn draw_statistics(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let block = panel("STATISTICS  ·  Esc dashboard", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let rows = app
        .stats
        .last_seven_days
        .iter()
        .map(|day| {
            let bar_len = ((day.minutes.min(180) as f64 / 180.0) * 22.0).round() as usize;
            let bar = format!("{}{}", "█".repeat(bar_len), "·".repeat(22 - bar_len));
            Line::from(vec![
                Span::styled(
                    day.date.format("%a %d").to_string(),
                    Style::default().fg(theme.secondary),
                ),
                Span::raw("  "),
                Span::styled(bar, Style::default().fg(theme.accent)),
                Span::styled(format!(" {:>3}m", day.minutes), Style::default().fg(theme.primary)),
            ])
        })
        .collect::<Vec<_>>();
    let summary = vec![
        Line::from(Span::styled(
            format!(
                "{} focus sessions  ·  {} completed minutes",
                app.stats.total_focus_sessions, app.stats.total_focus_minutes
            ),
            theme.title(),
        )),
        Line::from(Span::styled(
            format!(
                "Current streak: {} days  ·  Longest: {} days",
                app.stats.current_streak, app.stats.longest_streak
            ),
            Style::default().fg(theme.secondary),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "LAST 7 DAYS",
            Style::default().fg(theme.muted).add_modifier(Modifier::BOLD),
        )),
    ];
    let mut lines = summary;
    lines.extend(rows);
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn draw_settings(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let block = panel("SETTINGS  ·  Esc dashboard", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let text = vec![
        Line::from(Span::styled("Current configuration", theme.title())),
        Line::from(""),
        Line::from(format!("Focus             {:>3} minutes", app.config.timer.focus_minutes)),
        Line::from(format!(
            "Short break       {:>3} minutes",
            app.config.timer.short_break_minutes
        )),
        Line::from(format!("Long break        {:>3} minutes", app.config.timer.long_break_minutes)),
        Line::from(format!(
            "Long break every  {:>3} focus sessions",
            app.config.timer.long_break_interval
        )),
        Line::from(format!(
            "Daily goal        {:>3} minutes",
            app.config.productivity.daily_goal_minutes
        )),
        Line::from(format!("Theme             {}", app.config.ui.theme)),
        Line::from(""),
        Line::from(Span::styled(
            "Edit the TOML file, then relaunch Tickt.",
            Style::default().fg(theme.secondary),
        )),
        Line::from(Span::styled(
            format!("{}", app.paths.config_file.display()),
            Style::default().fg(theme.muted),
        )),
    ];
    frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), inner);
}

fn draw_help(frame: &mut Frame<'_>, _app: &App, area: Rect, theme: Theme) {
    let block = panel("HELP  ·  Esc dashboard", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let text = Text::from(vec![
        Line::from(Span::styled("THE DAILY LOOP", theme.title())),
        Line::from("Start a focus session, protect the block, then take the suggested break."),
        Line::from("A focus session counts only when its full planned duration completes."),
        Line::from(""),
        Line::from(Span::styled(
            "TIMER",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        )),
        Line::from("Space  start / pause / resume     n  next suggested session"),
        Line::from("r      reset (confirmation)       s  skip (confirmation)"),
        Line::from("b      short break                l  long break"),
        Line::from(""),
        Line::from(Span::styled(
            "VIEWS",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        )),
        Line::from("h      history       t  statistics       ,  settings"),
        Line::from("?      help          q  quit             Esc  back"),
        Line::from(""),
        Line::from(Span::styled(
            "RECOVERY",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        )),
        Line::from("An active timer is saved locally. On restart, choose whether to recover it."),
        Line::from(
            "If it expired while Tickt was closed, it is recorded as interrupted, not completed.",
        ),
    ]);
    frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), inner);
}

fn draw_footer(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let hint = match app.view {
        View::Dashboard => {
            "Space start/pause   n next   h history   t stats   , settings   ? help   q quit"
        }
        View::History | View::Statistics | View::Settings | View::Help => {
            "Esc dashboard   ? help   q quit"
        }
    };
    let message = app.message.as_deref().unwrap_or(hint);
    frame.render_widget(
        Paragraph::new(message)
            .style(Style::default().fg(if app.message.is_some() {
                theme.warning
            } else {
                theme.muted
            }))
            .alignment(Alignment::Center),
        area,
    );
}

fn draw_compact(frame: &mut Frame<'_>, app: &App, theme: Theme) {
    let area = frame.area();
    let display = app.timer.display(Instant::now());
    let lines = vec![
        Line::from(Span::styled("TICKT", theme.title())),
        Line::from(Span::styled(display.session_type.label(), Style::default().fg(theme.accent))),
        Line::from(Span::styled(format_duration(display.remaining), theme.title())),
        Line::from("").style(Style::default().fg(theme.secondary)),
        Line::from("Terminal too small for the full dashboard."),
        Line::from("Resize to at least 64×18 for the full view."),
        Line::from("Space start/pause  ·  q quit"),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme.border)
                    .style(theme.panel_style()),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_confirmation(
    frame: &mut Frame<'_>,
    app: &App,
    action: crate::app::ConfirmAction,
    theme: Theme,
) {
    let area = centered_rect(58, 25, frame.area());
    let title = match action {
        crate::app::ConfirmAction::Reset => "Reset current session?",
        crate::app::ConfirmAction::Skip => "Skip current session?",
        crate::app::ConfirmAction::Quit => "Quit Tickt?",
    };
    let message = match action {
        crate::app::ConfirmAction::Reset => "Progress will not count as completed work.",
        crate::app::ConfirmAction::Skip => "This session will be recorded as skipped.",
        crate::app::ConfirmAction::Quit => "Your active timer will be saved for recovery.",
    };
    let content = Paragraph::new(vec![
        Line::from(title),
        Line::from(""),
        Line::from(message),
        Line::from(""),
        Line::from("[y] confirm   [n/Esc] cancel"),
    ])
    .alignment(Alignment::Center)
    .style(Style::default().fg(theme.primary))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.warning)
            .title("CONFIRM")
            .style(theme.surface),
    );
    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(content, area);
    let _ = app;
}

fn draw_recovery(frame: &mut Frame<'_>, _app: &App, expired: bool, theme: Theme) {
    let area = centered_rect(66, 28, frame.area());
    let message = if expired {
        "This timer reached its deadline while Tickt was closed.\nIt will not be counted as completed work."
    } else {
        "Tickt found an unfinished timer from a previous run."
    };
    let content = Paragraph::new(vec![
        Line::from("RECOVER ACTIVE TIMER"),
        Line::from(""),
        Line::from(message),
        Line::from(""),
        Line::from(if expired {
            "[d/r] record interrupted   [Esc] keep"
        } else {
            "[r] recover   [d] discard   [Esc] keep"
        }),
    ])
    .alignment(Alignment::Center)
    .style(Style::default().fg(theme.primary))
    .block(Block::default().borders(Borders::ALL).border_style(theme.accent).style(theme.surface));
    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(content, area);
}

fn panel(title: &str, theme: Theme) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border)
        .title(Span::styled(
            title.to_owned(),
            Style::default().fg(theme.muted).add_modifier(Modifier::BOLD),
        ))
        .style(theme.panel_style())
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width.saturating_sub(2));
    let height = height.min(area.height.saturating_sub(2));
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((area.width.saturating_sub(width)) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(area);
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(horizontal[1]);
    vertical[1]
}

fn format_duration(duration: std::time::Duration) -> String {
    let seconds = duration.as_secs();
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

fn short_type(value: SessionType) -> &'static str {
    match value {
        SessionType::Focus => "focus",
        SessionType::ShortBreak => "short break",
        SessionType::LongBreak => "long break",
    }
}
