//! Streaming protocol definitions.

use serde::{Deserialize, Serialize};

/// Wire-level message types for ring synchronization.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    /// Full ring snapshot for bootstrap.
    RingSnapshot,
    /// Incremental node addition.
    NodeAdded,
    /// Incremental node removal.
    NodeRemoved,
    /// Data chunk during migration.
    DataChunk,
    /// Keep-alive heartbeat.
    Heartbeat,
    /// Acknowledgement of a prior message.
    Ack,
}

/// A framed streaming message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub msg_type: MessageType,
    pub sequence: u64,
    pub payload: Vec<u8>,
}

impl Message {
    pub fn new(msg_type: MessageType, sequence: u64, payload: Vec<u8>) -> Self {
        Self {
            msg_type,
            sequence,
            payload,
        }
    }

    pub fn heartbeat(sequence: u64) -> Self {
        Self::new(MessageType::Heartbeat, sequence, Vec::new())
    }

    pub fn ack(sequence: u64) -> Self {
        Self::new(MessageType::Ack, sequence, Vec::new())
    }
}
