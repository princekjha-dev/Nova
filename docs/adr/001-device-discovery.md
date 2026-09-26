# ADR-001: Device Discovery Architecture (mDNS / DNS-SD & QR Pairing)

## Metadata
- **Status**: Accepted
- **Date**: September 2026
- **Deciders**: Systems Architect

## Technology
- `mdns-sd` (Pure Rust crate)
- Avahi Daemon (Linux D-Bus)
- `qrcode` (Pure Rust SVG renderer)

## Purpose & Problem Solved
Automates zero-configuration device discovery on local Wi-Fi networks without requiring cloud rendezvous, while providing a fallback pairing mechanism via QR codes when multicast UDP traffic is blocked by enterprise Wi-Fi routers.

## Current Nova Equivalent
Implemented in `crates/nova-discovery` (`NovaMdnsAnnouncer`, `NovaMdnsBrowser`, `QrPairingPayload`, and `PairingManager`).

## Integration Method
- **Primary**: Direct Rust crate (`mdns-sd` 0.13) for cross-platform zero-dependency multicast DNS (`_nova._tcp.local.`, port 53418).
- **Secondary**: QR code pairing payload encoding device ID, public keys, LAN IP addresses, and a 6-digit confirmation PIN.

## Compatibility & Licensing
- **Rust Compatibility**: Excellent (pure Rust).
- **Linux Compatibility**: Standard UDP 5353.
- **Android Compatibility**: Supported natively via Android `NsdManager` or embedded Rust runtime.
- **License**: MIT / Apache-2.0.

## Security Implications
mDNS broadcasts unencrypted hostnames and fingerprints. Security is not delegated to mDNS; discovery only establishes IP endpoints. Full cryptographic mutual authentication is subsequently enforced via the Noise XX handshake.

## Performance Implications
Lightweight UDP broadcast on startup and service shutdown; minimal battery impact on mobile.

## Decision
**ADOPT**. Use `mdns-sd` for discovery, supplemented by QR pairing for zero-trust confirmation.
