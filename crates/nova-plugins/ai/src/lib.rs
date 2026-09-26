use anyhow::{anyhow, Result};
use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub const DEFAULT_MCP_PORT: u16 = 40199;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPermissions {
    pub allow_notes: bool,
    pub allow_clipboard: bool,
    pub allow_files: bool,
    pub allow_tasks: bool,
    pub allow_devices: bool,
}

impl Default for AiPermissions {
    fn default() -> Self {
        Self {
            allow_notes: true,
            allow_clipboard: false, // Strict default: clipboard requires explicit user grant
            allow_files: false,
            allow_tasks: true,
            allow_devices: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    pub result: Option<Value>,
    pub error: Option<Value>,
}

pub struct NovaAiOrchestrator {
    permissions: Arc<RwLock<AiPermissions>>,
    notes_search_fn: Option<Arc<dyn Fn(&str) -> Vec<Value> + Send + Sync>>,
    clipboard_fn: Option<Arc<dyn Fn() -> Vec<Value> + Send + Sync>>,
    devices_fn: Option<Arc<dyn Fn() -> Vec<Value> + Send + Sync>>,
}

impl NovaAiOrchestrator {
    pub fn new() -> Self {
        Self {
            permissions: Arc::new(RwLock::new(AiPermissions::default())),
            notes_search_fn: None,
            clipboard_fn: None,
            devices_fn: None,
        }
    }

    pub fn set_notes_provider<F>(&mut self, f: F)
    where
        F: Fn(&str) -> Vec<Value> + Send + Sync + 'static,
    {
        self.notes_search_fn = Some(Arc::new(f));
    }

    pub fn set_clipboard_provider<F>(&mut self, f: F)
    where
        F: Fn() -> Vec<Value> + Send + Sync + 'static,
    {
        self.clipboard_fn = Some(Arc::new(f));
    }

    pub fn set_devices_provider<F>(&mut self, f: F)
    where
        F: Fn() -> Vec<Value> + Send + Sync + 'static,
    {
        self.devices_fn = Some(Arc::new(f));
    }

    pub async fn update_permissions(&self, perms: AiPermissions) {
        *self.permissions.write().await = perms;
    }

    pub fn list_tools(&self) -> Vec<McpToolDefinition> {
        vec![
            McpToolDefinition {
                name: "nova_search_notes".to_string(),
                description: "Search user's private local-first Markdown notes".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search query" }
                    },
                    "required": ["query"]
                }),
            },
            McpToolDefinition {
                name: "nova_get_recent_clipboard".to_string(),
                description: "Get recent clipboard history (requires explicit user grant)".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "limit": { "type": "number", "description": "Number of entries" }
                    }
                }),
            },
            McpToolDefinition {
                name: "nova_list_devices".to_string(),
                description: "List connected paired Nova ecosystem devices".to_string(),
                input_schema: json!({
                    "type": "object"
                }),
            },
        ]
    }

    pub async fn execute_tool(&self, name: &str, arguments: &Value) -> Result<Value> {
        let perms = self.permissions.read().await;

        match name {
            "nova_search_notes" => {
                if !perms.allow_notes {
                    return Err(anyhow!("Permission denied: AI note access is disabled by user policy"));
                }
                let query = arguments.get("query").and_then(|q| q.as_str()).unwrap_or("");
                if let Some(ref f) = self.notes_search_fn {
                    let results = f(query);
                    Ok(json!({ "notes": results }))
                } else {
                    Ok(json!({ "notes": [] }))
                }
            }
            "nova_get_recent_clipboard" => {
                if !perms.allow_clipboard {
                    return Err(anyhow!("Permission denied: AI clipboard access is strictly disabled. User must explicitly grant access in Settings."));
                }
                if let Some(ref f) = self.clipboard_fn {
                    let results = f();
                    Ok(json!({ "clipboard": results }))
                } else {
                    Ok(json!({ "clipboard": [] }))
                }
            }
            "nova_list_devices" => {
                if !perms.allow_devices {
                    return Err(anyhow!("Permission denied: Device access disabled"));
                }
                if let Some(ref f) = self.devices_fn {
                    let results = f();
                    Ok(json!({ "devices": results }))
                } else {
                    Ok(json!({ "devices": [] }))
                }
            }
            _ => Err(anyhow!("Unknown tool: {}", name)),
        }
    }
}

pub fn create_mcp_router(orchestrator: Arc<NovaAiOrchestrator>) -> Router {
    Router::new()
        .route("/mcp/tools/list", post({
            let orch = orchestrator.clone();
            move || {
                let tools = orch.list_tools();
                async move { Json(json!({ "tools": tools })) }
            }
        }))
        .route("/mcp/tools/call", post({
            let orch = orchestrator.clone();
            move |Json(req): Json<Value>| {
                let orch = orch.clone();
                async move {
                    let name = req.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let arguments = req.get("arguments").cloned().unwrap_or(json!({}));
                    match orch.execute_tool(name, &arguments).await {
                        Ok(res) => Json(json!({ "content": [{ "type": "text", "text": res.to_string() }] })),
                        Err(e) => Json(json!({ "isError": true, "content": [{ "type": "text", "text": e.to_string() }] })),
                    }
                }
            }
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ai_tool_permission_boundaries() -> Result<()> {
        let mut orch = NovaAiOrchestrator::new();
        orch.set_notes_provider(|q| {
            vec![json!({ "title": "WebRTC Research", "match": q })]
        });
        orch.set_clipboard_provider(|| {
            vec![json!({ "text": "secret password" })]
        });

        // Notes should succeed by default
        let note_res = orch.execute_tool("nova_search_notes", &json!({ "query": "WebRTC" })).await?;
        assert!(note_res.to_string().contains("WebRTC Research"));

        // Clipboard should FAIL by default due to strict permission boundary
        let clip_res = orch.execute_tool("nova_get_recent_clipboard", &json!({})).await;
        assert!(clip_res.is_err());
        assert!(clip_res.unwrap_err().to_string().contains("Permission denied"));

        // Grant clipboard permission explicitly
        orch.update_permissions(AiPermissions {
            allow_notes: true,
            allow_clipboard: true,
            allow_files: false,
            allow_tasks: true,
            allow_devices: true,
        }).await;

        let clip_res2 = orch.execute_tool("nova_get_recent_clipboard", &json!({})).await?;
        assert!(clip_res2.to_string().contains("secret password"));

        Ok(())
    }
}
