use anyhow::{anyhow, ensure, Result};
use chrono::{DateTime, Utc};
use nova_core::device::DeviceId;
use nova_core::protocol::{NovaMessage, PLUGIN_REMOTE};
use nova_crypto::DeviceKeys;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TouchPhase {
    Down,
    Move,
    Up,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InputEvent {
    MouseMove { x: i32, y: i32 },
    MouseButton { button: u8, pressed: bool },
    MouseScroll { dx: i32, dy: i32 },
    KeyPress { keycode: u32, pressed: bool },
    TouchEvent { touch_id: u64, x: f32, y: f32, phase: TouchPhase },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteControlState {
    Requested,
    Authorized,
    Rejected,
    Terminated,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RemoteControlMessage {
    AuthRequest {
        session_id: Uuid,
        device_name: String,
    },
    AuthResponse {
        session_id: Uuid,
        authorized: bool,
    },
    InputBatch {
        session_id: Uuid,
        events: Vec<InputEvent>,
    },
    Terminate {
        session_id: Uuid,
        reason: String,
    },
}

impl RemoteControlMessage {
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        rmp_serde::to_vec_named(self).map_err(|e| anyhow!("RemoteMessage serialize error: {}", e))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        rmp_serde::from_slice(bytes).map_err(|e| anyhow!("RemoteMessage deserialize error: {}", e))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSession {
    pub session_id: Uuid,
    pub peer_device_id: DeviceId,
    pub peer_device_name: String,
    pub state: RemoteControlState,
    pub created_at: DateTime<Utc>,
}

pub struct RemoteControlPlugin {
    local_device_id: DeviceId,
    sessions: Arc<RwLock<HashMap<Uuid, RemoteSession>>>,
    // Optional callback when input is injected
    input_sink: Option<Arc<dyn Fn(InputEvent) + Send + Sync>>,
}

impl RemoteControlPlugin {
    pub fn new(local_device_id: DeviceId) -> Self {
        Self {
            local_device_id,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            input_sink: None,
        }
    }

    pub fn set_input_sink<F>(&mut self, f: F)
    where
        F: Fn(InputEvent) + Send + Sync + 'static,
    {
        self.input_sink = Some(Arc::new(f));
    }

    /// Client requests remote control authorization.
    pub async fn request_session(
        &self,
        target_device: DeviceId,
        device_name: &str,
        keys: &DeviceKeys,
    ) -> Result<(RemoteSession, NovaMessage)> {
        let session_id = Uuid::new_v4();
        let session = RemoteSession {
            session_id,
            peer_device_id: target_device,
            peer_device_name: device_name.to_string(),
            state: RemoteControlState::Requested,
            created_at: Utc::now(),
        };

        self.sessions.write().await.insert(session_id, session.clone());

        let payload = RemoteControlMessage::AuthRequest {
            session_id,
            device_name: device_name.to_string(),
        };

        let msg = NovaMessage::create(
            keys,
            target_device,
            PLUGIN_REMOTE,
            payload.to_bytes()?,
        )?;

        Ok((session, msg))
    }

    /// Host user explicitly grants or rejects authorization via UI modal.
    pub async fn authorize_session(&self, session_id: Uuid, authorized: bool) -> Result<RemoteControlMessage> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.state = if authorized {
                RemoteControlState::Authorized
            } else {
                RemoteControlState::Rejected
            };
            info!("Remote control session {} authorization set to {}", session_id, authorized);
            Ok(RemoteControlMessage::AuthResponse {
                session_id,
                authorized,
            })
        } else {
            Err(anyhow!("Session {} not found", session_id))
        }
    }

    /// Handle incoming messages from peer.
    pub async fn handle_message(&self, peer_id: DeviceId, msg: &NovaMessage) -> Result<Option<RemoteControlMessage>> {
        let payload = RemoteControlMessage::from_bytes(&msg.payload)?;
        match payload {
            RemoteControlMessage::AuthRequest { session_id, device_name } => {
                info!("Remote control requested by {} ({})", device_name, peer_id);
                let session = RemoteSession {
                    session_id,
                    peer_device_id: peer_id,
                    peer_device_name: device_name,
                    state: RemoteControlState::Requested, // Awaiting explicit user confirmation!
                    created_at: Utc::now(),
                };
                self.sessions.write().await.insert(session_id, session);
                Ok(None)
            }
            RemoteControlMessage::AuthResponse { session_id, authorized } => {
                let mut sessions = self.sessions.write().await;
                if let Some(s) = sessions.get_mut(&session_id) {
                    s.state = if authorized {
                        RemoteControlState::Authorized
                    } else {
                        RemoteControlState::Rejected
                    };
                }
                Ok(None)
            }
            RemoteControlMessage::InputBatch { session_id, events } => {
                let sessions = self.sessions.read().await;
                let is_auth = sessions.get(&session_id)
                    .map(|s| s.state == RemoteControlState::Authorized)
                    .unwrap_or(false);

                // Security guarantee: Reject input from unapproved sessions
                ensure!(is_auth, "Unauthorized input event rejected! Session {} not approved", session_id);

                if let Some(ref sink) = self.input_sink {
                    for ev in events {
                        sink(ev);
                    }
                }
                Ok(None)
            }
            RemoteControlMessage::Terminate { session_id, .. } => {
                let mut sessions = self.sessions.write().await;
                if let Some(s) = sessions.get_mut(&session_id) {
                    s.state = RemoteControlState::Terminated;
                }
                Ok(None)
            }
        }
    }

    pub async fn get_session(&self, id: &Uuid) -> Option<RemoteSession> {
        let s = self.sessions.read().await;
        s.get(id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_remote_control_explicit_authorization_security() -> Result<()> {
        let linux_keys = DeviceKeys::generate();
        let phone_keys = DeviceKeys::generate();

        let linux_plugin = RemoteControlPlugin::new(linux_keys.device_id);
        let phone_plugin = RemoteControlPlugin::new(phone_keys.device_id);

        // 1. Phone requests remote control
        let (session, req_msg) = phone_plugin.request_session(
            linux_keys.device_id,
            "Pixel 9 Pro",
            &phone_keys,
        ).await?;

        // 2. Linux receives request
        linux_plugin.handle_message(phone_keys.device_id, &req_msg).await?;

        // 3. Attempting to inject input BEFORE authorization MUST FAIL
        let unauth_input = RemoteControlMessage::InputBatch {
            session_id: session.session_id,
            events: vec![InputEvent::MouseMove { x: 100, y: 100 }],
        };
        let unauth_wire = NovaMessage::create(
            &phone_keys,
            linux_keys.device_id,
            PLUGIN_REMOTE,
            unauth_input.to_bytes()?,
        )?;
        assert!(linux_plugin.handle_message(phone_keys.device_id, &unauth_wire).await.is_err());

        // 4. Linux user clicks [Allow]
        let auth_resp = linux_plugin.authorize_session(session.session_id, true).await?;
        let auth_wire = NovaMessage::create(
            &linux_keys,
            phone_keys.device_id,
            PLUGIN_REMOTE,
            auth_resp.to_bytes()?,
        )?;
        phone_plugin.handle_message(linux_keys.device_id, &auth_wire).await?;

        // 5. Input now accepted!
        assert!(linux_plugin.handle_message(phone_keys.device_id, &unauth_wire).await.is_ok());

        Ok(())
    }
}
