use anyhow::{anyhow, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use uuid::Uuid;
use zeroize::Zeroize;

/// Cryptographic identity for a Nova device using Ed25519.
pub struct DeviceKeys {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
    pub device_id: Uuid,
}

impl DeviceKeys {
    /// Generate a fresh device keypair using OS entropy.
    pub fn generate() -> Self {
        use rand::RngCore;
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        let device_id = Self::derive_device_id(&verifying_key);
        seed.zeroize();
        Self {
            signing_key,
            verifying_key,
            device_id,
        }
    }

    /// Restore keypair from raw 32-byte secret seed.
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(bytes);
        let verifying_key = signing_key.verifying_key();
        let device_id = Self::derive_device_id(&verifying_key);
        Self {
            signing_key,
            verifying_key,
            device_id,
        }
    }

    /// Export raw secret seed bytes for secure storage.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Derives a deterministic UUIDv5 identifier from the public key SHA3-256 hash.
    pub fn derive_device_id(pubkey: &VerifyingKey) -> Uuid {
        let hash = Sha3_256::digest(pubkey.as_bytes());
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&hash[..16]);
        // Set UUID version to 5 (SHA-based)
        bytes[6] = (bytes[6] & 0x0f) | 0x50;
        // Set variant to RFC 4122
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Uuid::from_bytes(bytes)
    }

    /// Human-verifiable fingerprint for UI pairing confirmation (e.g. 16-hex formatted in 4 blocks).
    pub fn fingerprint(&self) -> String {
        Self::format_fingerprint(&self.verifying_key)
    }

    /// Formats a verifying key into a readable 4-block hexadecimal string.
    pub fn format_fingerprint(pubkey: &VerifyingKey) -> String {
        let digest = Sha3_256::digest(pubkey.as_bytes());
        let hex_str = hex::encode(&digest[..8]);
        format!(
            "{}-{}-{}-{}",
            &hex_str[0..4],
            &hex_str[4..8],
            &hex_str[8..12],
            &hex_str[12..16]
        )
    }

    /// Sign data using the device's private key.
    pub fn sign(&self, data: &[u8]) -> [u8; 64] {
        self.signing_key.sign(data).to_bytes()
    }

    /// Verify signature with an arbitrary public key.
    pub fn verify(pubkey: &VerifyingKey, data: &[u8], sig_bytes: &[u8; 64]) -> bool {
        let signature = Signature::from_bytes(sig_bytes);
        pubkey.verify(data, &signature).is_ok()
    }

    /// Verify signature given raw 32-byte public key.
    pub fn verify_raw(pubkey_bytes: &[u8; 32], data: &[u8], sig_bytes: &[u8; 64]) -> Result<bool> {
        let verifying_key = VerifyingKey::from_bytes(pubkey_bytes)
            .map_err(|e| anyhow!("Invalid public key: {}", e))?;
        Ok(Self::verify(&verifying_key, data, sig_bytes))
    }
}

impl Drop for DeviceKeys {
    fn drop(&mut self) {
        // Zeroize memory of signing key upon drop
        let mut raw = self.signing_key.to_bytes();
        raw.zeroize();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicKeyInfo {
    pub device_id: Uuid,
    pub pubkey_bytes: Vec<u8>,
    pub fingerprint: String,
}

impl From<&DeviceKeys> for PublicKeyInfo {
    fn from(keys: &DeviceKeys) -> Self {
        Self {
            device_id: keys.device_id,
            pubkey_bytes: keys.verifying_key.as_bytes().to_vec(),
            fingerprint: keys.fingerprint(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_keys_generation_and_signing() {
        let keys = DeviceKeys::generate();
        assert_eq!(keys.verifying_key.as_bytes().len(), 32);
        assert!(!keys.fingerprint().is_empty());

        let message = b"Nova protocol test message";
        let sig = keys.sign(message);
        assert!(DeviceKeys::verify(&keys.verifying_key, message, &sig));

        let tampered = b"Nova protocol tampered message";
        assert!(!DeviceKeys::verify(&keys.verifying_key, tampered, &sig));
    }

    #[test]
    fn test_deterministic_device_id() {
        let keys1 = DeviceKeys::generate();
        let id1 = DeviceKeys::derive_device_id(&keys1.verifying_key);
        let id2 = DeviceKeys::derive_device_id(&keys1.verifying_key);
        assert_eq!(id1, id2);
        assert_eq!(keys1.device_id, id1);
    }

    #[test]
    fn test_key_serialization_roundtrip() {
        let keys = DeviceKeys::generate();
        let bytes = keys.to_bytes();
        let restored = DeviceKeys::from_bytes(&bytes);
        assert_eq!(keys.device_id, restored.device_id);
        assert_eq!(keys.fingerprint(), restored.fingerprint());
    }
}
