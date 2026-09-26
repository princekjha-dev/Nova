# Nova Security Threat Model & Risk Mitigation Matrix

**Author**: Lead Systems Architect  
**Classification**: Public Architecture Specification  
**Framework**: STRIDE + Zero-Trust Peer Defense

---

## 1. Threat Landscape Overview

Nova coordinates high-privilege operations across heterogeneous endpoints (Linux desktop and Android mobile): screen capture, synthetic keyboard/mouse injection, clipboard synchronization, and file system writes. A breach in any component could compromise the host operating system. Therefore, Nova operates under a **Zero-Trust LAN model**: proximity on a local Wi-Fi network does not confer trust.

---

## 2. Threat Analysis & Mitigations

### 2.1 Network Layer Threats

| Threat Vector | STRIDE | Severity | Attack Scenario | Nova Architectural Mitigation |
|---|---|---|---|---|
| **Man-in-the-Middle (MITM)** | Information Disclosure / Tampering | **CRITICAL** | An attacker on public/compromised Wi-Fi intercepts plaintext or alters handshake packets. | **Noise XX Pattern**: 3-message mutual Diffie-Hellman handshake. Both parties verify remote static public keys. No plaintext session payload ever leaves the device. |
| **Rogue / Impersonating Device** | Spoofing | **HIGH** | A rogue device broadcasts `_nova._tcp` with a legitimate device name to hijack sync. | **Cryptographic Device IDs**: Device ID is an RFC 4122 UUIDv5 derived deterministically from the Ed25519 public key. Names are purely decorative; trust is anchored strictly to the public key. |
| **Replay Attacks** | Repudiation / Tampering | **HIGH** | An attacker captures old valid signed messages (e.g., "Inject KeyPress") and replays them. | **Monotonic Microsecond Timestamps & Nonces**: Every `NovaMessage` envelope includes a microsecond timestamp and unique UUIDv4. Packets outside an acceptable window or matching historical replay caches are dropped. |
| **Packet Injection** | Tampering | **CRITICAL** | An attacker on the local network injects synthetic TCP frames into an existing connection. | **Mandatory Envelope Signatures & AEAD**: Every frame is signed with the source device's Ed25519 private key, and session transport packets are encrypted via ChaCha20-Poly1305 authenticated encryption. |
| **NAT Traversal Relay Abuse** | Denial of Service / Spoofing | **MEDIUM** | An attacker uses Nova's signaling server to route unauthorized traffic or spam WebRTC offers. | **End-to-End Encrypted SDP**: Signaling servers only route blobs encrypted with the recipient's public key. The signaling relay has zero visibility into session negotiation. |

---

### 2.2 Device & Physical Security Threats

| Threat Vector | STRIDE | Severity | Attack Scenario | Nova Architectural Mitigation |
|---|---|---|---|---|
| **Stolen / Compromised Phone** | Elevation of Privilege | **CRITICAL** | An attacker gains access to an unlocked paired phone and attempts to control the Linux PC. | **Granular Capability Permissions & Device Revocation**: Devices can be revoked instantly from the Linux UI. High-risk operations (Remote PC) require explicit session authorization on the host screen each time. |
| **Stolen Laptop (Offline Key Extraction)** | Information Disclosure | **HIGH** | An attacker steals a powered-off laptop and inspects SQLite databases or configuration files. | **OS Secure Store Root-of-Trust**: Identity keys are protected by `libsecret` (GNOME Keyring / KWallet) on Linux and `Android Keystore` (TEE/StrongBox) on Android. Key material is cleared using `Zeroize` on drop. |
| **Malicious Paired Device** | Elevation of Privilege | **HIGH** | A device was legitimately paired, but its software was replaced with a compromised client. | **Least Privilege Boundaries**: Paired status allows only granted plugins. A device granted "NotesRead" cannot issue "RemoteControl" or "FileWrite" without explicit capability updates. |

---

### 2.3 Application & Content Threats

| Threat Vector | STRIDE | Severity | Attack Scenario | Nova Architectural Mitigation |
|---|---|---|---|---|
| **Malicious Clipboard Injection** | Tampering / Elevation of Privilege | **HIGH** | An Android app copies a malicious shell command (`rm -rf ~`) hoping Nova auto-pastes or executes it. | **Passive Clipboard Synchronizer**: Nova never automatically executes clipboard contents. Contents are placed into the system clipboard only. History previews for sensitive tokens (API keys, passwords) are automatically masked. |
| **Path Traversal via File Transfer** | Tampering / Information Disclosure | **CRITICAL** | A peer sends a file with filename `../../../../etc/shadow` or `.bashrc`. | **Strict Filename Sanitization**: The receiving `FileTransferPlugin` strips all path segments, normalizing filenames to `Path::file_name()` and forcing all writes into a dedicated, isolated sandbox directory (`~/Downloads/Nova`). |
| **Corrupted File Delivery** | Tampering | **HIGH** | A file is modified in transit or partially downloaded due to network failure. | **SHA-256 Digest Verification**: Received chunks are buffered and the complete file is validated against the original SHA-256 hash before committing to disk. |
| **Unauthorized Remote Input** | Elevation of Privilege | **CRITICAL** | An Android client sends `InputEvent::KeyPress` packets without host user consent. | **Host Authorization Gate**: The `RemoteControlPlugin` drops all input packets with an error unless the host user explicitly clicked **[Allow Remote Session]** in a native dialog for that specific session UUID. |

---

### 2.4 Screen Streaming Threats

| Threat Vector | STRIDE | Severity | Attack Scenario | Nova Architectural Mitigation |
|---|---|---|---|---|
| **Unauthorized Screen Capture** | Information Disclosure | **CRITICAL** | Background process captures Linux screen silently without user knowledge. | **xdg-desktop-portal Enforcement**: On Wayland, capture cannot bypass the system compositor portal. The OS displays a mandatory permission dialog asking the user to select the display. |
| **Session Hijacking** | Information Disclosure | **HIGH** | An attacker intercepts the video stream of the remote PC. | **DTLS-SRTP Encryption**: WebRTC video and audio streams are encrypted end-to-end with ephemeral DTLS keys negotiated directly between paired peers. |

---

### 2.5 Synchronization & CRDT Threats

| Threat Vector | STRIDE | Severity | Attack Scenario | Nova Architectural Mitigation |
|---|---|---|---|---|
| **Synchronization Poisoning** | Denial of Service / Tampering | **MEDIUM** | A malicious device sends a flood of invalid CRDT deltas with huge clock values to exhaust memory. | **Lamport Vector Bound Checking**: Deltas are validated for clock continuity. Updates with invalid client IDs or absurd clock skips (>100k increments) are rejected before merging into the Yjs/yrs document. |
| **State Vector Replay** | Denial of Service | **LOW** | An attacker replays old state vectors to force redundant full-history diff generations. | **Idempotent Update Log**: The sync engine compares state vectors against cached document clocks and ignores duplicate requests. |
