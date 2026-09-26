use anyhow::{anyhow, ensure, Result};
use chrono::{DateTime, Utc};
use nova_core::device::DeviceId;
use nova_core::protocol::{NovaMessage, PLUGIN_FILES};
use nova_crypto::DeviceKeys;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

pub const CHUNK_SIZE: usize = 64 * 1024; // 64 KiB chunks for LAN transfer

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransferDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransferState {
    Pending,
    Accepted,
    InProgress { transferred_bytes: u64, total_bytes: u64, progress_percent: u8 },
    Completed,
    Rejected,
    Cancelled,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileMetadata {
    pub file_id: Uuid,
    pub file_name: String,
    pub file_size: u64,
    pub mime_type: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileTransferMessage {
    Offer {
        session_id: Uuid,
        files: Vec<FileMetadata>,
    },
    Decision {
        session_id: Uuid,
        accepted: bool,
    },
    Chunk {
        session_id: Uuid,
        file_id: Uuid,
        chunk_index: u32,
        total_chunks: u32,
        offset: u64,
        data: Vec<u8>,
    },
    Complete {
        session_id: Uuid,
        file_id: Uuid,
        sha256: String,
    },
    Cancel {
        session_id: Uuid,
        reason: String,
    },
}

impl FileTransferMessage {
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        rmp_serde::to_vec_named(self).map_err(|e| anyhow!("FileTransferMessage serialize error: {}", e))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        rmp_serde::from_slice(bytes).map_err(|e| anyhow!("FileTransferMessage deserialize error: {}", e))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTransfer {
    pub session_id: Uuid,
    pub direction: TransferDirection,
    pub peer_device_id: DeviceId,
    pub file: FileMetadata,
    pub state: TransferState,
    pub target_path: Option<PathBuf>,
    pub created_at: DateTime<Utc>,
}

pub struct FileTransferPlugin {
    local_device_id: DeviceId,
    downloads_dir: PathBuf,
    transfers: Arc<RwLock<HashMap<Uuid, ActiveTransfer>>>,
    incoming_buffers: Arc<RwLock<HashMap<Uuid, Vec<u8>>>>,
}

impl FileTransferPlugin {
    pub fn new<P: AsRef<Path>>(local_device_id: DeviceId, downloads_dir: P) -> Self {
        let dir = downloads_dir.as_ref().to_path_buf();
        let _ = std::fs::create_dir_all(&dir);
        Self {
            local_device_id,
            downloads_dir: dir,
            transfers: Arc::new(RwLock::new(HashMap::new())),
            incoming_buffers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates an outgoing file offer for a file on disk.
    pub async fn prepare_offer(
        &self,
        peer_device_id: DeviceId,
        file_path: &Path,
        mime_type: &str,
    ) -> Result<(FileTransferMessage, Vec<u8>)> {
        let file_bytes = std::fs::read(file_path)?;
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed_file")
            .to_string();

        let sha256 = Self::hash_file(&file_bytes);
        let session_id = Uuid::new_v4();
        let file_id = Uuid::new_v4();

        let meta = FileMetadata {
            file_id,
            file_name,
            file_size: file_bytes.len() as u64,
            mime_type: mime_type.to_string(),
            sha256,
        };

        let transfer = ActiveTransfer {
            session_id,
            direction: TransferDirection::Outgoing,
            peer_device_id,
            file: meta.clone(),
            state: TransferState::Pending,
            target_path: Some(file_path.to_path_buf()),
            created_at: Utc::now(),
        };

        self.transfers.write().await.insert(session_id, transfer);
        Ok((
            FileTransferMessage::Offer {
                session_id,
                files: vec![meta],
            },
            file_bytes,
        ))
    }

    /// Chunks file bytes into a series of FileTransferMessage::Chunk payloads.
    pub fn create_chunks(session_id: Uuid, file_id: Uuid, data: &[u8]) -> Vec<FileTransferMessage> {
        let total_chunks = ((data.len() + CHUNK_SIZE - 1) / CHUNK_SIZE) as u32;
        let mut chunks = Vec::new();

        for i in 0..total_chunks {
            let offset = (i as usize) * CHUNK_SIZE;
            let end = (offset + CHUNK_SIZE).min(data.len());
            let chunk_data = data[offset..end].to_vec();

            chunks.push(FileTransferMessage::Chunk {
                session_id,
                file_id,
                chunk_index: i,
                total_chunks,
                offset: offset as u64,
                data: chunk_data,
            });
        }

        chunks
    }

    /// Handles incoming FileTransferMessage.
    pub async fn handle_message(&self, peer_id: DeviceId, msg: FileTransferMessage) -> Result<Option<FileTransferMessage>> {
        match msg {
            FileTransferMessage::Offer { session_id, files } => {
                if let Some(file) = files.first() {
                    let target_path = self.downloads_dir.join(&file.file_name);
                    let transfer = ActiveTransfer {
                        session_id,
                        direction: TransferDirection::Incoming,
                        peer_device_id: peer_id,
                        file: file.clone(),
                        state: TransferState::Accepted, // Auto-accepted over paired trusted LAN
                        target_path: Some(target_path),
                        created_at: Utc::now(),
                    };
                    self.transfers.write().await.insert(session_id, transfer);
                    self.incoming_buffers.write().await.insert(session_id, Vec::with_capacity(file.file_size as usize));

                    // Return Decision: Accepted
                    return Ok(Some(FileTransferMessage::Decision {
                        session_id,
                        accepted: true,
                    }));
                }
                Ok(None)
            }
            FileTransferMessage::Decision { session_id, accepted } => {
                let mut transfers = self.transfers.write().await;
                if let Some(t) = transfers.get_mut(&session_id) {
                    t.state = if accepted {
                        TransferState::Accepted
                    } else {
                        TransferState::Rejected
                    };
                }
                Ok(None)
            }
            FileTransferMessage::Chunk { session_id, file_id, chunk_index, total_chunks, data, .. } => {
                let mut buffers = self.incoming_buffers.write().await;
                if let Some(buf) = buffers.get_mut(&session_id) {
                    buf.extend_from_slice(&data);

                    let mut transfers = self.transfers.write().await;
                    if let Some(t) = transfers.get_mut(&session_id) {
                        let transferred = buf.len() as u64;
                        let total = t.file.file_size;
                        let progress = if total > 0 { ((transferred * 100) / total) as u8 } else { 100 };
                        t.state = TransferState::InProgress {
                            transferred_bytes: transferred,
                            total_bytes: total,
                            progress_percent: progress,
                        };
                    }
                }
                Ok(None)
            }
            FileTransferMessage::Complete { session_id, file_id, sha256 } => {
                let mut buffers = self.incoming_buffers.write().await;
                if let Some(buf) = buffers.remove(&session_id) {
                    let actual_hash = Self::hash_file(&buf);
                    ensure!(
                        actual_hash == sha256,
                        "SHA-256 integrity verification failed! Expected {}, got {}",
                        sha256,
                        actual_hash
                    );

                    let mut transfers = self.transfers.write().await;
                    if let Some(t) = transfers.get_mut(&session_id) {
                        if let Some(ref path) = t.target_path {
                            std::fs::write(path, &buf)?;
                            info!("File written successfully to {:?}", path);
                        }
                        t.state = TransferState::Completed;
                    }
                }
                Ok(None)
            }
            FileTransferMessage::Cancel { session_id, reason } => {
                let mut transfers = self.transfers.write().await;
                if let Some(t) = transfers.get_mut(&session_id) {
                    t.state = TransferState::Cancelled;
                    warn!("Transfer {} cancelled by peer: {}", session_id, reason);
                }
                self.incoming_buffers.write().await.remove(&session_id);
                Ok(None)
            }
        }
    }

    pub async fn list_transfers(&self) -> Vec<ActiveTransfer> {
        let t = self.transfers.read().await;
        t.values().cloned().collect()
    }

    pub fn hash_file(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_transfer_end_to_end() -> Result<()> {
        let linux_id = Uuid::new_v4();
        let android_id = Uuid::new_v4();

        let temp_sender_dir = std::env::temp_dir().join(format!("nova_file_sender_{}", Uuid::new_v4()));
        let temp_recv_dir = std::env::temp_dir().join(format!("nova_file_recv_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_sender_dir)?;
        std::fs::create_dir_all(&temp_recv_dir)?;

        let test_file = temp_sender_dir.join("whitepaper.pdf");
        let sample_data = b"Nova Decentralized Ecosystem Whitepaper Content -- High speed P2P transfer.";
        std::fs::write(&test_file, sample_data)?;

        let sender = FileTransferPlugin::new(linux_id, &temp_sender_dir);
        let receiver = FileTransferPlugin::new(android_id, &temp_recv_dir);

        // Sender prepares offer
        let (offer_msg, file_bytes) = sender.prepare_offer(android_id, &test_file, "application/pdf").await?;

        // Receiver receives offer -> responds with Accept
        let decision_msg = receiver.handle_message(linux_id, offer_msg).await?.expect("decision expected");

        // Sender processes decision
        sender.handle_message(android_id, decision_msg).await?;

        let session_id = match sender.list_transfers().await.first() {
            Some(t) => t.session_id,
            None => panic!("Missing transfer"),
        };
        let file_id = sender.list_transfers().await.first().unwrap().file.file_id;

        // Sender chunks file and sends to Receiver
        let chunks = FileTransferPlugin::create_chunks(session_id, file_id, &file_bytes);
        for chunk in chunks {
            receiver.handle_message(linux_id, chunk).await?;
        }

        // Sender sends Complete
        let sha256 = FileTransferPlugin::hash_file(&file_bytes);
        let complete = FileTransferMessage::Complete { session_id, file_id, sha256 };
        receiver.handle_message(linux_id, complete).await?;

        // Verify receiver wrote file and matches
        let received_path = temp_recv_dir.join("whitepaper.pdf");
        assert!(received_path.exists());
        let received_bytes = std::fs::read(&received_path)?;
        assert_eq!(received_bytes, sample_data);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_sender_dir);
        let _ = std::fs::remove_dir_all(&temp_recv_dir);

        Ok(())
    }
}
