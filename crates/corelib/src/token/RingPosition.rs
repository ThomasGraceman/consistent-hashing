use std::cmp::Ordering;
use std::hash::Hash;
use std::sync::Arc;

// i am trying to use casandra implementation, i will try to come up with something on my own 
// in future implementatiosn
// i will try to write more about them.

pub trait RingPosition: Clone + Ord + Send + Sync + 'static {
    type TokenType: Token;
    type PartitionerType: Partitioner<TokenType = Self::TokenType>;
    
    fn token(&self) -> &Self::TokenType;
    fn partitioner(&self) -> Arc<Self::PartitionerType>;
    fn is_minimum(&self) -> bool;
    fn min_value(&self) -> Self;
}