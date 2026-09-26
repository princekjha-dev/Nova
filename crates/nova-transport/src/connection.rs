use anyhow::{anyhow, Result};
use bytes::BytesMut;
use nova_core::protocol::NovaMessage;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tracing::{debug, error, trace};

pub struct NovaConnection {
    stream_read: Mutex<tokio::net::tcp::OwnedReadHalf>,
    stream_write: Mutex<tokio::net::tcp::OwnedWriteHalf>,
    pub peer_addr: SocketAddr,
    read_buf: Mutex<BytesMut>,
}

impl NovaConnection {
    pub fn new(stream: TcpStream, peer_addr: SocketAddr) -> Self {
        let (read_half, write_half) = stream.into_split();
        Self {
            stream_read: Mutex::new(read_half),
            stream_write: Mutex::new(write_half),
            peer_addr,
            read_buf: Mutex::new(BytesMut::with_capacity(8192)),
        }
    }

    /// Sends a signed NovaMessage framed over the wire.
    pub async fn send_message(&self, msg: &NovaMessage) -> Result<()> {
        let frame = msg.encode_wire()?;
        let mut write = self.stream_write.lock().await;
        write.write_all(&frame).await
            .map_err(|e| anyhow!("Failed to write wire frame to peer {}: {}", self.peer_addr, e))?;
        write.flush().await
            .map_err(|e| anyhow!("Failed to flush wire frame to peer {}: {}", self.peer_addr, e))?;
        trace!("Sent message {} to {}", msg.id, self.peer_addr);
        Ok(())
    }

    /// Reads the next complete NovaMessage frame from the wire.
    pub async fn receive_message(&self) -> Result<Option<NovaMessage>> {
        let mut buf = self.read_buf.lock().await;
        let mut read_half = self.stream_read.lock().await;

        loop {
            // Check if we already have a full message in buffer
            if let Some(msg) = NovaMessage::decode_wire(&mut buf)? {
                return Ok(Some(msg));
            }

            // Read more bytes from socket
            let mut chunk = [0u8; 4096];
            let n = read_half.read(&mut chunk).await
                .map_err(|e| anyhow!("Read error from peer {}: {}", self.peer_addr, e))?;

            if n == 0 {
                // Connection closed cleanly by remote peer
                if buf.is_empty() {
                    return Ok(None);
                } else {
                    return Err(anyhow!("Connection closed with incomplete frame ({} bytes remaining)", buf.len()));
                }
            }

            buf.extend_from_slice(&chunk[..n]);
        }
    }
}
