use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use zeroize::Zeroize;

/// Secure storage abstraction for local device identity keys and paired device secrets.
pub struct KeyStore {
    base_dir: PathBuf,
}

impl KeyStore {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self> {
        let path = base_dir.as_ref().to_path_buf();
        if !path.exists() {
            fs::create_dir_all(&path)
                .with_context(|| format!("Failed to create keystore dir: {:?}", path))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o700));
            }
        }
        Ok(Self { base_dir: path })
    }

    /// Load or generate the local device Ed25519 keys.
    pub fn load_or_generate_device_keys(&self) -> Result<crate::keys::DeviceKeys> {
        let key_file = self.base_dir.join("device_identity.key");
        if key_file.exists() {
            let data = fs::read(&key_file)
                .with_context(|| format!("Failed to read device key from {:?}", key_file))?;
            if data.len() != 32 {
                return Err(anyhow!("Corrupt key file: length is {} bytes", data.len()));
            }
            let mut seed = [0u8; 32];
            seed.copy_from_slice(&data);
            let keys = crate::keys::DeviceKeys::from_bytes(&seed);
            seed.zeroize();
            Ok(keys)
        } else {
            let keys = crate::keys::DeviceKeys::generate();
            let mut seed = keys.to_bytes();
            fs::write(&key_file, &seed)
                .with_context(|| format!("Failed to write new device key to {:?}", key_file))?;
            seed.zeroize();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&key_file, fs::Permissions::from_mode(0o600));
            }
            Ok(keys)
        }
    }

    /// Load or generate static Noise X25519 key.
    pub fn load_or_generate_noise_key(&self) -> Result<[u8; 32]> {
        let key_file = self.base_dir.join("noise_static.key");
        if key_file.exists() {
            let data = fs::read(&key_file)?;
            if data.len() != 32 {
                return Err(anyhow!("Corrupt noise key file: length is {} bytes", data.len()));
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&data);
            Ok(key)
        } else {
            let key = crate::noise::generate_noise_key()?;
            fs::write(&key_file, &key)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&key_file, fs::Permissions::from_mode(0o600));
            }
            Ok(key)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keystore_load_or_generate() -> Result<()> {
        let temp_dir = std::env::temp_dir().join(format!("nova_keystore_test_{}", uuid::Uuid::new_v4()));
        let store = KeyStore::new(&temp_dir)?;

        // First call: generates
        let keys1 = store.load_or_generate_device_keys()?;
        let noise1 = store.load_or_generate_noise_key()?;

        // Second call: loads same
        let keys2 = store.load_or_generate_device_keys()?;
        let noise2 = store.load_or_generate_noise_key()?;

        assert_eq!(keys1.device_id, keys2.device_id);
        assert_eq!(keys1.verifying_key.as_bytes(), keys2.verifying_key.as_bytes());
        assert_eq!(noise1, noise2);

        let _ = fs::remove_dir_all(&temp_dir);
        Ok(())
    }
}
