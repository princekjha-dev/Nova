# ADR-006: Remote Desktop & Synthetic Input Injection (uinput vs RustDesk/Sunshine)

## Metadata
- **Status**: Accepted
- **Date**: September 2026

## Technology
- Linux Kernel `uinput` (`/dev/uinput`)
- Alternatives evaluated: RustDesk (AGPL-3.0), Sunshine (GPL-3.0), VNC (RFB)

## Purpose & Problem Solved
Allow an authorized paired Android device to send mouse movements, clicks, scrolling, and keyboard events to control the Linux PC with sub-millisecond local kernel injection latency.

## Licensing Analysis
- **RustDesk**: AGPL-3.0 copyleft forbids embedding inside an Apache-2.0 product.
- **Sunshine**: GPL-3.0 copyleft forbids embedding.
- **Linux `uinput`**: Standard Linux kernel character device interface. Interacting via `ioctl` across the system call boundary has no copyleft implications.

## Security Architecture
Unrestricted input injection over a network is a critical vector for arbitrary command execution.
Nova enforces:
1. Input events are strictly dropped at the protocol layer unless the session ID is in state `Authorized`.
2. The host UI renders a persistent warning modal with device name, fingerprint, and explicit `[Allow]` / `[Reject]` actions.
3. Every session can be terminated immediately from the system tray or top bar.

## Current Nova Equivalent
Implemented in `crates/nova-plugins/remote` (`RemoteControlPlugin`, `InputEvent`, `RemoteSession`, explicit authorization gating).

## Decision
**ADOPT Linux kernel `uinput` interface** behind strict host session authorization gates. **REJECT RustDesk and Sunshine** for embedding.
