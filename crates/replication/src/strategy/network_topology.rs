//! Network topology-aware replication strategy.

use crate::strategy::ReplicationStrategy;
use corelib::node::NodeId;
use corelib::ring::HashRing;
use std::collections::{HashMap, HashSet};

/// Replication strategy that prefers distinct data centers and racks.
#[derive(Debug, Clone)]
pub struct NetworkTopologyStrategy {
    replication_factor: usize,
    /// Maximum replicas per data center.
    replication_factor_per_dc: usize,
}

impl NetworkTopologyStrategy {
    /// Create a strategy with global and per-DC replication factors.
    pub fn new(replication_factor: usize, replication_factor_per_dc: usize) -> Self {
        Self {
            replication_factor,
            replication_factor_per_dc,
        }
    }

    /// Per-DC replication factor accessor.
    pub fn replication_factor_per_dc(&self) -> usize {
        self.replication_factor_per_dc
    }

    fn select_replicas(
        ring: &HashRing,
        key: &[u8],
        factor: usize,
        per_dc: usize,
    ) -> Vec<NodeId> {
        let max_candidates = ring.node_count().max(factor);
        let candidates = ring.replicas_for_key(key, max_candidates);
        let mut replicas = Vec::with_capacity(factor);
        let mut seen = HashSet::new();
        let mut dc_counts: HashMap<Option<String>, usize> = HashMap::new();
        let mut racks: HashSet<(Option<String>, Option<String>)> = HashSet::new();

        // First pass: prefer spreading across DCs and racks.
        for node_id in &candidates {
            let Some(node) = ring.get_node(node_id) else {
                continue;
            };

            let dc = node.datacenter.clone();
            let dc_count = *dc_counts.get(&dc).unwrap_or(&0);
            if dc_count >= per_dc {
                continue;
            }

            let rack = (dc.clone(), node.rack.clone());
            if racks.contains(&rack) {
                continue;
            }

            seen.insert(*node_id);
            dc_counts.insert(dc, dc_count + 1);
            racks.insert(rack);
            replicas.push(*node_id);

            if replicas.len() >= factor {
                return replicas;
            }
        }

        // Second pass: fill remaining slots without topology constraints.
        for node_id in candidates {
            if seen.insert(node_id) {
                replicas.push(node_id);
                if replicas.len() >= factor {
                    break;
                }
            }
        }

        replicas
    }
}

impl ReplicationStrategy for NetworkTopologyStrategy {
    fn replication_factor(&self) -> usize {
        self.replication_factor
    }

    fn replicas_for_key(&self, ring: &HashRing, key: &[u8]) -> Vec<NodeId> {
        if self.replication_factor == 0 {
            return Vec::new();
        }
        Self::select_replicas(
            ring,
            key,
            self.replication_factor,
            self.replication_factor_per_dc,
        )
    }

    fn name(&self) -> &'static str {
        "NetworkTopologyStrategy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use corelib::node::Node;

    #[test]
    fn test_network_topology_spreads_dcs() {
        let ring = HashRing::new();
        ring.add_node(
            Node::with_topology(NodeId(1), "n1", "dc1", "r1"),
            8,
        );
        ring.add_node(
            Node::with_topology(NodeId(2), "n2", "dc2", "r1"),
            8,
        );
        ring.add_node(
            Node::with_topology(NodeId(3), "n3", "dc1", "r2"),
            8,
        );

        let strategy = NetworkTopologyStrategy::new(2, 1);
        let replicas = strategy.replicas_for_key(&ring, b"key");
        assert_eq!(replicas.len(), 2);
        assert_ne!(replicas[0], replicas[1]);
    }
}
