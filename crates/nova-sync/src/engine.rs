use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::crdt::CrdtDoc;
use crate::protocol::{ClientId, DocDelta, StateVector, SyncMessage};

#[derive(Clone)]
pub struct SyncEngine {
    client_id: ClientId,
    docs: Arc<RwLock<HashMap<Uuid, CrdtDoc>>>,
}

impl SyncEngine {
    pub fn new(client_id: ClientId) -> Self {
        Self {
            client_id,
            docs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates or opens a CRDT document.
    pub async fn get_or_create_doc(&self, doc_id: Uuid) -> CrdtDoc {
        let mut docs = self.docs.write().await;
        docs.entry(doc_id)
            .or_insert_with(|| CrdtDoc::new(doc_id, self.client_id))
            .clone()
    }

    /// Insert text in a document and returns the delta to broadcast.
    pub async fn insert_text(&self, doc_id: Uuid, pos: usize, text: &str) -> Vec<DocDelta> {
        let mut docs = self.docs.write().await;
        let doc = docs.entry(doc_id).or_insert_with(|| CrdtDoc::new(doc_id, self.client_id));
        doc.insert_text(pos, text)
    }

    /// Read document text content.
    pub async fn read_doc_text(&self, doc_id: Uuid) -> Option<String> {
        let docs = self.docs.read().await;
        docs.get(&doc_id).map(|d| d.read_text())
    }

    /// Generate SyncStep1 message to initiate sync with remote peer.
    pub async fn create_sync_step1(&self, doc_id: Uuid) -> SyncMessage {
        let mut docs = self.docs.write().await;
        let doc = docs.entry(doc_id).or_insert_with(|| CrdtDoc::new(doc_id, self.client_id));
        SyncMessage::SyncStep1 {
            doc_id,
            state_vector: doc.state_vector.clone(),
        }
    }

    /// Handle incoming SyncStep1 and respond with SyncStep2 containing missing updates.
    pub async fn handle_sync_step1(&self, doc_id: Uuid, remote_vector: StateVector) -> Option<SyncMessage> {
        let docs = self.docs.read().await;
        if let Some(doc) = docs.get(&doc_id) {
            let updates = doc.get_missing_updates(&remote_vector);
            if !updates.is_empty() {
                return Some(SyncMessage::SyncStep2 { doc_id, updates });
            }
        }
        None
    }

    /// Handle incoming SyncStep2 updates from remote peer.
    pub async fn handle_sync_step2(&self, doc_id: Uuid, updates: Vec<DocDelta>) {
        let mut docs = self.docs.write().await;
        let doc = docs.entry(doc_id).or_insert_with(|| CrdtDoc::new(doc_id, self.client_id));
        for delta in updates {
            doc.apply_delta(&delta);
        }
    }

    /// Handle an incremental real-time update.
    pub async fn handle_update(&self, doc_id: Uuid, delta: DocDelta) {
        let mut docs = self.docs.write().await;
        let doc = docs.entry(doc_id).or_insert_with(|| CrdtDoc::new(doc_id, self.client_id));
        doc.apply_delta(&delta);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_engine_handshake() -> Result<()> {
        let engine_linux = SyncEngine::new(101);
        let engine_android = SyncEngine::new(202);
        let note_id = Uuid::new_v4();

        // Linux creates note content
        engine_linux.insert_text(note_id, 0, "# Cross-Device Architecture Notes").await;

        // Android connects and sends SyncStep1
        let step1 = engine_android.create_sync_step1(note_id).await;
        let (doc_id, remote_sv) = match step1 {
            SyncMessage::SyncStep1 { doc_id, state_vector } => (doc_id, state_vector),
            _ => panic!("Expected SyncStep1"),
        };

        // Linux produces SyncStep2
        let step2 = engine_linux.handle_sync_step1(doc_id, remote_sv).await.expect("updates expected");
        let (doc_id, updates) = match step2 {
            SyncMessage::SyncStep2 { doc_id, updates } => (doc_id, updates),
            _ => panic!("Expected SyncStep2"),
        };

        // Android applies updates
        engine_android.handle_sync_step2(doc_id, updates).await;

        // Verify Android has exact same text!
        let android_text = engine_android.read_doc_text(note_id).await.expect("text should exist");
        assert_eq!(android_text, "# Cross-Device Architecture Notes");

        Ok(())
    }
}
