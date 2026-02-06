//! Core partitioner trait definitions.

use crate::token::Token;
use std::sync::Arc;

/// A partitioner converts keys into tokens for placement on the hash ring.
///
/// Partitioners are stateless and thread-safe, allowing concurrent
/// token generation without synchronization overhead.
pub trait Partitioner: Send + Sync + 'static {
    /// The token type produced by this partitioner.
    type TokenType: Token;

    /// Converts a key into a token.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to partition
    ///
    /// # Returns
    ///
    /// A token representing the position on the ring
    fn partition(&self, key: &[u8]) -> Self::TokenType;

    /// Returns the minimum token value for this partitioner.
    fn min_token(&self) -> Self::TokenType;

    /// Returns the maximum token value for this partitioner.
    fn max_token(&self) -> Self::TokenType;

    /// Returns the name of this partitioner.
    fn name(&self) -> &'static str;
}
