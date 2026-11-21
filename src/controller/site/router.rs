
use crate::state::SharedState;

use super::index_handler::*;

pub fn router() -> axum::Router<SharedState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/", get(index))
        .route("/images", get(list))
        .route("/images/:id", get(detail))
        .route("/orders", post(create_order))
        .route("/pay/:order_no", get(pay_page))
        .route("/pay/mock/:order_no", post(mock_pay))
        .route("/api/order/:order_no/status", get(order_status))
        .route("/notify/wechatpay", post(wechat_notify))
}
