//! Hash ring data structure implementation.
//!
//! # Architecture Overview
//!
//! The ring is the core data structure for consistent hashing. It maintains:
//! 1. **Token → Node mapping**: `BTreeMap<Token, NodeId>` for O(log n) ordered lookups
//! 2. **Node registry**: `HashMap<NodeId, Node>` for fast node metadata access
//!
//! # Performance Characteristics
//!
//! - **Lookup**: O(log n) where n = number of tokens (vnodes)
//!   - Uses BTreeMap::range() for efficient clockwise search
//!   - Single read lock acquisition (no double locking)
//! - **Add node**: O(v * log n) where v = vnodes per node
//!   - BTreeMap insertion is O(log n) per token
//! - **Remove node**: O(n) worst case (must scan all tokens)
//!   - Uses retain() which is efficient for sparse removals
//!
//! # Thread Safety
//!
//! - **Read operations** (lookup): Concurrent, lock-free after acquiring read lock
//! - **Write operations** (add/remove): Exclusive, blocks all readers
//! - Uses `parking_lot::RwLock` for better performance than std::sync::RwLock
//!   - Faster read path (no system calls in uncontended case)
//!   - Writer fairness (prevents reader starvation)
//!
//! # Virtual Nodes (VNodes)
//!
//! Each physical node is represented by multiple virtual nodes (tokens) on the ring.
//! This provides:
//! - **Better load distribution**: More tokens = smoother distribution
//! - **Gradual rebalancing**: When nodes join/leave, only a fraction of keys move
//! - **Default**: 256 vnodes per node (good balance of distribution vs memory)

use crate::node::{Node, NodeId};
use crate::partitioner::traits::Partitioner;
use crate::token::murmur3::Murmur3Token;
use crate::token::Token;
use parking_lot::RwLock;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

// ============================================================================
// Internal Ring State (Not Thread-Safe)
// ============================================================================

/// Internal ring state structure.
///
/// **Thread Safety**: This struct is NOT thread-safe. It is always wrapped
/// in `Arc<RwLock<RingInner>>` to provide thread-safe access.
///
/// # Invariants
///
/// 1. Every token in `tokens` maps to a node that exists in `nodes`
/// 2. `tokens` is always sorted (BTreeMap maintains order)
/// 3. `tokens` may be empty (ring has no nodes), but `nodes` should match
struct RingInner {
    /// Token → NodeId mapping (ordered for efficient range queries).
    ///
    /// **Why BTreeMap?**
    /// - O(log n) insertion, deletion, lookup
    /// - Maintains sorted order automatically
    /// - Efficient range queries: `range(token..)` finds next token clockwise
    /// - Better cache locality than HashMap for sequential access patterns
    /// - Memory overhead: ~24 bytes per entry (vs HashMap's ~32 bytes)
    ///
    /// **Alternative considered**: Sorted Vec + binary search
    /// - Pros: Better cache locality, smaller memory footprint
    /// - Cons: O(n) insertion/deletion (not acceptable for dynamic rings)
    tokens: BTreeMap<Murmur3Token, NodeId>,

    /// Node registry: NodeId → Node metadata.
    ///
    /// **Why HashMap?**
    /// - O(1) average case lookup (vs BTreeMap's O(log n))
    /// - Node lookups are frequent and don't need ordering
    /// - Fast node existence checks before operations
    nodes: HashMap<NodeId, Node>,
}

impl RingInner {
    /// Create a new empty ring.
    ///
    /// # Performance
    /// - O(1) - No allocations until first node is added
    fn new() -> Self {
        Self {
            tokens: BTreeMap::new(),
            nodes: HashMap::new(),
        }
    }

