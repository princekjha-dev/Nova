use anyhow::{anyhow, ensure, Result};
use bytes::{Buf, BufMut, BytesMut};
use chrono::Utc;
use nova_crypto::DeviceKeys;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::device::DeviceId;

pub const WIRE_MAGIC: [u8; 2] = [0x4E, 0x56]; // "NV"
pub const PROTOCOL_VERSION: u16 = 0x0001;

pub type PluginId = u16;

pub const PLUGIN_CLIPBOARD: PluginId = 0x0001;
pub const PLUGIN_NOTES: PluginId = 0x0002;
pub const PLUGIN_FILES: PluginId = 0x0003;
pub const PLUGIN_HANDOFF: PluginId = 0x0004;
pub const PLUGIN_MIRROR: PluginId = 0x0005;
pub const PLUGIN_REMOTE: PluginId = 0x0006;
pub const PLUGIN_AI: PluginId = 0x0007;
pub const PLUGIN_COLLAB: PluginId = 0x0008;
pub const PLUGIN_CONTROL: PluginId = 0xFF00;

/// Wire-level envelope for all Nova communications.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NovaMessage {
    pub id: Uuid,
    pub source: DeviceId,
    pub target: DeviceId,
    pub timestamp: u64, // Unix microseconds
    pub plugin: PluginId,
    pub payload: Vec<u8>,
    #[serde(with = "serde_bytes_64")]
    pub signature: [u8; 64],
}

mod serde_bytes_64 {
    use serde::{Deserializer, Serializer};
    use serde::de::Error;

    pub fn serialize<S>(val: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(val)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = serde::Deserialize::deserialize(deserializer)?;
        if bytes.len() != 64 {
            return Err(D::Error::custom(format!("expected 64 bytes, got {}", bytes.len())));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }
}

impl NovaMessage {
    /// Builds and cryptographically signs a new NovaMessage using the source device's keys.
    pub fn create(
        keys: &DeviceKeys,
        target: DeviceId,
        plugin: PluginId,
        payload: Vec<u8>,
    ) -> Result<Self> {
        let id = Uuid::new_v4();
        let source = keys.device_id;
        let timestamp = Utc::now().timestamp_micros() as u64;

        let sign_bytes = Self::bytes_to_sign(&id, &source, &target, timestamp, plugin, &payload);
        let signature = keys.sign(&sign_bytes);

        Ok(Self {
            id,
            source,
            target,
            timestamp,
            plugin,
            payload,
            signature,
        })
    }

    /// Bytes digest to sign: (id || source || target || timestamp || plugin || payload)
    pub fn bytes_to_sign(
        id: &Uuid,
        source: &DeviceId,
        target: &DeviceId,
        timestamp: u64,
        plugin: PluginId,
        payload: &[u8],
    ) -> Vec<u8> {
        let mut buf = Vec::with_capacity(16 + 16 + 16 + 8 + 2 + payload.len());
        buf.extend_from_slice(id.as_bytes());
        buf.extend_from_slice(source.as_bytes());
        buf.extend_from_slice(target.as_bytes());
        buf.extend_from_slice(&timestamp.to_le_bytes());
        buf.extend_from_slice(&plugin.to_le_bytes());
        buf.extend_from_slice(payload);
        buf
    }

    /// Verifies the cryptographic signature of the message against sender's raw 32-byte Ed25519 public key.
    pub fn verify_signature(&self, sender_pubkey: &[u8; 32]) -> Result<bool> {
        let sign_bytes = Self::bytes_to_sign(
            &self.id,
            &self.source,
            &self.target,
            self.timestamp,
            self.plugin,
            &self.payload,
        );
        DeviceKeys::verify_raw(sender_pubkey, &sign_bytes, &self.signature)
    }

