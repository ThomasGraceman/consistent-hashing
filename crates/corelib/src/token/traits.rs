//! Core token trait definitions.
//!
//! The main `Token` trait is minimal so ring, partitioners, and position work
//! without serialization. Use `extended::ExtendedToken` for wire/storage.

use std::fmt::Debug;
use std::hash::Hash;

/// Errors that can occur when parsing or manipulating tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenError {
    /// Invalid byte sequence for this token type
    InvalidBytes(String),
    /// Token at boundary (e.g. next_valid_token at max)
    AtBoundary,
}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenError::InvalidBytes(s) => write!(f, "invalid token bytes: {}", s),
            TokenError::AtBoundary => write!(f, "token at boundary"),
        }
    }
}

impl std::error::Error for TokenError {}

/// Version of the byte-comparable encoding format (for wire/storage compatibility).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByteComparableVersion {
    V1,
}

/// Minimal token trait for the hash ring.
///
/// Tokens are immutable, comparable positions. Implementations must be
/// thread-safe and cheap to compare/hash.
pub trait Token: Clone + Ord + Hash + Send + Sync + Debug + 'static {
    /// Minimum token value (start of ring).
    fn zero() -> Self;
    /// Maximum token value (end of ring).
    fn max() -> Self;
    /// True if this token is the minimum.
    fn is_zero(&self) -> bool;
    /// True if this token is the maximum.
    fn is_max(&self) -> bool;
    /// Clockwise distance from `self` to `other` on the ring.
    fn distance_to(&self, other: &Self) -> Self;
}