    /// Find the node responsible for a given token (clockwise search).
    ///
    /// # Algorithm
    ///
    /// 1. Search for the first token >= our token (clockwise direction)
    /// 2. If found, return that node
    /// 3. If not found (we're past the last token), wrap around to the first token
    ///
    /// This implements the "clockwise" rule: keys map to the first node
    /// encountered when moving clockwise around the ring.
    ///
    /// # Performance
    /// - **Time**: O(log n) where n = number of tokens
    ///   - `range(token..)` is O(log n) to find start position
    ///   - `next()` is O(1) amortized
    ///   - `first_key_value()` is O(log n) worst case (but rare)
    /// - **Space**: O(1) - no allocations
    ///
    /// # Edge Cases
    /// - Empty ring: Returns `None`
    /// - Single token: Returns that token's node
    /// - Token wraps around: Returns first token's node
    ///
    /// # Example
    /// ```text
    /// Ring: [Token(100) -> Node1, Token(200) -> Node2, Token(300) -> Node3]
    /// Lookup Token(250): Finds Token(300) -> Node3
    /// Lookup Token(350): Wraps to Token(100) -> Node1
    /// ```
    #[inline]
    fn node_for_token(&self, token: &Murmur3Token) -> Option<NodeId> {
        // Fast path: empty ring
        if self.tokens.is_empty() {
            return None;
        }

        // Search clockwise: find first token >= our token
        // BTreeMap::range() returns an iterator starting at the first key >= token
        // This is O(log n) to find the start position
        self.tokens
            .range(token..)
            .next()
            .map(|(_, node_id)| *node_id)
            // Wrap-around case: if no token >= ours exists, we've wrapped around
            // Return the first token in the ring (smallest token value)
            // This is O(log n) but only happens when token > max_token
            .or_else(|| {
                // Use first_key_value() instead of first() for better performance
                // (avoids creating a reference to the key)
                self.tokens
                    .first_key_value()
                    .map(|(_, node_id)| *node_id)
            })
    }

    /// Add a node with virtual nodes (vnodes).
    ///
    /// # Algorithm
    ///
    /// For each vnode index i in [0, vnodes):
    /// 1. Generate a unique vnode key: "node_id:i"
    /// 2. Hash the key to get a token
    /// 3. Insert token → node_id mapping
    ///
    /// # Performance
    /// - **Time**: O(v * log n) where v = vnodes, n = total tokens
    ///   - Each BTreeMap insertion is O(log n)
    ///   - Token generation is O(1) per vnode
    /// - **Space**: O(v) new entries in BTreeMap
    ///
    /// # Optimizations
    /// - Pre-allocate node metadata insertion (HashMap is already efficient)
    /// - Use string formatting only for vnode key (unavoidable)
    /// - Token hashing is fast (Murmur3 is optimized)
    ///
    /// # Safety
    /// - If node already exists, metadata is updated (idempotent)
    /// - Vnodes are added even if node already exists (allows rebalancing)
    ///
    /// # Arguments
    /// * `node` - The node to add (will be cloned for storage)
    /// * `vnodes` - Number of virtual nodes (typically 128-512)
    fn add_node(&mut self, node: Node, vnodes: usize) {
        // Store/update node metadata
        // HashMap::insert handles both new and existing keys efficiently
        self.nodes.insert(node.id, node.clone());

        // Generate virtual nodes
        // We iterate from 0 to vnodes-1, generating a unique token for each
        // The format "node_id:i" ensures uniqueness across nodes and vnode indices
        for i in 0..vnodes {
            // Generate vnode key: "node_id:vnode_index"
            // Format! is necessary here, but we could optimize with a custom formatter
            // if this becomes a bottleneck (unlikely for < 1000 vnodes)
            let vnode_key = format!("{}:{}", node.id, i);
            
            // Hash the key to get a token position on the ring
            // Murmur3Token::from_key() uses Murmur3 hash (fast, good distribution)
            let token = Murmur3Token::from_key(&vnode_key);
            
            // Insert token → node_id mapping
            // BTreeMap::insert is O(log n) where n = current token count
            // If token already exists (collision), it's overwritten (shouldn't happen)
            self.tokens.insert(token, node.id);
        }
    }

