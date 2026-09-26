# Nova Architecture Document

This document outlines the system architecture, design decisions, and threat model of Nova.

---

## 1. Architectural Philosophy

1. **Local-First & P2P**: Direct LAN connections over TLS 1.3 / TCP without cloud intermediaries.
2. **Zero-Trust Identity**: Devices are identified by Ed25519 public keys and verified using Noise XX handshakes and human-readable fingerprints.
3. **CRDT-Driven Continuity**: State convergence across asynchronous and offline edits using state vectors and Lamport clocks.
4. **Least-Privilege Orchestration**: Capability-based permissions per paired device and strict permission boundaries for AI agents.

---

## 2. Component Diagram

```text
+-----------------------------------------------------------------+
|                       Nova Desktop Daemon                       |
|                                                                 |
|  [NovaMdnsAnnouncer]    [KeyStore: Ed25519]   [PairingManager]  |
|            |                      |                   |         |
|  [NovaServer (53418)]◄──[MessageBus]────────►[DeviceRegistry]  |
|                               |                     |           |
|  +----------------------------+------------------+  [SQLite DB] |
|  |             Feature Plugin Registry           |              |
|  |  * ClipboardPlugin (Hash Dedup, Masks)        |              |
|  |  * NotesPlugin (CRDT Engine, FTS5 Search)     |              |
|  |  * FileTransferPlugin (64KB Chunk, SHA-256)   |              |
|  |  * HandoffPlugin (URL / Browser Task)         |              |
|  |  * NovaAiOrchestrator (MCP JSON-RPC: 40199)   |              |
|  |  * MirrorPlugin (scrcpy / PipeWire)           |              |
|  |  * RemoteControlPlugin (Explicit Auth)        |              |
+--+-----------------------------------------------+--------------+
```

---

## 3. Cryptographic Identity & Transport

- **Identity**: Ed25519 keypair generated on first launch and stored securely in `~/.local/share/nova/device_identity.key` with `0600` permissions.
- **Device ID**: RFC 4122 UUIDv5 derived deterministically from the SHA3-256 hash of the Ed25519 public key.
- **Handshake**: Noise XX pattern (`Noise_XX_25519_ChaChaPoly_BLAKE2s`) providing mutual authentication, forward secrecy, and identity hiding.
- **Wire Envelope**:
  - `Header`: 8 bytes (`magic: 0x4E56`, `version: 0x0001`, `length: u32`)
  - `Payload`: MessagePack binary serialization of `NovaMessage`
  - `Signature`: 64-byte Ed25519 signature computed over `(id || source || target || timestamp || plugin || payload)`

---

## 4. Subsystem Details

### 4.1 Super Clipboard
- Uses SHA-256 content hashes to maintain a circular deduplication ring, eliminating infinite ping-pong echo loops.
- Automatically flags sensitive inputs (private keys, API tokens, 6-digit OTPs) and masks their preview in the UI history.

### 4.2 Notes & CRDT Sync
- Uses an operation-based CRDT model with character-level item IDs `(client_id, clock)`.
- Reconciles state across devices via two-step exchange:
  1. `SyncStep1`: Transmit local `StateVector`.
  2. `SyncStep2`: Transmit only missing updates identified by difference against the remote state vector.
- SQLite with `FTS5` virtual table enables instant substring and token search.

### 4.3 File EasyShare
- Protocol flow: `Offer -> Decision (Accept/Reject) -> Chunk (64 KiB) -> Complete (SHA-256)`.
- SHA-256 digest is validated over the reassembled payload before writing to the target directory.

### 4.4 Task Handoff
- Supports cross-device handoffs for URLs, browser tabs, note drafts, and AI prompt contexts.
- Handoff payloads are delivered with unique task IDs and consumed with explicit acknowledgment.

### 4.5 Nova AI Orchestrator
- Implements an HTTP Model Context Protocol (MCP) tool server on port `40199`.
- Provides tool endpoints: `nova_search_notes`, `nova_get_recent_clipboard`, `nova_list_devices`.
- Enforces strict capability boundaries: clipboard reading requires explicit manual user opt-in in settings.

### 4.6 Remote PC
- Demands explicit session authorization before any synthetic input (`uinput`) is accepted.
- Unapproved input messages are rejected at the protocol layer.
