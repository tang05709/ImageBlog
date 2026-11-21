//! 用户端控制器：首页 / 列表 / 详情 / 下单 / 支付 / 微信回调
//!
//! 数据库操作全部委托 crate::service，本文件只做参数解析、模板渲染与响应组装。

use crate::auth;
use crate::pay::{decrypt_notify, NotifyBody, TradeNotify, WxPayClient};
use crate::service::{cat_service, image_service, order_service};
use crate::state::{AppState, SharedState};
use askama_axum::Template;
use axum::extract::{Form, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::controller::yuan;

// ---------- 模板数据 ----------

pub struct Card {
    pub id: i64,
    pub title: String,
    pub file_path: String,
    pub category_name: String,
    pub price_text: String,
}

pub struct CatItem {
    pub id: i64,
    pub name: String,
}

#[derive(Template)]
#[template(path = "site/index.html")]
pub struct IndexTpl {
    pub cards: Vec<Card>,
    pub categories: Vec<CatItem>,
}

#[derive(Template)]
#[template(path = "site/list.html")]
pub struct ListTpl {
    pub cards: Vec<Card>,
    pub categories: Vec<CatItem>,
    pub current_cat: i64,
    pub page: i64,
    pub has_prev: bool,
    pub has_next: bool,
    pub prev_url: String,
    pub next_url: String,
}

pub struct DetailImage {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub category_name: String,
    pub model_name: String,
    pub width: i64,
    pub height: i64,
    pub file_path: String,
    pub prompt: String,
    pub price_cents: i64,
    pub price_text: String,
}

#[derive(Template)]
#[template(path = "site/detail.html")]
pub struct DetailTpl {
    pub img: DetailImage,
    pub unlocked: bool,
}

#[derive(Template)]
#[template(path = "site/pay.html")]
pub struct PayTpl {
    pub order_no: String,
    pub title: String,
    pub image_id: i64,
    pub amount_text: String,
    pub qr_svg: String,
    pub mock: bool,
    pub paid: bool,
    pub prompt: String,
}

// ---------- 视图组装辅助 ----------

fn card_from(r: image_service::CardRow) -> Card {
    Card {
        id: r.0,
        title: r.1,
        file_path: r.2,
        category_name: r.3,
        price_text: yuan(r.4),
    }
}

async fn load_categories(pool: &sqlx::PgPool) -> Vec<CatItem> {
    cat_service::list_simple(pool)
        .await
        .into_iter()
        .map(|(id, name)| CatItem { id, name })
        .collect()
}

async fn purchased_ids(state: &Arc<AppState>, headers: &HeaderMap) -> Vec<i64> {
    auth::get_cookie(headers, "pids")
        .and_then(|v| auth::decode_purchased(&state.config.secret, &v))
        .unwrap_or_default()
}

// ---------- 页面处理 ----------

pub async fn index(State(state): State<SharedState>) -> IndexTpl {
    let cards = image_service::latest_cards(&state.db, 8)
        .await
        .into_iter()
        .map(card_from)
        .collect();
    IndexTpl {
        cards,
        categories: load_categories(&state.db).await,
    }
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub category: Option<i64>,
    pub page: Option<i64>,
}

pub async fn list(State(state): State<SharedState>, Query(q): Query<ListQuery>) -> ListTpl {
    let pool = &state.db;
    let per_page: i64 = 12;
    let page = q.page.unwrap_or(1).max(1);
    let cat = q.category.unwrap_or(0);

    let total = image_service::published_count(pool, cat).await;
    let cards = image_service::card_page(pool, cat, per_page, (page - 1) * per_page)
        .await
        .into_iter()
        .map(card_from)
        .collect();

    let base = if cat > 0 {
        format!("/images?category={cat}&page=")
    } else {
        "/images?page=".to_string()
    };
    ListTpl {
        cards,
        categories: load_categories(pool).await,
        current_cat: cat,
        page,
        has_prev: page > 1,
        has_next: total > page * per_page,
        prev_url: format!("{base}{}", page - 1),
        next_url: format!("{base}{}", page + 1),
    }
}

pub async fn detail(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Response {
    let row = image_service::detail(&state.db, id).await;

    let mut img = match row {
        Some(r) => DetailImage {
            id: r.0,
            title: r.1,
            description: r.2,
            category_name: r.3,
            model_name: r.4,
            width: r.5,
            height: r.6,
            file_path: r.7,
            prompt: r.8,
            price_cents: r.9,
            price_text: String::new(),
        },
        None => return Redirect::to("/").into_response(),
    };
    img.price_text = yuan(img.price_cents);

    let unlocked = img.price_cents == 0 || purchased_ids(&state, &headers).await.contains(&id);
    DetailTpl { img, unlocked }.into_response()
}

// ---------- 订单与支付 ----------

#[derive(Deserialize)]
pub struct NewOrderForm {
    pub image_id: i64,
}

pub async fn create_order(
    State(state): State<SharedState>,
    Form(f): Form<NewOrderForm>,
) -> Response {
    let Some((price, _title)) = image_service::price_title(&state.db, f.image_id).await else {
        return Redirect::to("/").into_response();
    };

    let order_no = format!(
        "IMG{}{:04}",
        chrono::Local::now().timestamp_millis(),
        rand::random::<u16>() % 10000
    );
    order_service::create(&state.db, &order_no, f.image_id, price).await;

    Redirect::to(&format!("/pay/{order_no}")).into_response()
}

pub fn qr_svg(data: &str) -> String {
    use qrcode::render::svg;
    use qrcode::{EcLevel, QrCode};
    match QrCode::with_error_correction_level(data.as_bytes(), EcLevel::M) {
        Ok(code) => code
            .render::<svg::Color>()
            .dark_color(svg::Color("#111111"))
            .light_color(svg::Color("#ffffff"))
            .min_dimensions(240, 240)
            .build(),
        Err(_) => String::new(),
    }
}

pub async fn pay_page(
    State(state): State<SharedState>,
    Path(order_no): Path<String>,
) -> Response {
    let (image_id, title, amount, status) = match order_service::find_with_image(&state.db, &order_no).await
    {
        Some(v) => v,
        None => return Redirect::to("/").into_response(),
    };

    let paid = status == "paid";
    let mut qr_svg_str = String::new();
    let mut mock = !state.config.wechat_pay.enabled;

    if !paid {
        if state.config.wechat_pay.enabled {
            match WxPayClient::from_config(&state.config.wechat_pay) {
                Ok(client) => {
                    mock = false;
                    let desc = format!("AI图片提示词 - {}", title);
                    match client.native_pay(&order_no, &desc, amount).await {
                        Ok(url) => qr_svg_str = qr_svg(&url),
                        Err(e) => {
                            tracing::error!("微信下单失败: {e}");
                            qr_svg_str =
                                "<p style='color:#c00'>微信下单失败，请稍后重试</p>".into();
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("微信支付客户端初始化失败: {e}");
                    mock = true;
                }
            }
        }
        if mock {
            qr_svg_str = qr_svg(&format!("mock-pay:{order_no}"));
        }
    }

    let prompt = if paid {
        image_service::prompt_of(&state.db, image_id).await
    } else {
        String::new()
    };

    PayTpl {
        order_no,
        title,
        image_id,
        amount_text: yuan(amount),
        qr_svg: qr_svg_str,
        mock,
        paid,
        prompt,
    }
    .into_response()
}

/// 轮询订单状态；支付成功时把图片 id 写入已购 cookie
pub async fn order_status(
    State(state): State<SharedState>,
    Path(order_no): Path<String>,
    headers: HeaderMap,
) -> Response {
    match order_service::status_image(&state.db, &order_no).await {
        Some((status, image_id)) => {
            let mut resp =
                axum::Json(json!({ "status": status, "image_id": image_id })).into_response();
            if status == "paid" {
                let existing = auth::get_cookie(&headers, "pids");
                let cookie = auth::make_purchased_cookie(
                    &state.config.secret,
                    existing.as_deref(),
                    image_id,
                );
                resp.headers_mut().append(
                    axum::http::header::SET_COOKIE,
                    axum::http::HeaderValue::from_str(&auth::cookie_header(
                        "pids",
                        &cookie,
                        3600 * 24 * 30,
                    ))
                    .unwrap(),
                );
            }
            resp
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// 模拟支付（开发调试用，wechat_pay.enabled=false 时显示按钮）
pub async fn mock_pay(
    State(state): State<SharedState>,
    Path(order_no): Path<String>,
) -> Response {
    if state.config.wechat_pay.enabled {
        return StatusCode::FORBIDDEN.into_response();
    }
    order_service::mark_paid_mock(&state.db, &order_no).await;

    Redirect::to(&format!("/pay/{order_no}")).into_response()
}

/// 微信支付回调：验签略（依赖 APIv3 密钥解密校验），解密成功即认为可信
pub async fn wechat_notify(State(state): State<SharedState>, body: String) -> Response {
    let parsed: Result<NotifyBody, _> = serde_json::from_str(&body);
    let notify_body = match parsed {
        Ok(b) => b,
        Err(_) => {
            return axum::Json(json!({"code": "FAIL", "message": "报文格式错误"})).into_response()
        }
    };
    let trade: Result<TradeNotify, String> =
        decrypt_notify(&state.config.wechat_pay.apiv3_key, &notify_body.resource);
    match trade {
        Ok(t) => {
            if t.trade_state == "SUCCESS" && !t.out_trade_no.is_empty() {
                order_service::mark_paid_wechat(&state.db, &t.out_trade_no, &t.transaction_id)
                    .await;
                tracing::info!(
                    "微信支付成功: order_no={} txn={}",
                    t.out_trade_no,
                    t.transaction_id
                );
            }
            axum::Json(json!({"code": "SUCCESS", "message": "成功"})).into_response()
        }
        Err(e) => {
            tracing::error!("微信回调解密失败: {e}");
            axum::Json(json!({"code": "FAIL", "message": e})).into_response()
        }
    }
}
