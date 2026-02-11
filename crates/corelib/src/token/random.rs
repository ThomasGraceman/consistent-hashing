//! Random token implementation for consistent hashing.

use crate::token::traits::Token;
use std::hash::Hash;

/// Random token using u64 representation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct RandomToken(pub u64);

impl Token for RandomToken {
    fn zero() -> Self {
        RandomToken(0)
    }

    fn max() -> Self {
        RandomToken(u64::MAX)
    }

    fn is_zero(&self) -> bool {
        self.0 == 0
    }

    fn is_max(&self) -> bool {
        self.0 == u64::MAX
    }

    fn distance_to(&self, other: &Self) -> Self {
        if other.0 >= self.0 {
            RandomToken(other.0 - self.0)
        } else {
            RandomToken((u64::MAX - self.0) + other.0 + 1)
        }
    }
}

impl RandomToken {
    /// Creates a random token from a seed.
    pub fn from_seed(seed: u64) -> Self {
        RandomToken(seed)
    }
}
