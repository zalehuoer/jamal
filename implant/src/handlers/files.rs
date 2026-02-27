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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_list_directory_normal() {
        let dir = tempfile::tempdir().unwrap();
        
        // 创建一些测试文件和子目录
        std::fs::File::create(dir.path().join("file1.txt")).unwrap();
        std::fs::File::create(dir.path().join("file2.dat")).unwrap();
        std::fs::create_dir(dir.path().join("subdir")).unwrap();
        
        let req = GetDirectoryListing { path: dir.path().to_string_lossy().to_string() };
        let result = get_directory_listing(&req);
        
        assert!(result.error.is_none());
        assert_eq!(result.entries.len(), 3);
        
        // 目录应排在前面
        assert!(result.entries[0].is_dir);
        assert_eq!(result.entries[0].name, "subdir");
    }

    #[test]
    fn test_list_directory_not_exists() {
        let req = GetDirectoryListing { path: "Z:\\nonexistent_dir_xyz_123".to_string() };
        let result = get_directory_listing(&req);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_list_directory_file_path() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::File::create(&file_path).unwrap();
        
        let req = GetDirectoryListing { path: file_path.to_string_lossy().to_string() };
        let result = get_directory_listing(&req);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("not a directory"));
    }

    #[test]
    fn test_file_upload_download_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test_upload.bin");
        let content = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0xFF];
        
        // 上传
        let upload_req = FileUpload {
            path: file_path.to_string_lossy().to_string(),
            data: content.clone(),
            is_complete: true,
        };
        let upload_result = upload_file(&upload_req);
        assert!(upload_result.success);
        assert!(upload_result.error.is_none());
        
        // 下载
        let download_req = FileDownload { path: file_path.to_string_lossy().to_string() };
        let download_result = download_file(&download_req);
        assert!(download_result.error.is_none());
        assert_eq!(download_result.data, content);
        assert!(download_result.is_complete);
    }

    #[test]
    fn test_download_nonexistent_file() {
        let result = download_file(&FileDownload { path: "Z:\\no_such_file_xyz.bin".to_string() });
        assert!(result.error.is_some());
        assert!(result.data.is_empty());
    }

    #[test]
    fn test_download_directory() {
        let dir = tempfile::tempdir().unwrap();
        let result = download_file(&FileDownload { path: dir.path().to_string_lossy().to_string() });
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("directory"));
    }

    #[test]
    fn test_delete_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("to_delete.txt");
        let mut f = std::fs::File::create(&file_path).unwrap();
        f.write_all(b"delete me").unwrap();
        drop(f);
        
        let req = FileDelete { path: file_path.to_string_lossy().to_string() };
        let result = delete_file(&req);
        assert!(result.success);
        assert!(!file_path.exists());
    }

    #[test]
    fn test_delete_nonexistent_file() {
        let result = delete_file(&FileDelete { path: "Z:\\no_such_file_xyz.bin".to_string() });
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_delete_directory() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::File::create(sub.join("inner.txt")).unwrap();
        
        let req = FileDelete { path: sub.to_string_lossy().to_string() };
        let result = delete_file(&req);
        assert!(result.success);
        assert!(!sub.exists());
    }
}
