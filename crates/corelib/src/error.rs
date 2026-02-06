//! Error types for the core library.

use std::fmt;

/// Result type alias for the core library.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in the core library.
#[derive(Debug, Clone)]
pub enum Error {
    /// Invalid token value
    InvalidToken(String),
    /// Invalid node configuration
    InvalidNode(String),
    /// Ring operation failed
    RingOperation(String),
    /// Topology error
    Topology(String),
    /// Internal error
    Internal(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidToken(msg) => write!(f, "Invalid token: {}", msg),
            Error::InvalidNode(msg) => write!(f, "Invalid node: {}", msg),
            Error::RingOperation(msg) => write!(f, "Ring operation failed: {}", msg),
            Error::Topology(msg) => write!(f, "Topology error: {}", msg),
            Error::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}
