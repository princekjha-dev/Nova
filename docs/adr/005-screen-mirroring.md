# ADR-005: Screen Mirroring Architecture (scrcpy Integration & PipeWire)

## Metadata
- **Status**: Accepted
- **Date**: September 2026

## Technology
- `scrcpy-server` (Apache-2.0, Genymobile)
- PipeWire / `xdg-desktop-portal` (Linux Wayland Capture)
- GStreamer / WebRTC (Linux -> Android encode & streaming)

## Purpose & Problem Solved
High-FPS, low-latency screen projection between Android phones and Linux workstations in both directions:
1. **Android -> Linux**: Viewing and controlling the phone from the Linux desktop.
2. **Linux -> Android**: Streaming the desktop to the phone screen.

## Architectural Assessment
- **Android -> Linux**: `scrcpy` is an Apache-2.0 project that executes an in-memory Java server on Android, utilizing private MediaCodec surface encoders to stream raw H.264/H.265/AV1 frames over a socket with sub-35ms latency. Rebuilding this from scratch in Android userspace would introduce severe performance regressions.
- **Linux -> Android**: Wayland prevents unauthorized framebuffer access. Standard `xdg-desktop-portal` invokes PipeWire to capture the desktop stream with compositor consent.

## Current Nova Equivalent
Implemented in `crates/nova-plugins/mirror` (`MirrorPlugin`, `MirrorConfig`, `MirrorMessage`, `VideoCodec`).

## Decision
**ADOPT scrcpy server integration** for Android -> Linux screen projection.
**ADOPT PipeWire + xdg-desktop-portal** for Linux -> Android desktop capture.
