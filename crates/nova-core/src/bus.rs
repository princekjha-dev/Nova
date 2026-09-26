use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::device::DeviceId;
use crate::protocol::{NovaMessage, PluginId};
use crate::registry::DeviceRegistry;

pub type PluginHandler = Arc<dyn Fn(NovaMessage) -> Result<()> + Send + Sync>;

#[derive(Clone)]
pub struct MessageBus {
    registry: DeviceRegistry,
    handlers: Arc<RwLock<HashMap<PluginId, PluginHandler>>>,
    outgoing_tx: mpsc::UnboundedSender<NovaMessage>,
    outgoing_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<NovaMessage>>>,
}

impl MessageBus {
    pub fn new(registry: DeviceRegistry) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            registry,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            outgoing_tx: tx,
            outgoing_rx: Arc::new(tokio::sync::Mutex::new(rx)),
        }
    }

    /// Register a plugin handler for a given PluginId.
    pub async fn register_plugin<F>(&self, plugin_id: PluginId, handler: F)
    where
        F: Fn(NovaMessage) -> Result<()> + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().await;
        handlers.insert(plugin_id, Arc::new(handler));
    }

    /// Dispatches an incoming message from the network.
    /// Performs cryptographic verification and trust checking.
    pub async fn dispatch_incoming(&self, msg: NovaMessage) -> Result<()> {
        // 1. Verify sender exists and is trusted in registry
        let sender = self.registry.get_device(&msg.source).await
            .ok_or_else(|| anyhow!("Unknown sender device: {}", msg.source))?;

        if !sender.trusted {
            return Err(anyhow!("Untrusted device message rejected: {}", msg.source));
        }

        // 2. Cryptographic signature check
        if sender.pubkey.len() != 32 {
            return Err(anyhow!("Invalid public key length for device: {}", msg.source));
        }
        let mut pubkey_bytes = [0u8; 32];
        pubkey_bytes.copy_from_slice(&sender.pubkey);
        if !msg.verify_signature(&pubkey_bytes)? {
            return Err(anyhow!("Cryptographic signature verification failed for device: {}", msg.source));
        }

        // 3. Dispatch to registered handler
        let handlers = self.handlers.read().await;
        if let Some(handler) = handlers.get(&msg.plugin) {
            handler(msg)?;
            Ok(())
        } else {
            Err(anyhow!("No handler registered for plugin ID 0x{:04X}", msg.plugin))
        }
    }

    /// Send a message out over transport.
    pub fn send_outgoing(&self, msg: NovaMessage) -> Result<()> {
        self.outgoing_tx.send(msg)
            .map_err(|e| anyhow!("Failed to send outgoing message to bus: {}", e))
    }

    /// Poll outgoing message stream.
    pub async fn next_outgoing(&self) -> Option<NovaMessage> {
        let mut rx = self.outgoing_rx.lock().await;
        rx.recv().await
    }

    pub fn registry(&self) -> &DeviceRegistry {
        &self.registry
    }
}
