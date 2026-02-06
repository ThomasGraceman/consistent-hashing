//! Ring position implementation.

use crate::partitioner::traits::Partitioner;
use crate::token::Token;
use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::sync::Arc;

/// A position on the consistent hash ring.
///
/// Combines a token with its partitioner to provide a complete
/// position abstraction.
#[derive(Clone)]
pub struct Position<T: Token, P: Partitioner<TokenType = T>> {
    token: T,
    partitioner: Arc<P>,
}

impl<T: Token, P: Partitioner<TokenType = T>> Position<T, P> {
    /// Creates a new position with the given token and partitioner.
    pub fn new(token: T, partitioner: Arc<P>) -> Self {
        Self { token, partitioner }
    }

    /// Returns a reference to the token.
    pub fn token(&self) -> &T {
        &self.token
    }

    /// Returns a reference to the partitioner.
    pub fn partitioner(&self) -> &Arc<P> {
        &self.partitioner
    }
}

impl<T: Token, P: Partitioner<TokenType = T>> PartialEq for Position<T, P> {
    fn eq(&self, other: &Self) -> bool {
        self.token == other.token
    }
}

impl<T: Token, P: Partitioner<TokenType = T>> Eq for Position<T, P> {}

impl<T: Token, P: Partitioner<TokenType = T>> PartialOrd for Position<T, P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Token, P: Partitioner<TokenType = T>> Ord for Position<T, P> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.token.cmp(&other.token)
    }
}

impl<T: Token, P: Partitioner<TokenType = T>> Debug for Position<T, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Position")
            .field("token", &self.token)
            .field("partitioner", &self.partitioner.name())
            .finish()
    }
}

/// Trait for ring position operations.
pub trait RingPosition: Clone + Ord + Send + Sync + Debug + 'static {
    /// The token type used by this position.
    type TokenType: Token;

    /// The partitioner that generated this position's token.
    type PartitionerType: Partitioner<TokenType = Self::TokenType>;

    /// Returns a reference to this position's token.
    fn token(&self) -> &Self::TokenType;

    /// Returns a shared reference to the partitioner.
    fn partitioner(&self) -> Arc<Self::PartitionerType>;

    /// Checks if this is the minimum position on the ring.
    fn is_minimum(&self) -> bool;

    /// Creates a new position at the minimum ring value.
    fn min_value(&self) -> Self;

    /// Checks if this is the maximum position on the ring.
    fn is_maximum(&self) -> bool;

    /// Creates a new position at the maximum ring value.
    fn max_value(&self) -> Self;
}

impl<T: Token, P: Partitioner<TokenType = T>> RingPosition for Position<T, P> {
    type TokenType = T;
    type PartitionerType = P;

    fn token(&self) -> &Self::TokenType {
        &self.token
    }

    fn partitioner(&self) -> Arc<Self::PartitionerType> {
        Arc::clone(&self.partitioner)
    }

    fn is_minimum(&self) -> bool {
        self.token.is_zero()
    }

    fn min_value(&self) -> Self {
        Self::new(self.partitioner.min_token(), Arc::clone(&self.partitioner))
    }

    fn is_maximum(&self) -> bool {
        self.token.is_max()
    }

    fn max_value(&self) -> Self {
        Self::new(self.partitioner.max_token(), Arc::clone(&self.partitioner))
    }
}
