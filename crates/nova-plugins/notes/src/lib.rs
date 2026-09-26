use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use nova_core::device::DeviceId;
use nova_core::protocol::{NovaMessage, PLUGIN_NOTES};
use nova_crypto::DeviceKeys;
use nova_sync::{DocDelta, SyncEngine, SyncMessage};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Note {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub folder: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted: bool,
}

pub struct NotesPlugin {
    sync_engine: SyncEngine,
    conn: Arc<Mutex<Connection>>,
}

impl NotesPlugin {
    pub fn new<P: AsRef<Path>>(db_path: P, client_id: u64) -> Result<Self> {
        let path = db_path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let plugin = Self {
            sync_engine: SyncEngine::new(client_id),
            conn: Arc::new(Mutex::new(conn)),
        };
        plugin.init_schema()?;
        Ok(plugin)
    }

    pub fn in_memory(client_id: u64) -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let plugin = Self {
            sync_engine: SyncEngine::new(client_id),
            conn: Arc::new(Mutex::new(conn)),
        };
        plugin.init_schema()?;
        Ok(plugin)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                folder TEXT NOT NULL DEFAULT 'General',
                tags TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                deleted INTEGER NOT NULL DEFAULT 0
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
                id UNINDEXED,
                title,
                content,
                content='notes',
                content_rowid='rowid'
            );
            "#,
        )?;
        Ok(())
    }

    pub async fn create_note(&self, title: &str, content: &str, folder: &str, tags: Vec<String>) -> Result<Note> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        // Initialize in CRDT engine
        let _ = self.sync_engine.insert_text(id, 0, content).await;

        let note = Note {
            id,
            title: title.to_string(),
            content: content.to_string(),
            folder: folder.to_string(),
            tags,
            created_at: now,
            updated_at: now,
            deleted: false,
        };

        self.save_note_db(&note)?;
        Ok(note)
    }

    pub async fn update_note_text(&self, id: Uuid, new_content: &str) -> Result<(Note, Vec<DocDelta>)> {
        let mut note = self.get_note(id)?.context("Note not found")?;

        // Compute deltas in CRDT
        let deltas = self.sync_engine.insert_text(id, 0, new_content).await;
        note.content = new_content.to_string();
        note.updated_at = Utc::now();

        self.save_note_db(&note)?;
        Ok((note, deltas))
    }

    pub fn get_note(&self, id: Uuid) -> Result<Option<Note>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, content, folder, tags, created_at, updated_at, deleted FROM notes WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id.to_string()])?;

        if let Some(row) = rows.next()? {
            let id_str: String = row.get(0)?;
            let title: String = row.get(1)?;
            let content: String = row.get(2)?;
            let folder: String = row.get(3)?;
            let tags_str: String = row.get(4)?;
            let created_str: String = row.get(5)?;
            let updated_str: String = row.get(6)?;
            let deleted: i32 = row.get(7)?;

            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            let created_at = DateTime::parse_from_rfc3339(&created_str)?.with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)?.with_timezone(&Utc);

            Ok(Some(Note {
                id: Uuid::parse_str(&id_str)?,
                title,
                content,
                folder,
                tags,
                created_at,
                updated_at,
                deleted: deleted != 0,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_notes(&self, folder: Option<&str>) -> Result<Vec<Note>> {
        let conn = self.conn.lock().unwrap();
        let mut query = "SELECT id, title, content, folder, tags, created_at, updated_at, deleted FROM notes WHERE deleted = 0".to_string();
        if folder.is_some() {
            query.push_str(" AND folder = ?1");
        }
        query.push_str(" ORDER BY updated_at DESC");

        let mut stmt = conn.prepare(&query)?;
        let rows = if let Some(f) = folder {
            stmt.query(params![f])?
        } else {
            stmt.query([])?
        };

        let mut notes = Vec::new();
        let mut rows_iter = rows;
        while let Some(row) = rows_iter.next()? {
            let id_str: String = row.get(0)?;
            let title: String = row.get(1)?;
            let content: String = row.get(2)?;
            let folder: String = row.get(3)?;
            let tags_str: String = row.get(4)?;
            let created_str: String = row.get(5)?;
            let updated_str: String = row.get(6)?;
            let deleted: i32 = row.get(7)?;

            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            if let Ok(id) = Uuid::parse_str(&id_str) {
                notes.push(Note {
                    id,
                    title,
                    content,
                    folder,
                    tags,
                    created_at,
                    updated_at,
                    deleted: deleted != 0,
                });
            }
        }
        Ok(notes)
    }

    pub fn search_notes(&self, query: &str) -> Result<Vec<Note>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT n.id, n.title, n.content, n.folder, n.tags, n.created_at, n.updated_at, n.deleted
            FROM notes n
            JOIN notes_fts f ON n.id = f.id
            WHERE notes_fts MATCH ?1 AND n.deleted = 0
            ORDER BY rank
            "#,
        )?;

        let mut notes = Vec::new();
        // Safe FTS query construction: match term tokens
        let fts_query = format!("\"{}\"*", query.replace('"', ""));
        let mut rows = stmt.query(params![fts_query])?;

        while let Some(row) = rows.next()? {
            let id_str: String = row.get(0)?;
            let title: String = row.get(1)?;
            let content: String = row.get(2)?;
            let folder: String = row.get(3)?;
            let tags_str: String = row.get(4)?;
            let created_str: String = row.get(5)?;
            let updated_str: String = row.get(6)?;
            let deleted: i32 = row.get(7)?;

            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            if let Ok(id) = Uuid::parse_str(&id_str) {
                notes.push(Note {
                    id,
                    title,
                    content,
                    folder,
                    tags,
                    created_at,
                    updated_at,
                    deleted: deleted != 0,
                });
            }
        }

        // Fallback to LIKE if FTS yielded empty
        if notes.is_empty() {
            let mut stmt_like = conn.prepare(
                "SELECT id, title, content, folder, tags, created_at, updated_at, deleted FROM notes WHERE (title LIKE ?1 OR content LIKE ?1) AND deleted = 0",
            )?;
            let pattern = format!("%{}%", query);
            let mut rows = stmt_like.query(params![pattern])?;
            while let Some(row) = rows.next()? {
                let id_str: String = row.get(0)?;
                let title: String = row.get(1)?;
                let content: String = row.get(2)?;
                let folder: String = row.get(3)?;
                let tags_str: String = row.get(4)?;
                let created_str: String = row.get(5)?;
                let updated_str: String = row.get(6)?;
                let deleted: i32 = row.get(7)?;

                let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
                let created_at = DateTime::parse_from_rfc3339(&created_str).unwrap().with_timezone(&Utc);
                let updated_at = DateTime::parse_from_rfc3339(&updated_str).unwrap().with_timezone(&Utc);

                if let Ok(id) = Uuid::parse_str(&id_str) {
                    notes.push(Note {
                        id,
                        title,
                        content,
                        folder,
                        tags,
                        created_at,
                        updated_at,
                        deleted: deleted != 0,
                    });
                }
            }
        }

        Ok(notes)
    }

    fn save_note_db(&self, note: &Note) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tags_str = serde_json::to_string(&note.tags)?;
        let created_str = note.created_at.to_rfc3339();
        let updated_str = note.updated_at.to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO notes (id, title, content, folder, tags, created_at, updated_at, deleted)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                content = excluded.content,
                folder = excluded.folder,
                tags = excluded.tags,
                updated_at = excluded.updated_at,
                deleted = excluded.deleted
            "#,
            params![
                note.id.to_string(),
                note.title,
                note.content,
                note.folder,
                tags_str,
                created_str,
                updated_str,
                if note.deleted { 1 } else { 0 },
            ],
        )?;

        // Update FTS index
        let _ = conn.execute(
            "INSERT OR REPLACE INTO notes_fts (id, title, content) VALUES (?1, ?2, ?3)",
            params![note.id.to_string(), note.title, note.content],
        );

        Ok(())
    }

    pub fn sync_engine(&self) -> &SyncEngine {
        &self.sync_engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_notes_crud_and_fts_search() -> Result<()> {
        let plugin = NotesPlugin::in_memory(1)?;

        let note = plugin.create_note(
            "WebRTC Research",
            "# WebRTC Protocol\nLow latency P2P video and audio transport.",
            "Research",
            vec!["networking".to_string(), "video".to_string()],
        ).await?;

        assert_eq!(note.title, "WebRTC Research");
        assert_eq!(note.folder, "Research");

        let list = plugin.list_notes(Some("Research"))?;
        assert_eq!(list.len(), 1);

        // Full-text search
        let results = plugin.search_notes("latency")?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "WebRTC Research");

        Ok(())
    }
}
