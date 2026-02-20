//! Virtual node abstractions.
//!
//! # Virtual Nodes (VNodes) Concept
//!
//! Virtual nodes are a technique to improve load distribution in consistent hashing.
//! Instead of each physical node having a single token on the ring, each node has
//! multiple tokens (virtual nodes). This provides:
//!
//! 1. **Better Load Distribution**: More tokens = smoother distribution of keys
//! 2. **Gradual Rebalancing**: When nodes join/leave, only a fraction of keys move
//! 3. **Fault Tolerance**: Failure of one node affects fewer keys (distributed across vnodes)
//!
//! # Performance Characteristics
//!
//! - **Memory**: O(v) where v = number of vnodes per node
//! - **Lookup**: O(log n) where n = total vnodes (not affected by vnode count per node)
//! - **Rebalancing**: O(k/v) keys move when a node joins/leaves (k = total keys, v = vnodes/node)
//!
//! # Typical Configuration
//!
//! - **Small clusters** (< 10 nodes): 128-256 vnodes/node
//! - **Medium clusters** (10-100 nodes): 256-512 vnodes/node
//! - **Large clusters** (> 100 nodes): 512-1024 vnodes/node
//!
//! More vnodes = better distribution but more memory and slightly slower operations.

use crate::node::NodeId;
use crate::token::murmur3::Murmur3Token;
use crate::token::Token;

/// A virtual node on the hash ring.
///
/// Represents a single token position owned by a physical node. Each physical
/// node has multiple virtual nodes (typically 256) distributed around the ring.
///
/// # Invariants
///
/// - Every `VirtualNode` has a unique token (no two vnodes share the same token)
/// - Every `VirtualNode` belongs to exactly one physical node
/// - Tokens are ordered (can be sorted/comparable)
///
/// # Memory Layout
///
/// ```
/// VirtualNode {
///     token: Murmur3Token(u64),  // 8 bytes
///     node_id: NodeId(u128),     // 16 bytes
/// }
/// Total: ~24 bytes per vnode
/// ```
///
/// # Example
///
/// ```rust
/// use corelib::{NodeId, VirtualNode};
/// use corelib::token::murmur3::Murmur3Token;
///
/// let vnode = VirtualNode::new(
///     Murmur3Token::from_key("node1:0"),
///     NodeId(1)
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VirtualNode {
    /// Token position on the ring.
    ///
    /// This is the hash of a unique identifier like "node_id:vnode_index".
    /// The token determines where this vnode sits on the ring and which
    /// keys it's responsible for.
    pub token: Murmur3Token,

    /// The physical node that owns this virtual node.
    ///
    /// Multiple virtual nodes can share the same `node_id` (that's the point!).
    /// When looking up a key, we find the vnode's token, then use this
    /// `node_id` to route to the physical node.
    pub node_id: NodeId,
}

impl VirtualNode {
    /// Create a new virtual node.
    ///
    /// # Arguments
    /// * `token` - The token position on the ring
    /// * `node_id` - The physical node that owns this vnode
    ///
    /// # Performance
    /// - **Time**: O(1) - just struct construction
    /// - **Space**: O(1) - 24 bytes
    ///
    /// # Example
    /// ```rust
    /// let vnode = VirtualNode::new(
    ///     Murmur3Token::from_key("node1:0"),
    ///     NodeId(1)
    /// );
    /// ```
    #[inline]
    pub fn new(token: Murmur3Token, node_id: NodeId) -> Self {
        Self { token, node_id }
    }

