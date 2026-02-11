//! Murmur3 partitioner implementation.

use crate::partitioner::traits::Partitioner;
use crate::token::murmur3::Murmur3Token;
use crate::token::Token;

/// Murmur3 partitioner (Cassandra-compatible).
#[derive(Clone, Debug)]
pub struct Murmur3Partitioner;

impl Partitioner for Murmur3Partitioner {
    type TokenType = Murmur3Token;

    fn partition(&self, key: &[u8]) -> Self::TokenType {
        Murmur3Token::from_bytes(key)
    }

    fn min_token(&self) -> Self::TokenType {
        Murmur3Token::zero()
    }

    fn max_token(&self) -> Self::TokenType {
        <Murmur3Token as Token>::max()
    }

    fn name(&self) -> &'static str {
        "Murmur3Partitioner"
    }
}
