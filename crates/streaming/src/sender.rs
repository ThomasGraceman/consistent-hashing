//! Outbound streaming message sender.

use crate::codec;
use crate::error::StreamingError;
use crate::protocol::{Message, MessageType};
use crate::snapshot::RingSnapshot;
use corelib::node::NodeId;
use corelib::ring::HashRing;
use tokio::io::AsyncWrite;
use tokio::sync::mpsc;

/// Sends streaming messages over a channel or TCP connection.
pub struct StreamSender {
    sequence: u64,
    tx: Option<mpsc::Sender<Message>>,
}

impl StreamSender {
    /// Create an in-memory sender backed by an mpsc channel.
    pub fn channel(buffer: usize) -> (Self, mpsc::Receiver<Message>) {
        let (tx, rx) = mpsc::channel(buffer);
        (
            Self {
                sequence: 0,
                tx: Some(tx),
            },
            rx,
        )
    }

    fn next_sequence(&mut self) -> u64 {
        let seq = self.sequence;
        self.sequence += 1;
        seq
    }

    async fn dispatch(&mut self, message: Message) -> Result<(), StreamingError> {
        if let Some(tx) = &self.tx {
            tx.send(message)
                .await
                .map_err(|_| StreamingError::ConnectionClosed)?;
        }
        Ok(())
    }

    /// Send a raw message.
    pub async fn send(&mut self, message: Message) -> Result<(), StreamingError> {
        self.dispatch(message).await
    }

    /// Send a heartbeat.
    pub async fn send_heartbeat(&mut self) -> Result<(), StreamingError> {
        let seq = self.next_sequence();
        self.dispatch(Message::heartbeat(seq)).await
    }

    /// Send a ring snapshot.
    pub async fn send_snapshot(
        &mut self,
        ring: &HashRing,
        vnode_counts: &[(NodeId, usize)],
    ) -> Result<(), StreamingError> {
        let snapshot = RingSnapshot::from_ring(ring, vnode_counts);
        let payload = snapshot.to_bytes()?;
        let seq = self.next_sequence();
        self.dispatch(Message::new(MessageType::RingSnapshot, seq, payload))
            .await
    }

    /// Write a message directly to an async writer (TCP, etc.).
    pub async fn write_to<W: AsyncWrite + Unpin>(
        writer: &mut W,
        message: &Message,
    ) -> Result<(), StreamingError> {
        codec::write_message(writer, message).await
    }
}
