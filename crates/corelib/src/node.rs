//! Node abstractions for the consistent hash ring.
//!
//! Nodes represent logical participants in the ring. They are identified by a
//! compact `NodeId` that is cheap to compare and hash.

use std::fmt;

/// Compact identifier for a node in the cluster.
///
/// Newtype over `u128` so comparisons and hashing are very fast while giving
/// plenty of space for uniqueness.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NodeId(pub u128);

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

/// Logical node participating in the ring.
///
/// Keep this struct small and cheap to clone; heavy mutable state (connections,
/// metrics, etc.) should live elsewhere.
#[derive(Clone, Debug)]
pub struct Node {
    pub id: NodeId,
    /// Human‑readable name or hostname.
    pub name: String,
    /// Optional data center label for topology‑aware replication.
    pub datacenter: Option<String>,
    /// Optional rack label for rack‑aware replication.
    pub rack: Option<String>,
}

impl Node {
    /// Construct a new node with basic metadata.
    pub fn new(id: NodeId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            datacenter: None,
            rack: None,
        }
    }

    pub fn with_topology(
        id: NodeId,
        name: impl Into<String>,
        datacenter: impl Into<Option<String>>,
        rack: impl Into<Option<String>>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            datacenter: datacenter.into(),
            rack: rack.into(),
        }
    }
}

