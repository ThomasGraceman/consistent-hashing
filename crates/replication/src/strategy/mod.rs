//! Replication strategy abstractions.
//!
//! Replication strategies determine how many replicas to create and where
//! to place them on the ring. Different strategies optimize for different
//! goals:
//!
//! - **SimpleStrategy**: N replicas placed sequentially around the ring
//! - **NetworkTopologyStrategy**: Replicas placed across data centers/racks

pub mod network_topology;
pub mod simple;

pub use network_topology::NetworkTopologyStrategy;
pub use simple::SimpleStrategy;

/// Trait for replication strategies.
///
/// A replication strategy determines:
/// 1. How many replicas to create for a key
/// 2. Which nodes should hold those replicas
/// 3. How to handle node failures/removals
///
/// # Thread Safety
///
/// Implementations must be thread-safe (Send + Sync) as they may be
/// shared across threads.
pub trait ReplicationStrategy: Send + Sync + 'static {
    /// Get the number of replicas this strategy creates.
    ///
    /// # Returns
    /// Replication factor (typically 1-5)
    fn replication_factor(&self) -> usize;

    /// Find replica nodes for a given key.
    ///
    /// # Arguments
    /// * `ring` - The hash ring to query
    /// * `key` - The key to find replicas for
    ///
    /// # Returns
    /// Vec of NodeIds that should hold replicas (primary first)
    ///
    /// # Performance
    /// Should be O(r * log n) where r = replica count, n = tokens
    fn replicas_for_key(&self, ring: &corelib::ring::HashRing, key: &[u8]) -> Vec<corelib::node::NodeId>;

    /// Get the strategy name (for logging/debugging).
    ///
    /// # Returns
    /// Human-readable strategy name
    fn name(&self) -> &'static str;
}
