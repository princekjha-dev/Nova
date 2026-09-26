# ADR-010: Desktop Client Framework (Tauri / Web UI vs Qt / Electron)

## Metadata
- **Status**: Accepted
- **Date**: September 2026

## Framework Evaluation

| Criteria | Tauri 2.0 | Qt 6 / QML (C++) | Electron | GTK 4 / Libadwaita |
|---|---|---|---|---|
| **Binary Size** | ~10 MB | 40–80 MB | 150–220 MB | 20–40 MB |
| **RAM Footprint** | ~35 MB | ~45 MB | ~200 MB+ | ~35 MB |
| **Rust Integration** | Native IPC & Core | Cxx / Rust FFI | Node FFI | Relies on GTK-rs |
| **Licensing** | MIT / Apache-2.0 | LGPL-3.0 (Dynamic linking ceremony) | MIT (Chromium bloat) | LGPL-2.1 |
| **UI Aesthetics & Styling** | Web CSS / Glassmorphism | QML / CSS | Web CSS | Strict GNOME styling |

## Analysis
- **Electron**: Rejected due to 200 MB+ distribution overhead, high battery consumption on laptops, and slow startup time.
- **Qt**: Rejected due to LGPL-3.0 complications and poor Rust interoperability.
- **Tauri**: Combines the native system WebKit engine (WebKit2GTK on Linux) with a memory-safe Rust backend. Privileged Linux operations (system tray, discovery, crypto, uinput) remain strictly isolated in the Rust daemon process, communicating across Tauri's IPC bridge.

## Decision
**ADOPT Tauri / Web UI** for the Linux desktop client (`apps/linux`).
