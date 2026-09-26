# Nova — Linux & Android Cross-Device Ecosystem

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/Rust-1.98+-orange.svg)](Cargo.toml)

> **Nova** is a production-grade, local-first, peer-to-peer continuity platform connecting Linux desktops and Android mobile devices with cryptographic zero-trust security and modern developer aesthetics.

---

## 🌟 Key Capabilities

1. **Continuous Notes Sync**: Local-first Markdown notes powered by a deterministic CRDT engine with state vector exchange, FTS5 full-text search, and guaranteed conflict-free convergence.
2. **Super Clipboard**: Real-time bidirectional clipboard sync (text, URLs, images) with hash deduplication loop prevention, history eviction, and automated sensitive credential masking.
3. **File EasyShare**: High-speed direct LAN P2P file transfers with 64 KiB chunking, resumability, and SHA-256 integrity verification.
4. **Task Handoff**: Cross-device handoff of active tasks, browser tabs, URLs, notes drafts, and AI research prompts.
5. **Remote PC**: Remote desktop view and synthetic input injection (uinput) protected by mandatory explicit session authorization prompts.
6. **Screen Mirroring**: Low-latency screen streaming with scrcpy Android server protocol integration and PipeWire/xdg-desktop-portal capture.
7. **Nova AI Assistant & MCP Server**: Local Model Context Protocol (MCP) JSON-RPC 2.0 tool server (`localhost:40199`) providing strict permission-bounded context retrieval for private notes, clipboard, and devices.
8. **Infinite Collaboration**: Multi-device workspaces with live presence and CRDT real-time sync.

---

## 🏗️ System Architecture

```text
       Linux Desktop (Tauri / Web)              Android Device (Compose / Service)
   ┌────────────────────────────────┐        ┌────────────────────────────────┐
   │        Nova UI Client          │        │        Nova Android App        │
   └───────────────┬────────────────┘        └───────────────┬────────────────┘
                   │ Local REST / IPC                        │ JNI Bridge
   ┌───────────────▼────────────────┐        ┌───────────────▼────────────────┐
   │          Nova Daemon           │◄──────►│      Foreground Service        │
   │  (nova-core, plugins, store)   │  mTLS  │    & Quick Settings Tile       │
   └────────────────────────────────┘  TCP   └────────────────────────────────┘
```

Nova uses **cryptographic device identities** generated on first launch:
- **Ed25519** keypair for signatures and UUIDv5 device ID derivation
- **Noise Protocol XX** (`Noise_XX_25519_ChaChaPoly_BLAKE2s`) for mutual authentication and QR pairing
- **Wire Framing**: Magic `0x4E56` ("NV"), version `0x0001`, length `u32`, signed MessagePack envelopes

---

## 🚀 Getting Started

### Prerequisites

- **Linux**: Kernel 5.x+, GCC / Clang, Rust 1.98+, Node.js 20+
- **Android**: Android 8.0+ (API 26+)

### Building and Running the Rust Core Workspace

```bash
# Verify and run full test suite across all crates
cargo test --workspace

# Start Nova background daemon
cargo run -p nova-daemon
```

### Running the Linux Desktop UI

```bash
cd apps/linux
npm run build
# Or run live dev server:
npx vite
```

---

## 📁 Repository Structure

```text
nova/
├── Cargo.toml                    # Monorepo Cargo workspace
├── crates/
│   ├── nova-core/                # Device models, registry, message bus, protocol framing
│   ├── nova-crypto/              # Ed25519 keys, Noise XX handshake, secure keystore
│   ├── nova-discovery/           # mDNS announcer/browser, QR pairing payload & SVG
│   ├── nova-transport/           # Framed TCP client & server, connection manager
│   ├── nova-sync/                # Lamport clocks, state vectors, CRDT text engine
│   ├── nova-daemon/              # Linux background service with REST & MCP API
│   └── nova-plugins/
│       ├── clipboard/            # Super Clipboard with deduplication & sensitive filter
│       ├── notes/                # Markdown notes with SQLite FTS5 search & CRDT sync
│       ├── files/                # File EasyShare chunked transfer & SHA-256 verification
│       ├── handoff/              # Task handoff routing (URLs, tabs, prompts)
│       ├── ai/                   # AI context orchestrator & MCP JSON-RPC tool server
│       ├── mirror/               # Screen mirroring negotiation & scrcpy integration
│       └── remote/               # Remote PC explicit authorization & input routing
├── apps/
│   ├── linux/                    # Linux desktop client (Vite, TypeScript, Outfit CSS)
│   └── android/                  # Android client (Kotlin, Jetpack Compose, Foreground Service)
├── services/
│   └── signaling/                # WebSocket signaling server for WebRTC / WAN relay
├── ARCHITECTURE.md
├── SECURITY.md
├── PROTOCOL.md
├── DEVELOPMENT.md
├── CONTRIBUTING.md
└── OSS_NOTICES.md
```

---

## 📄 License

Licensed under the **Apache License, Version 2.0**. See [LICENSE](LICENSE) for details.
# Nova
