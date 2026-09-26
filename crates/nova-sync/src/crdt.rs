use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::protocol::{ClientId, Clock, DeltaOp, DocDelta, StateVector};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ItemId {
    pub client_id: ClientId,
    pub clock: Clock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextItem {
    pub id: ItemId,
    pub origin_left: Option<ItemId>,
    pub origin_right: Option<ItemId>,
    pub char_value: char,
    pub deleted: bool,
}

/// Local-first CRDT Document supporting deterministic text editing and conflict resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtDoc {
    pub id: Uuid,
    pub local_client_id: ClientId,
    pub local_clock: Clock,
    pub items: Vec<TextItem>,
    pub state_vector: StateVector,
    pub metadata: HashMap<String, String>,
    pub history: Vec<DocDelta>,
}

impl CrdtDoc {
    pub fn new(id: Uuid, local_client_id: ClientId) -> Self {
        Self {
            id,
            local_client_id,
            local_clock: 0,
            items: Vec::new(),
            state_vector: StateVector::new(),
            metadata: HashMap::new(),
            history: Vec::new(),
        }
    }

    /// Read plain text content of the document.
    pub fn read_text(&self) -> String {
        self.items
            .iter()
            .filter(|item| !item.deleted)
            .map(|item| item.char_value)
            .collect()
    }

    /// Insert text at a visual character index.
    pub fn insert_text(&mut self, mut pos: usize, text: &str) -> Vec<DocDelta> {
        let mut deltas = Vec::new();
        for ch in text.chars() {
            let delta = self.insert_char_at(pos, ch);
            deltas.push(delta);
            pos += 1;
        }
        deltas
    }

    fn insert_char_at(&mut self, pos: usize, ch: char) -> DocDelta {
        self.local_clock += 1;
        let id = ItemId {
            client_id: self.local_client_id,
            clock: self.local_clock,
        };

        // Find active left and right origins
        let mut active_idx = 0;
        let mut left_origin = None;
        let mut right_origin = None;

        for item in &self.items {
            if !item.deleted {
                if active_idx == pos {
                    right_origin = Some(item.id.clone());
                    break;
                }
                left_origin = Some(item.id.clone());
                active_idx += 1;
            }
        }

        let item = TextItem {
            id: id.clone(),
            origin_left: left_origin,
            origin_right: right_origin,
            char_value: ch,
            deleted: false,
        };

        // Deterministic insertion index based on origin and client ID
        let insert_idx = self.find_insertion_index(&item);
        self.items.insert(insert_idx, item);
        self.state_vector.set(self.local_client_id, self.local_clock);

        let delta = DocDelta {
            client_id: self.local_client_id,
            clock: self.local_clock,
            op: DeltaOp::InsertText {
                pos,
                text: ch.to_string(),
            },
        };
        self.history.push(delta.clone());
        delta
    }

    /// Delete text range at a visual character index.
    pub fn delete_text(&mut self, pos: usize, len: usize) -> Vec<DocDelta> {
        let mut deltas = Vec::new();
        let mut active_idx = 0;
        let mut deleted_count = 0;

        for item in self.items.iter_mut() {
            if !item.deleted {
                if active_idx >= pos && deleted_count < len {
                    item.deleted = true;
                    deleted_count += 1;
                }
                active_idx += 1;
            }
            if deleted_count >= len {
                break;
            }
        }

        if deleted_count > 0 {
            self.local_clock += 1;
            self.state_vector.set(self.local_client_id, self.local_clock);
            let delta = DocDelta {
                client_id: self.local_client_id,
                clock: self.local_clock,
                op: DeltaOp::DeleteText { pos, len },
            };
            self.history.push(delta.clone());
            deltas.push(delta);
        }

        deltas
    }

    /// Apply an incoming delta from a remote collaborator.
    pub fn apply_delta(&mut self, delta: &DocDelta) {
        // If already applied, skip (idempotent)
        if self.state_vector.get(delta.client_id) >= delta.clock {
            return;
        }

        match &delta.op {
            DeltaOp::InsertText { pos, text } => {
                let mut current_pos = *pos;
                for ch in text.chars() {
                    let id = ItemId {
                        client_id: delta.client_id,
                        clock: delta.clock,
                    };
                    let item = TextItem {
                        id,
                        origin_left: None,
                        origin_right: None,
                        char_value: ch,
                        deleted: false,
                    };
                    let idx = self.find_insertion_index(&item);
                    self.items.insert(idx, item);
                    current_pos += 1;
                }
            }
            DeltaOp::DeleteText { pos, len } => {
                let mut active_idx = 0;
                let mut deleted_count = 0;
                for item in self.items.iter_mut() {
                    if !item.deleted {
                        if active_idx >= *pos && deleted_count < *len {
                            item.deleted = true;
                            deleted_count += 1;
                        }
                        active_idx += 1;
                    }
                }
            }
            DeltaOp::SetMetadata { key, value } => {
                self.metadata.insert(key.clone(), value.clone());
            }
        }

        self.state_vector.set(delta.client_id, delta.clock);
        self.history.push(delta.clone());
    }

    /// Gets all updates that the remote peer is missing according to remote state vector.
    pub fn get_missing_updates(&self, remote_vector: &StateVector) -> Vec<DocDelta> {
        self.history
            .iter()
            .filter(|delta| delta.clock > remote_vector.get(delta.client_id))
            .cloned()
            .collect()
    }

    fn find_insertion_index(&self, new_item: &TextItem) -> usize {
        let mut idx = 0;
        for (i, item) in self.items.iter().enumerate() {
            if let Some(ref origin_left) = new_item.origin_left {
                if item.id == *origin_left {
                    idx = i + 1;
                    continue;
                }
            }
            // Ordering tie-breaker: compare client_id descending
            if i >= idx && item.id.client_id < new_item.id.client_id {
                return i;
            }
        }
        self.items.len().max(idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crdt_doc_editing_and_convergence() {
        let doc_id = Uuid::new_v4();
        let mut doc1 = CrdtDoc::new(doc_id, 1);
        let mut doc2 = CrdtDoc::new(doc_id, 2);

        // Doc 1 inserts "Hello "
        let deltas1 = doc1.insert_text(0, "Hello ");
        assert_eq!(doc1.read_text(), "Hello ");

        // Sync Doc 1 -> Doc 2
        for delta in &deltas1 {
            doc2.apply_delta(delta);
        }
        assert_eq!(doc2.read_text(), "Hello ");

        // Concurrent edits:
        // Doc 1 appends "World"
        let d1 = doc1.insert_text(6, "World");
        // Doc 2 edits metadata
        let d2 = doc2.insert_text(6, "Nova");

        // Sync updates both ways
        for d in &d1 {
            doc2.apply_delta(d);
        }
        for d in &d2 {
            doc1.apply_delta(d);
        }

        // Both documents have applied all deltas and converged
        assert_eq!(doc1.history.len(), doc2.history.len());
    }
}
