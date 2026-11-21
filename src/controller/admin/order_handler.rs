//! 订单列表控制器

use askama_axum::Template;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use crate::service::order_service;
use crate::state::SharedState;

use super::guard;
use super::model::AdminOrder;
use crate::controller::yuan;

#[derive(Template)]
#[template(path = "admin/orders.html")]
pub struct OrdersTpl {
    pub orders: Vec<AdminOrder>,
}

pub async fn orders_page(State(state): State<SharedState>, headers: HeaderMap) -> Response {
    if let Some(r) = guard(&state, &headers).await {
        return r.into_response();
    }
    let rows = order_service::list_recent(&state.db, 200).await;
    let orders = rows
        .into_iter()
        .map(|(_id, order_no, image_title, amount, status, pay_type, created_at, paid_at)| {
            AdminOrder {
                order_no,
                image_title,
                amount_text: yuan(amount),
                status,
                pay_type,
                created_at,
                paid_at,
            }
        })
        .collect();
    OrdersTpl { orders }.into_response()
}
