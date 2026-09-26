use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use nova_core::device::DeviceId;
use nova_core::protocol::{NovaMessage, PLUGIN_MIRROR};
use nova_crypto::DeviceKeys;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoCodec {
    H264,
    H265,
    AV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MirrorConfig {
    pub max_width: u32,
    pub max_height: u32,
    pub fps: u32,
    pub bitrate_bps: u32,
    pub codec: VideoCodec,
}

impl Default for MirrorConfig {
    fn default() -> Self {
        Self {
            max_width: 1920,
            max_height: 1080,
            fps: 60,
            bitrate_bps: 8_000_000, // 8 Mbps
            codec: VideoCodec::H264,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MirrorState {
    Idle,
    Negotiating,
    Active { stream_port: u16 },
    Stopped,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MirrorMessage {
    StartRequest {
        session_id: Uuid,
        config: MirrorConfig,
    },
    StartResponse {
        session_id: Uuid,
        accepted: bool,
        stream_port: Option<u16>,
        error: Option<String>,
    },
    StopRequest {
        session_id: Uuid,
    },
    QualityUpdate {
        session_id: Uuid,
        bitrate_bps: u32,
        fps: u32,
    },
}

impl MirrorMessage {
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        rmp_serde::to_vec_named(self).map_err(|e| anyhow!("MirrorMessage serialize error: {}", e))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        rmp_serde::from_slice(bytes).map_err(|e| anyhow!("MirrorMessage deserialize error: {}", e))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorSession {
    pub session_id: Uuid,
    pub peer_device_id: DeviceId,
    pub config: MirrorConfig,
    pub state: MirrorState,
    pub started_at: DateTime<Utc>,
}

pub struct MirrorPlugin {
    local_device_id: DeviceId,
    sessions: Arc<RwLock<HashMap<Uuid, MirrorSession>>>,
}

impl MirrorPlugin {
    pub fn new(local_device_id: DeviceId) -> Self {
        Self {
            local_device_id,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initiate screen mirror session to remote device.
    pub async fn request_mirror_start(
        &self,
        target_device: DeviceId,
        config: MirrorConfig,
        keys: &DeviceKeys,
    ) -> Result<(MirrorSession, NovaMessage)> {
        let session_id = Uuid::new_v4();
        let session = MirrorSession {
            session_id,
            peer_device_id: target_device,
            config: config.clone(),
            state: MirrorState::Negotiating,
            started_at: Utc::now(),
        };

        self.sessions.write().await.insert(session_id, session.clone());

        let payload = MirrorMessage::StartRequest { session_id, config };
        let wire_msg = NovaMessage::create(
            keys,
            target_device,
            PLUGIN_MIRROR,
            payload.to_bytes()?,
        )?;

        Ok((session, wire_msg))
    }

    /// Handle incoming mirror messages.
    pub async fn handle_message(&self, peer_id: DeviceId, msg: &NovaMessage) -> Result<Option<MirrorMessage>> {
        let payload = MirrorMessage::from_bytes(&msg.payload)?;
        match payload {
            MirrorMessage::StartRequest { session_id, config } => {
                info!("Incoming mirror start request from device {}", peer_id);
                let stream_port = 53420; // Default local streaming socket port
                let session = MirrorSession {
                    session_id,
                    peer_device_id: peer_id,
                    config,
                    state: MirrorState::Active { stream_port },
                    started_at: Utc::now(),
                };
                self.sessions.write().await.insert(session_id, session);

                Ok(Some(MirrorMessage::StartResponse {
                    session_id,
                    accepted: true,
                    stream_port: Some(stream_port),
                    error: None,
                }))
            }
            MirrorMessage::StartResponse { session_id, accepted, stream_port, error } => {
                let mut sessions = self.sessions.write().await;
                if let Some(s) = sessions.get_mut(&session_id) {
                    s.state = if accepted {
                        MirrorState::Active { stream_port: stream_port.unwrap_or(53420) }
                    } else {
                        MirrorState::Failed(error.unwrap_or_else(|| "Rejected".to_string()))
                    };
                }
                Ok(None)
            }
            MirrorMessage::StopRequest { session_id } => {
                let mut sessions = self.sessions.write().await;
                if let Some(s) = sessions.get_mut(&session_id) {
                    s.state = MirrorState::Stopped;
                }
                Ok(None)
            }
            MirrorMessage::QualityUpdate { session_id, bitrate_bps, fps } => {
                let mut sessions = self.sessions.write().await;
                if let Some(s) = sessions.get_mut(&session_id) {
                    s.config.bitrate_bps = bitrate_bps;
                    s.config.fps = fps;
                }
                Ok(None)
            }
        }
    }

    pub async fn get_session(&self, id: &Uuid) -> Option<MirrorSession> {
        let s = self.sessions.read().await;
        s.get(id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mirror_negotiation() -> Result<()> {
        let linux_keys = DeviceKeys::generate();
        let phone_keys = DeviceKeys::generate();

        let linux_plugin = MirrorPlugin::new(linux_keys.device_id);
        let phone_plugin = MirrorPlugin::new(phone_keys.device_id);

        let (session, wire_msg) = linux_plugin.request_mirror_start(
            phone_keys.device_id,
            MirrorConfig::default(),
            &linux_keys,
        ).await?;

        let response = phone_plugin.handle_message(linux_keys.device_id, &wire_msg).await?.expect("response expected");
        match response {
            MirrorMessage::StartResponse { session_id, accepted, stream_port, .. } => {
                assert_eq!(session_id, session.session_id);
                assert!(accepted);
                assert!(stream_port.is_some());
            }
            _ => panic!("Expected StartResponse"),
        }

        Ok(())
    }
}
