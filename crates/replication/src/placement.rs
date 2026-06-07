//! Replica placement utilities.

use corelib::node::NodeId;
use corelib::ring::HashRing;

/// Result of replica placement for a key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplicaPlacement {
    /// Primary (coordinator) node — first replica clockwise from the key.
    pub primary: NodeId,
    /// All replica nodes including the primary.
    pub replicas: Vec<NodeId>,
}

impl ReplicaPlacement {
    /// Place replicas for a key on the ring.
    pub fn for_key(ring: &HashRing, key: &[u8], replication_factor: usize) -> Option<Self> {
        let replicas = ring.replicas_for_key(key, replication_factor);
        if replicas.is_empty() {
            return None;
        }

        Some(Self {
            primary: replicas[0],
            replicas,
        })
    }

    /// Number of replicas placed.
    pub fn len(&self) -> usize {
        self.replicas.len()
    }

    /// True if no replicas were placed.
    pub fn is_empty(&self) -> bool {
        self.replicas.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use corelib::node::Node;

    #[test]
    fn test_replica_placement() {
        let ring = HashRing::new();
        ring.add_node(Node::new(NodeId(1), "n1"), 4);
        ring.add_node(Node::new(NodeId(2), "n2"), 4);

        let placement = ReplicaPlacement::for_key(&ring, b"key", 2).unwrap();
        assert_eq!(placement.len(), 2);
        assert_eq!(placement.primary, placement.replicas[0]);
    }
}
