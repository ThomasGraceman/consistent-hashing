// i am trying to use casandra implementation, i will try to come up with something on my own 

//! Ring-based token distribution system for consistent hashing.
//!
//! This module provides abstractions for managing positions on a consistent hash ring,
//! commonly used in distributed databases like Cassandra, DynamoDB, and Riak.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐
//! │  RingPosition   │  ← High-level position API
//! └────────┬────────┘
//!          │
//!    ┌─────┴──────┐
//!    ▼            ▼
//! ┌──────┐   ┌────────────┐
//! │Token │   │Partitioner │  ← Core abstractions
//! └──────┘   └────────────┘
//! ```
//! because A ring position depends on the Token and the hash function that does the hashing as  a partitioner.


// ================================================================================================
// Token Abstraction
// ================================================================================================

/// Represents a position token on the hash ring.
///
/// Tokens are immutable values that represent positions in the token space.
/// They must be:
/// - **Comparable**: To determine ordering on the ring
/// - **Hashable**: For efficient lookups and storage
/// - **Thread-safe**: For concurrent access patterns
///
/// # Design Rationale
///
/// Using a trait allows different token representations:
/// - `u64` for Murmur3 (Cassandra default)
/// - `i128` for ordered partitioners
/// - `Vec<u8>` for arbitrary-precision tokens


// ================================================================================================
// Ring Position Abstraction
// ================================================================================================

/// A position on the consistent hash ring.
///
/// # Why Use This Trait?
///
/// 1. **Abstraction**: Decouples ring logic from specific implementations
///    - Easy to swap partitioners (Murmur3 → Random → ByteOrdered)
///    - Test with mock implementations
///    - Support different token types (u64, u128, BigInt)
///
/// 2. **Type Safety**: Associated types prevent incompatible token/partitioner combinations
///    ```rust
///    // Compiler enforces matching types
///    impl RingPosition for Position {
///        type TokenType = U128Token;
///        type PartitionerType = Murmur3Partitioner; // Must use U128Token ✓
///    }
///    ```
///
/// 3. **Zero-Cost**: Trait methods compile to direct calls (no vtable overhead)
///
/// 4. **Composability**: Build higher-level abstractions (RingTopology, ReplicationStrategy)
///
/// # Concurrency Model
///
/// - `Clone`: Each thread gets its own position instance
/// - `Send`: Positions can be moved between threads (ownership transfer)
/// - `Sync`: Positions can be shared via `Arc<dyn RingPosition>` (shared reads)
/// - `'static`: No lifetime issues when spawning threads
///

/// so we have abstracted away the Token type and the Partitioner type so that we can use it in a more generic way.


use std::cmp::Ordering;
use std::error::Error as StdError;
use std::fmt::{self, Debug, Display};
use std::hash::Hash;
use std::sync::Arc;



pub trait RingPosition: Clone + Ord + Send + Sync + Debug + 'static {
    /// The token type used by this position.
    type TokenType: Token;

    /// The partitioner that generated this position's token.
    /// ensures type safety!
    type PartitionerType: Partitioner<TokenType = Self::TokenType>;

    /// Returns a reference to this position's token.
    ///
    /// # Performance
    ///
    /// Returns a reference to avoid copying potentially large token values.
    fn token(&self) -> &Self::TokenType;

    /// Returns a shared reference to the partitioner.
    ///
    /// # Rationale for `Arc`
    ///
    /// - Partitioners are shared across many positions
    /// - `Arc` enables efficient sharing without copying stateful partitioners
    /// - Thread-safe reference counting for concurrent access
    fn partitioner(&self) -> Arc<Self::PartitionerType>;

    /// Checks if this is the minimum position on the ring.
    ///
    /// # Use Cases
    ///
    /// - Detecting ring wrap-around in range queries
    /// - Validating token distribution
    /// - Initial cluster state checks
    fn is_minimum(&self) -> bool;

    /// Creates a new position at the minimum ring value.
    ///
    /// # Invariant
    ///
    /// `position.min_value().is_minimum()` must always be `true`.
    fn min_value(&self) -> Self;

    /// Checks if this is the maximum position on the ring.
    fn is_maximum(&self) -> bool {
        self.token() == &self.partitioner().max_token()
    }

    /// Creates a new position at the maximum ring value.
    fn max_value(&self) -> Self;

}