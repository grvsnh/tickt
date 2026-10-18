use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::error::TicktError;

#[derive(Clone, Debug)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub state_dir: PathBuf,
    pub config_file: PathBuf,
    pub database_file: PathBuf,
    pub log_file: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Result<Self> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .context("HOME must be set to an absolute Linux home directory")?;
        let config_root = xdg_path("XDG_CONFIG_HOME", &home.join(".config"));
        let data_root = xdg_path("XDG_DATA_HOME", &home.join(".local").join("share"));
        let state_root = xdg_path("XDG_STATE_HOME", &home.join(".local").join("state"));
        let config_dir = config_root.join("tickt");
        let data_dir = data_root.join("tickt");
        let state_dir = state_root.join("tickt");
        Ok(Self {
            config_file: config_dir.join("config.toml"),
            database_file: data_dir.join("tickt.db"),
            log_file: state_dir.join("tickt.log"),
            config_dir,
            data_dir,
            state_dir,
        })
    }

    pub fn for_test(root: &Path) -> Self {
        let config_dir = root.join("config").join("tickt");
        let data_dir = root.join("data").join("tickt");
        let state_dir = root.join("state").join("tickt");
        Self {
            config_file: config_dir.join("config.toml"),
            database_file: data_dir.join("tickt.db"),
            log_file: state_dir.join("tickt.log"),
            config_dir,
            data_dir,
            state_dir,
        }
    }
}

fn xdg_path(name: &str, fallback: &Path) -> PathBuf {
    std::env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| fallback.to_path_buf())
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    pub timer: TimerConfig,
    pub productivity: ProductivityConfig,
    pub ui: UiConfig,
    pub notifications: NotificationConfig,
    pub recovery: RecoveryConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TimerConfig {
    pub focus_minutes: u16,
    pub short_break_minutes: u16,
    pub long_break_minutes: u16,
    pub long_break_interval: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProductivityConfig {
    pub daily_goal_minutes: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UiConfig {
    pub theme: String,
    pub unicode: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NotificationConfig {
    pub enabled: bool,
    pub sound: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RecoveryConfig {
    pub on_startup: String,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            focus_minutes: 25,
            short_break_minutes: 5,
            long_break_minutes: 15,
            long_break_interval: 4,
        }
    }
}

impl Default for ProductivityConfig {
    fn default() -> Self {
        Self { daily_goal_minutes: 100 }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self { theme: "dark".to_owned(), unicode: true }
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self { enabled: true, sound: false }
    }
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self { on_startup: "ask".to_owned() }
    }
}

impl AppConfig {
    pub fn load_or_default(paths: &AppPaths) -> Result<Self> {
        if !paths.config_file.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(&paths.config_file)
            .with_context(|| format!("could not read {}", paths.config_file.display()))?;
        let config: Self = toml::from_str(&contents)
            .with_context(|| format!("could not parse {}", paths.config_file.display()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn init(paths: &AppPaths) -> Result<bool> {
        fs::create_dir_all(&paths.config_dir)
            .with_context(|| format!("could not create {}", paths.config_dir.display()))?;
        if paths.config_file.exists() {
            return Ok(false);
        }
        let contents =
            toml::to_string_pretty(&Self::default()).context("could not serialize config")?;
        fs::write(&paths.config_file, contents)
            .with_context(|| format!("could not write {}", paths.config_file.display()))?;
        Ok(true)
    }

    pub fn validate(&self) -> Result<()> {
        validate_minutes("timer.focus_minutes", self.timer.focus_minutes, 1, 240)?;
        validate_minutes("timer.short_break_minutes", self.timer.short_break_minutes, 1, 120)?;
        validate_minutes("timer.long_break_minutes", self.timer.long_break_minutes, 1, 240)?;
        if !(1..=12).contains(&self.timer.long_break_interval) {
            return Err(TicktError::InvalidConfiguration(
                "timer.long_break_interval must be between 1 and 12".to_owned(),
            )
            .into());
        }
        validate_minutes(
            "productivity.daily_goal_minutes",
            self.productivity.daily_goal_minutes,
            1,
            1_440,
        )?;
        if !matches!(self.ui.theme.as_str(), "dark" | "light" | "default") {
            return Err(TicktError::InvalidConfiguration(
                "ui.theme must be one of: dark, light, default".to_owned(),
            )
            .into());
        }
        if !matches!(self.recovery.on_startup.as_str(), "ask" | "resume" | "discard") {
            return Err(TicktError::InvalidConfiguration(
                "recovery.on_startup must be one of: ask, resume, discard".to_owned(),
            )
            .into());
        }
        Ok(())
    }
}

fn validate_minutes(field: &str, value: u16, min: u16, max: u16) -> Result<()> {
    if !(min..=max).contains(&value) {
        bail!(TicktError::InvalidConfiguration(format!(
            "{field} must be between {min} and {max} minutes"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn defaults_are_valid() {
        assert!(AppConfig::default().validate().is_ok());
    }

    #[test]
    fn malformed_and_invalid_configs_are_rejected() {
        assert!(toml::from_str::<AppConfig>("[timer\n").is_err());
        let mut config = AppConfig::default();
        config.timer.focus_minutes = 0;
        assert!(config.validate().is_err());
        assert!(toml::from_str::<AppConfig>("unknown = true").is_err());
    }

    #[test]
    fn validation_covers_all_public_enum_and_range_fields() {
        let mut config = AppConfig::default();
        config.timer.focus_minutes = 241;
        assert!(config.validate().is_err());

        let mut config = AppConfig::default();
        config.timer.short_break_minutes = 121;
        assert!(config.validate().is_err());

        let mut config = AppConfig::default();
        config.timer.long_break_minutes = 241;
        assert!(config.validate().is_err());

        let mut config = AppConfig::default();
        config.timer.long_break_interval = 13;
        assert!(config.validate().is_err());

        let mut config = AppConfig::default();
        config.productivity.daily_goal_minutes = 0;
        assert!(config.validate().is_err());

        let mut config = AppConfig::default();
        config.ui.theme = "neon".to_owned();
        assert!(config.validate().is_err());

        let mut config = AppConfig::default();
        config.recovery.on_startup = "always".to_owned();
        assert!(config.validate().is_err());
    }

    #[test]
    fn partial_config_keeps_nested_defaults_and_example_is_valid() {
        let partial: AppConfig =
            toml::from_str("[timer]\nfocus_minutes = 30").expect("partial config");
        assert_eq!(partial.timer.focus_minutes, 30);
        assert_eq!(partial.timer.short_break_minutes, 5);
        assert_eq!(partial.productivity.daily_goal_minutes, 100);

        let example: AppConfig =
            toml::from_str(include_str!("../examples/config.toml")).expect("example config parses");
        assert!(example.validate().is_ok());
    }

    #[test]
    fn init_never_overwrites_existing_config() {
        let root = tempdir().expect("tempdir");
        let paths = AppPaths::for_test(root.path());
        assert!(AppConfig::init(&paths).expect("init"));
        assert!(!AppConfig::init(&paths).expect("second init"));
        assert!(paths.config_file.exists());
    }

    #[test]
    fn load_reports_invalid_existing_config() {
        let root = tempdir().expect("tempdir");
        let paths = AppPaths::for_test(root.path());
        fs::create_dir_all(&paths.config_dir).expect("config directory");
        fs::write(&paths.config_file, "[ui]\ntheme = 'unknown'").expect("config file");
        assert!(AppConfig::load_or_default(&paths).is_err());
    }
}
