//! Token abstraction module for consistent hashing.
//!
//! Tokens represent positions on the hash ring and must be comparable,
//! hashable, and thread-safe.

pub mod murmur3;
pub mod random;
pub mod byte_ordered;
pub mod traits;

pub use traits::Token;
