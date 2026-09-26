use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    routing::get,
    Router,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info};

type PeerSender = mpsc::UnboundedSender<String>;
type Peers = Arc<RwLock<HashMap<String, PeerSender>>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let peers: Peers = Arc::new(RwLock::new(HashMap::new()));

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route(
            "/v1/signal",
            get({
                let peers = peers.clone();
                move |ws: WebSocketUpgrade| {
                    let peers = peers.clone();
                    async move { ws.on_upgrade(move |socket| handle_socket(socket, peers)) }
                }
            }),
        );

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Nova Signaling Server running on {}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn handle_socket(mut socket: WebSocket, peers: Peers) {
    let mut current_device_id: Option<String> = None;
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Forward messages from outbound channel to client
    let mut send_task = tokio::spawn(async move {
        // Wait, socket is split or we handle within loop
    });

    while let Some(msg_res) = socket.recv().await {
        match msg_res {
            Ok(Message::Text(text)) => {
                let parsed: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let msg_type = parsed.get("type").and_then(|t| t.as_str()).unwrap_or("");
                match msg_type {
                    "register" => {
                        if let Some(id) = parsed.get("device_id").and_then(|i| i.as_str()) {
                            current_device_id = Some(id.to_string());
                            peers.write().await.insert(id.to_string(), tx.clone());
                            info!("Device registered: {}", id);
                            let _ = socket
                                .send(Message::Text(
                                    r#"{"type":"registered","status":"ok"}"#.to_string(),
                                ))
                                .await;
                        }
                    }
                    "offer" | "answer" | "ice" => {
                        if let Some(to) = parsed.get("to").and_then(|t| t.as_str()) {
                            let peers_guard = peers.read().await;
                            if let Some(target_tx) = peers_guard.get(to) {
                                let _ = target_tx.send(text);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Message::Close(_)) => break,
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }

        // Drain any messages waiting for this socket
        while let Ok(outgoing) = rx.try_recv() {
            if socket.send(Message::Text(outgoing)).await.is_err() {
                break;
            }
        }
    }

    if let Some(ref id) = current_device_id {
        peers.write().await.remove(id);
        info!("Device disconnected: {}", id);
    }
}
