//! 图片管理控制器（上传 / 编辑 / 删除 / 上下架）

use askama_axum::Template;
use axum::extract::{Multipart, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use crate::service::{cat_service, image_service};
use crate::state::SharedState;

use super::guard;
use super::model::{AdminImage, ImageForm, SelectCat};
use crate::controller::yuan;

#[derive(Template)]
#[template(path = "admin/images.html")]
pub struct ImagesTpl {
    pub images: Vec<AdminImage>,
}

#[derive(Template)]
#[template(path = "admin/image_form.html")]
pub struct ImageFormTpl {
    pub form: ImageForm,
    pub categories: Vec<SelectCat>,
}

// ---------- 图片管理 ----------

pub async fn images_page(State(state): State<SharedState>, headers: HeaderMap) -> Response {
    if let Some(r) = guard(&state, &headers).await {
        return r.into_response();
    }
    let rows = image_service::list_all_admin(&state.db).await;
    let images = rows
        .into_iter()
        .map(|(id, title, category_name, model_name, price_cents, published, thumb, created_at)| {
            AdminImage {
                id,
                title,
                category_name,
                model_name,
                price_text: yuan(price_cents),
                is_published: published != 0,
                thumb,
                created_at,
            }
        })
        .collect();
    ImagesTpl { images }.into_response()
}

pub async fn image_new_page(State(state): State<SharedState>, headers: HeaderMap) -> Response {
    if let Some(r) = guard(&state, &headers).await {
        return r.into_response();
    }
    let cats: Vec<SelectCat> = cat_service::list_simple(&state.db)
        .await
        .into_iter()
        .map(|(id, name)| SelectCat { id, name })
        .collect();
    ImageFormTpl {
        form: ImageForm {
            id: 0,
            title: String::new(),
            description: String::new(),
            category_id: 0,
            model_name: String::new(),
            width: 0,
            height: 0,
            file_path: String::new(),
            prompt: String::new(),
            price_yuan: "1.00".into(),
            is_published: true,
            is_new: true,
        },
        categories: cats,
    }
    .into_response()
}

/// 处理上传/编辑共用的 multipart，返回 (各字段, 已保存的文件路径)
struct UploadedFields {
    title: String,
    description: String,
    category_id: i64,
    model_name: String,
    prompt: String,
    price_cents: i64,
    is_published: bool,
    saved_path: Option<String>,
    width: i64,
    height: i64,
}

async fn read_multipart(mut mp: Multipart) -> Result<UploadedFields, String> {
    let mut title = String::new();
    let mut description = String::new();
    let mut category_id = 0i64;
    let mut model_name = String::new();
    let mut prompt = String::new();
    let mut price_cents = 0i64;
    let mut is_published = false;
    let mut saved_path: Option<String> = None;
    let mut width = 0i64;
    let mut height = 0i64;

    while let Some(field) = mp
        .next_field()
        .await
        .map_err(|e| format!("读取表单失败: {e}"))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "title" => title = field.text().await.unwrap_or_default(),
            "description" => description = field.text().await.unwrap_or_default(),
            "category_id" => {
                category_id = field
                    .text()
                    .await
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0)
            }
            "model_name" => model_name = field.text().await.unwrap_or_default(),
            "prompt" => prompt = field.text().await.unwrap_or_default(),
            "price" => {
                let s = field.text().await.unwrap_or_default();
                price_cents = s
                    .trim()
                    .parse::<f64>()
                    .map(|v| (v * 100.0).round() as i64)
                    .unwrap_or(0);
            }
            "is_published" => is_published = field.text().await.unwrap_or_default() == "1",
            "file" => {
                let fname = field.file_name().map(|s| s.to_string());
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| format!("读取上传文件失败: {e}"))?;
                if !bytes.is_empty() {
                    if bytes.len() > 20 * 1024 * 1024 {
                        return Err("图片不能超过 20MB".into());
                    }
                    let src_ext = fname
                        .as_deref()
                        .and_then(|n| {
                            std::path::Path::new(n)
                                .extension()
                                .map(|e| e.to_string_lossy().to_lowercase())
                        })
                        .filter(|e| ["jpg", "jpeg", "png", "webp", "gif"].contains(&e.as_str()))
                        .unwrap_or_else(|| "png".into());
                    // 压缩后再落盘，显著减少磁盘占用
                    let out = crate::img_compress::compress(&bytes, &src_ext);
                    tracing::info!(
                        "图片压缩: {} KB -> {} KB ({}×{})",
                        bytes.len() / 1024,
                        out.bytes.len() / 1024,
                        out.width,
                        out.height
                    );
                    let fname_disk = format!("{}.{}", uuid::Uuid::new_v4(), out.ext);
                    std::fs::create_dir_all("uploads").ok();
                    let path = format!("uploads/{fname_disk}");
                    std::fs::write(&path, &out.bytes).map_err(|e| format!("保存文件失败: {e}"))?;
                    width = out.width;
                    height = out.height;
                    saved_path = Some(path);
                }
            }
            _ => {}
        }
    }

    Ok(UploadedFields {
        title,
        description,
        category_id,
        model_name,
        prompt,
        price_cents,
        is_published,
        saved_path,
        width,
        height,
    })
}

