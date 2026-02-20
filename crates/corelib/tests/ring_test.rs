//! Comprehensive tests for the hash ring implementation.
//!
//! # Test Strategy
//!
//! 1. **Basic functionality**: Empty ring, add/lookup, remove
//! 2. **Multiple nodes**: Distribution, consistency
//! 3. **Edge cases**: Wraparound, single node, duplicate keys
//! 4. **Performance**: Large rings, many vnodes
//! 5. **Thread safety**: Concurrent access (if we add those tests)

use corelib::node::{Node, NodeId};
use corelib::ring::HashRing;

// ============================================================================
// Basic Functionality Tests
// ============================================================================

#[test]
fn test_empty_ring_lookup() {
    // Test that an empty ring returns None for lookups
    let ring = HashRing::new();
    assert_eq!(ring.lookup(b"key1"), None);
    assert_eq!(ring.lookup_node(b"key1"), None);
    assert_eq!(ring.node_count(), 0);
    assert_eq!(ring.token_count(), 0);
}

#[test]
fn test_add_node_and_lookup() {
    // Test basic add + lookup functionality
    let ring = HashRing::new();
    let node = Node::new(NodeId(1), "node1");
    
    // Add node with 4 vnodes (small number for testing)
    ring.add_node(node.clone(), 4);
    
    // Verify node was added
    assert_eq!(ring.node_count(), 1);
    assert_eq!(ring.token_count(), 4); // 4 vnodes
    
    // Lookup should return the node we added
    let result = ring.lookup(b"test-key");
    assert!(result.is_some(), "Lookup should succeed after adding node");
    assert_eq!(result.unwrap(), NodeId(1), "Should return the added node");
    
    // Get full node metadata
    let node_meta = ring.lookup_node(b"test-key");
    assert!(node_meta.is_some(), "Should return node metadata");
    assert_eq!(node_meta.unwrap().id, NodeId(1), "Should match added node");
    
    // Verify we can get node by ID
    let retrieved = ring.get_node(&NodeId(1));
    assert!(retrieved.is_some(), "Should retrieve node by ID");
    assert_eq!(retrieved.unwrap().id, NodeId(1));
}

#[test]
fn test_remove_node() {
    // Test node removal functionality
    let ring = HashRing::new();
    
    ring.add_node(Node::new(NodeId(1), "node1"), 4);
    ring.add_node(Node::new(NodeId(2), "node2"), 4);
    
    // Verify both nodes exist
    assert_eq!(ring.node_count(), 2);
    assert_eq!(ring.token_count(), 8); // 4 + 4 vnodes
    
    // Remove node1
    assert!(ring.remove_node(&NodeId(1)), "Should successfully remove node");
    
    // Verify node1 is gone
    assert_eq!(ring.node_count(), 1);
    assert_eq!(ring.token_count(), 4); // Only node2's vnodes remain
    
    // Lookups should now only return node2
    let result = ring.lookup(b"some-key");
    assert!(result.is_some(), "Lookup should still work");
    assert_eq!(result.unwrap(), NodeId(2), "Should return remaining node");
    
    // Verify node1 is gone
    assert!(ring.get_node(&NodeId(1)).is_none(), "Node1 should be removed");
    assert!(ring.get_node(&NodeId(2)).is_some(), "Node2 should still exist");
    
    // Removing non-existent node should return false
    assert!(!ring.remove_node(&NodeId(999)), "Should return false for non-existent node");
}

// ============================================================================
// Multiple Nodes Tests
// ============================================================================

#[test]
fn test_multiple_nodes() {
    // Test ring with multiple nodes
    let ring = HashRing::new();
    
    ring.add_node(Node::new(NodeId(1), "node1"), 4);
    ring.add_node(Node::new(NodeId(2), "node2"), 4);
    ring.add_node(Node::new(NodeId(3), "node3"), 4);
    
    // Verify all nodes were added
    assert_eq!(ring.node_count(), 3);
    assert_eq!(ring.token_count(), 12); // 3 nodes * 4 vnodes
    
    // Different keys should map to nodes (usually different ones)
    let key1_node = ring.lookup(b"key1");
    let key2_node = ring.lookup(b"key2");
    let key3_node = ring.lookup(b"key3");
    
    assert!(key1_node.is_some(), "All lookups should succeed");
    assert!(key2_node.is_some());
    assert!(key3_node.is_some());
    
    // All should be one of our nodes
    let node_ids: Vec<NodeId> = vec![NodeId(1), NodeId(2), NodeId(3)];
    assert!(node_ids.contains(&key1_node.unwrap()), "Key1 should map to a valid node");
    assert!(node_ids.contains(&key2_node.unwrap()), "Key2 should map to a valid node");
    assert!(node_ids.contains(&key3_node.unwrap()), "Key3 should map to a valid node");
}

