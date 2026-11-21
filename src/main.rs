mod auth;
mod config;
mod controller;
mod db;
mod img_compress;
mod pay;
mod service;
mod state;

use state::{AppState, SharedState};
use std::sync::Arc;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // 加载 .env 文件（存在才加载，不覆盖已有环境变量）
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = config::load();
    std::fs::create_dir_all("uploads").ok();
    std::fs::create_dir_all("static").ok();

    let db_url = config.database_url.clone();
    let conn = db::init(&db_url)
        .await
        .expect("数据库初始化失败，请检查 .env 中的 DATABASE_URL");
    let state: SharedState = Arc::new(AppState {
        db: conn,
        config,
    });

    let addr = format!("{}:{}", state.config.server.host, state.config.server.port);

    let app = controller::site::router()
        // 管理后台（/admin/*）
        .nest("/admin", controller::admin::router())
        // 放宽请求体限制：axum 默认 2MB，ComfyUI 生成的 PNG 通常有数 MB，
        // 与上传检查的 20MB 上限保持一致
        .layer(axum::extract::DefaultBodyLimit::max(
            20 * 1024 * 1024,
        ))
        // 静态资源
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/uploads", ServeDir::new("uploads"))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("无法监听 {addr}: {e}"));
    tracing::info!("服务已启动: http://{addr}  后台: http://{addr}/admin");
    axum::serve(listener, app).await.unwrap();
}
