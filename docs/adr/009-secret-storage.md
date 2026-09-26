# ADR-009: Secret Storage Architecture (libsecret & Android Keystore)

## Metadata
- **Status**: Accepted
- **Date**: September 2026

## Problem Statement
Never store long-term private keys, static Noise keys, or API credentials as plaintext in SQLite or standard user files.

## Platform Strategy

### Linux Workstation
- Primary: **Freedesktop Secret Service API** (`org.freedesktop.secrets` via GNOME Keyring, KWallet, or KeePassXC) using the `secret-service` pure Rust crate over session D-Bus.
- Fallback: Local directory `~/.local/share/nova/` locked to Unix file permissions `0600` (read/write by owner only).

### Android Device
- Primary: **Android Keystore System** (`KeyGenParameterSpec.Builder(PURPOSE_ENCRYPT or PURPOSE_DECRYPT)`).
- Keys are backed by hardware security modules (TEE / StrongBox) where available, preventing extraction even on rooted devices.

## Current Nova Equivalent
Implemented in `crates/nova-crypto/src/secure_store.rs` (`KeyStore` with filesystem permission enforcement and `Zeroize` memory clearing on drop).

## Decision
**ADOPT platform-native secure stores** (Secret Service on Linux, Android Keystore on Android).
