//! Inbound streaming message receiver.

use crate::codec::{self, FrameDecoder};
use crate::error::StreamingError;
use crate::protocol::{Message, MessageType};
use crate::snapshot::RingSnapshot;
use corelib::ring::HashRing;
use tokio::io::AsyncRead;
use tokio::sync::mpsc;

/// Receives and processes streaming messages.
pub struct StreamReceiver {
    rx: Option<mpsc::Receiver<Message>>,
    decoder: FrameDecoder,
}

impl StreamReceiver {
    /// Create an in-memory receiver from an mpsc channel.
    pub fn from_channel(rx: mpsc::Receiver<Message>) -> Self {
        Self {
            rx: Some(rx),
            decoder: FrameDecoder::new(),
        }
    }

    /// Receive the next message from the in-memory channel.
    pub async fn recv(&mut self) -> Result<Option<Message>, StreamingError> {
        match &mut self.rx {
            Some(rx) => match rx.recv().await {
                Some(msg) => Ok(Some(msg)),
                None => Ok(None),
            },
            None => Err(StreamingError::ConnectionClosed),
        }
    }

    /// Read one message from an async byte stream.
    pub async fn read_from<R: AsyncRead + Unpin>(
        reader: &mut R,
    ) -> Result<Message, StreamingError> {
        codec::read_message(reader).await
    }

    /// Push bytes into the incremental frame decoder (buffered TCP).
    pub fn push_bytes(&mut self, chunk: &[u8]) {
        self.decoder.push(chunk);
    }

    /// Try to decode a complete frame from the internal buffer.
    pub fn try_decode(&mut self) -> Result<Option<Message>, StreamingError> {
        self.decoder.try_decode()
    }

    /// Apply a ring snapshot message and return the rebuilt ring.
    pub fn apply_snapshot(message: &Message) -> Result<HashRing, StreamingError> {
        if message.msg_type != MessageType::RingSnapshot {
            return Err(StreamingError::Protocol(
                "expected RingSnapshot message".into(),
            ));
        }
        let snapshot = RingSnapshot::from_bytes(&message.payload)?;
        snapshot.build_ring()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sender::StreamSender;
    use corelib::node::{Node, NodeId};

    #[tokio::test]
    async fn channel_roundtrip() {
        let (mut sender, rx) = StreamSender::channel(8);
        let mut receiver = StreamReceiver::from_channel(rx);

        let ring = HashRing::new();
        ring.add_node(Node::new(NodeId(1), "n1"), 4);
        sender
            .send_snapshot(&ring, &[(NodeId(1), 4)])
            .await
            .unwrap();

        let msg = receiver.recv().await.unwrap().unwrap();
        let rebuilt = StreamReceiver::apply_snapshot(&msg).unwrap();
        assert_eq!(rebuilt.node_count(), 1);
    }
}