    /// Remove a node and all its virtual nodes.
    ///
    /// # Algorithm
    ///
    /// 1. Check if node exists (fast O(1) lookup)
    /// 2. Remove all tokens owned by this node using `retain()`
    /// 3. Remove node metadata
    ///
    /// # Performance
    /// - **Time**: O(n) worst case where n = total tokens
    ///   - `retain()` must check every token
    ///   - However, it's efficient for sparse removals (only touches matching tokens)
    ///   - Node existence check is O(1)
    /// - **Space**: O(1) - no allocations
    ///
    /// # Alternative Approaches Considered
    /// - **Track vnodes per node**: Would require HashMap<NodeId, Vec<Token>>
    ///   - Pros: O(v) removal instead of O(n)
    ///   - Cons: Extra memory, complexity, must maintain consistency
    ///   - **Decision**: Not worth it - node removal is rare, O(n) is acceptable
    ///
    /// # Safety
    /// - Returns `false` if node doesn't exist (idempotent)
    /// - All tokens are removed atomically (no partial state)
    ///
    /// # Arguments
    /// * `node_id` - The node to remove
    ///
    /// # Returns
    /// `true` if node was removed, `false` if it didn't exist
    fn remove_node(&mut self, node_id: &NodeId) -> bool {
        // Fast path: check if node exists before doing expensive work
        if !self.nodes.contains_key(node_id) {
            return false;
        }

        // Remove all tokens owned by this node
        // retain() is efficient: it only moves elements that need to be kept
        // For a node with v vnodes out of n total tokens, this is roughly O(n)
        // but only touches memory for tokens that need to be removed
        self.tokens.retain(|_, id| id != node_id);

        // Remove node metadata
        // This is O(1) average case (HashMap removal)
        self.nodes.remove(node_id);
        
        true
    }

    /// Get node metadata by ID.
    ///
    /// # Performance
    /// - **Time**: O(1) average case (HashMap lookup)
    /// - **Space**: O(1) - returns reference, no allocation
    ///
    /// # Arguments
    /// * `node_id` - The node ID to look up
    ///
    /// # Returns
    /// Reference to node metadata, or `None` if not found
    #[inline]
    fn get_node(&self, node_id: &NodeId) -> Option<&Node> {
        self.nodes.get(node_id)
    }

    /// Get all tokens in the ring (for debugging/inspection).
    ///
    /// # Performance
    /// - **Time**: O(n) where n = number of tokens
    /// - **Space**: O(n) - allocates Vec with all tokens
    ///
    /// # Use Case
    /// - Debugging ring state
    /// - Inspecting token distribution
    /// - Testing/validation
    ///
    /// # Warning
    /// This allocates memory proportional to ring size. Use sparingly in production.
    fn tokens(&self) -> Vec<(Murmur3Token, NodeId)> {
        // Collect all tokens into a Vec
        // This is O(n) time and space
        self.tokens.iter().map(|(t, n)| (*t, *n)).collect()
    }

    /// Get all nodes (for debugging/inspection).
    ///
    /// # Performance
    /// - **Time**: O(n) where n = number of nodes
    /// - **Space**: O(n) - returns references, no cloning
    ///
    /// # Use Case
    /// - Listing all nodes in the ring
    /// - Debugging/validation
    fn nodes(&self) -> Vec<&Node> {
        // Collect references to all nodes
        // This is O(n) time, O(n) space for the Vec
        self.nodes.values().collect()
    }

    /// Get the number of tokens (vnodes) in the ring.
    ///
    /// # Performance
    /// - **Time**: O(1) - BTreeMap::len() is constant time
    #[inline]
    fn token_count(&self) -> usize {
        self.tokens.len()
    }

