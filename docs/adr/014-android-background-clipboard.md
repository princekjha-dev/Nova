# ADR-014: Android Background Restrictions & Clipboard Sync Strategy (API 29+)

## Metadata
- **Status**: Accepted
- **Date**: September 2026

## Problem Statement
Starting in Android 10 (API 29), Android strictly blocks background apps and background services from calling `ClipboardManager.getPrimaryClip()`. Attempts result in an empty clip or security exception.

## Multi-Pronged Architectural Solution
To provide reliable Android -> Linux clipboard sync within Android OS restrictions without requiring root:

1. **Quick Settings Tile (`ClipboardTile`)**:
   - Implements `android.service.quicksettings.TileService`.
   - When the user taps the tile in the notification shade, the system launches it with **foreground user intent**, allowing immediate read access to the clipboard and transmitting it to the Linux PC.
2. **Persistent Foreground Service (`NovaService`)**:
   - Operates with an ongoing low-priority notification (`FOREGROUND_SERVICE_CONNECTED_DEVICE`).
   - Receives incoming clipboard sync from Linux and applies it to the Android clipboard.
3. **Floating Bubble / Accessibility Action**:
   - Optional accessibility service for power users desiring automatic background capture.

## Current Nova Equivalent
Implemented in `apps/android/app/src/main/kotlin/dev/nova/tile/ClipboardTile.kt` and `apps/android/app/src/main/kotlin/dev/nova/core/NovaService.kt`.

## Decision
**ADOPT Quick Settings Tile + Persistent Foreground Service** as the standard non-root solution for Android 10+.
