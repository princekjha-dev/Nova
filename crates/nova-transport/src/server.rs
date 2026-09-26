use anyhow::{anyhow, Result};
use nova_core::bus::MessageBus;
use nova_core::protocol::NovaMessage;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{debug, error, info, warn};

use crate::connection::NovaConnection;

pub struct NovaServer {
    listen_addr: SocketAddr,
    bus: MessageBus,
    running: Arc<AtomicBool>,
}

impl NovaServer {
    pub fn new(listen_addr: SocketAddr, bus: MessageBus) -> Self {
        Self {
            listen_addr,
            bus,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(self.listen_addr).await
            .map_err(|e| anyhow!("Failed to bind TCP listener to {}: {}", self.listen_addr, e))?;

        info!("Nova TCP Server listening on {}", self.listen_addr);
        self.running.store(true, Ordering::SeqCst);

        while self.running.load(Ordering::SeqCst) {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    info!("Incoming Nova connection accepted from {}", peer_addr);
                    let bus = self.bus.clone();
                    let conn = Arc::new(NovaConnection::new(stream, peer_addr));

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_peer(conn, bus).await {
                            warn!("Peer {} disconnected: {}", peer_addr, e);
                        }
                    });
                }
                Err(e) => {
                    if self.running.load(Ordering::SeqCst) {
                        error!("TCP accept error: {}", e);
                    }
                    break;
                }
            }
        }
        Ok(())
    }

    async fn handle_peer(conn: Arc<NovaConnection>, bus: MessageBus) -> Result<()> {
        while let Some(msg) = conn.receive_message().await? {
            debug!("Received message {} plugin 0x{:04X} from {}", msg.id, msg.plugin, conn.peer_addr);
            if let Err(e) = bus.dispatch_incoming(msg).await {
                warn!("Failed to dispatch message from {}: {}", conn.peer_addr, e);
            }
        }
        Ok(())
    }

    pub fn shutdown(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}
