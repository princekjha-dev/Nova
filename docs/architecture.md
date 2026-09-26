# Nova Architectural Specification & Systems Design

**Version**: 1.0  
**Target Environment**: Linux Workstation (Wayland/X11) ↔ Android 8.0+ Mobile  
**License**: Apache-2.0

---

## 1. High-Level Architectural Overview

Nova is structured as a **modular Rust core monorepo** with platform-specific presentation layers and isolated system background services.

```text
                                  +-----------------------+
                                  |     Desktop UI        |
                                  |   (Tauri 2.0 / Web)   |
                                  +-----------+-----------+
                                              | Local REST / IPC (Port 40199)
                                              v
+-------------------------------------------------------------------------------------------------+
|                                        Nova Daemon                                              |
|                                                                                                 |
|   +-----------------------+   +------------------------+   +--------------------------------+   |
|   |      nova-crypto      |   |     nova-discovery     |   |         nova-transport         |   |
|   |  - Ed25519 Keys       |   |  - mdns-sd (53418)     |   |  - Framed TCP Server / Client  |   |
|   |  - Noise XX (snow)    |   |  - QR Code Engine      |   |  - webrtc-rs (Media/Data)      |   |
|   |  - Zeroize / Storage  |   |  - PairingManager      |   |  - WebSocket Signaling Client  |   |
|   +-----------+-----------+   +-----------+------------+   +---------------+----------------+   |
|               |                           |                                |                    |
|               +---------------------------+--------------------------------+                    |
|                                           v                                                     |
|                               +-----------------------+                                         |
|                               |       nova-core       |                                         |
|                               |  - DeviceRegistry     |                                         |
|                               |  - MessageBus Router  |                                         |
|                               |  - rusqlite SQLite DB |                                         |
|                               +-----------+-----------+                                         |
|                                           |                                                     |
|                   +-----------------------+-----------------------+                             |
|                   |                       |                       |                             |
|                   v                       v                       v                             |
|       +-----------------------+   +-----------------------+   +-----------------------+         |
|       |     nova-clipboard    |   |      nova-notes       |   |      nova-files       |         |
|       |  - SHA-256 Dedup Ring |   |  - FTS5 Token Search  |   |  - 64 KiB Chunking    |         |
|       |  - Sensitive Filters  |   |  - Folders & Tags     |   |  - SHA-256 Integrity  |         |
|       +-----------------------+   +-----------+-----------+   +-----------------------+         |
|                                               |                                                 |
|                                               v                                                 |
|                                   +-----------------------+                                     |
|                                   |       nova-sync       |                                     |
|                                   |  - yrs / Yjs CRDT     |                                     |
|                                   |  - State Vectors      |                                     |
|                                   |  - Lamport Clocks     |                                     |
|                                   +-----------------------+                                     |
|                                                                                                 |
|   +-----------------------+   +------------------------+   +--------------------------------+   |
|   |     nova-handoff      |   |      nova-mirror       |   |          nova-remote           |   |
|   |  - URLs & Browser Tabs|   |  - scrcpy Protocol     |   |  - uinput Kernel Injection     |   |
|   |  - Note Drafts / Tasks|   |  - PipeWire / Portals  |   |  - Explicit Session Gates      |   |
|   +-----------------------+   +------------------------+   +--------------------------------+   |
|                                                                                                 |
|                               +------------------------+                                        |
|                               |        nova-ai         |                                        |
|                               |  - Local MCP Server    |                                        |
|                               |  - Permission Guards   |                                        |
|                               +------------------------+                                        |
+-------------------------------------------------------------------------------------------------+
```

---

## 2. Subsystem Mapping & Responsibilities

1. **`nova-core`**: Defines the shared vocabulary of the system (`Device`, `DeviceId`, `Platform`, `Permission`, `NovaMessage`, and `ControlMessage`). Manages the local persistent SQLite store and dispatches verified network packets to registered plugin handlers.
2. **`nova-crypto`**: Enforces cryptographic boundaries: Ed25519 signing/verifying, UUIDv5 deterministic device ID derivation, Noise XX mutual authentication handshakes (`snow`), and zero-overhead memory zeroization on drop.
3. **`nova-discovery`**: Advertises and discovers Nova nodes on local networks via DNS-SD (`_nova._tcp.local.`, port 53418) and renders SVG QR pairing invitations containing public keys and a 6-digit confirmation PIN.
4. **`nova-transport`**: Implements asynchronous framed TCP connections with 8-byte binary headers (`magic: 0x4E56`, `version: 0x0001`, `length: u32`), backoff retries, and clean connection teardowns.
5. **`nova-sync`**: Houses the CRDT synchronization engine, generating state vectors and computing minimal delta diffs for conflict-free document convergence.
6. **`nova-plugins/clipboard`**: Manages the bidirectional clipboard ring buffer with SHA-256 hash deduplication and sensitive token masking.
7. **`nova-plugins/notes`**: Local-first Markdown note editor with persistent SQLite `FTS5` full-text search indexing.
8. **`nova-plugins/files`**: LocalSend-compatible file transfer state machine with 64 KiB chunk streaming, resumability, and SHA-256 integrity checks.
9. **`nova-plugins/handoff`**: Cross-device task handoff for URLs, browser tabs, note drafts, and AI prompts.
10. **`nova-plugins/mirror`**: Screen mirroring negotiation and scrcpy server integration.
11. **`nova-plugins/remote`**: Remote control session management with mandatory explicit host user approval before accepting any synthetic input.
12. **`nova-plugins/ai`**: Model Context Protocol (MCP) JSON-RPC 2.0 tool server on `localhost:40199` with strict capability boundaries.
13. **`nova-daemon`**: Linux background daemon uniting all engines into a single native process with local REST & MCP HTTP endpoints.
14. **`apps/linux`**: High-performance, dark-mode glassmorphic desktop interface built with modern web technologies and bundled via Vite.
15. **`apps/android`**: Native Kotlin and Jetpack Compose mobile application with foreground service (`NovaService`), JNI bridge, and Quick Settings tile (`ClipboardTile`) for background clipboard sync on Android 10+.
