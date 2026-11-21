//! 图片服务：图片（作品）的查询与增删改

use sqlx::PgPool;

use super::now_str;

// ---------- 行类型别名（字段顺序与 SQL 列一一对应）----------

/// 卡片行: id, title, file_path, 分类名, price_cents
pub type CardRow = (i64, String, String, String, i64);
/// 详情行: id, title, description, 分类名, model_name, width, height, file_path, prompt, price_cents
pub type DetailRow = (i64, String, String, String, String, i64, i64, String, String, i64);
/// 后台列表行: id, title, 分类名, model_name, price_cents, is_published, file_path, created_at
pub type AdminRow = (i64, String, String, String, i64, i64, String, String);
/// 编辑表单行: title, description, category_id(0=未分类), model_name, width, height, file_path, prompt, price_cents, is_published
pub type EditRow = (String, String, i64, String, i64, i64, String, String, i64, i64);

const CARD_SQL: &str = "SELECT i.id, i.title, i.file_path, COALESCE(c.name, '未分类'), i.price_cents \
        FROM images i LEFT JOIN categories c ON c.id = i.category_id \
        WHERE i.is_published = 1";

// ---------- 用户端查询 ----------

/// 首页最新作品
pub async fn latest_cards(pool: &PgPool, limit: i64) -> Vec<CardRow> {
    sqlx::query_as(&format!("{CARD_SQL} ORDER BY i.id DESC LIMIT $1"))
        .bind(limit)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
}

/// 已发布图片总数（category 为 0 表示全部）
pub async fn published_count(pool: &PgPool, category: i64) -> i64 {
    if category > 0 {
        sqlx::query_scalar("SELECT COUNT(*) FROM images WHERE is_published = 1 AND category_id = $1")
            .bind(category)
            .fetch_one(pool)
            .await
            .unwrap_or(0)
    } else {
        sqlx::query_scalar("SELECT COUNT(*) FROM images WHERE is_published = 1")
            .fetch_one(pool)
            .await
            .unwrap_or(0)
    }
}

/// 列表页分页卡片（category 为 0 表示全部）
pub async fn card_page(pool: &PgPool, category: i64, limit: i64, offset: i64) -> Vec<CardRow> {
    if category > 0 {
        sqlx::query_as(&format!(
            "{CARD_SQL} AND i.category_id = $1 ORDER BY i.id DESC LIMIT $2 OFFSET $3"
        ))
        .bind(category)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    } else {
        sqlx::query_as(&format!("{CARD_SQL} ORDER BY i.id DESC LIMIT $1 OFFSET $2"))
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .unwrap_or_default()
    }
}

/// 已发布图片详情（不存在或未发布返回 None）
pub async fn detail(pool: &PgPool, id: i64) -> Option<DetailRow> {
    sqlx::query_as(
        "SELECT i.id, i.title, i.description, COALESCE(c.name, '未分类'), i.model_name, \
                i.width, i.height, i.file_path, i.prompt, i.price_cents \
         FROM images i LEFT JOIN categories c ON c.id = i.category_id \
         WHERE i.id = $1 AND i.is_published = 1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None)
}

/// 图片价格与标题（下单用），不存在或未发布返回 None
pub async fn price_title(pool: &PgPool, id: i64) -> Option<(i64, String)> {
    sqlx::query_as("SELECT price_cents, title FROM images WHERE id = $1 AND is_published = 1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
}

/// 图片提示词（支付成功页展示）
pub async fn prompt_of(pool: &PgPool, image_id: i64) -> String {
    sqlx::query_scalar("SELECT prompt FROM images WHERE id = $1")
        .bind(image_id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
        .unwrap_or_default()
}

// ---------- 后台查询 ----------

/// 后台图片列表（全部，含未发布）
pub async fn list_all_admin(pool: &PgPool) -> Vec<AdminRow> {
    sqlx::query_as(
        "SELECT i.id, i.title, COALESCE(c.name, '未分类'), i.model_name, i.price_cents, \
                i.is_published, i.file_path, i.created_at \
         FROM images i LEFT JOIN categories c ON c.id = i.category_id ORDER BY i.id DESC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

/// 编辑表单数据
pub async fn get_for_edit(pool: &PgPool, id: i64) -> Option<EditRow> {
    sqlx::query_as(
        "SELECT title, description, COALESCE(category_id, 0), model_name, width, height, \
         file_path, prompt, price_cents, is_published FROM images WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None)
}

/// 图片文件路径（删除磁盘文件前查询）
pub async fn file_path(pool: &PgPool, id: i64) -> Option<String> {
    sqlx::query_scalar("SELECT file_path FROM images WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
}

// ---------- 后台写入 ----------

/// 新建图片
#[allow(clippy::too_many_arguments)]
pub async fn create_image(
    pool: &PgPool,
    title: &str,
    description: &str,
    category_id: Option<i64>,
    model_name: &str,
    width: i64,
    height: i64,
    file_path: &str,
    prompt: &str,
    price_cents: i64,
    is_published: bool,
) {
    sqlx::query(
        "INSERT INTO images (title, description, category_id, model_name, width, height, \
         file_path, prompt, price_cents, is_published, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(title)
    .bind(description)
    .bind(category_id)
    .bind(model_name)
    .bind(width)
    .bind(height)
    .bind(file_path)
    .bind(prompt)
    .bind(price_cents)
    .bind(is_published as i64)
    .bind(now_str())
    .execute(pool)
    .await
    .ok();
}

/// 更新图片（含新文件）
#[allow(clippy::too_many_arguments)]
pub async fn update_with_file(
    pool: &PgPool,
    id: i64,
    title: &str,
    description: &str,
    category_id: Option<i64>,
    model_name: &str,
    width: i64,
    height: i64,
    file_path: &str,
    prompt: &str,
    price_cents: i64,
    is_published: bool,
) {
    sqlx::query(
        "UPDATE images SET title=$1, description=$2, category_id=$3, model_name=$4, \
         width=$5, height=$6, file_path=$7, prompt=$8, price_cents=$9, is_published=$10 \
         WHERE id=$11",
    )
    .bind(title)
    .bind(description)
    .bind(category_id)
    .bind(model_name)
    .bind(width)
    .bind(height)
    .bind(file_path)
    .bind(prompt)
    .bind(price_cents)
    .bind(is_published as i64)
    .bind(id)
    .execute(pool)
    .await
    .ok();
}

/// 更新图片（不换文件）
#[allow(clippy::too_many_arguments)]
pub async fn update_without_file(
    pool: &PgPool,
    id: i64,
    title: &str,
    description: &str,
    category_id: Option<i64>,
    model_name: &str,
    prompt: &str,
    price_cents: i64,
    is_published: bool,
) {
    sqlx::query(
        "UPDATE images SET title=$1, description=$2, category_id=$3, model_name=$4, \
         prompt=$5, price_cents=$6, is_published=$7 WHERE id=$8",
    )
    .bind(title)
    .bind(description)
    .bind(category_id)
    .bind(model_name)
    .bind(prompt)
    .bind(price_cents)
    .bind(is_published as i64)
    .bind(id)
    .execute(pool)
    .await
    .ok();
}

/// 删除图片记录（磁盘文件由控制器负责删除）
pub async fn delete(pool: &PgPool, id: i64) {
    sqlx::query("DELETE FROM images WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .ok();
}

/// 上/下架切换
pub async fn toggle_publish(pool: &PgPool, id: i64) {
    sqlx::query("UPDATE images SET is_published = 1 - is_published WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .ok();
}

// ---------- 统计 ----------
