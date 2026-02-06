//! Partitioner abstraction for consistent hashing.
//!
//! Partitioners are responsible for converting keys into tokens
//! that can be placed on the hash ring.

pub mod murmur3;
pub mod random;
pub mod byte_ordered;
pub mod traits;

pub use traits::Partitioner;
