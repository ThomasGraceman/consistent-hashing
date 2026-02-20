//! Streaming protocol for ring state synchronization.
//!
//! This crate provides the protocol and codecs for streaming:
//! - Ring state between nodes
//! - Data migration during rebalancing
//! - Bootstrap operations

pub mod codec;
pub mod error;
pub mod protocol;
pub mod receiver;
pub mod sender;
pub mod snapshot;

pub use error::StreamingError;
pub use protocol::{Message, MessageType};
pub use receiver::StreamReceiver;
pub use sender::StreamSender;
