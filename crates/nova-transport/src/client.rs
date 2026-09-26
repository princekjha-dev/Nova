use anyhow::{anyhow, Result};
use nova_core::protocol::NovaMessage;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

use crate::connection::NovaConnection;

pub struct NovaClient {
    remote_addr: SocketAddr,
    connection: Option<Arc<NovaConnection>>,
}

impl NovaClient {
    pub fn new(remote_addr: SocketAddr) -> Self {
        Self {
            remote_addr,
            connection: None,
        }
    }

    /// Connects to remote endpoint with exponential backoff retries.
    pub async fn connect(&mut self, max_retries: u32) -> Result<Arc<NovaConnection>> {
        let mut delay = Duration::from_millis(100);
        let mut last_err = anyhow!("Could not connect to {}", self.remote_addr);

        for attempt in 1..=max_retries {
            match TcpStream::connect(self.remote_addr).await {
                Ok(stream) => {
                    info!("Connected to Nova peer at {}", self.remote_addr);
                    let conn = Arc::new(NovaConnection::new(stream, self.remote_addr));
                    self.connection = Some(conn.clone());
                    return Ok(conn);
                }
                Err(e) => {
                    warn!("Connection attempt {}/{} to {} failed: {}", attempt, max_retries, self.remote_addr, e);
                    last_err = anyhow!("Connection error: {}", e);
                    sleep(delay).await;
                    delay = (delay * 2).min(Duration::from_secs(3));
                }
            }
        }
        Err(last_err)
    }

    pub fn connection(&self) -> Option<Arc<NovaConnection>> {
        self.connection.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::NovaServer;
    use tokio::net::TcpListener;
    use nova_core::bus::MessageBus;
    use nova_core::device::{Device, Platform};
    use nova_core::protocol::{ControlMessage, PLUGIN_CONTROL};
    use nova_core::registry::DeviceRegistry;
    use nova_core::store::DeviceStore;
    use nova_crypto::DeviceKeys;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_transport_client_server_message_flow() -> Result<()> {
        let store = DeviceStore::in_memory()?;
        let registry = DeviceRegistry::new(store).await?;

        let server_keys = DeviceKeys::generate();
        let client_keys = DeviceKeys::generate();

        // Register client device in server's registry
        let client_device = Device::new(
            client_keys.device_id,
            client_keys.verifying_key.as_bytes().to_vec(),
            client_keys.fingerprint(),
            "Client Node".to_string(),
            Platform::Linux,
            vec![PLUGIN_CONTROL],
        );
        registry.register_device(client_device).await?;

        let bus = MessageBus::new(registry);
        let received_counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = received_counter.clone();

        bus.register_plugin(PLUGIN_CONTROL, move |msg| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            let control = ControlMessage::from_bytes(&msg.payload)?;
            match control {
                ControlMessage::Ping { nonce } => assert_eq!(nonce, 42),
                _ => panic!("Expected Ping message"),
            }
            Ok(())
        }).await;

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        drop(listener);

        let server = NovaServer::new(addr, bus.clone());
        tokio::spawn(async move {
            let _ = server.run().await;
        });

        // Client connects and sends Ping
        let mut client = NovaClient::new(addr);
        let conn = client.connect(5).await?;

        let ping = ControlMessage::Ping { nonce: 42 };
        let msg = NovaMessage::create(
            &client_keys,
            server_keys.device_id,
            PLUGIN_CONTROL,
            ping.to_bytes()?,
        )?;

        conn.send_message(&msg).await?;

        // Wait briefly for dispatch
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(received_counter.load(Ordering::SeqCst), 1);

        Ok(())
    }
}
