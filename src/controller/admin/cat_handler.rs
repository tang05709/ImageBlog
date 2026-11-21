//! 分类管理控制器

use askama_axum::Template;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Redirect, Response};
use crate::service::cat_service;
use crate::state::SharedState;

use super::guard;
use super::model::AdminCat;

#[derive(Template)]
#[template(path = "admin/categories.html")]
pub struct CategoriesTpl {
    pub categories: Vec<AdminCat>,
}

// ---------- 分类管理 ----------

pub async fn categories_page(State(state): State<SharedState>, headers: HeaderMap) -> Response {
    if let Some(r) = guard(&state, &headers).await {
        return r.into_response();
    }
    let rows = cat_service::list_with_count(&state.db).await;
    let categories = rows
        .into_iter()
        .map(|(id, name, sort, image_count)| AdminCat {
            id,
            name,
            sort,
            image_count,
        })
        .collect();
    CategoriesTpl { categories }.into_response()
}

#[derive(serde::Deserialize)]
pub struct CategoryForm {
    pub name: String,
    #[serde(default)]
    pub sort: i64,
}

pub async fn category_create(
    State(state): State<SharedState>,
    headers: HeaderMap,
    axum::Form(f): axum::Form<CategoryForm>,
) -> Response {
    if let Some(r) = guard(&state, &headers).await {
        return r.into_response();
    }
    if !f.name.trim().is_empty() {
        cat_service::create(&state.db, f.name.trim(), f.sort).await;
    }
    Redirect::to("/admin/categories").into_response()
}

pub async fn category_update(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    axum::Form(f): axum::Form<CategoryForm>,
) -> Response {
    if let Some(r) = guard(&state, &headers).await {
        return r.into_response();
    }
    cat_service::update(&state.db, id, f.name.trim(), f.sort).await;
    Redirect::to("/admin/categories").into_response()
}

pub async fn category_delete(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    if let Some(r) = guard(&state, &headers).await {
        return r.into_response();
    }
    cat_service::delete(&state.db, id).await;
    Redirect::to("/admin/categories").into_response()
}
