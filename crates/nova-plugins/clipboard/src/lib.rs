use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use nova_core::device::DeviceId;
use nova_core::protocol::{NovaMessage, PLUGIN_CLIPBOARD};
use nova_crypto::DeviceKeys;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum ClipboardContent {
    Text(String),
    Url(String),
    ImagePng(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClipboardEntry {
    pub id: Uuid,
    pub source_device: DeviceId,
    pub content: ClipboardContent,
    pub hash: String,
    pub sensitive: bool,
    pub preview: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClipboardPayload {
    pub entry_id: Uuid,
    pub content: ClipboardContent,
    pub sensitive: bool,
    pub timestamp_micros: u64,
}

pub struct ClipboardPlugin {
    local_device_id: DeviceId,
    history: Arc<RwLock<VecDeque<ClipboardEntry>>>,
    max_history: usize,
    last_applied_hash: Arc<RwLock<Option<String>>>,
}

impl ClipboardPlugin {
    pub fn new(local_device_id: DeviceId, max_history: usize) -> Self {
        Self {
            local_device_id,
            history: Arc::new(RwLock::new(VecDeque::with_capacity(max_history))),
            max_history,
            last_applied_hash: Arc::new(RwLock::new(None)),
        }
    }

    /// Process text copied locally on this device.
    /// Returns Some(entry) if content is new, or None if deduplicated.
    pub async fn on_local_copy(&self, text: &str) -> Option<ClipboardEntry> {
        let hash = Self::compute_hash(text.as_bytes());

        // Deduplication check against last applied clipboard
        {
            let last = self.last_applied_hash.read().await;
            if let Some(ref last_h) = *last {
                if last_h == &hash {
                    return None; // Loop prevention
                }
            }
        }

        let is_sensitive = Self::detect_sensitive(text);
        let content = if text.starts_with("http://") || text.starts_with("https://") {
            ClipboardContent::Url(text.to_string())
        } else {
            ClipboardContent::Text(text.to_string())
        };

        let preview = if is_sensitive {
            "•••••••••••••••• (Sensitive Content)".to_string()
        } else {
            text.chars().take(120).collect()
        };

        let entry = ClipboardEntry {
            id: Uuid::new_v4(),
            source_device: self.local_device_id,
            content,
            hash: hash.clone(),
            sensitive: is_sensitive,
            preview,
            timestamp: Utc::now(),
        };

        self.add_to_history(entry.clone()).await;
        *self.last_applied_hash.write().await = Some(hash);

        Some(entry)
    }

    /// Creates a NovaMessage to sync this clipboard entry to a paired device.
    pub fn create_sync_message(
        &self,
        entry: &ClipboardEntry,
        target_device_id: DeviceId,
        keys: &DeviceKeys,
    ) -> Result<NovaMessage> {
        let payload = ClipboardPayload {
            entry_id: entry.id,
            content: entry.content.clone(),
            sensitive: entry.sensitive,
            timestamp_micros: entry.timestamp.timestamp_micros() as u64,
        };

        let payload_bytes = rmp_serde::to_vec_named(&payload)
            .map_err(|e| anyhow!("Failed to serialize ClipboardPayload: {}", e))?;

        NovaMessage::create(keys, target_device_id, PLUGIN_CLIPBOARD, payload_bytes)
    }

    /// Handle incoming clipboard message from a remote paired device.
    pub async fn on_remote_clipboard(&self, msg: &NovaMessage) -> Result<ClipboardEntry> {
        let payload: ClipboardPayload = rmp_serde::from_slice(&msg.payload)
            .map_err(|e| anyhow!("Failed to deserialize ClipboardPayload: {}", e))?;

        let (hash, preview) = match &payload.content {
            ClipboardContent::Text(t) => {
                let h = Self::compute_hash(t.as_bytes());
                let p = if payload.sensitive {
                    "•••••••••••••••• (Sensitive Content)".to_string()
                } else {
                    t.chars().take(120).collect()
                };
                (h, p)
            }
            ClipboardContent::Url(u) => {
                let h = Self::compute_hash(u.as_bytes());
                (h, u.clone())
            }
            ClipboardContent::ImagePng(img) => {
                let h = Self::compute_hash(img);
                (h, format!("[PNG Image - {} bytes]", img.len()))
            }
        };

        // Loop prevention: check if this matches our last clipboard hash
        {
            let last = self.last_applied_hash.read().await;
            if let Some(ref last_h) = *last {
                if last_h == &hash {
                    debug!("Ignoring echoed clipboard update");
                    return Err(anyhow!("Echoed clipboard deduplicated"));
                }
            }
        }

        *self.last_applied_hash.write().await = Some(hash.clone());

        let entry = ClipboardEntry {
            id: payload.entry_id,
            source_device: msg.source,
            content: payload.content,
            hash,
            sensitive: payload.sensitive,
            preview,
            timestamp: Utc::now(),
        };

        self.add_to_history(entry.clone()).await;
        info!("Synchronized remote clipboard from device {}", msg.source);
        Ok(entry)
    }

    async fn add_to_history(&self, entry: ClipboardEntry) {
        let mut hist = self.history.write().await;
        // Remove prior duplicate if present
        hist.retain(|e| e.hash != entry.hash);
        hist.push_front(entry);
        while hist.len() > self.max_history {
            hist.pop_back();
        }
    }

    pub async fn list_history(&self) -> Vec<ClipboardEntry> {
        let hist = self.history.read().await;
        hist.iter().cloned().collect()
    }

    pub async fn clear_history(&self) {
        let mut hist = self.history.write().await;
        hist.clear();
        *self.last_applied_hash.write().await = None;
    }

    pub fn compute_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    /// Heuristic sensitive content detection (API tokens, private keys, passwords, OTPs).
    pub fn detect_sensitive(text: &str) -> bool {
        let lower = text.to_lowercase();
        if text.contains("BEGIN PRIVATE KEY") || text.contains("BEGIN RSA PRIVATE KEY") {
            return true;
        }
        if lower.starts_with("ghp_") || lower.starts_with("eyj") || lower.starts_with("sk-") {
            return true;
        }
        // 6-digit pure numerical OTP pattern
        if text.len() == 6 && text.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clipboard_deduplication_and_sync() -> Result<()> {
        let linux_keys = DeviceKeys::generate();
        let phone_keys = DeviceKeys::generate();

        let linux_plugin = ClipboardPlugin::new(linux_keys.device_id, 20);
        let phone_plugin = ClipboardPlugin::new(phone_keys.device_id, 20);

        // User copies text on Linux
        let text = "https://github.com/nova-ecosystem/nova";
        let entry1 = linux_plugin.on_local_copy(text).await.expect("new entry");
        assert_eq!(entry1.preview, text);

        // Immediate identical copy is deduplicated
        let duplicate = linux_plugin.on_local_copy(text).await;
        assert!(duplicate.is_none());

        // Create wire message and receive on Phone
        let msg = linux_plugin.create_sync_message(&entry1, phone_keys.device_id, &linux_keys)?;
        let phone_entry = phone_plugin.on_remote_clipboard(&msg).await?;
        assert_eq!(phone_entry.hash, entry1.hash);

        // Phone receives echo: deduplicated
        assert!(phone_plugin.on_remote_clipboard(&msg).await.is_err());

        // Sensitive detection test
        let token = "ghp_1234567890abcdefghijklmnopqrstuvwxyz";
        let sens_entry = linux_plugin.on_local_copy(token).await.expect("sens");
        assert!(sens_entry.sensitive);
        assert!(sens_entry.preview.contains("Sensitive Content"));

        Ok(())
    }
}
