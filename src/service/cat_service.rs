//! 分类服务：分类的查询与增删改

use sqlx::PgPool;

/// 简单列表 (id, name)，按 sort 排序 —— 用户端分类导航
pub async fn list_simple(pool: &PgPool) -> Vec<(i64, String)> {
    sqlx::query_as("SELECT id, name FROM categories ORDER BY sort, id")
        .fetch_all(pool)
        .await
        .unwrap_or_default()
}

/// 后台列表 (id, name, sort, 图片数量)
pub async fn list_with_count(pool: &PgPool) -> Vec<(i64, String, i64, i64)> {
    sqlx::query_as(
        "SELECT c.id, c.name, c.sort, \
                (SELECT COUNT(*) FROM images i WHERE i.category_id = c.id) \
         FROM categories c ORDER BY c.sort, c.id",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

/// 新建分类
pub async fn create(pool: &PgPool, name: &str, sort: i64) {
    sqlx::query("INSERT INTO categories (name, sort) VALUES ($1, $2)")
        .bind(name)
        .bind(sort)
        .execute(pool)
        .await
        .ok();
}

/// 更新分类
pub async fn update(pool: &PgPool, id: i64, name: &str, sort: i64) {
    sqlx::query("UPDATE categories SET name = $1, sort = $2 WHERE id = $3")
        .bind(name)
        .bind(sort)
        .bind(id)
        .execute(pool)
        .await
        .ok();
}

/// 删除分类，其下图片改为未分类
pub async fn delete(pool: &PgPool, id: i64) {
    sqlx::query("UPDATE images SET category_id = NULL WHERE category_id = $1")
        .bind(id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM categories WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .ok();
}