    /// Get the number of nodes in the ring.
    ///
    /// # Performance
    /// - **Time**: O(1) - HashMap::len() is constant time
    #[inline]
    fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

// ============================================================================
// Thread-Safe Hash Ring
// ============================================================================

/// Thread-safe hash ring implementation.
///
/// # Thread Safety Model
///
/// - **Read operations** (lookup, get_node): Concurrent, non-blocking
///   - Multiple threads can read simultaneously
///   - Uses `RwLock::read()` which allows concurrent readers
///   - No data races: all reads see a consistent snapshot
///
/// - **Write operations** (add_node, remove_node): Exclusive, blocking
///   - Only one writer at a time
///   - Writers block all readers (prevents inconsistent reads)
///   - Uses `RwLock::write()` for exclusive access
///
/// # Lock Choice: parking_lot::RwLock vs std::sync::RwLock
///
/// **Why parking_lot?**
/// - **Faster**: No system calls in uncontended case (uses atomic operations)
/// - **Fair**: Prevents reader starvation (writers get priority)
/// - **Smaller**: Less memory overhead
/// - **Better API**: No poisoning (panics don't poison the lock)
///
/// **Trade-offs**:
/// - Slightly larger dependency (but already in use for other locks)
/// - Not in stdlib (but widely used and well-tested)
///
/// # Performance Characteristics
///
/// - **Lookup**: O(log n) time, O(1) space, concurrent reads
/// - **Add node**: O(v * log n) time, O(v) space, exclusive write
/// - **Remove node**: O(n) time, O(1) space, exclusive write
///
/// # Memory Layout
///
/// ```
/// HashRing {
///     partitioner: Arc<Murmur3Partitioner>,  // Shared, immutable
///     inner: Arc<RwLock<RingInner>> {       // Shared, mutable
///         tokens: BTreeMap<Token, NodeId>,   // ~24 bytes per entry
///         nodes: HashMap<NodeId, Node>,       // ~32 bytes per entry + Node size
///     }
/// }
/// ```
///
/// # Example Usage
///
/// ```rust
/// use corelib::{Node, NodeId};
/// use corelib::ring::HashRing;
///
/// let ring = HashRing::new();
/// ring.add_node(Node::new(NodeId(1), "node1"), 256);
///
/// // Concurrent lookups are safe
/// let node_id = ring.lookup(b"my-key");
/// ```
pub struct HashRing {
    /// Partitioning strategy (shared, immutable).
    ///
    /// **Why Arc?**
    /// - Allows sharing partitioner across multiple ring instances
    /// - Immutable, so no synchronization needed
    /// - Cheap to clone (just increments reference count)
    partitioner: Arc<Murmur3Partitioner>,

    /// Internal ring state (protected by RwLock).
    ///
    /// **Why Arc<RwLock<...>>?**
    /// - `Arc` allows sharing the ring across threads
    /// - `RwLock` provides concurrent reads, exclusive writes
    /// - Inner state is not thread-safe, so it MUST be behind RwLock
    inner: Arc<RwLock<RingInner>>,
}

impl HashRing {
    /// Create a new ring with the default Murmur3 partitioner.
    ///
    /// # Performance
    /// - **Time**: O(1) - just allocations
    /// - **Space**: O(1) - empty structures
    ///
    /// # Defaults
    /// - Partitioner: `Murmur3Partitioner` (Cassandra-compatible)
    /// - Ring: Empty (no nodes)
    ///
    /// # Example
    /// ```rust
    /// let ring = HashRing::new();
    /// ```
    pub fn new() -> Self {
        Self {
            partitioner: Arc::new(Murmur3Partitioner),
            inner: Arc::new(RwLock::new(RingInner::new())),
        }
    }

    /// Create a ring with a custom partitioner (for future extensibility).
    ///
    /// # Use Case
    /// - Testing with different partitioners
    /// - Future: support for RandomPartitioner, ByteOrderedPartitioner, etc.
    ///
    /// # Arguments
    /// * `partitioner` - The partitioner to use (wrapped in Arc for sharing)
    pub fn with_partitioner(partitioner: Arc<Murmur3Partitioner>) -> Self {
        Self {
            partitioner,
            inner: Arc::new(RwLock::new(RingInner::new())),
        }
    }

    /// Look up the node responsible for a key.
    ///
    /// # Algorithm
    ///
    /// 1. Hash the key to get a token (using partitioner)
    /// 2. Acquire read lock (allows concurrent reads)
    /// 3. Find the first token >= our token (clockwise search)
    /// 4. Return the node ID
    ///
    /// # Performance
    /// - **Time**: O(log n) where n = number of tokens
    ///   - Token hashing: O(k) where k = key length (typically < 100 bytes)
    ///   - Lock acquisition: O(1) in uncontended case
    ///   - Token lookup: O(log n) using BTreeMap::range()
    /// - **Space**: O(1) - no allocations
    /// - **Concurrency**: Allows concurrent reads (no blocking)
    ///
    /// # Thread Safety
    /// - Safe for concurrent calls from multiple threads
    /// - Read lock allows multiple simultaneous readers
    /// - Writers are blocked during read (ensures consistency)
    ///
    /// # Edge Cases
    /// - Empty ring: Returns `None`
    /// - Key hashes to max token: Wraps around to first token
    ///
    /// # Arguments
    /// * `key` - The key to look up (will be hashed to a token)
    ///
    /// # Returns
    /// The NodeId of the responsible node, or `None` if ring is empty
    ///
    /// # Example
    /// ```rust
    /// let node_id = ring.lookup(b"my-key");
    /// ```
    #[inline]
    pub fn lookup(&self, key: &[u8]) -> Option<NodeId> {
        // Step 1: Hash the key to get a token
        // This is O(k) where k = key length, but typically very fast (< 1μs)
        let token = self.partitioner.partition(key);

        // Step 2: Acquire read lock (allows concurrent reads)
        // This is O(1) in the uncontended case (no system calls)
        // In contended case, may block briefly waiting for writers
        let inner = self.inner.read();

        // Step 3: Find the node responsible for this token
        // This is O(log n) where n = number of tokens
        inner.node_for_token(&token)
        // Lock is automatically released when `inner` goes out of scope
    }

