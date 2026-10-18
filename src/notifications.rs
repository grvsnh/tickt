use std::io::{self, Write};
use std::process::Command;

use crate::config::NotificationConfig;
use crate::model::SessionType;

pub fn session_complete(config: &NotificationConfig, session_type: SessionType) -> Option<String> {
    let message = match session_type {
        SessionType::Focus => "Focus session complete — take a mindful break.",
        SessionType::ShortBreak | SessionType::LongBreak => "Break complete — ready to focus?",
    };
    if config.sound {
        let _ = io::stdout().write_all(b"\x07");
        let _ = io::stdout().flush();
    }
    if !config.enabled {
        return None;
    }
    if Command::new("notify-send").args(["Tickt", message]).spawn().is_err() {
        return Some("desktop notification unavailable; timer continues normally".to_owned());
    }
    None
}
