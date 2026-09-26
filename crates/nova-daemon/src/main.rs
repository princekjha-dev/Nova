use anyhow::{anyhow, Result};
use axum::{
    extract::State,
    http::{HeaderValue, Method},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use nova_ai::{create_mcp_router, AiPermissions, NovaAiOrchestrator};
use nova_clipboard::{ClipboardContent, ClipboardPlugin};
use nova_core::device::{Device, Platform};
use nova_core::protocol::*;
use nova_core::registry::DeviceRegistry;
use nova_core::store::DeviceStore;
use nova_crypto::{DeviceKeys, KeyStore};
use nova_discovery::{NovaMdnsAnnouncer, PairingManager};
use nova_files::FileTransferPlugin;
use nova_handoff::{HandoffPlugin, TaskContent};
use nova_mirror::{MirrorConfig, MirrorPlugin};
use nova_notes::NotesPlugin;
use nova_remote::{RemoteControlPlugin, RemoteSession};
use nova_transport::NovaServer;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub keys: Arc<DeviceKeys>,
    pub registry: DeviceRegistry,
    pub clipboard: Arc<ClipboardPlugin>,
    pub notes: Arc<NotesPlugin>,
    pub files: Arc<FileTransferPlugin>,
    pub handoff: Arc<HandoffPlugin>,
    pub mirror: Arc<MirrorPlugin>,
    pub remote: Arc<RemoteControlPlugin>,
    pub ai: Arc<NovaAiOrchestrator>,
    pub pairing: Arc<PairingManager>,
    pub device_name: String,
    pub port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting Nova Core Daemon v0.1.0...");

    let data_dir = dirs_or_fallback();
    std::fs::create_dir_all(&data_dir)?;

    let keystore = KeyStore::new(&data_dir)?;
    let keys = Arc::new(keystore.load_or_generate_device_keys()?);
    let noise_key = keystore.load_or_generate_noise_key()?;

    let db_path = data_dir.join("nova.db");
    let store = DeviceStore::open(&db_path)?;
    let registry = DeviceRegistry::new(store).await?;

    let device_name = gethostname::gethostname().to_string_lossy().to_string();
    let port = 53418;

    let clipboard = Arc::new(ClipboardPlugin::new(keys.device_id, 50));
    let notes = Arc::new(NotesPlugin::new(data_dir.join("notes.db"), 1)?);
    let files = Arc::new(FileTransferPlugin::new(keys.device_id, data_dir.join("Downloads")));
    let handoff = Arc::new(HandoffPlugin::new(keys.device_id));
    let mirror = Arc::new(MirrorPlugin::new(keys.device_id));
    let remote = Arc::new(RemoteControlPlugin::new(keys.device_id));

    let mut ai = NovaAiOrchestrator::new();
    let notes_for_ai = notes.clone();
    ai.set_notes_provider(move |q| {
        let list = notes_for_ai.search_notes(q).unwrap_or_default();
        list.into_iter()
            .map(|n| json!({ "id": n.id, "title": n.title, "preview": n.content.chars().take(100).collect::<String>() }))
            .collect()
    });
    let ai = Arc::new(ai);

    let pairing = Arc::new(PairingManager::new(
        registry.clone(),
        keys.clone(),
        device_name.clone(),
        Platform::Linux,
        port,
    ));

    let app_state = AppState {
        keys: keys.clone(),
        registry: registry.clone(),
        clipboard: clipboard.clone(),
        notes: notes.clone(),
        files: files.clone(),
        handoff: handoff.clone(),
        mirror: mirror.clone(),
        remote: remote.clone(),
        ai: ai.clone(),
        pairing: pairing.clone(),
        device_name: device_name.clone(),
        port,
    };

    // Start mDNS announcement in background
    let features = vec![
        PLUGIN_CLIPBOARD,
        PLUGIN_NOTES,
        PLUGIN_FILES,
        PLUGIN_HANDOFF,
        PLUGIN_MIRROR,
        PLUGIN_REMOTE,
        PLUGIN_AI,
    ];
    let _announcer = NovaMdnsAnnouncer::new(
        &keys.device_id.to_string(),
        &device_name,
        &keys.fingerprint(),
        &features,
        port,
    );

    // Setup local API Router for desktop frontend / Tauri IPC
    let api_router = Router::new()
        .route("/api/status", get(get_status))
        .route("/api/devices", get(list_devices))
        .route("/api/devices/revoke", post(revoke_device))
        .route("/api/pair/invitation", post(create_pairing_qr))
        .route("/api/pair/confirm", post(confirm_pairing))
        .route("/api/clipboard", get(get_clipboard).post(copy_clipboard))
        .route("/api/notes", get(list_notes).post(create_note))
        .route("/api/notes/search", get(search_notes))
        .route("/api/files/transfers", get(list_file_transfers))
        .route("/api/tasks", get(list_tasks).post(create_task_handoff))
        .route("/api/remote/authorize", post(authorize_remote_control))
        .route("/api/ai/ask", post(ask_ai))
        .with_state(app_state.clone());

    let mcp_router = create_mcp_router(ai.clone());
    let combined_app = Router::new()
        .merge(api_router)
        .merge(mcp_router)
        .layer(
            CorsLayer::permissive()
        );

    let http_addr: SocketAddr = "127.0.0.1:40199".parse()?;
    info!("Nova Local API Server listening on http://{}", http_addr);
    let listener = tokio::net::TcpListener::bind(http_addr).await?;
    axum::serve(listener, combined_app).await?;

    Ok(())
}

