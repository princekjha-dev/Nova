# ADR-015: WAN Relay & NAT Traversal (Signaling Server WebSocket + coturn STUN/TURN fallback)

## Metadata
- **Status**: Accepted
- **Date**: September 2026

## Problem Statement
When devices are not on the same local Wi-Fi network (e.g. mobile 5G to home workstation), direct TCP connections are blocked by carrier-grade NAT (CGNAT) and stateful firewalls.

## Network Hierarchy
```text
1. Direct LAN TCP + TLS (Port 53418)   -> Lowest latency (<2ms), primary path
2. Local P2P WebRTC ICE (LAN Host)      -> High-throughput media streaming
3. P2P WebRTC via STUN (WAN Hole-punch) -> Direct connection across WAN
4. TURN Relay (coturn fallback)         -> Encrypted relay when symmetric NAT prevents direct P2P
```

## Signaling Architecture
- Lightweight WebSocket server (`services/signaling` using Axum + Tokio).
- Only relays end-to-end encrypted SDP offer/answer blobs and ICE candidate strings.
- Server has zero visibility into user data, note contents, or video feeds.

## Current Nova Equivalent
Implemented in `services/signaling/src/main.rs`.

## Decision
**ADOPT LAN-first with self-hostable WebSocket signaling & coturn TURN fallback**.
