//! 控制器层：HTTP 处理器，不直接写 SQL，查询统一走 crate::service
//!
//! - `admin`  管理后台（/admin/*）
//! - `site`   用户端页面

pub mod admin;
pub mod site;

/// 金额（分）→ 显示文本
pub(crate) fn yuan(cents: i64) -> String {
    format!("¥{:.2}", cents as f64 / 100.0)
}
