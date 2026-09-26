# Nova Wire Protocol Specification

Version: **1.0**
Port: **53418** (TCP/UDP)
Service Type: `_nova._tcp.local.`

---

## 1. Framing

All frames transmitted over TCP stream use an 8-byte binary header followed by a MessagePack encoded body:

```text
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|       Magic (0x4E56 - "NV")   |         Version (0x0001)      |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                     Payload Length (u32 LE)                   |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                     MessagePack Body ...                      |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

---

## 2. Envelope (`NovaMessage`)

The body deserializes into the following envelope:

```rust
pub struct NovaMessage {
    pub id: Uuid,              // 16 bytes UUIDv4
    pub source: DeviceId,      // 16 bytes UUIDv5
    pub target: DeviceId,      // 16 bytes UUIDv5
    pub timestamp: u64,        // Unix microseconds
    pub plugin: u16,           // Plugin ID
    pub payload: Vec<u8>,      // Inner serialized payload
    pub signature: [u8; 64],   // Ed25519 signature
}
```

The signature is computed over:
`(id || source || target || timestamp_le || plugin_le || payload)`.

---

## 3. Plugin Registry

| Plugin ID | Name | Description |
|-----------|------|-------------|
| `0x0001` | `CLIPBOARD` | Super Clipboard synchronized entries |
| `0x0002` | `NOTES` | CRDT note delta and vector exchange |
| `0x0003` | `FILES` | File EasyShare offers, chunks, and digests |
| `0x0004` | `HANDOFF` | Task and URL handoff routing |
| `0x0005` | `MIRROR` | Screen mirror stream negotiation |
| `0x0006` | `REMOTE` | Remote control sessions and input injection |
| `0x0007` | `AI` | AI orchestration events |
| `0x0008` | `COLLAB` | Shared workspace presence |
| `0xFF00` | `CONTROL` | Hello, Ping, Pong, Revocation, Permissions |

---

## 4. Control Messages (`0xFF00`)

```rust
pub enum ControlMessage {
    Hello {
        version: u16,
        device_name: String,
        platform: String,
        features: Vec<u16>,
    },
    Ping { nonce: u64 },
    Pong { nonce: u64 },
    DeviceRevoked { device_id: DeviceId },
    PermissionRequest { permissions: Vec<String> },
    PermissionGrant { permissions: Vec<String> },
    PermissionDeny { permissions: Vec<String> },
    Goodbye { reason: String },
}
```
