//! Consistency level definitions for read/write operations.

/// Read/write consistency levels (Cassandra-inspired).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConsistencyLevel {
    /// Single replica responds.
    One,
    /// Two replicas respond.
    Two,
    /// Majority of replicas respond.
    Quorum,
    /// All replicas respond.
    All,
    /// Quorum within the local data center.
    LocalQuorum,
    /// Single replica in local data center.
    LocalOne,
}

impl ConsistencyLevel {
    /// Minimum number of replica acknowledgements required.
    pub fn required_acks(&self, replication_factor: usize) -> usize {
        match self {
            ConsistencyLevel::One | ConsistencyLevel::LocalOne => 1,
            ConsistencyLevel::Two => 2.min(replication_factor),
            ConsistencyLevel::Quorum | ConsistencyLevel::LocalQuorum => {
                (replication_factor / 2) + 1
            }
            ConsistencyLevel::All => replication_factor,
        }
    }

    /// Returns true if `acks` satisfies this consistency level.
    pub fn is_satisfied(&self, acks: usize, replication_factor: usize) -> bool {
        acks >= self.required_acks(replication_factor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quorum_calculation() {
        assert_eq!(ConsistencyLevel::Quorum.required_acks(3), 2);
        assert_eq!(ConsistencyLevel::Quorum.required_acks(5), 3);
        assert!(ConsistencyLevel::Quorum.is_satisfied(2, 3));
        assert!(!ConsistencyLevel::All.is_satisfied(2, 3));
    }
}
