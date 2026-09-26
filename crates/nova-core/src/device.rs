use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::SocketAddr;
use uuid::Uuid;

pub type DeviceId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Linux,
    Android,
    Unknown,
}

impl Default for Platform {
    fn default() -> Self {
        Platform::Unknown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    ClipboardRead,
    ClipboardWrite,
    NotesRead,
    NotesWrite,
    FileTransfer,
    ScreenMirror,
    RemoteControl,
    TaskHandoff,
    AiContext,
}

impl Permission {
    pub fn all() -> HashSet<Permission> {
        let mut set = HashSet::new();
        set.insert(Permission::ClipboardRead);
        set.insert(Permission::ClipboardWrite);
        set.insert(Permission::NotesRead);
        set.insert(Permission::NotesWrite);
        set.insert(Permission::FileTransfer);
        set.insert(Permission::ScreenMirror);
        set.insert(Permission::RemoteControl);
        set.insert(Permission::TaskHandoff);
        set.insert(Permission::AiContext);
        set
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Device {
    pub id: DeviceId,
    pub pubkey: Vec<u8>,
    pub fingerprint: String,
    pub name: String,
    pub platform: Platform,
    pub features: Vec<u16>,
    pub trusted: bool,
    pub permissions: HashSet<Permission>,
    pub last_seen: DateTime<Utc>,
    pub address_hints: Vec<SocketAddr>,
    pub created_at: DateTime<Utc>,
}

impl Device {
    pub fn new(
        id: DeviceId,
        pubkey: Vec<u8>,
        fingerprint: String,
        name: String,
        platform: Platform,
        features: Vec<u16>,
    ) -> Self {
        Self {
            id,
            pubkey,
            fingerprint,
            name,
            platform,
            features,
            trusted: true,
            permissions: Permission::all(),
            last_seen: Utc::now(),
            address_hints: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn has_permission(&self, perm: Permission) -> bool {
        self.trusted && self.permissions.contains(&perm)
    }

    pub fn grant_permission(&mut self, perm: Permission) {
        self.permissions.insert(perm);
    }

    pub fn revoke_permission(&mut self, perm: Permission) {
        self.permissions.remove(&perm);
    }
}
