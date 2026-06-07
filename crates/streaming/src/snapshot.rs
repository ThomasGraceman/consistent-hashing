//! Ring snapshot serialization for bootstrap and recovery.

use crate::error::StreamingError;
use corelib::node::{Node, NodeId};
use corelib::ring::HashRing;
use serde::{Deserialize, Serialize};

/// Serializable node entry in a ring snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotNode {
    pub id: u128,
    pub name: String,
    pub datacenter: Option<String>,
    pub rack: Option<String>,
    pub vnodes: usize,
}

impl From<&Node> for SnapshotNode {
    fn from(node: &Node) -> Self {
        Self {
            id: node.id.0,
            name: node.name.clone(),
            datacenter: node.datacenter.clone(),
            rack: node.rack.clone(),
            vnodes: 0,
        }
    }
}

/// Full ring state for streaming bootstrap.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RingSnapshot {
    pub nodes: Vec<SnapshotNode>,
    pub partitioner: String,
}

impl RingSnapshot {
    /// Capture the current ring state.
    pub fn from_ring(ring: &HashRing, vnode_counts: &[(NodeId, usize)]) -> Self {
        let mut nodes: Vec<SnapshotNode> = ring
            .nodes()
            .into_iter()
            .map(|n| {
                let mut snap = SnapshotNode::from(&n);
                if let Some((_, vnodes)) = vnode_counts.iter().find(|(id, _)| *id == n.id) {
                    snap.vnodes = *vnodes;
                }
                snap
            })
            .collect();
        nodes.sort_by_key(|n| n.id);
        Self {
            nodes,
            partitioner: ring.partitioner_name().to_string(),
        }
    }

    /// Serialize to bytes for wire transfer.
    pub fn to_bytes(&self) -> Result<Vec<u8>, StreamingError> {
        bincode::serialize(self).map_err(|e| StreamingError::InvalidSnapshot(e.to_string()))
    }

    /// Deserialize from bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, StreamingError> {
        bincode::deserialize(data).map_err(|e| StreamingError::InvalidSnapshot(e.to_string()))
    }

    /// Rebuild a hash ring from this snapshot.
    pub fn build_ring(&self) -> Result<HashRing, StreamingError> {
        let ring = HashRing::new();
        for entry in &self.nodes {
            if entry.vnodes == 0 {
                return Err(StreamingError::InvalidSnapshot(format!(
                    "node {} missing vnode count",
                    entry.name
                )));
            }
            let node = if let (Some(dc), Some(rack)) = (&entry.datacenter, &entry.rack) {
                Node::with_topology(NodeId(entry.id), entry.name.clone(), dc, rack)
            } else {
                let mut node = Node::new(NodeId(entry.id), entry.name.clone());
                node.datacenter = entry.datacenter.clone();
                node.rack = entry.rack.clone();
                node
            };
            ring.add_node(node, entry.vnodes);
        }
        Ok(ring)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use corelib::node::Node;

    #[test]
    fn snapshot_roundtrip() {
        let ring = HashRing::new();
        ring.add_node(Node::new(NodeId(1), "n1"), 4);
        ring.add_node(Node::new(NodeId(2), "n2"), 4);

        let vnode_counts = vec![(NodeId(1), 4), (NodeId(2), 4)];
        let snap = RingSnapshot::from_ring(&ring, &vnode_counts);
        let bytes = snap.to_bytes().unwrap();
        let restored = RingSnapshot::from_bytes(&bytes).unwrap();
        let rebuilt = restored.build_ring().unwrap();

        assert_eq!(rebuilt.node_count(), 2);
        assert_eq!(rebuilt.token_count(), 8);
    }
}
