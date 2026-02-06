//! Murmur3 hash token implementation (Cassandra-compatible).

use crate::token::traits::Token;
use siphasher::sip::SipHasher13;
use std::hash::{Hash, Hasher};

/// Murmur3 token using u64 representation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Murmur3Token(pub u64);

impl Token for Murmur3Token {
    fn zero() -> Self {
        Murmur3Token(0)
    }

    fn max() -> Self {
        Murmur3Token(u64::MAX)
    }

    fn is_zero(&self) -> bool {
        self.0 == 0
    }

    fn is_max(&self) -> bool {
        self.0 == u64::MAX
    }

    fn distance_to(&self, other: &Self) -> Self {
        if other.0 >= self.0 {
            Murmur3Token(other.0 - self.0)
        } else {
            Murmur3Token((u64::MAX - self.0) + other.0 + 1)
        }
    }
}

impl Murmur3Token {
    /// Creates a token from a byte slice using Murmur3 hashing.
    pub fn from_bytes(data: &[u8]) -> Self {
        let mut hasher = SipHasher13::new();
        data.hash(&mut hasher);
        Murmur3Token(hasher.finish())
    }

    /// Creates a token from a string key.
    pub fn from_key(key: &str) -> Self {
        Self::from_bytes(key.as_bytes())
    }
}
