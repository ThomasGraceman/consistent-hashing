//! Length-prefixed bincode framing for streaming messages.

use crate::error::StreamingError;
use crate::protocol::Message;
use bytes::{Buf, BufMut, BytesMut};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// Encode a message to bytes with a 4-byte big-endian length prefix.
pub fn encode(message: &Message) -> Result<Vec<u8>, StreamingError> {
    let body = bincode::serialize(message)
        .map_err(|e| StreamingError::Codec(e.to_string()))?;
    if body.len() > MAX_FRAME_SIZE {
        return Err(StreamingError::Codec("frame too large".into()));
    }
    let mut buf = Vec::with_capacity(4 + body.len());
    buf.put_u32(body.len() as u32);
    buf.extend_from_slice(&body);
    Ok(buf)
}

/// Decode a message from a complete frame (without length prefix).
pub fn decode(frame: &[u8]) -> Result<Message, StreamingError> {
    bincode::deserialize(frame).map_err(|e| StreamingError::Codec(e.to_string()))
}

/// Read one length-prefixed message from an async stream.
pub async fn read_message<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<Message, StreamingError> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_FRAME_SIZE {
        return Err(StreamingError::Codec("frame too large".into()));
    }
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body).await?;
    decode(&body)
}

/// Write one length-prefixed message to an async stream.
pub async fn write_message<W: AsyncWrite + Unpin>(
    writer: &mut W,
    message: &Message,
) -> Result<(), StreamingError> {
    let encoded = encode(message)?;
    writer.write_all(&encoded).await?;
    writer.flush().await?;
    Ok(())
}

/// Incremental decoder for buffered TCP reads.
#[derive(Default)]
pub struct FrameDecoder {
    buffer: BytesMut,
}

impl FrameDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, chunk: &[u8]) {
        self.buffer.extend_from_slice(chunk);
    }

    pub fn try_decode(&mut self) -> Result<Option<Message>, StreamingError> {
        if self.buffer.len() < 4 {
            return Ok(None);
        }
        let len = u32::from_be_bytes(self.buffer[..4].try_into().unwrap()) as usize;
        if len > MAX_FRAME_SIZE {
            return Err(StreamingError::Codec("frame too large".into()));
        }
        if self.buffer.len() < 4 + len {
            return Ok(None);
        }
        self.buffer.advance(4);
        let frame = self.buffer.split_to(len);
        decode(&frame).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{MessageType, Message};

    #[test]
    fn roundtrip_message() {
        let msg = Message::new(MessageType::Heartbeat, 42, vec![1, 2, 3]);
        let encoded = encode(&msg).unwrap();
        let decoded = decode(&encoded[4..]).unwrap();
        assert_eq!(msg, decoded);
    }
}