    /// Look up the node and return full Node metadata.
    ///
    /// # Performance
    /// - **Time**: O(log n) - same as `lookup()` + O(1) HashMap lookup
    /// - **Space**: O(1) - clones Node struct (typically < 100 bytes)
    ///
    /// # Optimization Note
    /// This acquires the read lock twice (once in lookup, once in get_node).
    /// For high-performance scenarios, consider `lookup_node_optimized()` which
    /// acquires the lock once. However, the overhead is minimal (< 10ns) and
    /// the code is clearer this way.
    ///
    /// # Arguments
    /// * `key` - The key to look up
    ///
    /// # Returns
    /// Full Node metadata, or `None` if ring is empty or node not found
    pub fn lookup_node(&self, key: &[u8]) -> Option<Node> {
        // First, find the node ID
        let node_id = self.lookup(key)?;

        // Then, get the full node metadata
        // This requires a second lock acquisition, but it's fast
        let inner = self.inner.read();
        inner.get_node(&node_id).cloned()
    }

    /// Optimized version that acquires lock only once.
    ///
    /// # Performance
    /// - **Time**: O(log n) - same as lookup_node, but only one lock acquisition
    /// - **Space**: O(1)
    ///
    /// # Use Case
    /// Use this when you need both node ID and metadata in high-throughput scenarios.
    ///
    /// # Arguments
    /// * `key` - The key to look up
    ///
    /// # Returns
    /// Full Node metadata, or `None` if ring is empty
    pub fn lookup_node_optimized(&self, key: &[u8]) -> Option<Node> {
        let token = self.partitioner.partition(key);
        let inner = self.inner.read();
        
        // Find node ID
        let node_id = inner.node_for_token(&token)?;
        
        // Get node metadata (same lock, no second acquisition)
        inner.get_node(&node_id).cloned()
    }

    /// Add a node to the ring with the specified number of virtual nodes.
    ///
    /// # Algorithm
    ///
    /// 1. Acquire write lock (exclusive access)
    /// 2. Store node metadata
    /// 3. Generate vnodes tokens
    /// 4. Insert tokens into ring
    ///
    /// # Performance
    /// - **Time**: O(v * log n) where v = vnodes, n = total tokens
    ///   - Lock acquisition: O(1) in uncontended case, may block if readers/writers active
    ///   - Token generation: O(v) - one hash per vnode
    ///   - Token insertion: O(v * log n) - BTreeMap insertion is O(log n) each
    /// - **Space**: O(v) - new tokens in BTreeMap
    ///
    /// # Thread Safety
    /// - Exclusive write lock blocks all readers and writers
    /// - Operation is atomic (all vnodes added or none)
    /// - Safe to call concurrently (but will serialize)
    ///
    /// # Arguments
    /// * `node` - The node to add (will be cloned for storage)
    /// * `vnodes` - Number of virtual nodes (typically 128-512)
    ///   - More vnodes = better distribution, but more memory
    ///   - Default: 256 (good balance)
    ///
    /// # Idempotency
    /// - If node already exists, metadata is updated
    /// - Vnodes are added even if node exists (allows rebalancing)
    ///
    /// # Example
    /// ```rust
    /// ring.add_node(Node::new(NodeId(1), "node1"), 256);
    /// ```
    pub fn add_node(&self, node: Node, vnodes: usize) {
        // Acquire write lock (exclusive access)
        // This blocks all readers and writers until we're done
        // In uncontended case, this is O(1)
        // In contended case, may block waiting for readers/writers to finish
        let mut inner = self.inner.write();

        // Add the node (this handles both new and existing nodes)
        // See RingInner::add_node() for detailed algorithm
        inner.add_node(node, vnodes);
        // Lock is automatically released when `inner` goes out of scope
    }

