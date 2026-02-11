//! Consistent hash ring implementation.
//!
//! The ring manages token positions and provides efficient lookup
//! operations for finding nodes responsible for keys.

pub mod ring;
pub mod position;
pub mod topology;

pub use position::RingPosition;
pub use ring::{HashRing, RingBuilder};
pub use topology::RingTopology;

/// Alias for the main ring type (used by lib.rs).
pub type Ring = HashRing;
