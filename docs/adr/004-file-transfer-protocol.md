# ADR-004: File EasyShare Protocol (LocalSend Specification vs Syncthing)

## Metadata
- **Status**: Accepted
- **Date**: September 2026

## Technology
- LocalSend Protocol v2.1 Specification
- Syncthing Block Exchange Protocol (BEP)

## Purpose & Problem Solved
Ad-hoc, high-speed transfer of documents, photos, and archives between Linux and Android devices over the local network with progress feedback, cancellability, and integrity verification.

## Analysis
- **Syncthing**: Designed for continuous bidirectional directory tree synchronization. It maintains persistent database indexes of entire folders. Too heavy and intrusive for user-initiated "Send this photo to my PC" workflows.
- **LocalSend**: Designed specifically for ad-hoc LAN file sharing. Simple, clean state machine: `Offer -> Decision -> Chunked Stream -> Complete`.

## Current Nova Equivalent
Implemented in `crates/nova-plugins/files` (`FileTransferPlugin`, `FileTransferMessage`, `CHUNK_SIZE = 64 KiB`, SHA-256 verification).

## Licensing & Architecture
- LocalSend protocol specification is MIT/Apache-2.0.
- Nova implements the transfer state machine natively in Rust without depending on the Flutter runtime.

## Decision
**ADOPT LocalSend-compatible protocol**. Keep Syncthing as an optional independent external backend for continuous folder mirroring if users explicitly configure it.