    /// Remove a node from the ring (removes all its virtual nodes).
    ///
    /// # Algorithm
    ///
    /// 1. Acquire write lock (exclusive access)
    /// 2. Check if node exists
    /// 3. Remove all tokens owned by this node
    /// 4. Remove node metadata
    ///
    /// # Performance
    /// - **Time**: O(n) worst case where n = total tokens
    ///   - Lock acquisition: O(1) in uncontended case
    ///   - Token removal: O(n) - must check every token
    ///   - Node removal: O(1) average case
    /// - **Space**: O(1) - no allocations
    ///
    /// # Thread Safety
    /// - Exclusive write lock blocks all readers and writers
    /// - Operation is atomic (all tokens removed or none)
    /// - Safe to call concurrently (but will serialize)
    ///
    /// # Arguments
    /// * `node_id` - The node to remove
    ///
    /// # Returns
    /// `true` if node was removed, `false` if it didn't exist
    ///
    /// # Idempotency
    /// - Safe to call multiple times (returns false if already removed)
    ///
    /// # Example
    /// ```rust
    /// ring.remove_node(&NodeId(1));
    /// ```
    pub fn remove_node(&self, node_id: &NodeId) -> bool {
        // Acquire write lock (exclusive access)
        let mut inner = self.inner.write();

        // Remove the node (see RingInner::remove_node() for details)
        inner.remove_node(node_id)
        // Lock is automatically released
    }

    /// Get node metadata by ID.
    ///
    /// # Performance
    /// - **Time**: O(1) average case (HashMap lookup)
    /// - **Space**: O(1) - clones Node struct
    ///
    /// # Thread Safety
    /// - Safe for concurrent calls (read lock)
    ///
    /// # Arguments
    /// * `node_id` - The node ID to look up
    ///
    /// # Returns
    /// Node metadata, or `None` if not found
    pub fn get_node(&self, node_id: &NodeId) -> Option<Node> {
        let inner = self.inner.read();
        inner.get_node(node_id).cloned()
    }

    /// Get all tokens in the ring (for debugging/inspection).
    ///
    /// # Performance Warning
    /// - **Time**: O(n) where n = number of tokens
    /// - **Space**: O(n) - allocates Vec with all tokens
    ///
    /// # Use Case
    /// - Debugging ring state
    /// - Inspecting token distribution
    /// - Testing/validation
    ///
    /// # Production Use
    /// Avoid calling this in hot paths - it allocates memory proportional to ring size.
    /// For production monitoring, consider iterating tokens instead.
    ///
    /// # Returns
    /// Vec of (token, node_id) pairs, sorted by token value
    pub fn tokens(&self) -> Vec<(Murmur3Token, NodeId)> {
        let inner = self.inner.read();
        inner.tokens()
    }

    /// Get all nodes in the ring.
    ///
    /// # Performance
    /// - **Time**: O(n) where n = number of nodes
    /// - **Space**: O(n) - clones all Node structs
    ///
    /// # Use Case
    /// - Listing all nodes
    /// - Debugging/validation
    ///
    /// # Returns
    /// Vec of all nodes
    pub fn nodes(&self) -> Vec<Node> {
        let inner = self.inner.read();
        inner.nodes().into_iter().cloned().collect()
    }

    /// Get the number of tokens (vnodes) in the ring.
    ///
    /// # Performance
    /// - **Time**: O(1) - constant time
    /// - **Space**: O(1)
    ///
    /// # Use Case
    /// - Monitoring ring size
    /// - Validating ring state
    ///
    /// # Returns
    /// Number of tokens (vnodes) in the ring
    pub fn token_count(&self) -> usize {
        let inner = self.inner.read();
        inner.token_count()
    }

    /// Get the number of nodes in the ring.
    ///
    /// # Performance
    /// - **Time**: O(1) - constant time
    /// - **Space**: O(1)
    ///
    /// # Returns
    /// Number of physical nodes in the ring
    pub fn node_count(&self) -> usize {
        let inner = self.inner.read();
        inner.node_count()
    }