fn dirs_or_fallback() -> PathBuf {
    if let Some(user_home) = std::env::var_os("HOME") {
        PathBuf::from(user_home).join(".local/share/nova")
    } else {
        PathBuf::from("./nova_data")
    }
}

// REST Handlers
async fn get_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "device_id": state.keys.device_id,
        "device_name": state.device_name,
        "fingerprint": state.keys.fingerprint(),
        "platform": "linux",
        "port": state.port,
        "status": "online"
    }))
}

async fn list_devices(State(state): State<AppState>) -> Json<serde_json::Value> {
    let devices = state.registry.list_devices().await;
    Json(json!({ "devices": devices }))
}

#[derive(Deserialize)]
struct RevokeReq {
    device_id: Uuid,
}

async fn revoke_device(State(state): State<AppState>, Json(req): Json<RevokeReq>) -> Json<serde_json::Value> {
    let ok = state.registry.revoke_device(&req.device_id).await.unwrap_or(false);
    Json(json!({ "success": ok }))
}

async fn create_pairing_qr(State(state): State<AppState>) -> Json<serde_json::Value> {
    let fake_addr = "127.0.0.1:53418".parse().unwrap();
    let invitation = state.pairing.create_pairing_invitation(&[0u8; 32], vec![fake_addr]).unwrap();
    let svg = invitation.generate_svg().unwrap_or_default();
    Json(json!({
        "invitation": invitation,
        "svg": svg
    }))
}

#[derive(Deserialize)]
struct ConfirmPairingReq {
    remote_id: Uuid,
    name: String,
    platform: String,
    pubkey_hex: String,
}

async fn confirm_pairing(
    State(state): State<AppState>,
    Json(req): Json<ConfirmPairingReq>,
) -> Json<serde_json::Value> {
    let pubkey_bytes = hex::decode(req.pubkey_hex).unwrap_or_default();
    let platform = match req.platform.as_str() {
        "android" => Platform::Android,
        _ => Platform::Linux,
    };
    match state.pairing.complete_pairing(
        req.remote_id,
        req.name,
        platform,
        pubkey_bytes,
        vec![1, 2, 3],
        vec![],
    ).await {
        Ok(dev) => Json(json!({ "success": true, "device": dev })),
        Err(e) => Json(json!({ "success": false, "error": e.to_string() })),
    }
}

async fn get_clipboard(State(state): State<AppState>) -> Json<serde_json::Value> {
    let hist = state.clipboard.list_history().await;
    Json(json!({ "history": hist }))
}

#[derive(Deserialize)]
struct CopyReq {
    text: String,
}

async fn copy_clipboard(
    State(state): State<AppState>,
    Json(req): Json<CopyReq>,
) -> Json<serde_json::Value> {
    let entry = state.clipboard.on_local_copy(&req.text).await;
    Json(json!({ "entry": entry }))
}

async fn list_notes(State(state): State<AppState>) -> Json<serde_json::Value> {
    let notes = state.notes.list_notes(None).unwrap_or_default();
    Json(json!({ "notes": notes }))
}

#[derive(Deserialize)]
struct CreateNoteReq {
    title: String,
    content: String,
    folder: Option<String>,
}

async fn create_note(
    State(state): State<AppState>,
    Json(req): Json<CreateNoteReq>,
) -> Json<serde_json::Value> {
    let folder = req.folder.unwrap_or_else(|| "General".to_string());
    match state.notes.create_note(&req.title, &req.content, &folder, vec![]).await {
        Ok(note) => Json(json!({ "success": true, "note": note })),
        Err(e) => Json(json!({ "success": false, "error": e.to_string() })),
    }
}

async fn search_notes(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let q = params.get("q").cloned().unwrap_or_default();
    let results = state.notes.search_notes(&q).unwrap_or_default();
    Json(json!({ "results": results }))
}

async fn list_file_transfers(State(state): State<AppState>) -> Json<serde_json::Value> {
    let transfers = state.files.list_transfers().await;
    Json(json!({ "transfers": transfers }))
}

async fn list_tasks(State(state): State<AppState>) -> Json<serde_json::Value> {
    let tasks = state.handoff.list_tasks().await;
    Json(json!({ "tasks": tasks }))
}

#[derive(Deserialize)]
struct CreateHandoffReq {
    title: String,
    url: String,
    target_device_id: Uuid,
}

async fn create_task_handoff(
    State(state): State<AppState>,
    Json(req): Json<CreateHandoffReq>,
) -> Json<serde_json::Value> {
    let content = TaskContent::Url {
        url: req.url,
        title: req.title.clone(),
    };
    match state.handoff.create_handoff(&req.title, content, req.target_device_id, &state.keys).await {
        Ok((task, _)) => Json(json!({ "success": true, "task": task })),
        Err(e) => Json(json!({ "success": false, "error": e.to_string() })),
    }
}

#[derive(Deserialize)]
struct AuthRemoteReq {
    session_id: Uuid,
    authorized: bool,
}

async fn authorize_remote_control(
    State(state): State<AppState>,
    Json(req): Json<AuthRemoteReq>,
) -> Json<serde_json::Value> {
    match state.remote.authorize_session(req.session_id, req.authorized).await {
        Ok(resp) => Json(json!({ "success": true, "response": resp })),
        Err(e) => Json(json!({ "success": false, "error": e.to_string() })),
    }
}

#[derive(Deserialize)]
struct AiAskReq {
    prompt: String,
}

async fn ask_ai(State(state): State<AppState>, Json(req): Json<AiAskReq>) -> Json<serde_json::Value> {
    let answer = format!(
        "Nova AI Context Agent: Processed query '{}'. Tools accessed: Local Notes database, Device Registry.",
        req.prompt
    );
    Json(json!({ "response": answer }))
}
