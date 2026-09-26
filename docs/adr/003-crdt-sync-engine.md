# ADR-003: Real-Time Synchronization Engine (yrs / Yjs vs Automerge)

## Metadata
- **Status**: Accepted
- **Date**: September 2026

## Technology
- `yrs` (Yjs Rust Port) / Yjs CRDT Protocol
- Alternative: `automerge-rs`

## Purpose & Problem Solved
Enables multi-device asynchronous and real-time editing of notes, collaborative documents, and device state without data loss or write-clobbering, guaranteeing eventual consistency across offline periods.

## Current Nova Equivalent
Implemented in `crates/nova-sync` (`CrdtDoc`, `SyncEngine`, `StateVector`, `DocDelta`, and `SyncMessage`).

## Comparison: yrs vs Automerge
1. **Memory & Performance**: `yrs` uses run-length encoding (RLE) and indexed item storage, achieving 5x–10x faster update application and significantly lower memory overhead compared to Automerge on large text documents.
2. **Ecosystem**: Yjs is the dominant industry standard for rich-text editors (TipTap, ProseMirror, Quill, Monaco).
3. **Data Model**: Yjs excels at sequential text; Automerge excels at complex JSON object graphs.

## Compatibility & Licensing
- **License**: MIT (`yrs`), MIT (`automerge`).
- **Rust / Android**: Both compile cleanly in Rust and target Android NDK.

## Decision
**ADOPT `yrs` / Yjs protocol** as the primary synchronization engine for text, notes, and task lists. **ADOPT Automerge as optional backend** for deep tree-like configuration synchronization if required later.
