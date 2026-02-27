//! Shell 命令处理器 (同步版本)

use shared::messages::{ShellExecute, ShellExecuteResponse};
use std::process::Command;
use std::time::Duration;

/// 命令执行超时时间（秒）
const COMMAND_TIMEOUT_SECS: u64 = 30;

/// 将字节转换为字符串，Windows 使用 GBK 解码，其他平台使用 UTF-8
fn decode_output(bytes: &[u8]) -> String {
    #[cfg(windows)]
    {
        // Windows cmd 输出使用 GBK (代码页 936)
        use encoding_rs::GBK;
        let (decoded, _, _) = GBK.decode(bytes);
        decoded.into_owned()
    }
    
    #[cfg(not(windows))]
    {
        String::from_utf8_lossy(bytes).to_string()
    }
}

/// 检查命令是否是后台运行命令
fn is_background_command(command: &str) -> bool {
    let cmd_trimmed = command.trim();
    let cmd_lower = cmd_trimmed.to_lowercase();
    
    // Windows: start command
    if cmd_lower.starts_with("start ") || cmd_lower.starts_with("start/") {
        return true;
    }
    
    // Linux: nohup or ending with &
    if cmd_lower.starts_with("nohup ") || cmd_trimmed.ends_with(" &") || cmd_trimmed.ends_with("&") {
        return true;
    }
    
    false
}

/// 执行 Shell 命令（同步，带超时）
pub fn execute_shell_sync(cmd: &ShellExecute) -> ShellExecuteResponse {
    // 对于 start 命令，使用 spawn 不等待完成
    if is_background_command(&cmd.command) {
        return execute_background_command(&cmd.command);
    }
    
    // 同步执行带超时
    execute_command_with_timeout(&cmd.command, Duration::from_secs(COMMAND_TIMEOUT_SECS))
}

/// 执行后台命令（不等待完成）
fn execute_background_command(command: &str) -> ShellExecuteResponse {
    #[cfg(windows)]
    let result = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const DETACHED_PROCESS: u32 = 0x00000008;
        
        Command::new("cmd")
            .args(["/C", command])
            .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
            .spawn()
    };
    
    #[cfg(not(windows))]
    let result = {
        Command::new("sh")
            .args(["-c", &format!("{} &", command)])
            .spawn()
    };
    
    match result {
        Ok(_) => ShellExecuteResponse {
            output: format!("Background command started: {}", command),
            is_error: false,
        },
        Err(e) => ShellExecuteResponse {
            output: format!("Error starting background command: {}", e),
            is_error: true,
        },
    }
}

/// 执行命令并带超时（同步版本使用线程）
fn execute_command_with_timeout(command: &str, timeout: Duration) -> ShellExecuteResponse {
    use std::sync::mpsc;
    use std::thread;
    
    let command = command.to_string();
    let (tx, rx) = mpsc::channel();
    
    // 在新线程中执行命令
    thread::spawn(move || {
        let result = execute_command_inner(&command);
        let _ = tx.send(result);
    });
    
    // 等待结果或超时
    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(_) => ShellExecuteResponse {
            output: format!("Command timed out after {}s. For long-running programs, use: start <program>", COMMAND_TIMEOUT_SECS),
            is_error: true,
        },
    }
}

/// 实际执行命令
fn execute_command_inner(command: &str) -> ShellExecuteResponse {
    #[cfg(windows)]
    let result = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        
        Command::new("cmd")
            .args(["/C", command])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
    };
    
    #[cfg(not(windows))]
    let result = {
        Command::new("sh")
            .args(["-c", command])
            .output()
    };
    
    match result {
        Ok(output) => {
            let stdout = decode_output(&output.stdout);
            let stderr = decode_output(&output.stderr);
            
            if output.status.success() {
                ShellExecuteResponse {
                    output: stdout,
                    is_error: false,
                }
            } else {
                ShellExecuteResponse {
                    output: if stderr.is_empty() { stdout } else { stderr },
                    is_error: true,
                }
            }
        }
        Err(e) => ShellExecuteResponse {
            output: format!("Error: {}", e),
            is_error: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_background_command_windows() {
        assert!(is_background_command("start notepad"));
        assert!(is_background_command("START calc.exe"));
        assert!(is_background_command("start/min cmd"));
    }

    #[test]
    fn test_is_background_command_linux() {
        assert!(is_background_command("nohup ./server &"));
        assert!(is_background_command("sleep 10 &"));
        assert!(is_background_command("./run.sh&"));
    }

    #[test]
    fn test_is_not_background_command() {
        assert!(!is_background_command("echo hello"));
        assert!(!is_background_command("dir"));
        assert!(!is_background_command("whoami"));
        assert!(!is_background_command("starting up"));  // "start" 不在开头
    }

    #[test]
    fn test_execute_echo() {
        let cmd = ShellExecute { command: "echo hello_test_123".to_string() };
        let result = execute_shell_sync(&cmd);
        assert!(!result.is_error);
        assert!(result.output.contains("hello_test_123"));
    }

    #[test]
    fn test_execute_invalid_command() {
        let cmd = ShellExecute { command: "this_command_surely_does_not_exist_xyz_123".to_string() };
        let result = execute_shell_sync(&cmd);
        // 命令不存在应该报错
        assert!(result.is_error || result.output.contains("not") || result.output.contains("Error"));
    }
}
