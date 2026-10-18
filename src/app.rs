use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use chrono::{NaiveDate, Utc};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use crate::config::{AppConfig, AppPaths};
use crate::model::{SessionRecord, SessionType};
use crate::notifications;
use crate::statistics::{FocusAccumulator, Statistics, local_today};
use crate::storage::Storage;
use crate::terminal::TerminalGuard;
use crate::timer::{TimerEngine, TimerStatus};
use crate::ui;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum View {
    Dashboard,
    History,
    Statistics,
    Settings,
    Help,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfirmAction {
    Reset,
    Skip,
    Quit,
}

#[derive(Clone, Debug)]
pub struct RecoveryPrompt {
    pub expired: bool,
}

pub struct App {
    pub(crate) paths: AppPaths,
    pub(crate) config: AppConfig,
    pub(crate) storage: Storage,
    pub(crate) timer: TimerEngine,
    pub(crate) view: View,
    pub(crate) history: Vec<SessionRecord>,
    pub(crate) stats: Statistics,
    focus_accumulator: FocusAccumulator,
    pub(crate) message: Option<String>,
    pub(crate) confirmation: Option<ConfirmAction>,
    pub(crate) recovery: Option<RecoveryPrompt>,
    pub(crate) persistence_warning: bool,
    pub(crate) history_scroll: usize,
    stats_date: NaiveDate,
    should_quit: bool,
}

pub fn run(paths: AppPaths, config: AppConfig) -> Result<()> {
    config.validate()?;
    let storage = Storage::open(&paths.database_file)?;
    let timer =
        TimerEngine::new(config.timer.clone(), 0).map_err(|error| anyhow::anyhow!(error))?;
    let mut app = App::new(paths, config, storage, timer)?;
    app.prepare_recovery()?;
    let mut terminal = TerminalGuard::new()?;
    let result = app.event_loop(&mut terminal);
    drop(terminal);
    result
}

impl App {
    fn new(
        paths: AppPaths,
        config: AppConfig,
        storage: Storage,
        timer: TimerEngine,
    ) -> Result<Self> {
        let mut app = Self {
            paths,
            config,
            storage,
            timer,
            view: View::Dashboard,
            history: Vec::new(),
            stats: Statistics::empty(local_today(), 0),
            focus_accumulator: FocusAccumulator::default(),
            message: None,
            confirmation: None,
            recovery: None,
            persistence_warning: false,
            history_scroll: 0,
            stats_date: local_today(),
            should_quit: false,
        };
        app.refresh_data()?;
        app.timer.set_completed_focus_sessions(app.stats.total_focus_sessions);
        Ok(app)
    }

    fn prepare_recovery(&mut self) -> Result<()> {
        let Some(snapshot) = self.storage.load_active()? else {
            return Ok(());
        };
        let expired = self
            .timer
            .restore(snapshot, Instant::now(), Utc::now())
            .map_err(|error| anyhow::anyhow!(error))?;
        if self.config.recovery.on_startup == "discard" {
            self.discard_recovery()?;
        } else if self.config.recovery.on_startup == "resume" && !expired {
            self.recovery = None;
            self.persist_active()?;
        } else {
            self.recovery = Some(RecoveryPrompt { expired });
        }
        Ok(())
    }

    fn event_loop(&mut self, terminal: &mut TerminalGuard) -> Result<()> {
        while !self.should_quit {
            let now = Instant::now();
            if self.stats_date != local_today() {
                self.refresh_statistics();
            }
            if let Some(record) = self.timer.tick(now) {
                self.complete_session(record)?;
            }
            let timeout = if self.timer.status() == TimerStatus::Running {
                Duration::from_millis(100)
            } else {
                Duration::from_millis(250)
            };
            if event::poll(timeout).context("could not poll terminal events")? {
                match event::read().context("could not read terminal event")? {
                    Event::Key(key) => self.handle_key(key)?,
                    Event::Resize(_, _) => {}
                    _ => {}
                }
            }
            terminal
                .terminal
                .draw(|frame| ui::draw(frame, self))
                .context("could not render dashboard")?;
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.confirmation = Some(ConfirmAction::Quit);
            return Ok(());
        }
        if self.recovery.is_some() {
            return self.handle_recovery_key(key);
        }
        if self.confirmation.is_some() {
            return self.handle_confirmation_key(key);
        }
        match self.view {
            View::Dashboard => self.handle_dashboard_key(key),
            View::History => self.handle_history_key(key),
            View::Statistics | View::Settings | View::Help => self.handle_view_key(key),
        }
    }

    fn handle_dashboard_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') => self.confirmation = Some(ConfirmAction::Quit),
            KeyCode::Char('?') => self.view = View::Help,
            KeyCode::Char('h') => {
                self.history_scroll = 0;
                self.view = View::History;
            }
            KeyCode::Char('t') => self.view = View::Statistics,
            KeyCode::Char(',') => self.view = View::Settings,
            KeyCode::Char(' ') => self.toggle_timer()?,
            KeyCode::Char('n') => self.start_next()?,
            KeyCode::Char('b') => self.start_break(SessionType::ShortBreak)?,
            KeyCode::Char('l') => self.start_break(SessionType::LongBreak)?,
            KeyCode::Char('r')
                if matches!(self.timer.status(), TimerStatus::Running | TimerStatus::Paused) =>
            {
                self.confirmation = Some(ConfirmAction::Reset)
            }
            KeyCode::Char('r') => {
                self.message = Some("There is no active session to reset.".to_owned())
            }
            KeyCode::Char('s')
                if matches!(self.timer.status(), TimerStatus::Running | TimerStatus::Paused) =>
            {
                self.confirmation = Some(ConfirmAction::Skip)
            }
            KeyCode::Char('s') => {
                self.message = Some("There is no active session to skip.".to_owned())
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_history_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('d') => self.view = View::Dashboard,
            KeyCode::Char('q') => self.confirmation = Some(ConfirmAction::Quit),
            KeyCode::Char('j') | KeyCode::Down => {
                self.history_scroll =
                    (self.history_scroll + 1).min(self.history.len().saturating_sub(1));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.history_scroll = self.history_scroll.saturating_sub(1)
            }
            KeyCode::Char('?') => self.view = View::Help,
            _ => {}
        }
        Ok(())
    }

    fn handle_view_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('d') => self.view = View::Dashboard,
            KeyCode::Char('q') => self.confirmation = Some(ConfirmAction::Quit),
            KeyCode::Char('?') => self.view = View::Help,
            _ => {}
        }
        Ok(())
    }

    fn handle_confirmation_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                let action = self.confirmation.take();
                match action {
                    Some(ConfirmAction::Reset) => self.reset_timer()?,
                    Some(ConfirmAction::Skip) => self.skip_timer()?,
                    Some(ConfirmAction::Quit) => self.quit()?,
                    None => {}
                }
            }
            KeyCode::Char('n') | KeyCode::Esc => self.confirmation = None,
            _ => {}
        }
        Ok(())
    }

    fn handle_recovery_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('r') if self.recovery.as_ref().is_some_and(|prompt| !prompt.expired) => {
                self.recovery = None;
                self.persist_active()?;
                self.message = Some("Recovered the unfinished timer.".to_owned());
            }
            KeyCode::Char('d') | KeyCode::Char('r') => self.discard_recovery()?,
            KeyCode::Esc if self.recovery.as_ref().is_some_and(|prompt| !prompt.expired) => {
                self.recovery = None;
                self.persist_active()?;
                self.message = Some("Kept the recovered timer active.".to_owned());
            }
            _ => {}
        }
        Ok(())
    }

    fn toggle_timer(&mut self) -> Result<()> {
        match self.timer.status() {
            TimerStatus::Idle => self.start_focus(),
            TimerStatus::Running => {
                self.timer.pause(Instant::now()).map_err(|error| anyhow::anyhow!(error))?;
                self.persist_active()
            }
            TimerStatus::Paused => {
                self.timer
                    .resume(Instant::now(), Utc::now())
                    .map_err(|error| anyhow::anyhow!(error))?;
                self.persist_active()
            }
            TimerStatus::Completed => self.start_next(),
        }
    }

    fn start_focus(&mut self) -> Result<()> {
        if self.timer.status() == TimerStatus::Completed {
            self.timer.clear_completed();
        }
        self.timer
            .start_focus(Instant::now(), Utc::now())
            .map_err(|error| anyhow::anyhow!(error))?;
        self.persist_active()?;
        self.message = Some("Focus started. Protect the block.".to_owned());
        Ok(())
    }

    fn start_break(&mut self, session_type: SessionType) -> Result<()> {
        if self.timer.status() == TimerStatus::Completed {
            self.timer.clear_completed();
        }
        if self.timer.status() != TimerStatus::Idle {
            self.message = Some("Finish or reset the current session first.".to_owned());
            return Ok(());
        }
        self.timer
            .start_break(session_type, Instant::now(), Utc::now())
            .map_err(|error| anyhow::anyhow!(error))?;
        self.persist_active()?;
        self.message = Some(format!("{} started.", session_type.label()));
        Ok(())
    }

    fn start_next(&mut self) -> Result<()> {
        let next = match self.timer.status() {
            TimerStatus::Completed if self.timer.session_type().is_focus() => {
                self.timer.suggested_break()
            }
            TimerStatus::Completed => SessionType::Focus,
            TimerStatus::Idle => SessionType::Focus,
            _ => {
                self.message =
                    Some("Pause, reset, or finish the current session first.".to_owned());
                return Ok(());
            }
        };
        if self.timer.status() == TimerStatus::Completed {
            self.timer.clear_completed();
        }
        match next {
            SessionType::Focus => self.start_focus(),
            break_type => self.start_break(break_type),
        }
    }

    fn reset_timer(&mut self) -> Result<()> {
        let result = self.timer.reset(Instant::now()).map_err(|error| anyhow::anyhow!(error))?;
        let had_record = result.is_some();
        if let Some(record) = result {
            self.storage.finish_session(&record)?;
        }
        if !had_record {
            self.storage.clear_active()?;
        }
        self.message = Some("Session reset; it will not count as completed work.".to_owned());
        self.refresh_history()?;
        self.refresh_statistics();
        Ok(())
    }

    fn skip_timer(&mut self) -> Result<()> {
        let record = self.timer.skip(Instant::now()).map_err(|error| anyhow::anyhow!(error))?;
        self.storage.finish_session(&record)?;
        self.message = Some("Session skipped; it will not count as completed work.".to_owned());
        self.refresh_history()?;
        self.refresh_statistics();
        Ok(())
    }

    fn complete_session(&mut self, record: SessionRecord) -> Result<()> {
        let changed = self.storage.finish_session(&record)?;
        if changed {
            self.focus_accumulator.add_session(&record);
        }
        if let Some(warning) =
            notifications::session_complete(&self.config.notifications, record.session_type)
        {
            self.message = Some(warning);
        } else {
            self.message = Some(format!("{} complete.", record.session_type.label()));
        }
        self.refresh_history()?;
        self.refresh_statistics();
        Ok(())
    }

    fn discard_recovery(&mut self) -> Result<()> {
        if let Some(record) =
            self.timer.interrupt(Instant::now()).map_err(|error| anyhow::anyhow!(error))?
        {
            self.storage.finish_session(&record)?;
        } else {
            self.storage.clear_active()?;
        }
        self.recovery = None;
        self.message = Some("Unfinished session recorded as interrupted.".to_owned());
        self.refresh_history()?;
        self.refresh_statistics();
        Ok(())
    }

    fn persist_active(&mut self) -> Result<()> {
        if let Some(snapshot) = self.timer.snapshot(Instant::now()) {
            self.storage.save_active(&snapshot)?;
        } else {
            self.storage.clear_active()?;
        }
        Ok(())
    }

    fn refresh_data(&mut self) -> Result<()> {
        self.refresh_history()?;
        let records = self.storage.completed_focus_since(0)?;
        self.focus_accumulator = FocusAccumulator::from_completions(&records);
        self.refresh_statistics();
        Ok(())
    }

    fn refresh_history(&mut self) -> Result<()> {
        self.history = self.storage.recent_sessions(100)?;
        Ok(())
    }

    fn refresh_statistics(&mut self) {
        self.stats = Statistics::from_accumulator(
            &self.focus_accumulator,
            local_today(),
            u32::from(self.config.productivity.daily_goal_minutes),
        );
        self.stats_date = local_today();
    }

    fn quit(&mut self) -> Result<()> {
        self.persist_active().context("could not save active timer before quitting")?;
        self.should_quit = true;
        Ok(())
    }

    pub(crate) fn theme(&self) -> crate::theme::Theme {
        crate::theme::Theme::from_name(&self.config.ui.theme)
    }
}
