# Nova Dependency & Open-Source License Audit

**Project License**: Apache License, Version 2.0  
**Target Environments**: Linux (x86_64, aarch64) & Android (arm64-v8a, x86_64)

---

## 1. Executive Summary & Policy

Nova's codebase is distributed under the **Apache License, Version 2.0**. To maintain strict legal integrity, ensure commercial redistributability, and prevent viral copyleft contamination:

1. **Permissive Licenses (MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, CC0, Public Domain)**:
   - Permitted for direct static compilation and vendoring within Nova crates.
2. **Weak Copyleft Licenses (LGPL-2.1, LGPL-3.0, MPL-2.0)**:
   - **Allowed conditionally**: Must be consumed via **dynamic linking** (`.so` system packages) or **IPC/D-Bus boundaries**. Static linking is strictly forbidden.
   - Any modifications to MPL-2.0 files must be published under MPL-2.0.
3. **Strong Copyleft Licenses (GPL-2.0, GPL-3.0, AGPL-3.0)**:
   - **Forbidden from compilation/embedding** into Nova binaries.
   - If interoperability is required (e.g. GNOME Shell extension), it must be distributed as an independent, isolated package running over standard IPC (D-Bus, stdin/stdout, or network sockets).

---

## 2. In-Depth Dependency Audit

### 2.1 Core Rust Dependencies

| Crate / Library | Version | License | Embedding Allowed? | Obligations & Attribution Notes |
|---|---|---|---|---|
| `tokio` | 1.40+ | MIT | ✅ YES | Include MIT copyright notice in binary notices. |
| `serde`, `serde_json` | 1.0+ | MIT / Apache-2.0 | ✅ YES | Standard dual-license permissive crate. |
| `rmp-serde` | 1.3+ | MIT | ✅ YES | Pure Rust MessagePack serializer. |
| `uuid` | 1.10+ | Apache-2.0 / MIT | ✅ YES | Used for UUIDv4 & UUIDv5 identity derivation. |
| `ed25519-dalek` | 2.1+ | Apache-2.0 / BSD-3-Clause | ✅ YES | Core cryptographic signature engine. |
| `x25519-dalek` | 2.0+ | BSD-3-Clause | ✅ YES | Diffie-Hellman primitive for Noise Protocol. |
| `snow` | 0.9+ | Apache-2.0 / MIT | ✅ YES | Noise Protocol XX state machine. Pure Rust. |
| `sha3`, `sha2` | 0.10+ | MIT / Apache-2.0 | ✅ YES | Cryptographic hashing (SHA3-256 for device IDs). |
| `zeroize` | 1.8+ | Apache-2.0 / MIT | ✅ YES | Memory clearing for sensitive private keys on drop. |
| `mdns-sd` | 0.13+ | MIT / Apache-2.0 | ✅ YES | Pure Rust DNS-SD / mDNS multicast engine. |
| `qrcode` | 0.14+ | MIT / Apache-2.0 | ✅ YES | Vector SVG QR generation for device pairing. |
| `rusqlite` | 0.32+ | MIT | ✅ YES | Bundles public domain SQLite engine via C compiler. |
| `axum`, `tower-http` | 0.7+, 0.5+ | MIT | ✅ YES | Local REST API and MCP JSON-RPC tool server. |
| `yrs` | 0.21+ | MIT | ✅ YES | Yjs-compatible CRDT document synchronization. |
| `webrtc` (webrtc-rs) | 0.11+ | MIT / Apache-2.0 | ✅ YES | Pure Rust WebRTC ICE/DTLS/SCTP stack. |

---

### 2.2 System & Dynamic Link Dependencies (Linux)

| Component | License | Link Method | Compliance Strategy |
|---|---|---|---|
| **glibc / musl** | LGPL-2.1 / MIT | Dynamic link (`libc.so.6`) | Standard system library boundary. |
| **WebKit2GTK** (Tauri) | LGPL-2.1 / BSD | Dynamic link via dlopen/pkg-config | Handled by Tauri runtime. Relies on distro package. |
| **GStreamer (core & base)** | LGPL-2.0+ | Dynamic link via `gstreamer-sys` | Distro packages (`libgstreamer1.0-dev`). Must NOT use `plugins-ugly`. |
| **Avahi** | LGPL-2.1 | D-Bus IPC (No C-FFI linking) | Communication over system bus (`org.freedesktop.Avahi`). |
| **libsecret** | LGPL-2.1 | D-Bus IPC (`secret-service` crate)| Communicates with GNOME Keyring/KWallet via standard Secret Service D-Bus API. Zero linking risk. |
| **xdg-desktop-portal** | LGPL-2.1 | D-Bus IPC (`ashpd` crate) | Invokes portals over session D-Bus. No shared library linkage required. |
| **Linux Kernel uinput** | GPL-2.0 with Syscall Exception | Kernel ioctl syscalls (`/dev/uinput`)| Linux kernel userland syscall boundary is explicitly non-infectious. |

---

### 2.3 Evaluated External Technologies & Copyleft Boundaries

#### KDE Connect (`kdeconnect-kde`, `kdeconnect-android`)
- **License**: GPL-2.0-or-later.
- **Audit Finding**: Embedding C++ or Java classes from KDE Connect would trigger the GPL viral clause, forcing all of Nova to be relicensed under GPL.
- **Architectural Decision**: **Clean-room architecture**. Nova defines its own protocol (`NV01`) and MessagePack schema. Nova does not import any KDE Connect source code.

#### RustDesk (`rustdesk`)
- **License**: AGPL-3.0.
- **Audit Finding**: AGPL-3.0 has network copyleft terms: providing remote desktop services over a network requires source disclosure under AGPL. Embedding RustDesk code is completely incompatible with an Apache 2.0 license.
- **Architectural Decision**: **Rejected for embedding**. Nova uses standard Linux `uinput` for input injection and `xdg-desktop-portal` for capture.

#### Sunshine & Moonlight
- **License**: Sunshine is GPL-3.0; Moonlight Android is GPL-2.0.
- **Audit Finding**: Sunshine cannot be compiled into Nova's daemon.
- **Architectural Decision**: **Rejected for direct embedding**. Nova implements its own WebRTC-based low-latency pipeline.

#### scrcpy
- **License**: Apache License, Version 2.0.
- **Audit Finding**: Fully compatible with Nova's Apache 2.0 license.
- **Architectural Decision**: **Adopted**. The `scrcpy-server` Java jar/APK can be distributed and executed directly, with Nova speaking the documented scrcpy binary protocol over a local socket.

#### LocalSend
- **License**: Apache License, Version 2.0.
- **Audit Finding**: LocalSend's Protocol v2.1 is open, cleanly documented, and permissive.
- **Architectural Decision**: **Adopted Protocol Specification**. Nova implements wire-compatible endpoints natively in Rust without pulling in Flutter runtime binaries.

---

## 3. Packaging & Distribution Directives

1. **Linux Flatpak / AppImage / Deb**:
   - Must include `OSS_NOTICES.md` and third-party license texts in `/usr/share/doc/nova/copyright`.
   - Never statically link LGPL libraries (GStreamer, WebKit2GTK).
2. **Android APK**:
   - All Rust crates compiled into `libnova_android_jni.so` via `cargo-ndk` must adhere strictly to Apache-2.0/MIT/BSD. No GPL code may enter the APK bundle.
