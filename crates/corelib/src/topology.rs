//! Ring topology abstractions and operations.
//!
//! This module provides high-level views and operations over the hash ring:
//! - Ownership ranges (which tokens belong to which nodes)
//! - Ring description (human-readable ring state)
//! - Routing helpers (find nodes for keys, ranges)
//! - Load distribution analysis
//!
//! # Use Cases
//!
//! - **Debugging**: Inspect ring state, see token distribution
//! - **Monitoring**: Track ownership percentages, load balance
//! - **Operations**: Understand which keys map to which nodes
//! - **Rebalancing**: Identify nodes that need rebalancing

use crate::node::{Node, NodeId};
use crate::ring::HashRing;
use crate::token::murmur3::Murmur3Token;
use std::collections::HashMap;

/// Ring topology view and operations.
///
/// Provides high-level operations for inspecting and analyzing the ring state.
/// This is a lightweight wrapper around `HashRing` that adds topology-specific
/// operations without modifying the ring itself.
///
/// # Performance
///
/// Most operations require acquiring a read lock and iterating tokens:
/// - **Time**: O(n) where n = number of tokens (vnodes)
/// - **Space**: O(n) for operations that collect data
///
/// # Thread Safety
///
/// - All operations are read-only (don't modify the ring)
/// - Safe for concurrent access (uses read locks)
/// - Can be created from a shared `Arc<HashRing>`
#[derive(Clone)]
pub struct Topology {
    /// Reference to the underlying ring.
    ring: HashRing,
}

impl Topology {
    /// Create a new topology view from a ring.
    ///
    /// # Performance
    /// - **Time**: O(1) - just stores reference
    /// - **Space**: O(1)
    ///
    /// # Arguments
    /// * `ring` - The hash ring to analyze
    pub fn new(ring: HashRing) -> Self {
        Self { ring }
    }

    /// Get ownership information: which tokens belong to which nodes.
    ///
    /// # Algorithm
    ///
    /// 1. Collect all tokens from the ring
    /// 2. Group tokens by node_id
    /// 3. Return mapping: NodeId -> Vec<Token>
    ///
    /// # Performance
    /// - **Time**: O(n) where n = number of tokens
    /// - **Space**: O(n) - allocates Vec for each node
    ///
    /// # Returns
    /// HashMap mapping NodeId to their tokens
    ///
    /// # Example
    /// ```rust
    /// let topology = Topology::new(ring);
    /// let ownership = topology.ownership();
    /// // ownership[NodeId(1)] = [Token(100), Token(200), ...]
    /// ```
    pub fn ownership(&self) -> HashMap<NodeId, Vec<Murmur3Token>> {
        let tokens = self.ring.tokens();
        let mut ownership: HashMap<NodeId, Vec<Murmur3Token>> = HashMap::new();

        for (token, node_id) in tokens {
            ownership.entry(node_id).or_insert_with(Vec::new).push(token);
        }

        // Sort tokens for each node (useful for range queries)
        for tokens in ownership.values_mut() {
            tokens.sort();
        }

        ownership
    }

    /// Get ownership percentages: what fraction of the ring each node owns.
    ///
    /// # Algorithm
    ///
    /// 1. Count tokens per node
    /// 2. Calculate percentage: (node_tokens / total_tokens) * 100
    ///
    /// # Performance
    /// - **Time**: O(n) where n = number of tokens
    /// - **Space**: O(m) where m = number of nodes
    ///
    /// # Returns
    /// HashMap mapping NodeId to ownership percentage (0.0 - 100.0)
    ///
    /// # Example
    /// ```rust
    /// let percentages = topology.ownership_percentages();
    /// // percentages[NodeId(1)] = 33.33 (if node1 owns 1/3 of tokens)
    /// ```
    pub fn ownership_percentages(&self) -> HashMap<NodeId, f64> {
        let ownership = self.ownership();
        let total_tokens = self.ring.token_count() as f64;

        if total_tokens == 0.0 {
            return HashMap::new();
        }

        ownership
            .into_iter()
            .map(|(node_id, tokens)| {
                let percentage = (tokens.len() as f64 / total_tokens) * 100.0;
                (node_id, percentage)
            })
            .collect()
    }

