use anyhow::Result;
use clap::{Args, Parser, Subcommand};

use crate::config::{AppConfig, AppPaths};
use crate::model::{SessionStatus, SessionType};
use crate::statistics::{Statistics, format_local_time, local_today};
use crate::storage::Storage;

#[derive(Debug, Parser)]
#[command(name = "tickt", version, about = "A terminal focus dashboard — make time count.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage the local TOML configuration.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Print a concise productivity summary.
    Stats,
    /// Print recent sessions without entering the dashboard.
    History(HistoryArgs),
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Print the resolved configuration file path.
    Path,
    /// Create a sample configuration without overwriting an existing file.
    Init,
}

#[derive(Debug, Args)]
pub struct HistoryArgs {
    /// Number of sessions to print (maximum 50).
    #[arg(short, long, default_value_t = 10)]
    pub limit: u32,
}

impl ConfigCommand {
    pub fn run(self, paths: &AppPaths) -> Result<()> {
        match self {
            Self::Path => {
                println!("{}", paths.config_file.display());
                Ok(())
            }
            Self::Init => {
                if AppConfig::init(paths)? {
                    println!("Created {}", paths.config_file.display());
                } else {
                    println!("Configuration already exists: {}", paths.config_file.display());
                }
                Ok(())
            }
        }
    }
}

pub fn print_stats(storage: &Storage, config: &AppConfig) -> Result<()> {
    let records = storage.completed_focus_since(0)?;
    let stats = Statistics::calculate_focus(
        &records,
        local_today(),
        u32::from(config.productivity.daily_goal_minutes),
    );
    println!("Tickt · today's focus");
    println!(
        "{} sessions · {} minutes · {}% of {} minute goal",
        stats.completed_focus_sessions,
        stats.completed_focus_minutes,
        stats.goal_percent,
        stats.daily_goal_minutes
    );
    println!(
        "streak {} day{} · total {} sessions / {} minutes",
        stats.current_streak,
        if stats.current_streak == 1 { "" } else { "s" },
        stats.total_focus_sessions,
        stats.total_focus_minutes
    );
    Ok(())
}

pub fn print_history(storage: &Storage, requested_limit: u32) -> Result<()> {
    let limit = requested_limit.clamp(1, 50);
    let sessions = storage.recent_sessions(limit)?;
    if sessions.is_empty() {
        println!("No sessions recorded yet. Start Tickt to make time count.");
        return Ok(());
    }
    println!("Tickt · recent sessions");
    for session in sessions {
        let status = match session.status {
            SessionStatus::Completed => "done",
            SessionStatus::Skipped => "skip",
            SessionStatus::Reset => "reset",
            SessionStatus::Interrupted => "interrupt",
        };
        println!(
            "{:<11} {:>5}m  {:<9} {}",
            match session.session_type {
                SessionType::Focus => "focus",
                SessionType::ShortBreak => "short break",
                SessionType::LongBreak => "long break",
            },
            session.actual_seconds / 60,
            status,
            format_local_time(session.started_at)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_noninteractive_commands() {
        assert!(matches!(
            Cli::try_parse_from(["tickt", "stats"]).expect("stats"),
            Cli { command: Some(Command::Stats) }
        ));
        assert!(matches!(
            Cli::try_parse_from(["tickt", "config", "path"]).expect("config path"),
            Cli { command: Some(Command::Config { command: ConfigCommand::Path }) }
        ));
        assert!(matches!(
            Cli::try_parse_from(["tickt", "history", "--limit", "25"]).expect("history"),
            Cli { command: Some(Command::History(HistoryArgs { limit: 25 })) }
        ));
    }

    #[test]
    fn history_limit_is_bounded_for_user_input() {
        assert_eq!(0_u32.clamp(1, 50), 1);
        assert_eq!(50_u32.clamp(1, 50), 50);
        assert_eq!(500_u32.clamp(1, 50), 50);
    }
}
