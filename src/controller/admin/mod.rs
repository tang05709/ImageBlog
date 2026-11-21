//! 管理后台控制器：登录 / 概览 / 分类 / 图片 / 订单
//!
//! 登录会话持久化在数据库（sessions 表），见 service::session_service。

pub mod auth_handler;
pub mod cat_handler;
pub mod image_handler;
pub mod index_handler;
pub mod model;
pub mod order_handler;
pub mod router;

pub use router::router;

use crate::auth;
use crate::service::session_service;
use crate::state::AppState;

/// 未登录则返回跳转，已登录返回 None（会话查数据库校验）
pub(crate) async fn guard(
    state: &AppState,
    headers: &axum::http::HeaderMap,
) -> Option<axum::response::Redirect> {
    if let Some(sid) = auth::get_cookie(headers, "sid") {
        if session_service::exists(&state.db, &sid).await {
            return None;
        }
    }
    Some(axum::response::Redirect::to("/admin/login"))
}
