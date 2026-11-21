use crate::config::Config;

pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: Config,
}

pub type SharedState = std::sync::Arc<AppState>;
