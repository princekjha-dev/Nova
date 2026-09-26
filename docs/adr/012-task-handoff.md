# ADR-012: Task Handoff & Cross-Device Activity Routing

## Metadata
- **Status**: Accepted
- **Date**: September 2026

## Purpose & Problem Solved
Enables frictionless handoff of open work between Linux and Android devices (browser tabs, research URLs, note drafts, and AI prompts) without requiring proprietary browser sync accounts.

## Current Nova Equivalent
Implemented in `crates/nova-plugins/handoff` (`HandoffPlugin`, `TaskContent`, `HandoffTask`, `HandoffMessage`).

## Implementation Model
- **Task Types**:
  - `TaskContent::Url { url, title }`
  - `TaskContent::BrowserTab { url, scroll_y, title }`
  - `TaskContent::NoteDraft { title, content }`
  - `TaskContent::FilePreview { filename, path_or_url }`
  - `TaskContent::AiPrompt { prompt, context_summary }`
- **Linux Integration**:
  - D-Bus service `dev.nova.Handoff` and browser WebExtension (Firefox/Chrome).
- **Android Integration**:
  - `ACTION_VIEW` intent dispatching to native apps or browser.

## Decision
**ADOPT extensible task model** with native platform intent dispatchers.
