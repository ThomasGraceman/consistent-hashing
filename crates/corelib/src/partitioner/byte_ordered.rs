//! Byte-ordered partitioner implementation.

use crate::partitioner::traits::Partitioner;
use crate::token::byte_ordered::ByteOrderedToken;
use crate::token::Token;

/// Byte-ordered partitioner.
#[derive(Clone, Debug)]
pub struct ByteOrderedPartitioner;

impl Partitioner for ByteOrderedPartitioner {
    type TokenType = ByteOrderedToken;

    fn partition(&self, key: &[u8]) -> Self::TokenType {
        ByteOrderedToken::from_bytes(key.to_vec())
    }

    fn min_token(&self) -> Self::TokenType {
        ByteOrderedToken::zero()
    }

    fn max_token(&self) -> Self::TokenType {
        <ByteOrderedToken as Token>::max()
    }

    fn name(&self) -> &'static str {
        "ByteOrderedPartitioner"
    }
}
