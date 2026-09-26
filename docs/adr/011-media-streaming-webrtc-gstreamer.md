# ADR-011: Media Pipeline & Low-Latency Streaming (WebRTC & GStreamer vs FFmpeg)

## Metadata
- **Status**: Accepted
- **Date**: September 2026

## Problem Statement
Real-time screen projection and remote desktop require sub-50ms latency, adaptive bitrates, and hardware-accelerated encoding (VAAPI / NVENC) on Linux, streamed to Android over Wi-Fi.

## Architectural Pipeline
```text
Wayland Desktop
      ↓
xdg-desktop-portal (ashpd)
      ↓
PipeWire Stream (fd)
      ↓
GStreamer / VAAPI Encoder (H.264 / AV1)
      ↓
webrtc-rs (DTLS-SRTP / RTP Transport)
      ↓
Android MediaCodec Hardware Decoder (SurfaceView)
```

## Technology Selection
1. **Capture**: PipeWire via `xdg-desktop-portal` (standards-compliant, compositor-agnostic).
2. **Encoding**: GStreamer dynamically linked (`gstreamer-rs`), detecting hardware acceleration at runtime (`vaapih264enc`, `nvh264enc`, or software fallback `x264enc`).
3. **Transport**: Pure Rust async WebRTC (`webrtc-rs`), handling congestion control, packet loss retransmission, and NAT traversal.

## Decision
**ADOPT GStreamer (dynamic) + webrtc-rs** for the Linux -> Android streaming pipeline.
