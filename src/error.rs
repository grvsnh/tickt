use thiserror::Error;

#[derive(Debug, Error)]
pub enum TicktError {
    #[error("invalid configuration: {0}")]
    InvalidConfiguration(String),
    #[error("invalid timer transition: {0}")]
    InvalidTransition(String),
}
