use anyhow::Result;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::device::{Device, DeviceId, Permission};
use crate::store::DeviceStore;

#[derive(Clone)]
pub struct DeviceRegistry {
    store: DeviceStore,
    cache: Arc<RwLock<HashMap<DeviceId, Device>>>,
}

impl DeviceRegistry {
    pub async fn new(store: DeviceStore) -> Result<Self> {
        let devices = store.list_devices()?;
        let mut cache = HashMap::new();
        for dev in devices {
            cache.insert(dev.id, dev);
        }
        Ok(Self {
            store,
            cache: Arc::new(RwLock::new(cache)),
        })
    }

    /// Registers or updates a paired device.
    pub async fn register_device(&self, device: Device) -> Result<()> {
        self.store.save_device(&device)?;
        let mut cache = self.cache.write().await;
        cache.insert(device.id, device);
        Ok(())
    }

    pub async fn get_device(&self, id: &DeviceId) -> Option<Device> {
        let cache = self.cache.read().await;
        cache.get(id).cloned()
    }

    pub async fn list_devices(&self) -> Vec<Device> {
        let cache = self.cache.read().await;
        cache.values().cloned().collect()
    }

    pub async fn revoke_device(&self, id: &DeviceId) -> Result<bool> {
        let mut cache = self.cache.write().await;
        if let Some(dev) = cache.get_mut(id) {
            dev.trusted = false;
            self.store.save_device(dev)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn set_permissions(&self, id: &DeviceId, permissions: HashSet<Permission>) -> Result<bool> {
        let mut cache = self.cache.write().await;
        if let Some(dev) = cache.get_mut(id) {
            dev.permissions = permissions;
            self.store.save_device(dev)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn update_last_seen(&self, id: &DeviceId, addr: Option<SocketAddr>) -> Result<()> {
        let mut cache = self.cache.write().await;
        if let Some(dev) = cache.get_mut(id) {
            dev.last_seen = Utc::now();
            if let Some(addr) = addr {
                if !dev.address_hints.contains(&addr) {
                    dev.address_hints.push(addr);
                }
            }
            self.store.save_device(dev)?;
        }
        Ok(())
    }

    pub async fn is_trusted(&self, id: &DeviceId) -> bool {
        let cache = self.cache.read().await;
        cache.get(id).map(|d| d.trusted).unwrap_or(false)
    }

    pub async fn has_permission(&self, id: &DeviceId, perm: Permission) -> bool {
        let cache = self.cache.read().await;
        cache.get(id).map(|d| d.has_permission(perm)).unwrap_or(false)
    }

    pub async fn delete_device(&self, id: &DeviceId) -> Result<()> {
        self.store.delete_device(id)?;
        let mut cache = self.cache.write().await;
        cache.remove(id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::Platform;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_registry_operations() -> Result<()> {
        let store = DeviceStore::in_memory()?;
        let registry = DeviceRegistry::new(store).await?;

        let id = Uuid::new_v4();
        let dev = Device::new(
            id,
            vec![4, 5, 6],
            "9999-0000".to_string(),
            "ThinkPad X1".to_string(),
            Platform::Linux,
            vec![1, 2],
        );

        registry.register_device(dev.clone()).await?;

        let fetched = registry.get_device(&id).await.expect("found");
        assert_eq!(fetched.name, "ThinkPad X1");
        assert!(registry.is_trusted(&id).await);
        assert!(registry.has_permission(&id, Permission::ClipboardRead).await);

        // Revoke
        assert!(registry.revoke_device(&id).await?);
        assert!(!registry.is_trusted(&id).await);
        assert!(!registry.has_permission(&id, Permission::ClipboardRead).await);

        Ok(())
    }
}
