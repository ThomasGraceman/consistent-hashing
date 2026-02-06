//! Core token trait definitions.

use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::Hash;

/// Represents a position token on the hash ring.
///
/// Tokens are immutable values that represent positions in the token space.
/// They must be:
/// - **Comparable**: To determine ordering on the ring
/// - **Hashable**: For efficient lookups and storage
/// - **Thread-safe**: For concurrent access patterns
pub trait Token: Clone + Ord + Hash + Send + Sync + Debug + 'static {
    /// Returns the zero/minimum token value.
    fn zero() -> Self;

    /// Returns the maximum token value.
    fn max() -> Self;

    /// Checks if this token is the minimum value.
    fn is_zero(&self) -> bool;

    /// Checks if this token is the maximum value.
    fn is_max(&self) -> bool;

    /// Computes the distance to another token on the ring.
    /// Returns the clockwise distance.
    fn distance_to(&self, other: &Self) -> Self;
}
