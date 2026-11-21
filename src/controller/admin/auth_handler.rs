

use askama_axum::Template;
use axum::extract::State;
use axum::response::{IntoResponse, Redirect, Response};
use crate::auth;
use crate::service::session_service;
use crate::state::SharedState;

#[derive(Template)]
#[template(path = "admin/login.html")]
pub struct AdminLoginTpl {
    pub error: String,
}


// ---------- 登录 / 登出 ----------

#[derive(serde::Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

pub async fn login_page() -> AdminLoginTpl {
    AdminLoginTpl {
        error: String::new(),
    }
}

pub async fn login_submit(
    State(state): State<SharedState>,
    axum::Form(f): axum::Form<LoginForm>,
) -> Response {
    if f.username == state.config.admin.username && f.password == state.config.admin.password {
        let token = uuid::Uuid::new_v4().to_string();
        // 会话写入数据库持久化，重启/关浏览器后仍然有效
        session_service::create(&state.db, &token).await;
        let mut resp = Redirect::to("/admin").into_response();
        resp.headers_mut().append(
            axum::http::header::SET_COOKIE,
            axum::http::HeaderValue::from_str(&auth::cookie_header(
                "sid",
                &token,
                session_service::TTL_SECS,
            ))
            .unwrap(),
        );
        resp
    } else {
        AdminLoginTpl {
            error: "用户名或密码错误".into(),
        }
        .into_response()
    }
}

pub async fn logout(State(state): State<SharedState>, headers: axum::http::HeaderMap) -> Response {
    if let Some(sid) = auth::get_cookie(&headers, "sid") {
        session_service::delete(&state.db, &sid).await;
    }
    // 同时清掉浏览器 cookie
    let mut resp = Redirect::to("/admin/login").into_response();
    resp.headers_mut().append(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&auth::cookie_header("sid", "", 0)).unwrap(),
    );
    resp
}