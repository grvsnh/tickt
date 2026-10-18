use anyhow::Result;
use clap::Parser;

use tickt::cli::{Cli, Command};
use tickt::config::{AppConfig, AppPaths};
use tickt::storage::Storage;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let paths = AppPaths::resolve()?;

    match cli.command {
        Some(Command::Config { command }) => command.run(&paths),
        Some(Command::Stats) => {
            let config = AppConfig::load_or_default(&paths)?;
            let storage = Storage::open(&paths.database_file)?;
            tickt::cli::print_stats(&storage, &config)
        }
        Some(Command::History(args)) => {
            let storage = Storage::open(&paths.database_file)?;
            tickt::cli::print_history(&storage, args.limit)
        }
        None => {
            let config = AppConfig::load_or_default(&paths)?;
            tickt::app::run(paths, config)
        }
    }
}