pub async fn image_create(
    State(state): State<SharedState>,
    headers: HeaderMap,
    mp: Multipart,
) -> Response {
    if let Some(r) = guard(&state, &headers).await {
        return r.into_response();
    }
    let f = match read_multipart(mp).await {
        Ok(f) => f,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };
    if f.title.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "标题不能为空").into_response();
    }
    let Some(path) = f.saved_path else {
        return (StatusCode::BAD_REQUEST, "请上传图片文件").into_response();
    };
    let cat_id = if f.category_id > 0 { Some(f.category_id) } else { None };
    image_service::create_image(
        &state.db,
        f.title.trim(),
        &f.description,
        cat_id,
        &f.model_name,
        f.width,
        f.height,
        &path,
        &f.prompt,
        f.price_cents,
        f.is_published,
    )
    .await;

    Redirect::to("/admin/images").into_response()
}

pub async fn image_edit_page(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    if let Some(r) = guard(&state, &headers).await {
        return r.into_response();
    }
    match image_service::get_for_edit(&state.db, id).await {
        Some((title, description, category_id, model_name, width, height, file_path, prompt, price_cents, published)) => {
            let cats: Vec<SelectCat> = cat_service::list_simple(&state.db)
                .await
                .into_iter()
                .map(|(cid, name)| SelectCat { id: cid, name })
                .collect();
            let form = ImageForm {
                id,
                title,
                description,
                category_id,
                model_name,
                width,
                height,
                file_path,
                prompt,
                price_yuan: format!("{:.2}", price_cents as f64 / 100.0),
                is_published: published != 0,
                is_new: false,
            };
            ImageFormTpl { form, categories: cats }.into_response()
        }
        None => Redirect::to("/admin/images").into_response(),
    }
}

pub async fn image_update(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    mp: Multipart,
) -> Response {
    if let Some(r) = guard(&state, &headers).await {
        return r.into_response();
    }
    let f = match read_multipart(mp).await {
        Ok(f) => f,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };
    let cat_id = if f.category_id > 0 { Some(f.category_id) } else { None };
    let pool = &state.db;
    if let Some(path) = f.saved_path {
        image_service::update_with_file(
            pool,
            id,
            f.title.trim(),
            &f.description,
            cat_id,
            &f.model_name,
            f.width,
            f.height,
            &path,
            &f.prompt,
            f.price_cents,
            f.is_published,
        )
        .await;
    } else {
        image_service::update_without_file(
            pool,
            id,
            f.title.trim(),
            &f.description,
            cat_id,
            &f.model_name,
            &f.prompt,
            f.price_cents,
            f.is_published,
        )
        .await;
    }
    Redirect::to("/admin/images").into_response()
}

pub async fn image_delete(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    if let Some(r) = guard(&state, &headers).await {
        return r.into_response();
    }
    if let Some(p) = image_service::file_path(&state.db, id).await {
        std::fs::remove_file(&p).ok();
    }
    image_service::delete(&state.db, id).await;
    Redirect::to("/admin/images").into_response()
}

pub async fn image_toggle(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    if let Some(r) = guard(&state, &headers).await {
        return r.into_response();
    }
    image_service::toggle_publish(&state.db, id).await;
    Redirect::to("/admin/images").into_response()
}