    /// Describe the ring in a human-readable format.
    ///
    /// # Format
    ///
    /// ```
    /// Ring Description:
    ///   Nodes: 3
    ///   Total Tokens: 768
    ///   Partitioner: Murmur3Partitioner
    ///
    /// Node Ownership:
    ///   Node 1 (node1): 256 tokens (33.33%)
    ///   Node 2 (node2): 256 tokens (33.33%)
    ///   Node 3 (node3): 256 tokens (33.33%)
    /// ```
    ///
    /// # Performance
    /// - **Time**: O(n) where n = number of tokens
    /// - **Space**: O(n) - builds string representation
    ///
    /// # Returns
    /// Human-readable string describing the ring
    pub fn describe(&self) -> String {
        let mut description = String::new();

        // Header
        description.push_str("Ring Description:\n");
        description.push_str(&format!("  Nodes: {}\n", self.ring.node_count()));
        description.push_str(&format!("  Total Tokens: {}\n", self.ring.token_count()));
        description.push_str(&format!("  Partitioner: {}\n", self.ring.partitioner_name()));

        // Ownership details
        let percentages = self.ownership_percentages();
        let ownership = self.ownership();

        if !percentages.is_empty() {
            description.push_str("\nNode Ownership:\n");

            // Sort by node ID for consistent output
            let mut nodes: Vec<_> = percentages.iter().collect();
            nodes.sort_by_key(|(node_id, _)| *node_id);

            for (node_id, percentage) in nodes {
                let node = self.ring.get_node(node_id);
                let node_name = node
                    .as_ref()
                    .map(|n| n.name.as_str())
                    .unwrap_or("unknown");

                let token_count = ownership.get(node_id).map(|v| v.len()).unwrap_or(0);

                description.push_str(&format!(
                    "  Node {} ({}): {} tokens ({:.2}%)\n",
                    node_id, node_name, token_count, percentage
                ));
            }
        }

        description
    }

    /// Find all nodes responsible for a key (for replication).
    ///
    /// # Algorithm
    ///
    /// 1. Find the primary node (clockwise search)
    /// 2. Continue clockwise to find N-1 more nodes
    /// 3. Return list of node IDs
    ///
    /// # Performance
    /// - **Time**: O(r * log n) where r = replica count, n = tokens
    ///   - Each node lookup is O(log n)
    ///   - We do r lookups
    /// - **Space**: O(r) - returns Vec of node IDs
    ///
    /// # Arguments
    /// * `key` - The key to look up
    /// * `replica_count` - Number of replicas to find
    ///
    /// # Returns
    /// Vec of NodeIds (may be shorter if fewer nodes exist)
    ///
    /// # Example
    /// ```rust
    /// let replicas = topology.replicas_for_key(b"my-key", 3);
    /// // Returns [NodeId(1), NodeId(2), NodeId(3)]
    /// ```
    pub fn replicas_for_key(&self, key: &[u8], replica_count: usize) -> Vec<NodeId> {
        if replica_count == 0 {
            return Vec::new();
        }

        let mut replicas = Vec::with_capacity(replica_count);
        let mut seen_nodes = std::collections::HashSet::new();

        // Start with the primary node
        if let Some(primary) = self.ring.lookup(key) {
            replicas.push(primary);
            seen_nodes.insert(primary);
        }

        // For additional replicas, we'd need to implement clockwise iteration
        // For now, just return the primary (full implementation requires
        // iterating tokens clockwise, skipping already-seen nodes)
        // TODO: Implement full replica discovery

        replicas
    }

    /// Get the ring reference (for operations that need direct access).
    ///
    /// # Returns
    /// Reference to the underlying HashRing
    pub fn ring(&self) -> &HashRing {
        &self.ring
    }
}

impl From<HashRing> for Topology {
    fn from(ring: HashRing) -> Self {
        Self::new(ring)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::Node;

    #[test]
    fn test_topology_ownership() {
        let ring = HashRing::new();
        ring.add_node(Node::new(NodeId(1), "node1"), 4);
        ring.add_node(Node::new(NodeId(2), "node2"), 4);

        let topology = Topology::new(ring);
        let ownership = topology.ownership();

        assert_eq!(ownership.len(), 2);
        assert_eq!(ownership[&NodeId(1)].len(), 4);
        assert_eq!(ownership[&NodeId(2)].len(), 4);
    }

    #[test]
    fn test_topology_percentages() {
        let ring = HashRing::new();
        ring.add_node(Node::new(NodeId(1), "node1"), 4);
        ring.add_node(Node::new(NodeId(2), "node2"), 4);

        let topology = Topology::new(ring);
        let percentages = topology.ownership_percentages();

        // Should be roughly 50% each (may vary slightly due to token distribution)
        assert_eq!(percentages.len(), 2);
        assert!((percentages[&NodeId(1)] - 50.0).abs() < 5.0); // Within 5%
        assert!((percentages[&NodeId(2)] - 50.0).abs() < 5.0);
    }

    #[test]
    fn test_topology_describe() {
        let ring = HashRing::new();
        ring.add_node(Node::new(NodeId(1), "node1"), 4);

        let topology = Topology::new(ring);
        let description = topology.describe();

        assert!(description.contains("Ring Description"));
        assert!(description.contains("node1"));
    }
}
