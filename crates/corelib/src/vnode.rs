//! Virtual node abstractions.
//!
//! This module encapsulates virtual node (vnode) concepts used to smooth
//! out token distribution and rebalance the ring.

use crate::node::NodeId;

/// A virtual node on the ring (token + owning node).
#[derive(Debug, Clone)]
pub struct VirtualNode {
    /// Token position (placeholder: will use generic Token when ring is generic).
    pub token_index: u64,
    /// Node that owns this vnode.
    pub node_id: NodeId,
}