    /// Serializes envelope to binary wire frame:
    /// [0:2] Magic 0x4E56
    /// [2:4] Protocol Version (u16 le)
    /// [4:8] Payload Length (u32 le)
    /// [8..] MessagePack serialized NovaMessage
    pub fn encode_wire(&self) -> Result<Vec<u8>> {
        let packed_payload = rmp_serde::to_vec_named(self)
            .map_err(|e| anyhow!("Failed to serialize NovaMessage: {}", e))?;
        let payload_len = packed_payload.len() as u32;

        let mut frame = Vec::with_capacity(8 + packed_payload.len());
        frame.extend_from_slice(&WIRE_MAGIC);
        frame.extend_from_slice(&PROTOCOL_VERSION.to_le_bytes());
        frame.extend_from_slice(&payload_len.to_le_bytes());
        frame.extend_from_slice(&packed_payload);
        Ok(frame)
    }

    /// Parses wire frame from buffer, advancing buffer if complete frame is present.
    pub fn decode_wire(buf: &mut BytesMut) -> Result<Option<Self>> {
        if buf.len() < 8 {
            return Ok(None);
        }

        // Check magic
        if buf[0..2] != WIRE_MAGIC {
            return Err(anyhow!("Invalid wire magic: {:?}", &buf[0..2]));
        }

        let version = u16::from_le_bytes([buf[2], buf[3]]);
        if version != PROTOCOL_VERSION {
            return Err(anyhow!("Unsupported protocol version: {}", version));
        }

        let payload_len = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]) as usize;
        let total_frame_len = 8 + payload_len;

        if buf.len() < total_frame_len {
            // Incomplete frame, wait for more bytes
            return Ok(None);
        }

        buf.advance(8);
        let payload_bytes = buf.split_to(payload_len);
        let msg: NovaMessage = rmp_serde::from_slice(&payload_bytes)
            .map_err(|e| anyhow!("Failed to deserialize NovaMessage payload: {}", e))?;

        Ok(Some(msg))
    }
}

/// Control plane messages exchanged under PLUGIN_CONTROL
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ControlMessage {
    Hello {
        version: u16,
        device_name: String,
        platform: String,
        features: Vec<u16>,
    },
    Ping {
        nonce: u64,
    },
    Pong {
        nonce: u64,
    },
    DeviceRevoked {
        device_id: DeviceId,
    },
    PermissionRequest {
        permissions: Vec<String>,
    },
    PermissionGrant {
        permissions: Vec<String>,
    },
    PermissionDeny {
        permissions: Vec<String>,
    },
    Goodbye {
        reason: String,
    },
}

impl ControlMessage {
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        rmp_serde::to_vec_named(self).map_err(|e| anyhow!("ControlMessage serialize error: {}", e))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        rmp_serde::from_slice(bytes).map_err(|e| anyhow!("ControlMessage deserialize error: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation_signing_and_wire_framing() -> Result<()> {
        let sender_keys = DeviceKeys::generate();
        let target_id = Uuid::new_v4();

        let control = ControlMessage::Hello {
            version: PROTOCOL_VERSION,
            device_name: "Arch Linux Laptop".to_string(),
            platform: "linux".to_string(),
            features: vec![PLUGIN_CLIPBOARD, PLUGIN_NOTES, PLUGIN_FILES],
        };
        let control_bytes = control.to_bytes()?;

        let msg = NovaMessage::create(
            &sender_keys,
            target_id,
            PLUGIN_CONTROL,
            control_bytes,
        )?;

        // Verify signature
        let mut sender_pub = [0u8; 32];
        sender_pub.copy_from_slice(sender_keys.verifying_key.as_bytes());
        assert!(msg.verify_signature(&sender_pub)?);

        // Encode wire
        let wire_bytes = msg.encode_wire()?;
        assert!(wire_bytes.len() > 8);

        // Decode wire
        let mut buf = BytesMut::from(&wire_bytes[..]);
        let decoded = NovaMessage::decode_wire(&mut buf)?.expect("Message should decode");
        assert_eq!(decoded.id, msg.id);
        assert_eq!(decoded.source, sender_keys.device_id);
        assert_eq!(decoded.target, target_id);
        assert_eq!(decoded.plugin, PLUGIN_CONTROL);

        let decoded_control = ControlMessage::from_bytes(&decoded.payload)?;
        assert_eq!(decoded_control, control);

        Ok(())
    }
}