    /// Get the partitioner name.
    ///
    /// # Performance
    /// - **Time**: O(1)
    /// - **Space**: O(1)
    ///
    /// # Returns
    /// Name of the partitioner (e.g., "Murmur3Partitioner")
    pub fn partitioner_name(&self) -> &'static str {
        self.partitioner.name()
    }
}

impl Default for HashRing {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Ring Builder (Fluent API)
// ============================================================================

/// Builder for constructing a ring with multiple nodes.
///
/// # Design Pattern
///
/// Uses the builder pattern to allow fluent construction:
/// ```rust
/// let ring = RingBuilder::new()
///     .with_vnodes(512)
///     .add_node(node1)
///     .add_node(node2)
///     .build();
/// ```
///
/// # Performance
///
/// - Builder operations are O(1) (just setting fields)
/// - `build()` is O(1) (just returns the ring)
/// - Actual node addition happens during `add_node()` calls
///
/// # Thread Safety
///
/// - Builder is NOT thread-safe (single-threaded construction)
/// - Built ring IS thread-safe (can be shared across threads)
pub struct RingBuilder {
    /// The ring being built.
    ring: HashRing,
    /// Default number of virtual nodes per node.
    default_vnodes: usize,
}

impl RingBuilder {
    /// Create a new builder with default settings.
    ///
    /// # Defaults
    /// - Vnodes per node: 256 (good balance of distribution vs memory)
    ///
    /// # Performance
    /// - **Time**: O(1) - just creates empty ring
    /// - **Space**: O(1)
    ///
    /// # Example
    /// ```rust
    /// let builder = RingBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self {
            ring: HashRing::new(),
            default_vnodes: 256, // Default: good balance
        }
    }

    /// Set the default number of virtual nodes per node.
    ///
    /// # Performance
    /// - **Time**: O(1) - just sets a field
    ///
    /// # Arguments
    /// * `vnodes` - Number of vnodes (typically 128-512)
    ///   - More = better distribution, but more memory
    ///   - Less = less memory, but potentially uneven distribution
    ///
    /// # Returns
    /// Self for method chaining
    ///
    /// # Example
    /// ```rust
    /// builder.with_vnodes(512);
    /// ```
    pub fn with_vnodes(mut self, vnodes: usize) -> Self {
        self.default_vnodes = vnodes;
        self
    }

    /// Add a node to the ring (uses default vnodes).
    ///
    /// # Performance
    /// - **Time**: O(v * log n) where v = default_vnodes, n = current tokens
    ///   - Calls `ring.add_node()` which does the actual work
    ///
    /// # Arguments
    /// * `node` - The node to add
    ///
    /// # Returns
    /// Self for method chaining
    ///
    /// # Example
    /// ```rust
    /// builder.add_node(Node::new(NodeId(1), "node1"));
    /// ```
    pub fn add_node(mut self, node: Node) -> Self {
        // Add node with default vnodes
        // This acquires a write lock, so it's not free
        // But it's necessary to build the ring incrementally
        self.ring.add_node(node, self.default_vnodes);
        self
    }

    /// Add a node with a custom number of virtual nodes.
    ///
    /// # Use Case
    /// - Different nodes need different vnode counts
    /// - Fine-tuning distribution for specific nodes
    ///
    /// # Arguments
    /// * `node` - The node to add
    /// * `vnodes` - Number of virtual nodes for this specific node
    ///
    /// # Returns
    /// Self for method chaining
    ///
    /// # Example
    /// ```rust
    /// builder.add_node_with_vnodes(Node::new(NodeId(1), "node1"), 512);
    /// ```
    pub fn add_node_with_vnodes(mut self, node: Node, vnodes: usize) -> Self {
        self.ring.add_node(node, vnodes);
        self
    }

    /// Build the ring (consumes the builder).
    ///
    /// # Performance
    /// - **Time**: O(1) - just returns the ring
    /// - **Space**: O(1) - moves ownership
    ///
    /// # Returns
    /// The constructed HashRing (ready to use)
    ///
    /// # Example
    /// ```rust
    /// let ring = builder.build();
    /// ```
    pub fn build(self) -> HashRing {
        self.ring
    }
}

impl Default for RingBuilder {
    fn default() -> Self {
        Self::new()
    }
}
