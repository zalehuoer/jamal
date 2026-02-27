//! JamalC2 Web Server - Main Entry Point
//! 跨平台 C2 控制面板 Web 服务

mod state;
mod listener;
mod api;
mod auth;
mod db;

use std::sync::Arc;
use std::net::SocketAddr;
use axum::{Router, middleware};
use tower_http::cors::{CorsLayer, Any};
use tower_http::services::ServeDir;

use state::{AppState, SharedState};

#[tokio::main]
async fn main() {
    println!("========================================");
    println!("   JamalC2 Web Server v0.1.0");
    println!("========================================");
    
    // 创建全局状态
    let state: SharedState = Arc::new(AppState::new());
    
    // 创建 Web UI 路由 (仅管理面板，C2 监听器通过 API 动态创建)
    let app = Router::new()
        // API 路由 (前端控制面板使用)
        .nest("/api", api::create_api_routes())
        // 静态文件服务（前端）
        .fallback_service(ServeDir::new("static"))
        // HTTP Basic Auth 保护
        .layer(middleware::from_fn(auth::basic_auth))
        .layer(CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any))
        .with_state(state.clone());
    
    // Web 控制面板端口（支持环境变量覆盖）
    let web_port: u16 = std::env::var("JAMAL_WEB_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(443);
    let web_addr = SocketAddr::from(([0, 0, 0, 0], web_port));
    
    println!("[*] Web UI listening on http://{}", web_addr);
    println!("[*] Auth: admin/jamal123 (or set JAMAL_USERNAME/JAMAL_PASSWORD env vars)");
    println!("[*] Create a listener to start C2 service on a custom port");
    println!("----------------------------------------");
    
    // 启动服务器
    let listener = tokio::net::TcpListener::bind(web_addr).await.unwrap();
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

