//! 文件管理处理器

use shared::messages::{
    DirectoryListingResponse, FileDelete, FileDeleteResponse, FileDownload,
    FileDownloadResponse, FileInfo, FileUpload, FileUploadResponse, GetDirectoryListing,
};
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

/// 获取目录列表
pub fn get_directory_listing(req: &GetDirectoryListing) -> DirectoryListingResponse {
    let path = if req.path.is_empty() {
        // 默认返回所有驱动器（Windows）或根目录（Linux）
        #[cfg(windows)]
        {
            return get_drives();
        }
        #[cfg(not(windows))]
        {
            "/".to_string()
        }
    } else {
        req.path.clone()
    };

    let dir_path = Path::new(&path);

    if !dir_path.exists() {
        return DirectoryListingResponse {
            path: path.clone(),
            entries: vec![],
            error: Some(format!("Path does not exist: {}", path)),
        };
    }

    if !dir_path.is_dir() {
        return DirectoryListingResponse {
            path: path.clone(),
            entries: vec![],
            error: Some(format!("Path is not a directory: {}", path)),
        };
    }

    match fs::read_dir(&path) {
        Ok(entries) => {
            let mut files: Vec<FileInfo> = entries
                .filter_map(|e| e.ok())
                .filter_map(|entry| {
                    let file_path = entry.path();
                    let metadata = entry.metadata().ok()?;
                    
                    let name = entry.file_name().to_string_lossy().to_string();
                    let full_path = file_path.to_string_lossy().to_string();
                    let is_dir = metadata.is_dir();
                    let size = if is_dir { 0 } else { metadata.len() };
                    let modified = metadata
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);

                    Some(FileInfo {
                        name,
                        path: full_path,
                        is_dir,
                        size,
                        modified,
                    })
                })
                .collect();

            // 排序：目录优先，然后按名称
            files.sort_by(|a, b| {
                if a.is_dir == b.is_dir {
                    a.name.to_lowercase().cmp(&b.name.to_lowercase())
                } else if a.is_dir {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            });

            DirectoryListingResponse {
                path,
                entries: files,
                error: None,
            }
        }
        Err(e) => DirectoryListingResponse {
            path,
            entries: vec![],
            error: Some(format!("Failed to read directory: {}", e)),
        },
    }
}

/// 获取 Windows 驱动器列表
#[cfg(windows)]
fn get_drives() -> DirectoryListingResponse {
    let mut drives = Vec::new();
    
    // 检查 A-Z 驱动器
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        let path = Path::new(&drive);
        if path.exists() {
            drives.push(FileInfo {
                name: drive.clone(),
                path: drive,
                is_dir: true,
                size: 0,
                modified: 0,
            });
        }
    }
    
    DirectoryListingResponse {
        path: String::new(),
        entries: drives,
        error: None,
    }
}

/// 下载文件
pub fn download_file(req: &FileDownload) -> FileDownloadResponse {
    let path = Path::new(&req.path);
    
    if !path.exists() {
        return FileDownloadResponse {
            path: req.path.clone(),
            data: vec![],
            is_complete: true,
            error: Some("File does not exist".to_string()),
        };
    }
    
    if path.is_dir() {
        return FileDownloadResponse {
            path: req.path.clone(),
            data: vec![],
            is_complete: true,
            error: Some("Cannot download a directory".to_string()),
        };
    }
    
    match fs::read(&req.path) {
        Ok(data) => FileDownloadResponse {
            path: req.path.clone(),
            data,
            is_complete: true,
            error: None,
        },
        Err(e) => FileDownloadResponse {
            path: req.path.clone(),
            data: vec![],
            is_complete: true,
            error: Some(format!("Failed to read file: {}", e)),
        },
    }
}

/// 上传文件
pub fn upload_file(req: &FileUpload) -> FileUploadResponse {
    match fs::write(&req.path, &req.data) {
        Ok(_) => FileUploadResponse {
            path: req.path.clone(),
            success: true,
            error: None,
        },
        Err(e) => FileUploadResponse {
            path: req.path.clone(),
            success: false,
            error: Some(format!("Failed to write file: {}", e)),
        },
    }
}

/// 删除文件或目录
pub fn delete_file(req: &FileDelete) -> FileDeleteResponse {
    let path = Path::new(&req.path);
    
    if !path.exists() {
        return FileDeleteResponse {
            path: req.path.clone(),
            success: false,
            error: Some("Path does not exist".to_string()),
        };
    }
    
    let result = if path.is_dir() {
        fs::remove_dir_all(&req.path)
    } else {
        fs::remove_file(&req.path)
    };
    
    match result {
        Ok(_) => FileDeleteResponse {
            path: req.path.clone(),
            success: true,
            error: None,
        },
        Err(e) => FileDeleteResponse {
            path: req.path.clone(),
            success: false,
            error: Some(format!("Failed to delete: {}", e)),
        },
    }
}