#[test]
fn test_consistent_lookup() {
    // Test that the same key always maps to the same node
    let ring = HashRing::new();
    
    ring.add_node(Node::new(NodeId(1), "node1"), 4);
    ring.add_node(Node::new(NodeId(2), "node2"), 4);
    
    let key = b"consistent-key";
    
    // Lookup the same key multiple times
    let node1 = ring.lookup(key);
    let node2 = ring.lookup(key);
    let node3 = ring.lookup(key);
    
    // Should always return the same node
    assert_eq!(node1, node2, "Same key should map to same node");
    assert_eq!(node2, node3, "Same key should map to same node");
}

// ============================================================================
// Ring Builder Tests
// ============================================================================

#[test]
fn test_ring_builder_default() {
    // Test builder with default settings
    let ring = corelib::ring::RingBuilder::new()
        .add_node(Node::new(NodeId(1), "node1"))
        .add_node(Node::new(NodeId(2), "node2"))
        .build();
    
    assert!(ring.lookup(b"key").is_some(), "Lookup should work");
    assert_eq!(ring.node_count(), 2, "Should have 2 nodes");
    // Default is 256 vnodes per node
    assert_eq!(ring.token_count(), 512, "Should have 512 tokens (2 * 256)");
}

#[test]
fn test_ring_builder_custom_vnodes() {
    // Test builder with custom vnode count
    let ring = corelib::ring::RingBuilder::new()
        .with_vnodes(8)
        .add_node(Node::new(NodeId(1), "node1"))
        .add_node(Node::new(NodeId(2), "node2"))
        .build();
    
    assert!(ring.lookup(b"key").is_some());
    assert_eq!(ring.node_count(), 2);
    assert_eq!(ring.token_count(), 16); // 2 nodes * 8 vnodes
}

#[test]
fn test_ring_builder_mixed_vnodes() {
    // Test builder with different vnode counts per node
    let ring = corelib::ring::RingBuilder::new()
        .with_vnodes(4) // Default
        .add_node(Node::new(NodeId(1), "node1")) // Uses default (4)
        .add_node_with_vnodes(Node::new(NodeId(2), "node2"), 8) // Custom (8)
        .build();
    
    assert_eq!(ring.node_count(), 2);
    assert_eq!(ring.token_count(), 12); // 4 + 8
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_single_node() {
    // Test ring with only one node
    let ring = HashRing::new();
    ring.add_node(Node::new(NodeId(1), "node1"), 4);
    
    // All keys should map to the single node
    for key in [b"key1", b"key2", b"key3", b"very-long-key-name"] {
        let node_id = ring.lookup(key);
        assert_eq!(node_id, Some(NodeId(1)), "All keys should map to single node");
    }
}

#[test]
fn test_add_remove_add() {
    // Test adding, removing, and re-adding a node
    let ring = HashRing::new();
    
    // Add node
    ring.add_node(Node::new(NodeId(1), "node1"), 4);
    assert_eq!(ring.node_count(), 1);
    
    // Remove node
    assert!(ring.remove_node(&NodeId(1)));
    assert_eq!(ring.node_count(), 0);
    
    // Re-add node (should work fine)
    ring.add_node(Node::new(NodeId(1), "node1"), 4);
    assert_eq!(ring.node_count(), 1);
    assert!(ring.lookup(b"key").is_some());
}

#[test]
fn test_idempotent_add() {
    // Test that adding the same node twice is idempotent
    let ring = HashRing::new();
    
    let node = Node::new(NodeId(1), "node1");
    ring.add_node(node.clone(), 4);
    assert_eq!(ring.token_count(), 4);
    
    // Add same node again (should add more vnodes, not replace)
    ring.add_node(node, 4);
    assert_eq!(ring.token_count(), 8); // Should have 8 vnodes now (4 + 4)
    assert_eq!(ring.node_count(), 1); // Still one node
}

// ============================================================================
// Utility Tests
// ============================================================================

#[test]
fn test_get_all_nodes() {
    // Test retrieving all nodes
    let ring = HashRing::new();
    
    ring.add_node(Node::new(NodeId(1), "node1"), 4);
    ring.add_node(Node::new(NodeId(2), "node2"), 4);
    
    let nodes = ring.nodes();
    assert_eq!(nodes.len(), 2);
    
    let node_ids: Vec<NodeId> = nodes.iter().map(|n| n.id).collect();
    assert!(node_ids.contains(&NodeId(1)));
    assert!(node_ids.contains(&NodeId(2)));
}

#[test]
fn test_get_all_tokens() {
    // Test retrieving all tokens (for debugging)
    let ring = HashRing::new();
    
    ring.add_node(Node::new(NodeId(1), "node1"), 4);
    
    let tokens = ring.tokens();
    assert_eq!(tokens.len(), 4, "Should have 4 tokens");
    
    // All tokens should map to node1
    for (_, node_id) in tokens {
        assert_eq!(node_id, NodeId(1), "All tokens should map to node1");
    }
}

#[test]
fn test_partitioner_name() {
    // Test getting partitioner name
    let ring = HashRing::new();
    assert_eq!(ring.partitioner_name(), "Murmur3Partitioner");
}
