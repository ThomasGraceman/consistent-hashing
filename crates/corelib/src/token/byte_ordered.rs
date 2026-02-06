//! Byte-ordered token implementation.

use crate::token::traits::Token;
use std::cmp::Ordering;

/// Byte-ordered token using byte vector representation.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ByteOrderedToken(pub Vec<u8>);

impl Ord for ByteOrderedToken {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for ByteOrderedToken {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Token for ByteOrderedToken {
    fn zero() -> Self {
        ByteOrderedToken(vec![0])
    }

    fn max() -> Self {
        ByteOrderedToken(vec![u8::MAX])
    }

    fn is_zero(&self) -> bool {
        self.0 == [0]
    }

    fn is_max(&self) -> bool {
        self.0 == [u8::MAX]
    }

    fn distance_to(&self, other: &Self) -> Self {
        // Simplified distance calculation for byte-ordered tokens
        ByteOrderedToken(other.0.clone())
    }
}

impl ByteOrderedToken {
    /// Creates a token directly from bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        ByteOrderedToken(bytes)
    }

    /// Creates a token from a string key.
    pub fn from_key(key: &str) -> Self {
        ByteOrderedToken(key.as_bytes().to_vec())
    }
}
