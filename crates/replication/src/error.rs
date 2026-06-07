//! Replication-specific error types.

use thiserror::Error;

/// Errors that can occur during replication operations.
#[derive(Debug, Error)]
pub enum ReplicationError {
    #[error("insufficient replicas: need {required}, found {found}")]
    InsufficientReplicas { required: usize, found: usize },

    #[error("empty ring: cannot place replicas")]
    EmptyRing,

    #[error("invalid replication factor: {0}")]
    InvalidReplicationFactor(usize),

    #[error("topology constraint violated: {0}")]
    TopologyViolation(String),

    #[error("consistency level not satisfied: {0}")]
    ConsistencyNotMet(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
