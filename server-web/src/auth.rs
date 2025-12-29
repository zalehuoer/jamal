//! HTTP Basic Authentication 中间件

use axum::{
    body::Body,
    http::{Request, Response, StatusCode, header},
    middleware::Next,
};
use base64::Engine;

// 默认用户名密码（可通过环境变量覆盖）
const DEFAULT_USERNAME: &str = "admin";
const DEFAULT_PASSWORD: &str = "jamal123";

/// 获取认证凭据
fn get_credentials() -> (String, String) {
    let username = std::env::var("JAMAL_USERNAME").unwrap_or_else(|_| DEFAULT_USERNAME.to_string());
    let password = std::env::var("JAMAL_PASSWORD").unwrap_or_else(|_| DEFAULT_PASSWORD.to_string());
    (username, password)
}

/// Basic Auth 中间件
pub async fn basic_auth(
    request: Request<Body>,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    let (expected_user, expected_pass) = get_credentials();
    
    // 检查 Authorization header
    if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Basic ") {
                let encoded = &auth_str[6..];
                if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(encoded) {
                    if let Ok(credentials) = String::from_utf8(decoded) {
                        if let Some((user, pass)) = credentials.split_once(':') {
                            if user == expected_user && pass == expected_pass {
                                // 验证通过
                                return Ok(next.run(request).await);
                            }
                        }
                    }
                }
            }
        }
    }
    
    // 验证失败，返回 401
    let response = Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(header::WWW_AUTHENTICATE, "Basic realm=\"JamalC2 Control Panel\"")
        .body(Body::from("Unauthorized"))
        .unwrap();
    
    Ok(response)
}
