//! 订单服务：订单的创建、状态流转、列表与统计

use sqlx::PgPool;

use super::now_str;

// ---------- 行类型别名 ----------

/// 后台订单行: order_no, 图片标题, amount_cents, status, pay_type, created_at, paid_at
pub type AdminRow = (i64, String, String, i64, String, String, String, String);
/// 支付页行: image_id, 图片标题, amount_cents, status
pub type PayRow = (i64, String, i64, String);

// ---------- 用户端 ----------

/// 创建订单
pub async fn create(pool: &PgPool, order_no: &str, image_id: i64, amount_cents: i64) {
    sqlx::query(
        "INSERT INTO orders (order_no, image_id, amount_cents, status, created_at) \
         VALUES ($1, $2, $3, 'pending', $4)",
    )
    .bind(order_no)
    .bind(image_id)
    .bind(amount_cents)
    .bind(now_str())
    .execute(pool)
    .await
    .expect("insert order");
}

/// 按订单号查支付页数据（含图片标题），不存在返回 None
pub async fn find_with_image(pool: &PgPool, order_no: &str) -> Option<PayRow> {
    sqlx::query_as(
        "SELECT o.image_id, i.title, o.amount_cents, o.status FROM orders o \
         JOIN images i ON i.id = o.image_id WHERE o.order_no = $1",
    )
    .bind(order_no)
    .fetch_optional(pool)
    .await
    .unwrap_or(None)
}

/// 轮询用：订单状态与图片 id
pub async fn status_image(pool: &PgPool, order_no: &str) -> Option<(String, i64)> {
    sqlx::query_as("SELECT status, image_id FROM orders WHERE order_no = $1")
        .bind(order_no)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
}

/// 模拟支付成功（仅开发模式）
pub async fn mark_paid_mock(pool: &PgPool, order_no: &str) {
    sqlx::query(
        "UPDATE orders SET status = 'paid', pay_type = 'mock', paid_at = $1 \
         WHERE order_no = $2 AND status = 'pending'",
    )
    .bind(now_str())
    .bind(order_no)
    .execute(pool)
    .await
    .ok();
}

/// 微信回调标记支付成功
pub async fn mark_paid_wechat(pool: &PgPool, order_no: &str, transaction_id: &str) {
    sqlx::query(
        "UPDATE orders SET status = 'paid', pay_type = 'wechat', \
         transaction_id = $1, paid_at = $2 \
         WHERE order_no = $3 AND status = 'pending'",
    )
    .bind(transaction_id)
    .bind(now_str())
    .bind(order_no)
    .execute(pool)
    .await
    .ok();
}

// ---------- 后台 ----------

/// 最近订单列表
pub async fn list_recent(pool: &PgPool, limit: i64) -> Vec<AdminRow> {
    sqlx::query_as(
        "SELECT o.id, o.order_no, i.title, o.amount_cents, o.status, o.pay_type, \
                o.created_at, COALESCE(o.paid_at, '') \
         FROM orders o JOIN images i ON i.id = o.image_id ORDER BY o.id DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

// ---------- 统计 ----------

/// 仪表盘统计
pub struct DashboardStats {
    pub image_count: i64,
    pub published_count: i64,
    pub category_count: i64,
    pub order_count: i64,
    pub paid_count: i64,
    pub revenue: i64,
}

async fn scalar(pool: &PgPool, sql: &str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(pool).await.unwrap_or(0)
}

pub async fn dashboard_stats(pool: &PgPool) -> DashboardStats {
    DashboardStats {
        image_count: scalar(pool, "SELECT COUNT(*) FROM images").await,
        published_count: scalar(pool, "SELECT COUNT(*) FROM images WHERE is_published = 1").await,
        category_count: scalar(pool, "SELECT COUNT(*) FROM categories").await,
        order_count: scalar(pool, "SELECT COUNT(*) FROM orders").await,
        paid_count: scalar(pool, "SELECT COUNT(*) FROM orders WHERE status = 'paid'").await,
        revenue: scalar(
            pool,
            "SELECT COALESCE(SUM(amount_cents), 0) FROM orders WHERE status = 'paid'",
        )
        .await,
    }
}
