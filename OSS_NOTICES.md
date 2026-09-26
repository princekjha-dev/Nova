# Open-Source Software Notices & Licensing Audit

Nova builds upon and references mature open-source projects. This document records all third-party components, their licensing terms, and integration models.

---

## 1. Integrated Open-Source Libraries

| Component | License | Role in Nova | Integration Strategy |
|-----------|---------|--------------|----------------------|
| **snow** | Apache-2.0 / MIT | Noise Protocol XX Handshake | Embedded Rust crate |
| **ed25519-dalek** | Apache-2.0 / BSD-3 | Device Keypair & Signatures | Embedded Rust crate |
| **mdns-sd** | MIT / Apache-2.0 | Pure Rust mDNS discovery | Embedded Rust crate |
| **rusqlite** | MIT | SQLite persistence (bundled engine) | Embedded Rust crate (Public Domain SQLite) |
| **rmp-serde** | MIT | Binary MessagePack wire framing | Embedded Rust crate |
| **axum** | MIT | Local REST & MCP HTTP JSON-RPC Server | Embedded Rust crate |
| **tokio** | MIT | Asynchronous I/O runtime | Embedded Rust crate |
| **uuid** | Apache-2.0 / MIT | Device & Session Identifiers | Embedded Rust crate |
| **qrcode** | MIT / Apache-2.0 | QR code generation for pairing | Embedded Rust crate |

---

## 2. Open-Source Ecosystem Integrations & Architectural References

### scrcpy (Apache-2.0)
- **Role**: Android to Linux screen mirroring and input injection.
- **Integration**: The scrcpy Android server component (Apache-2.0) is deployed to the Android runtime, speaking the scrcpy binary protocol directly over the encrypted TCP tunnel.

### LocalSend (Apache-2.0)
- **Role**: LAN-first file transfer protocol inspiration.
- **Integration**: Nova integrates a compatible chunked transfer state machine with SHA-256 integrity verification, avoiding heavy Flutter binary overhead on Linux.

### Yjs / yrs (MIT)
- **Role**: Conflict-free Replicated Data Type (CRDT) engine for continuous notes sync.
- **Integration**: State vector exchange, update diffing, and deterministic text convergence.

### KDE Connect (GPL-2.0)
- **Role**: Architectural reference for plugin-based cross-device continuity.
- **License Boundary**: Studied for protocol semantics and capability negotiation; no GPL code is copied or embedded.

### RustDesk (AGPL-3.0) & Sunshine (GPL-3.0)
- **Role**: Architecture reference for remote desktop and streaming.
- **License Boundary**: Incompatible with Apache 2.0 distribution; Nova uses an independent pipeline built on standard Linux `uinput` and PipeWire/xdg-desktop-portal abstractions.
