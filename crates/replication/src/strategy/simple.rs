//! Simple replication strategy.
//!
//! Places N replicas sequentially around the ring (clockwise from the primary).
//! This is the simplest replication strategy and works well for:
//!
//! - Small clusters (< 10 nodes)
//! - Single data center deployments
//! - When network topology doesn't matter
//!
//! # Algorithm
//!
//! 1. Find primary node (clockwise search from key's token)
//! 2. Continue clockwise to find N-1 more unique nodes
//! 3. Return list of node IDs (primary first)
//!
//! # Performance
//!
//! - **Time**: O(r * log n) where r = replica count, n = tokens
//!   - Each node lookup is O(log n)
//!   - We do r lookups
//! - **Space**: O(r) - returns Vec of node IDs
//!
//! # Limitations
//!
//! - Doesn't consider data center/rack placement
//! - May place replicas on nodes in the same failure domain
//! - Not optimal for multi-DC deployments

use crate::strategy::ReplicationStrategy;
use corelib::node::NodeId;
use corelib::ring::HashRing;

/// Simple replication strategy: N replicas placed sequentially around the ring.
///
/// This strategy finds the primary node (first node clockwise from the key's token),
/// then continues clockwise to find N-1 more unique nodes for replicas.
///
/// # Example
///
/// ```rust
/// use replication::SimpleStrategy;
/// use corelib::ring::HashRing;
///
/// let strategy = SimpleStrategy::new(3); // 3 replicas
/// let ring = HashRing::new();
/// // ... add nodes ...
///
/// let replicas = strategy.replicas_for_key(&ring, b"my-key");
/// // Returns [NodeId(1), NodeId(2), NodeId(3)] - primary + 2 replicas
/// ```
#[derive(Debug, Clone)]
pub struct SimpleStrategy {
    /// Number of replicas to create (including primary).
    replication_factor: usize,
}

impl SimpleStrategy {
    /// Create a new simple strategy with the given replication factor.
    ///
    /// # Arguments
    /// * `replication_factor` - Number of replicas (typically 1-5)
    ///   - 1: No replication (single copy)
    ///   - 3: Standard (primary + 2 replicas)
    ///   - 5: High availability (primary + 4 replicas)
    ///
    /// # Performance
    /// - **Time**: O(1) - just stores the factor
    /// - **Space**: O(1)
    ///
    /// # Example
    /// ```rust
    /// let strategy = SimpleStrategy::new(3);
    /// ```
    pub fn new(replication_factor: usize) -> Self {
        Self {
            replication_factor,
        }
    }

    /// Get the default strategy (3 replicas).
    ///
    /// # Returns
    /// SimpleStrategy with replication_factor = 3
    pub fn default() -> Self {
        Self::new(3)
    }
}

impl ReplicationStrategy for SimpleStrategy {
    fn replication_factor(&self) -> usize {
        self.replication_factor
    }

    fn replicas_for_key(&self, ring: &HashRing, key: &[u8]) -> Vec<NodeId> {
        if self.replication_factor == 0 {
            return Vec::new();
        }

        // Find the primary node (first replica)
        let primary = match ring.lookup(key) {
            Some(node_id) => node_id,
            None => return Vec::new(), // Empty ring
        };

        // Get all tokens sorted by position
        let mut tokens = ring.tokens();
        if tokens.is_empty() {
            return Vec::new();
        }

        // Sort tokens by position
        tokens.sort_by_key(|(token, _)| *token);

        // Find the starting position (first token that maps to primary node)
        // We need to find where the primary node's tokens start
        let start_idx = tokens
            .iter()
            .position(|(_, node_id)| *node_id == primary)
            .unwrap_or(0);

        // Collect replicas (wrapping around if needed)
        let mut replicas = Vec::with_capacity(self.replication_factor);
        let mut seen_nodes = std::collections::HashSet::new();

        // Add primary first
        replicas.push(primary);
        seen_nodes.insert(primary);

        // Continue clockwise to find more replicas
        for i in 1..tokens.len() {
            let idx = (start_idx + i) % tokens.len();
            let (_, node_id) = tokens[idx];

            // Skip if we've already seen this node
            if seen_nodes.contains(&node_id) {
                continue;
            }

            replicas.push(node_id);
            seen_nodes.insert(node_id);

            // Stop when we have enough replicas
            if replicas.len() >= self.replication_factor {
                break;
            }
        }

        replicas
    }

    fn name(&self) -> &'static str {
        "SimpleStrategy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use corelib::node::Node;

    #[test]
    fn test_simple_strategy_replication_factor() {
        let strategy = SimpleStrategy::new(3);
        assert_eq!(strategy.replication_factor(), 3);
    }

    #[test]
    fn test_simple_strategy_replicas() {
        let ring = HashRing::new();
        ring.add_node(Node::new(NodeId(1), "node1"), 4);
        ring.add_node(Node::new(NodeId(2), "node2"), 4);
        ring.add_node(Node::new(NodeId(3), "node3"), 4);

        let strategy = SimpleStrategy::new(3);
        let replicas = strategy.replicas_for_key(&ring, b"test-key");

        assert_eq!(replicas.len(), 3);
        // Should have unique nodes
        let unique: std::collections::HashSet<_> = replicas.iter().collect();
        assert_eq!(unique.len(), 3);
    }
}
