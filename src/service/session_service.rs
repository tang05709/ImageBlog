//! 会话服务：管理员登录会话的数据库持久化
//!
//! 会话存放在 PostgreSQL 的 sessions 表中，服务重启、浏览器关闭后依然有效，
//! 超过 TTL 的会话在新建登录时自动清理。

use sqlx::PgPool;

use super::now_str;

/// 会话有效期：7 天
pub const TTL_SECS: i64 = 7 * 24 * 3600;

/// 新建会话，并顺带清理过期会话
pub async fn create(pool: &PgPool, token: &str) {
    sqlx::query(
        "INSERT INTO sessions (token, created_at) VALUES ($1, $2) \
         ON CONFLICT (token) DO NOTHING",
    )
    .bind(token)
    .bind(now_str())
    .execute(pool)
    .await
    .ok();
    purge_expired(pool, TTL_SECS).await;
}

/// 会话是否存在（即是否已登录）
pub async fn exists(pool: &PgPool, token: &str) -> bool {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sessions WHERE token = $1")
        .bind(token)
        .fetch_one(pool)
        .await
        .unwrap_or(0)
        > 0
}

/// 删除会话（登出）
pub async fn delete(pool: &PgPool, token: &str) {
    sqlx::query("DELETE FROM sessions WHERE token = $1")
        .bind(token)
        .execute(pool)
        .await
        .ok();
}

/// 清理超过 max_age_secs 的过期会话
pub async fn purge_expired(pool: &PgPool, max_age_secs: i64) {
    let cutoff = (chrono::Local::now() - chrono::Duration::seconds(max_age_secs))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    sqlx::query("DELETE FROM sessions WHERE created_at < $1")
        .bind(cutoff)
        .execute(pool)
        .await
        .ok();
}
