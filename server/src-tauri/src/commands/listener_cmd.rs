//! 监听器相关 Tauri 命令

use crate::state::{ListenerConfig, SharedState};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct ListenerInfo {
    pub id: String,
    pub name: String,
    pub bind_address: String,
    pub port: u16,
    pub is_running: bool,
    pub encryption_key: String,
}

/// 获取所有监听器
#[tauri::command]
pub fn get_listeners(state: State<SharedState>) -> Vec<ListenerInfo> {
    state.get_listeners()
        .into_iter()
        .map(|l| ListenerInfo {
            id: l.id,
            name: l.name,
            bind_address: l.bind_address,
            port: l.port,
            is_running: l.is_running,
            encryption_key: l.encryption_key,
        })
        .collect()
}

#[derive(Debug, Deserialize)]
pub struct CreateListenerRequest {
    pub name: String,
    pub bind_address: String,
    pub port: u16,
}

/// 创建监听器
#[tauri::command]
pub async fn create_listener(
    state: State<'_, SharedState>,
    request: CreateListenerRequest,
) -> Result<ListenerInfo, String> {
    let id = Uuid::new_v4().to_string();
    
    // 生成加密密钥
    let encryption_key = shared::crypto::Crypto::generate_key_hex();
    
    let listener = ListenerConfig {
        id: id.clone(),
        name: request.name.clone(),
        bind_address: request.bind_address.clone(),
        port: request.port,
        is_running: false,
        encryption_key: encryption_key.clone(),
    };
    
    state.add_listener(listener.clone());
    
    println!("[*] Created listener '{}' with encryption key: {}", request.name, encryption_key);
    
    Ok(ListenerInfo {
        id: listener.id,
        name: listener.name,
        bind_address: listener.bind_address,
        port: listener.port,
        is_running: listener.is_running,
        encryption_key: listener.encryption_key,
    })
}

/// 启动监听器
#[tauri::command]
pub async fn start_listener(
    state: State<'_, SharedState>,
    listener_id: String,
) -> Result<(), String> {
    // 获取监听器配置
    let listener = {
        let listeners = state.listeners.read();
        listeners.get(&listener_id).cloned()
    };
    
    let listener = listener.ok_or("Listener not found")?;
    
    if listener.is_running {
        return Err("Listener already running".to_string());
    }
    
    // 启动 HTTP 服务器
    let bind_addr = format!("{}:{}", listener.bind_address, listener.port);
    let state_clone = (*state).clone();
    
    tokio::spawn(async move {
        if let Err(e) = crate::listener::start_server(state_clone, &bind_addr).await {
            eprintln!("[!] Server error: {}", e);
        }
    });
    
    // 更新监听器状态
    {
        let mut listeners = state.listeners.write();
        if let Some(l) = listeners.get_mut(&listener_id) {
            l.is_running = true;
        }
    }
    
    Ok(())
}

/// 停止监听器
#[tauri::command]
pub async fn stop_listener(
    state: State<'_, SharedState>,
    listener_id: String,
) -> Result<(), String> {
    // TODO: 实现优雅关闭
    let mut listeners = state.listeners.write();
    if let Some(l) = listeners.get_mut(&listener_id) {
        l.is_running = false;
    }
    
    Ok(())
}

/// 删除监听器
#[tauri::command]
pub async fn delete_listener(
    state: State<'_, SharedState>,
    listener_id: String,
) -> Result<(), String> {
    // 停止并删除监听器
    let mut listeners = state.listeners.write();
    if let Some(l) = listeners.get(&listener_id) {
        if l.is_running {
            // 标记为停止（实际进程可能仍在运行，需要重启程序）
            println!("[!] Listener {} is still running, it will be stopped on restart", listener_id);
        }
    }
    listeners.remove(&listener_id);
    println!("[*] Listener {} deleted", listener_id);
    
    Ok(())
}

