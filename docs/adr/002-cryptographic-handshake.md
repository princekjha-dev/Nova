# ADR-002: Cryptographic Identity & Noise XX Handshake

## Metadata
- **Status**: Accepted
- **Date**: September 2026

## Technology
- `snow` (Noise Protocol Framework crate)
- `ed25519-dalek` (Ed25519 keypairs and signatures)
- `sha3` (SHA3-256 for deterministic UUIDv5 device identifiers)

## Purpose & Problem Solved
Provides zero-trust mutual authentication, forward secrecy, and authenticated encryption for cross-device communication without requiring public PKI or third-party certificate authorities.

## Current Nova Equivalent
Implemented in `crates/nova-crypto` (`DeviceKeys`, `NovaNoiseInitiator`, `NovaNoiseResponder`, `NovaNoiseTransport`, and `KeyStore`).

## Integration Method
Direct Rust crate dependency (`snow 0.9` using pattern `Noise_XX_25519_ChaChaPoly_BLAKE2s`).

## Compatibility & Licensing
- **Rust Compatibility**: Pure Rust.
- **Linux Compatibility**: Universal.
- **Android Compatibility**: Compiles into NDK shared library.
- **License**: Apache-2.0 / MIT.

## Security Implications
- Noise XX provides mutual authentication: both initiator and responder exchange and verify their static public keys.
- Forward secrecy: compromised long-term keys do not decrypt historical recorded session traffic.
- Zeroize trait ensures private key seeds are cleared from RAM upon drop.

## Performance Implications
ChaCha20-Poly1305 and BLAKE2s are hardware-efficient on both modern x86_64 CPUs and ARM64 mobile chipsets, outperforming RSA or legacy TLS handshakes.

## Decision
**ADOPT**. `snow` + `ed25519-dalek` form Nova's cryptographic backbone.