    /// Create a virtual node from a node ID and vnode index.
    ///
    /// This is a convenience method that generates the token automatically
    /// by hashing "node_id:vnode_index".
    ///
    /// # Algorithm
    ///
    /// 1. Format string: "node_id:vnode_index"
    /// 2. Hash the string to get a token
    /// 3. Create VirtualNode with token and node_id
    ///
    /// # Performance
    /// - **Time**: O(k) where k = length of formatted string (~20-30 bytes)
    ///   - Formatting: O(k)
    ///   - Hashing: O(k) (Murmur3)
    /// - **Space**: O(k) temporary for formatted string
    ///
    /// # Arguments
    /// * `node_id` - The physical node ID
    /// * `vnode_index` - The index of this vnode (0, 1, 2, ...)
    ///
    /// # Returns
    /// A new VirtualNode with a token derived from node_id and index
    ///
    /// # Example
    /// ```rust
    /// // Create vnode #0 for node 1
    /// let vnode0 = VirtualNode::from_index(NodeId(1), 0);
    ///
    /// // Create vnode #1 for node 1
    /// let vnode1 = VirtualNode::from_index(NodeId(1), 1);
    /// ```
    pub fn from_index(node_id: NodeId, vnode_index: usize) -> Self {
        // Generate unique key for this vnode
        // Format: "node_id:vnode_index" ensures uniqueness
        let vnode_key = format!("{}:{}", node_id, vnode_index);
        
        // Hash to get token position
        let token = Murmur3Token::from_key(&vnode_key);
        
        Self::new(token, node_id)
    }

    /// Get the token position.
    ///
    /// # Performance
    /// - **Time**: O(1)
    /// - **Space**: O(1)
    #[inline]
    pub fn token(&self) -> Murmur3Token {
        self.token
    }

    /// Get the owning node ID.
    ///
    /// # Performance
    /// - **Time**: O(1)
    /// - **Space**: O(1)
    #[inline]
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Calculate the distance to another virtual node (clockwise).
    ///
    /// # Algorithm
    ///
    /// Uses the token's `distance_to()` method to calculate clockwise distance.
    /// This is useful for:
    /// - Finding the closest vnode
    /// - Measuring ring gaps
    /// - Load balancing analysis
    ///
    /// # Performance
    /// - **Time**: O(1) - token distance calculation
    /// - **Space**: O(1)
    ///
    /// # Arguments
    /// * `other` - The other virtual node
    ///
    /// # Returns
    /// Distance as a token (can be converted to numeric value)
    ///
    /// # Example
    /// ```rust
    /// let vnode1 = VirtualNode::from_index(NodeId(1), 0);
    /// let vnode2 = VirtualNode::from_index(NodeId(2), 0);
    /// let distance = vnode1.distance_to(&vnode2);
    /// ```
    #[inline]
    pub fn distance_to(&self, other: &Self) -> Murmur3Token {
        self.token.distance_to(&other.token)
    }
}

impl std::fmt::Display for VirtualNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VNode(token={:016x}, node={})", self.token.0, self.node_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vnode_creation() {
        let vnode = VirtualNode::new(Murmur3Token(100), NodeId(1));
        assert_eq!(vnode.token(), Murmur3Token(100));
        assert_eq!(vnode.node_id(), NodeId(1));
    }

    #[test]
    fn test_vnode_from_index() {
        let vnode0 = VirtualNode::from_index(NodeId(1), 0);
        let vnode1 = VirtualNode::from_index(NodeId(1), 1);
        
        // Should have different tokens
        assert_ne!(vnode0.token(), vnode1.token());
        
        // But same node_id
        assert_eq!(vnode0.node_id(), vnode1.node_id());
        assert_eq!(vnode0.node_id(), NodeId(1));
    }

    #[test]
    fn test_vnode_distance() {
        let vnode1 = VirtualNode::new(Murmur3Token(100), NodeId(1));
        let vnode2 = VirtualNode::new(Murmur3Token(200), NodeId(2));
        
        let distance = vnode1.distance_to(&vnode2);
        assert_eq!(distance, Murmur3Token(100)); // 200 - 100
    }

    #[test]
    fn test_vnode_ordering() {
        let vnode1 = VirtualNode::new(Murmur3Token(100), NodeId(1));
        let vnode2 = VirtualNode::new(Murmur3Token(200), NodeId(2));
        
        assert!(vnode1 < vnode2); // Ordered by token
    }
}
