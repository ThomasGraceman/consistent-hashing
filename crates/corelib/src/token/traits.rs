//! Core token trait definitions.

use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::Hash;

/// Represents a position token on the hash ring.
///
/// Tokens are immutable values that represent positions in the token space.
/// They must be:
/// - **Comparable**: To determine ordering on the ring
/// - **Hashable**: For efficient lookups and storage
/// - **Thread-safe**: For concurrent access patterns
/// tokens are meant to be stable, owned values that can safely live anywhere (in maps, across threads, on disk) without lifetime headaches or dangling references
/// Represents a distance metric in the token space
pub trait Distance: 
    Clone + 
    Debug + 
    PartialOrd + 
    From<u128> + 
    Into<u128>
{
    /// Zero distance
    fn zero() -> Self;
    
    /// Maximum possible distance
    fn max() -> Self;
    
    /// Add two distances
    fn add(&self, other: &Self) -> Self;
    
    /// Subtract two distances
    fn sub(&self, other: &Self) -> Option<Self>;
    
    /// Convert to floating point for normalization
    fn to_f64(&self) -> f64;
}


/// Core token trait - foundation for all token types
/// 
/// This trait abstracts over different token implementations,
/// allowing for various partitioning strategies.
pub trait Token: 
    Clone + 
    Debug + 
    Display + 
    Eq + 
    Hash + 
    Ord + 
    Send + 
    Sync + 
    Serialize + 
    for<'de> Deserialize<'de> 
{
    /// The type used to represent distances in the token space
    type Distance: Distance;
    
    /// Byte representation (may be fixed or variable length)
    type Bytes: AsRef<[u8]> + Into<Vec<u8>>;
    
    /// Convert token to byte representation
    fn to_bytes(&self) -> Self::Bytes;
    
    /// Create token from byte slice
    fn from_bytes(bytes: &[u8]) -> Result<Self, TokenError> 
    where 
        Self: Sized;
    
    /// Get minimum possible token value
    fn min_value() -> Self 
    where 
        Self: Sized;
    
    /// Get maximum possible token value
    fn max_value() -> Self 
    where 
        Self: Sized;
    
    /// Calculate distance to another token
    /// 
    /// The distance is always calculated in the forward direction
    /// around the ring (this -> other).
    fn distance(&self, other: &Self) -> Self::Distance;
    
    /// Split the token space evenly into n parts
    fn split_evenly(n: usize) -> Vec<Self> 
    where 
        Self: Sized;
    
    // --- Byte-Comparable Operations ---
    
    /// Produce a byte-comparable representation
    /// 
    /// This must satisfy the weakly prefix-free property:
    /// For any valid tokens x, y and bytes b1, b2 ∈ [0x10, 0xEF]:
    ///   - compare(x, y) == compare_bytes(as_comparable_bytes(x) + b1, as_comparable_bytes(y) + b2)
    ///   - as_comparable_bytes(x) + b1 is not a prefix of as_comparable_bytes(y)
    fn as_comparable_bytes(&self, version: ByteComparableVersion) -> Vec<u8>;
    
    /// Create token from byte-comparable representation
    fn from_comparable_bytes(bytes: &[u8], version: ByteComparableVersion) -> Result<Self, TokenError>
    where
        Self: Sized;
    
    // --- Token Space Navigation ---
    
    /// Get the next valid token in the token space
    /// 
    /// Returns an error if at maximum token.
    fn next_valid_token(&self) -> Result<Self, TokenError> 
    where 
        Self: Sized;
    
    /// Get a token slightly larger than this
    /// 
    /// May not be the immediate next token. Default implementation
    /// delegates to `next_valid_token()`.
    fn increase_slightly(&self) -> Result<Self, TokenError> 
    where 
        Self: Sized,
    {
        self.next_valid_token()
    }
    
    /// Get a token slightly smaller than this
    /// 
    /// Returns an error if at minimum token.
    fn decrease_slightly(&self) -> Result<Self, TokenError> 
    where 
        Self: Sized;
    
    /// Calculate normalized size/distance to next token
    /// 
    /// Returns a value between 0.0 and 1.0 representing the
    /// fraction of the total token space.
    fn size(&self, next: &Self) -> f64 {
        let distance = self.distance(next);
        let max = Self::Distance::max();
        distance.to_f64() / max.to_f64()
    }
    
    // --- Predicates ---
    
    /// Check if this is the minimum token
    fn is_minimum(&self) -> bool {
        self == &Self::min_value()
    }
    
    /// Check if this is the maximum token
    fn is_maximum(&self) -> bool {
        self == &Self::max_value()
    }
    
    // --- Hashing and Memory ---
    
    /// Get hash of token value (for hash maps)
    /// 
    /// This should be a fast hash, not necessarily cryptographic.
    fn token_hash(&self) -> u64;
    
    /// Get heap size in bytes (for memory accounting)
    /// 
    /// Should include any dynamically allocated memory.
    fn heap_size(&self) -> usize;
    
    // --- Optional: Midpoint Calculation ---
    
    /// Calculate the midpoint between two tokens
    /// 
    /// This is optional and may not be supported by all token types.
    fn midpoint(&self, _other: &Self) -> Option<Self>
    where
        Self: Sized,
    {
        None
    }
    
    // --- Serialization Hints ---
    
    /// Whether this token type has a fixed byte length
    fn has_fixed_length() -> bool
    where
        Self: Sized,
    {
        false
    }
    
    /// Maximum byte size for this token type (if fixed length)
    fn max_byte_size() -> Option<usize>
    where
        Self: Sized,
    {
        None
    }
}