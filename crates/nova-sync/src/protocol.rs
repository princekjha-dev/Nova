use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub type ClientId = u64;
pub type Clock = u64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct StateVector {
    pub clocks: HashMap<ClientId, Clock>,
}

impl StateVector {
    pub fn new() -> Self {
        Self {
            clocks: HashMap::new(),
        }
    }

    pub fn set(&mut self, client_id: ClientId, clock: Clock) {
        self.clocks.insert(client_id, clock);
    }

    pub fn get(&self, client_id: ClientId) -> Clock {
        self.clocks.get(&client_id).copied().unwrap_or(0)
    }

    pub fn is_ahead_of(&self, other: &StateVector) -> bool {
        self.clocks.iter().any(|(&client, &clock)| clock > other.get(client))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncMessage {
    /// Step 1: Exchange local state vector to request missing updates
    SyncStep1 {
        doc_id: Uuid,
        state_vector: StateVector,
    },
    /// Step 2: Respond with missing updates needed by remote peer
    SyncStep2 {
        doc_id: Uuid,
        updates: Vec<DocDelta>,
    },
    /// Incremental real-time update broadcast
    SyncUpdate {
        doc_id: Uuid,
        delta: DocDelta,
    },
    /// Cursor / Presence information for infinite collaboration
    Awareness {
        doc_id: Uuid,
        client_id: ClientId,
        user_name: String,
        cursor_pos: Option<usize>,
        active: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocDelta {
    pub client_id: ClientId,
    pub clock: Clock,
    pub op: DeltaOp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeltaOp {
    InsertText { pos: usize, text: String },
    DeleteText { pos: usize, len: usize },
    SetMetadata { key: String, value: String },
}

impl SyncMessage {
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        rmp_serde::to_vec_named(self).map_err(|e| anyhow!("SyncMessage serialize error: {}", e))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        rmp_serde::from_slice(bytes).map_err(|e| anyhow!("SyncMessage deserialize error: {}", e))
    }
}
