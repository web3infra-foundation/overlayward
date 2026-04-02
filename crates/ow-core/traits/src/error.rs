use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OwError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("invalid state transition: {from:?} -> {to:?}")]
    InvalidTransition { from: String, to: String },

    #[error("container not found: {0}")]
    NotFound(String),

    #[error("operation not supported: {0}")]
    Unsupported(String),

    #[error("build failed at step {step}: {source}")]
    BuildFailed {
        step: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("config error: {0}")]
    Config(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, OwError>;
