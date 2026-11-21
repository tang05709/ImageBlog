use sqlx::PgPool;

pub async fn init(url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPool::connect(url).await?;

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS categories (
            id   BIGSERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            sort BIGINT NOT NULL DEFAULT 0
        )"#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS images (
            id           BIGSERIAL PRIMARY KEY,
            title        TEXT NOT NULL,
            description  TEXT NOT NULL DEFAULT '',
            category_id  BIGINT,
            model_name   TEXT NOT NULL DEFAULT '',
            width        BIGINT NOT NULL DEFAULT 0,
            height       BIGINT NOT NULL DEFAULT 0,
            file_path    TEXT NOT NULL,
            prompt       TEXT NOT NULL DEFAULT '',
            price_cents  BIGINT NOT NULL DEFAULT 0,
            is_published BIGINT NOT NULL DEFAULT 1,
            created_at   TEXT NOT NULL
        )"#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS orders (
            id             BIGSERIAL PRIMARY KEY,
            order_no       TEXT NOT NULL UNIQUE,
            image_id       BIGINT NOT NULL,
            amount_cents   BIGINT NOT NULL,
            status         TEXT NOT NULL DEFAULT 'pending',
            pay_type       TEXT NOT NULL DEFAULT '',
            transaction_id TEXT NOT NULL DEFAULT '',
            created_at     TEXT NOT NULL,
            paid_at        TEXT
        )"#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS sessions (
            token      TEXT PRIMARY KEY,
            created_at TEXT NOT NULL
        )"#,
    )
    .execute(&pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_images_category ON images(category_id)")
        .execute(&pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_orders_no ON orders(order_no)")
        .execute(&pool)
        .await?;

    // 预置演示分类
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM categories")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);
    if count == 0 {
        sqlx::query("INSERT INTO categories (name, sort) VALUES ('人像写真', 1), ('风景艺术', 2), ('概念设计', 3)")
            .execute(&pool)
            .await?;
    }

    Ok(pool)
}
