use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use nova_core::device::DeviceId;
use nova_core::protocol::{NovaMessage, PLUGIN_HANDOFF};
use nova_crypto::DeviceKeys;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum TaskContent {
    Url { url: String, title: String },
    BrowserTab { url: String, scroll_y: u32, title: String },
    NoteDraft { title: String, content: String },
    FilePreview { filename: String, path_or_url: String },
    AiPrompt { prompt: String, context_summary: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HandoffTask {
    pub id: Uuid,
    pub source_device: DeviceId,
    pub title: String,
    pub content: TaskContent,
    pub created_at: DateTime<Utc>,
    pub consumed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HandoffMessage {
    HandoffRequest { task: HandoffTask },
    HandoffAck { task_id: Uuid, accepted: bool },
}

impl HandoffMessage {
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        rmp_serde::to_vec_named(self).map_err(|e| anyhow!("Handoff serialize error: {}", e))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        rmp_serde::from_slice(bytes).map_err(|e| anyhow!("Handoff deserialize error: {}", e))
    }
}

pub struct HandoffPlugin {
    local_device_id: DeviceId,
    tasks: Arc<RwLock<HashMap<Uuid, HandoffTask>>>,
}

impl HandoffPlugin {
    pub fn new(local_device_id: DeviceId) -> Self {
        Self {
            local_device_id,
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create and dispatch a task handoff to a target device.
    pub async fn create_handoff(
        &self,
        title: &str,
        content: TaskContent,
        target_device_id: DeviceId,
        keys: &DeviceKeys,
    ) -> Result<(HandoffTask, NovaMessage)> {
        let task = HandoffTask {
            id: Uuid::new_v4(),
            source_device: self.local_device_id,
            title: title.to_string(),
            content,
            created_at: Utc::now(),
            consumed: false,
        };

        self.tasks.write().await.insert(task.id, task.clone());

        let msg_payload = HandoffMessage::HandoffRequest { task: task.clone() };
        let wire_msg = NovaMessage::create(
            keys,
            target_device_id,
            PLUGIN_HANDOFF,
            msg_payload.to_bytes()?,
        )?;

        Ok((task, wire_msg))
    }

    /// Handle incoming handoff task.
    pub async fn handle_message(&self, msg: &NovaMessage) -> Result<Option<HandoffTask>> {
        let payload = HandoffMessage::from_bytes(&msg.payload)?;
        match payload {
            HandoffMessage::HandoffRequest { task } => {
                self.tasks.write().await.insert(task.id, task.clone());
                Ok(Some(task))
            }
            HandoffMessage::HandoffAck { task_id, accepted } => {
                let mut tasks = self.tasks.write().await;
                if let Some(t) = tasks.get_mut(&task_id) {
                    t.consumed = accepted;
                }
                Ok(None)
            }
        }
    }

    pub async fn list_tasks(&self) -> Vec<HandoffTask> {
        let t = self.tasks.read().await;
        t.values().cloned().collect()
    }

    pub async fn consume_task(&self, task_id: Uuid) -> Option<HandoffTask> {
        let mut tasks = self.tasks.write().await;
        if let Some(t) = tasks.get_mut(&task_id) {
            t.consumed = true;
            return Some(t.clone());
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_handoff_workflow() -> Result<()> {
        let linux_keys = DeviceKeys::generate();
        let phone_keys = DeviceKeys::generate();

        let linux_plugin = HandoffPlugin::new(linux_keys.device_id);
        let phone_plugin = HandoffPlugin::new(phone_keys.device_id);

        let (task, wire_msg) = linux_plugin.create_handoff(
            "Research: WebRTC",
            TaskContent::Url {
                url: "https://webrtc.org".to_string(),
                title: "WebRTC Standards".to_string(),
            },
            phone_keys.device_id,
            &linux_keys,
        ).await?;

        let received_task = phone_plugin.handle_message(&wire_msg).await?.expect("task received");
        assert_eq!(received_task.id, task.id);
        assert_eq!(received_task.title, "Research: WebRTC");

        // Phone consumes task
        let consumed = phone_plugin.consume_task(task.id).await.expect("consumed");
        assert!(consumed.consumed);

        Ok(())
    }
}
