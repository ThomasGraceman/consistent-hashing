//! Random partitioner implementation.

use crate::partitioner::traits::Partitioner;
use crate::token::random::RandomToken;
use crate::token::Token;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Random partitioner for consistent hashing.
#[derive(Clone, Debug)]
pub struct RandomPartitioner;

impl Partitioner for RandomPartitioner {
    type TokenType = RandomToken;

    fn partition(&self, key: &[u8]) -> Self::TokenType {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        RandomToken::from_seed(hasher.finish())
    }

    fn min_token(&self) -> Self::TokenType {
        RandomToken::zero()
    }

    fn max_token(&self) -> Self::TokenType {
        <RandomToken as Token>::max()
    }

    fn name(&self) -> &'static str {
        "RandomPartitioner"
    }
}
