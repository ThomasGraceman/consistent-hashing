//! Extended token trait (serialization, distance, byte-comparable).
//! Use this when you need wire/storage formats; ring logic uses the minimal `Token` trait.

use std::fmt::Display;
use std::hash::Hash;

use serde::{Deserialize, Serialize};

use super::traits::{ByteComparableVersion, TokenError};

/// Distance metric in token space (for extended token implementations).
pub trait Distance: Clone + std::fmt::Debug + PartialOrd + From<u128> + Into<u128> {
    fn zero() -> Self;
    fn max() -> Self;
    fn add(&self, other: &Self) -> Self;
    fn sub(&self, other: &Self) -> Option<Self>;
    fn to_f64(&self) -> f64;
}

/// Extended token trait: minimal Token + serialization, distance type, byte-comparable ops.
/// Implement this in addition to `Token` when you need persistence or wire format.
pub trait ExtendedToken:
    super::Token + Display + Serialize + for<'de> Deserialize<'de>
{
    type Distance: Distance;
    type Bytes: AsRef<[u8]> + Into<Vec<u8>>;

    fn to_bytes(&self) -> Self::Bytes;
    fn from_bytes(bytes: &[u8]) -> Result<Self, TokenError>
    where
        Self: Sized;
    fn min_value() -> Self
    where
        Self: Sized;
    fn max_value() -> Self
    where
        Self: Sized;
    fn distance(&self, other: &Self) -> Self::Distance;
    fn split_evenly(n: usize) -> Vec<Self>
    where
        Self: Sized;
    fn as_comparable_bytes(&self, version: ByteComparableVersion) -> Vec<u8>;
    fn from_comparable_bytes(bytes: &[u8], version: ByteComparableVersion) -> Result<Self, TokenError>
    where
        Self: Sized;
    fn next_valid_token(&self) -> Result<Self, TokenError>
    where
        Self: Sized;
    fn decrease_slightly(&self) -> Result<Self, TokenError>
    where
        Self: Sized;
    fn token_hash(&self) -> u64;
    fn heap_size(&self) -> usize;
}
