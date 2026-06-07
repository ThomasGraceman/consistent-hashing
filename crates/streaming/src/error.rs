//! Streaming-specific error types.

use thiserror::Error;

/// Errors that can occur during streaming operations.
#[derive(Debug, Error)]
pub enum StreamingError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("codec error: {0}")]
    Codec(String),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("connection closed")]
    ConnectionClosed,

    #[error("invalid snapshot: {0}")]
    InvalidSnapshot(String),
}
