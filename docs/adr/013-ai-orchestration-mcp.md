# ADR-013: AI Assistant Orchestration & Model Context Protocol (MCP) Boundary

## Metadata
- **Status**: Accepted
- **Date**: September 2026

## Problem Statement
Providing cross-device AI context assistance without exposing private user data (passwords, clipboard, confidential notes) to cloud LLMs by default.

## Architecture
Nova functions as an **AI Orchestration Layer** rather than hosting or training model weights locally:
1. Implements a local HTTP **Model Context Protocol (MCP)** JSON-RPC 2.0 tool server on `localhost:40199`.
2. Connects to user-configured local models (Ollama, vLLM) or remote providers (Claude, OpenAI).
3. Enforces **strict capability boundaries**:
   - `nova_search_notes`: Allowed by default (read-only search over user's local notes).
   - `nova_get_recent_clipboard`: **Blocked by default**; requires explicit user permission grant in Settings.
   - `nova_list_devices`: Read-only device inventory.

## Current Nova Equivalent
Implemented in `crates/nova-plugins/ai` (`NovaAiOrchestrator`, `create_mcp_router`, `AiPermissions`).

## Decision
**ADOPT MCP JSON-RPC 2.0 tool server** with runtime permission guards.
