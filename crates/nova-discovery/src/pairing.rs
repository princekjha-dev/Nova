use anyhow::{anyhow, ensure, Result};
use nova_core::device::{Device, Platform};
use nova_core::registry::DeviceRegistry;
use nova_crypto::DeviceKeys;
use rand::Rng;
use std::net::SocketAddr;
use std::sync::Arc;
use uuid::Uuid;

use crate::qr::QrPairingPayload;

pub struct PairingManager {
    registry: DeviceRegistry,
    keys: Arc<DeviceKeys>,
    local_name: String,
    local_platform: Platform,
    local_port: u16,
}

impl PairingManager {
    pub fn new(
        registry: DeviceRegistry,
        keys: Arc<DeviceKeys>,
        local_name: String,
        local_platform: Platform,
        local_port: u16,
    ) -> Self {
        Self {
            registry,
            keys,
            local_name,
            local_platform,
            local_port,
        }
    }

    /// Creates a QR pairing invitation payload with a random 6-digit PIN.
    pub fn create_pairing_invitation(
        &self,
        noise_pubkey_bytes: &[u8; 32],
        local_addresses: Vec<SocketAddr>,
    ) -> Result<QrPairingPayload> {
        let pin: u32 = rand::thread_rng().gen_range(100_000..999_999);
        let platform_str = match self.local_platform {
            Platform::Linux => "linux",
            Platform::Android => "android",
            Platform::Unknown => "unknown",
        };

        Ok(QrPairingPayload {
            device_id: self.keys.device_id,
            device_name: self.local_name.clone(),
            platform: platform_str.to_string(),
            pubkey_hex: hex::encode(self.keys.verifying_key.as_bytes()),
            fingerprint: self.keys.fingerprint(),
            noise_static_pubkey_hex: hex::encode(noise_pubkey_bytes),
            addresses: local_addresses,
            port: self.local_port,
            pin: pin.to_string(),
        })
    }

    /// Verifies and registers a paired device given candidate info and remote pubkey.
    pub async fn complete_pairing(
        &self,
        remote_device_id: Uuid,
        remote_name: String,
        remote_platform: Platform,
        remote_pubkey: Vec<u8>,
        features: Vec<u16>,
        addresses: Vec<SocketAddr>,
    ) -> Result<Device> {
        ensure!(remote_pubkey.len() == 32, "Invalid public key length");

        let mut pubkey_arr = [0u8; 32];
        pubkey_arr.copy_from_slice(&remote_pubkey);
        let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&pubkey_arr)
            .map_err(|e| anyhow!("Invalid remote verifying key: {}", e))?;

        let expected_id = DeviceKeys::derive_device_id(&verifying_key);
        ensure!(
            expected_id == remote_device_id,
            "Device ID does not match public key derivation! Expected {}, got {}",
            expected_id,
            remote_device_id
        );

        let fingerprint = DeviceKeys::format_fingerprint(&verifying_key);

        let mut device = Device::new(
            remote_device_id,
            remote_pubkey,
            fingerprint,
            remote_name,
            remote_platform,
            features,
        );
        device.address_hints = addresses;
        device.trusted = true;

        self.registry.register_device(device.clone()).await?;
        Ok(device)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_core::store::DeviceStore;

    #[tokio::test]
    async fn test_pairing_workflow() -> Result<()> {
        let store = DeviceStore::in_memory()?;
        let registry = DeviceRegistry::new(store).await?;
        let linux_keys = Arc::new(DeviceKeys::generate());
        let phone_keys = DeviceKeys::generate();

        let manager = PairingManager::new(
            registry.clone(),
            linux_keys.clone(),
            "Prince's Linux PC".to_string(),
            Platform::Linux,
            53418,
        );

        let invitation = manager.create_pairing_invitation(&[0u8; 32], vec!["127.0.0.1:53418".parse()?])?;
        assert_eq!(invitation.device_id, linux_keys.device_id);
        assert_eq!(invitation.pin.len(), 6);

        // Android scans QR and completes pairing
        let paired = manager.complete_pairing(
            phone_keys.device_id,
            "Pixel 9 Pro".to_string(),
            Platform::Android,
            phone_keys.verifying_key.as_bytes().to_vec(),
            vec![1, 2, 3],
            vec!["192.168.1.100:53418".parse()?],
        ).await?;

        assert_eq!(paired.name, "Pixel 9 Pro");
        assert!(registry.is_trusted(&phone_keys.device_id).await);

        Ok(())
    }
}
