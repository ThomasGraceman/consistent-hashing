//! Core library for consistent hashing implementation.
//!
//! This crate provides the fundamental abstractions for consistent hashing:
//! - Token types and implementations
//! - Partitioner algorithms
//! - Ring position management
//! - Node and virtual node abstractions
//! - Ring topology and routing

pub mod error;
pub mod node;
pub mod partitioner;
pub mod ring;
pub mod token;
pub mod topology;
pub mod vnode;

pub use error::{Error, Result};
pub use node::{Node, NodeId};
pub use partitioner::Partitioner;
pub use ring::{Ring, RingBuilder};
pub use token::Token;
pub use topology::Topology;
pub use vnode::VirtualNode;
