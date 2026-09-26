# Nova Security Policy & Architecture

## Security Principles

1. **No Cloud Data Relay by Default**: All sensitive clipboard, note, and file content stays within the local network over direct encrypted sockets.
2. **Mutual Cryptographic Authentication**: Devices must be paired and trusted in the local `DeviceRegistry` before any plugin payload is dispatched.
3. **Memory Zeroization**: Secret seeds and ephemeral keys implement the `Zeroize` trait to wipe key material from memory upon drop.
4. **Mandatory Explicit Remote Authorization**: Remote desktop sessions and input control cannot be activated silently. A modal prompts the host user with device name and fingerprint.
5. **Least Privilege AI Access**: AI agents connecting via the MCP server cannot read private clipboard history unless explicitly enabled in settings.

---

## Wire Protocol Security

- Every packet on the wire is signed with Ed25519.
- Tampered payloads are rejected during deserialization.
- Replay protection is enforced through monotonically increasing 64-bit microsecond timestamps and unique UUIDs.

---

## Vulnerability Reporting

If you discover a security vulnerability in Nova, please report it via encrypted channel to security@nova.dev. Do not open public GitHub issues for security reports.
