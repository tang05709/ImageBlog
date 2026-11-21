//! 仪表盘控制器

use askama_axum::Template;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use crate::service::order_service;
use crate::state::SharedState;

use super::guard;
use crate::controller::yuan;

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
pub struct DashboardTpl {
    pub image_count: i64,
    pub published_count: i64,
    pub category_count: i64,
    pub order_count: i64,
    pub paid_count: i64,
    pub revenue_text: String,
}

// ---------- 仪表盘 ----------

pub async fn dashboard(State(state): State<SharedState>, headers: HeaderMap) -> Response {
    if let Some(r) = guard(&state, &headers).await {
        return r.into_response();
    }
    let s = order_service::dashboard_stats(&state.db).await;
    DashboardTpl {
        image_count: s.image_count,
        published_count: s.published_count,
        category_count: s.category_count,
        order_count: s.order_count,
        paid_count: s.paid_count,
        revenue_text: yuan(s.revenue),
    }
    .into_response()
}
