# Nova Open-Source Integration & Implementation Plan

This roadmap coordinates the phased rollout of validated open-source components into the Nova ecosystem.

---

## Phase 1 — Core Foundation & Cryptographic Identity
**Status**: ✅ COMPLETED & VERIFIED

### Objectives
- Establish the Cargo monorepo workspace.
- Implement device cryptographic identity: Ed25519 keypair generation, UUIDv5 deterministic device IDs, and SHA3-256 fingerprints.
- Implement Noise Protocol XX handshake (`Noise_XX_25519_ChaChaPoly_BLAKE2s`) via `snow` for mutual authentication and forward-secure session encryption.
- Implement secure storage abstraction (`KeyStore`) with strict filesystem permissions (`0600`) and `Zeroize` memory hygiene.
- Implement SQLite persistence with WAL mode and `FTS5` full-text search indexing via bundled `rusqlite`.

### Test Verification
- `test_device_keys_generation_and_signing` ... **PASS**
- `test_deterministic_device_id` ... **PASS**
- `test_noise_xx_handshake_and_transport` ... **PASS**
- `test_keystore_load_or_generate` ... **PASS**
- `test_device_store_crud` ... **PASS**

---

## Phase 2 — Discovery & QR Pairing
**Status**: ✅ COMPLETED & VERIFIED

### Objectives
- Pure Rust DNS-SD / mDNS announcer and browser (`_nova._tcp.local.`, port 53418) via `mdns-sd`.
- QR pairing payload encoding and SVG rendering with 6-digit confirmation PIN via `qrcode`.
- `PairingManager` state machine: QR creation, remote public key cryptographic verification, and device registry enrollment.

### Test Verification
- `test_qr_payload_and_svg_generation` ... **PASS**
- `test_pairing_workflow` ... **PASS**

---

## Phase 3 — Wire Protocol & Transport
**Status**: ✅ COMPLETED & VERIFIED

### Objectives
- Versioned wire framing: Magic `0x4E56` ("NV"), version `0x0001`, length `u32`, MessagePack body via `rmp-serde`.
- `NovaMessage` envelope with mandatory Ed25519 signatures.
- Asynchronous TCP framed connection (`NovaConnection`) with buffer management.
- `NovaServer` listener on port 53418 and `NovaClient` with exponential reconnect backoff.

### Test Verification
- `test_message_creation_signing_and_wire_framing` ... **PASS**
- `test_transport_client_server_message_flow` ... **PASS**

---

## Phase 4 — Synchronization Engine & Super Clipboard
**Status**: ✅ COMPLETED & VERIFIED

### Objectives
- CRDT text model with Lamport clocks, item origins, and conflict-free convergence.
- `SyncEngine` supporting `SyncStep1` (state vector) and `SyncStep2` (update exchange).
- Super Clipboard plugin with circular SHA-256 hash deduplication to eliminate ping-pong loops.
- Heuristic sensitive content detection and preview masking.

### Test Verification
- `test_crdt_doc_editing_and_convergence` ... **PASS**
- `test_sync_engine_handshake` ... **PASS**
- `test_clipboard_deduplication_and_sync` ... **PASS**

---

## Phase 5 — Continuous Notes & File EasyShare
**Status**: ✅ COMPLETED & VERIFIED

### Objectives
- Markdown notes storage with folder categorization and SQLite `FTS5` token search.
- LocalSend-compatible file transfer state machine: Offer -> Decision -> 64 KiB chunking -> Complete.
- SHA-256 file hash validation upon completion before disk commit.

### Test Verification
- `test_notes_crud_and_fts_search` ... **PASS**
- `test_file_transfer_end_to_end` ... **PASS**

---

## Phase 6 — Task Handoff, Screen Mirroring & Remote PC
**Status**: ✅ COMPLETED & VERIFIED

### Objectives
- Extensible task model for URLs, browser tabs, note drafts, and AI prompts.
- Screen mirror negotiation with codec options (H.264, H.265, AV1) and scrcpy integration interfaces.
- Remote control authorization state machine requiring host confirmation before any synthetic input is accepted.

### Test Verification
- `test_task_handoff_workflow` ... **PASS**
- `test_mirror_negotiation` ... **PASS**
- `test_remote_control_explicit_authorization_security` ... **PASS**

---

## Phase 7 — Nova AI Assistant & MCP Server
**Status**: ✅ COMPLETED & VERIFIED

### Objectives
- Model Context Protocol (MCP) JSON-RPC 2.0 tool server on `localhost:40199`.
- Tool definitions: `nova_search_notes`, `nova_get_recent_clipboard`, `nova_list_devices`.
- Strict capability permission guard preventing private clipboard reading without explicit user grant.

### Test Verification
- `test_ai_tool_permission_boundaries` ... **PASS**

---

## Phase 8 — Desktop UI & Android Platform Client
**Status**: ✅ COMPLETED & VERIFIED

### Objectives
- Linux Desktop Client (`apps/linux`): Built with Vite, TypeScript, and dark-mode glassmorphic CSS. Includes full navigation and QR pairing modal.
- Android Native Client (`apps/android`): Built with Kotlin and Jetpack Compose. Includes JNI bridge (`NovaCoreBinding`), foreground service (`NovaService`), and Quick Settings tile (`ClipboardTile`) for Android 10+ background clipboard sync.
