# ADR-008: Database Persistence Architecture (rusqlite vs SQLx)

## Metadata
- **Status**: Accepted
- **Date**: September 2026

## Technology
- SQLite 3 with Write-Ahead Logging (WAL) and `FTS5` full-text search
- Rust interface: `rusqlite` (bundled static C engine) vs `sqlx`

## Purpose & Problem Solved
Structured, durable, and queryable on-device persistence for device identities, capability permissions, Markdown note documents, clipboard history, and file transfer records.

## Comparison: rusqlite vs SQLx
- **SQLx**: Provides async runtime pooling and compile-time checked queries, but requires live database connections during compilation or cached `.sqlx` schema files. It also increases binary size and build complexity.
- **rusqlite (bundled)**: Statically compiles SQLite from verified C sources directly into the binary with zero external `.so` dependencies. Guarantees consistent support for WAL mode, strict tables, and the `FTS5` extension on all Linux distros and Android NDK targets.

## Current Nova Equivalent
Implemented in `crates/nova-core/src/store.rs` (`DeviceStore`) and `crates/nova-plugins/notes/src/lib.rs` (`NotesPlugin`).

## Decision
**ADOPT `rusqlite` with bundled SQLite**. Enable WAL mode and `FTS5` token indexing for fast local search.
