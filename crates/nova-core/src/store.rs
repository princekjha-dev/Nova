use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::device::{Device, DeviceId, Permission, Platform};

#[derive(Clone)]
pub struct DeviceStore {
    conn: Arc<Mutex<Connection>>,
    db_path: PathBuf,
}

impl DeviceStore {
    /// Opens or creates the SQLite database at `db_path`.
    pub fn open<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let path = db_path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)
            .with_context(|| format!("Failed to open SQLite database at {:?}", path))?;

        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: path,
        };
        store.init_schema()?;
        Ok(store)
    }

    /// In-memory database for testing.
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: PathBuf::from(":memory:"),
        };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS devices (
                id TEXT PRIMARY KEY,
                pubkey BLOB NOT NULL,
                fingerprint TEXT NOT NULL,
                name TEXT NOT NULL,
                platform TEXT NOT NULL,
                features TEXT NOT NULL DEFAULT '[]',
                trusted INTEGER NOT NULL DEFAULT 1,
                permissions TEXT NOT NULL DEFAULT '[]',
                last_seen TEXT NOT NULL,
                address_hints TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS clipboard_history (
                id TEXT PRIMARY KEY,
                source_device TEXT NOT NULL,
                content_type TEXT NOT NULL,
                content_text TEXT,
                content_blob BLOB,
                sensitive INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_clipboard_created ON clipboard_history(created_at DESC);

            CREATE TABLE IF NOT EXISTS file_transfers (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                direction TEXT NOT NULL,
                filename TEXT NOT NULL,
                mime_type TEXT NOT NULL,
                total_bytes INTEGER NOT NULL,
                transferred_bytes INTEGER NOT NULL DEFAULT 0,
                sha256 TEXT,
                state TEXT NOT NULL DEFAULT 'pending',
                source_device TEXT NOT NULL,
                target_path TEXT,
                created_at TEXT NOT NULL,
                completed_at TEXT
            );
            "#,
        )?;
        Ok(())
    }

    pub fn save_device(&self, device: &Device) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let id_str = device.id.to_string();
        let platform_str = serde_json::to_string(&device.platform)?;
        let features_str = serde_json::to_string(&device.features)?;
        let perms_str = serde_json::to_string(&device.permissions)?;
        let hints_str = serde_json::to_string(&device.address_hints)?;
        let last_seen_str = device.last_seen.to_rfc3339();
        let created_at_str = device.created_at.to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO devices (
                id, pubkey, fingerprint, name, platform, features, trusted, permissions, last_seen, address_hints, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                features = excluded.features,
                trusted = excluded.trusted,
                permissions = excluded.permissions,
                last_seen = excluded.last_seen,
                address_hints = excluded.address_hints
            "#,
            params![
                id_str,
                device.pubkey,
                device.fingerprint,
                device.name,
                platform_str,
                features_str,
                if device.trusted { 1 } else { 0 },
                perms_str,
                last_seen_str,
                hints_str,
                created_at_str
            ],
        )?;
        Ok(())
    }

    pub fn get_device(&self, id: &DeviceId) -> Result<Option<Device>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, pubkey, fingerprint, name, platform, features, trusted, permissions, last_seen, address_hints, created_at
            FROM devices WHERE id = ?1
            "#,
        )?;
        let id_str = id.to_string();
        let mut rows = stmt.query(params![id_str])?;

        if let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let pubkey: Vec<u8> = row.get(1)?;
            let fingerprint: String = row.get(2)?;
            let name: String = row.get(3)?;
            let platform_str: String = row.get(4)?;
            let features_str: String = row.get(5)?;
            let trusted_int: i32 = row.get(6)?;
            let perms_str: String = row.get(7)?;
            let last_seen_str: String = row.get(8)?;
            let hints_str: String = row.get(9)?;
            let created_at_str: String = row.get(10)?;

            let platform: Platform = serde_json::from_str(&platform_str).unwrap_or_default();
            let features: Vec<u16> = serde_json::from_str(&features_str).unwrap_or_default();
            let permissions: HashSet<Permission> = serde_json::from_str(&perms_str).unwrap_or_default();
            let address_hints = serde_json::from_str(&hints_str).unwrap_or_default();
            let last_seen = DateTime::parse_from_rfc3339(&last_seen_str)?.with_timezone(&Utc);
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc);

            Ok(Some(Device {
                id: Uuid::parse_str(&id)?,
                pubkey,
                fingerprint,
                name,
                platform,
                features,
                trusted: trusted_int != 0,
                permissions,
                last_seen,
                address_hints,
                created_at,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_devices(&self) -> Result<Vec<Device>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, pubkey, fingerprint, name, platform, features, trusted, permissions, last_seen, address_hints, created_at
            FROM devices ORDER BY last_seen DESC
            "#,
        )?;
        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let pubkey: Vec<u8> = row.get(1)?;
            let fingerprint: String = row.get(2)?;
            let name: String = row.get(3)?;
            let platform_str: String = row.get(4)?;
            let features_str: String = row.get(5)?;
            let trusted_int: i32 = row.get(6)?;
            let perms_str: String = row.get(7)?;
            let last_seen_str: String = row.get(8)?;
            let hints_str: String = row.get(9)?;
            let created_at_str: String = row.get(10)?;

            Ok((
                id_str,
                pubkey,
                fingerprint,
                name,
                platform_str,
                features_str,
                trusted_int,
                perms_str,
                last_seen_str,
                hints_str,
                created_at_str,
            ))
        })?;

        let mut devices = Vec::new();
        for r in rows {
            let (id, pubkey, fingerprint, name, platform_str, features_str, trusted_int, perms_str, last_seen_str, hints_str, created_at_str) = r?;
            if let Ok(id) = Uuid::parse_str(&id) {
                let platform: Platform = serde_json::from_str(&platform_str).unwrap_or_default();
                let features: Vec<u16> = serde_json::from_str(&features_str).unwrap_or_default();
                let permissions: HashSet<Permission> = serde_json::from_str(&perms_str).unwrap_or_default();
                let address_hints = serde_json::from_str(&hints_str).unwrap_or_default();
                let last_seen = DateTime::parse_from_rfc3339(&last_seen_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                devices.push(Device {
                    id,
                    pubkey,
                    fingerprint,
                    name,
                    platform,
                    features,
                    trusted: trusted_int != 0,
                    permissions,
                    last_seen,
                    address_hints,
                    created_at,
                });
            }
        }
        Ok(devices)
    }

    pub fn delete_device(&self, id: &DeviceId) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM devices WHERE id = ?1", params![id.to_string()])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_store_crud() -> Result<()> {
        let store = DeviceStore::in_memory()?;
        let id = Uuid::new_v4();
        let device = Device::new(
            id,
            vec![1, 2, 3],
            "1234-5678".to_string(),
            "Pixel 9 Pro".to_string(),
            Platform::Android,
            vec![1, 2, 3],
        );

        store.save_device(&device)?;

        let loaded = store.get_device(&id)?.expect("device should exist");
        assert_eq!(loaded.name, "Pixel 9 Pro");
        assert_eq!(loaded.platform, Platform::Android);
        assert!(loaded.trusted);

        let list = store.list_devices()?;
        assert_eq!(list.len(), 1);

        store.delete_device(&id)?;
        assert!(store.get_device(&id)?.is_none());

        Ok(())
    }
}
