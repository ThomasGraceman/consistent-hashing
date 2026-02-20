//! Replication strategies for consistent hashing.
//!
//! This crate provides pluggable replication strategies that determine:
//! - How many replicas to create
//! - Where to place replicas (which nodes)
//! - How to handle consistency levels

pub mod consistency;
pub mod error;
pub mod placement;
pub mod strategy;

pub use consistency::ConsistencyLevel;
pub use error::ReplicationError;
pub use placement::ReplicaPlacement;
pub use strategy::{ReplicationStrategy, SimpleStrategy, NetworkTopologyStrategy};
