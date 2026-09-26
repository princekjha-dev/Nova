# ADR-007: Screen Extension Feasibility on Wayland (Virtual Monitor vs Window Mirroring)

## Metadata
- **Status**: Accepted
- **Date**: September 2026

## Problem Statement
Can an Android phone or tablet function as a true extended second monitor for a Linux workstation under Wayland?

## Technical Feasibility Analysis
Wayland explicitly delegates display topology to the compositor (Mutter in GNOME, KWin in KDE Plasma, wlroots in Sway/Hyprland). There is no single universal X11-style `xrandr --addmode` command.

1. **wlroots (Sway / Hyprland)**:
   - Compositors implement the `wlr-output-management-unstable-v1` and `headless` backend protocols (`swaymsg create_output`).
   - Allows creating an actual virtual monitor buffer, rendering workspaces onto it, and streaming via PipeWire.
   - **Feasibility: High (Native OS-level extension)**.
2. **GNOME (Mutter)**:
   - Does not expose virtual monitor creation to unprivileged clients without a private Mutter D-Bus API (`org.gnome.Mutter.ScreenCast.RecordVirtual`).
   - Requires a dedicated GNOME Shell extension with system permissions.
   - **Feasibility: Moderate (Requires separate extension)**.
3. **KDE Plasma (KWin)**:
   - KWin supports virtual outputs via KScreen and D-Bus interfaces.
   - **Feasibility: Moderate**.

## Architectural Decision
**Clearly distinguish Screen Mirroring from True Screen Extension**.
- Do not fake "screen extension" by merely capturing an active window.
- Implement true headless virtual monitor creation as a compositor-specific plugin:
  - Phase 1: Native support for wlroots headless outputs.
  - Phase 2: Independent GNOME Shell extension package for Mutter virtual outputs.
