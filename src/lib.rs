#[cfg(not(target_os = "linux"))]
compile_error!("Tickt v1.0 supports Linux terminal environments only.");

pub mod app;
pub mod cli;
pub mod config;
pub mod error;
pub mod model;
pub mod notifications;
pub mod statistics;
pub mod storage;
pub mod terminal;
pub mod theme;
pub mod timer;
pub mod ui;
