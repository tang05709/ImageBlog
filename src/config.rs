//! 配置：全部来自 .env / 环境变量（不再使用 config.toml）
//!
//! 必填：DATABASE_URL
//! 可选（有默认值）：HOST / PORT / ADMIN_USERNAME / ADMIN_PASSWORD / SECRET / WECHAT_PAY_*

fn env_str(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(|v| v.trim().to_string())
}

#[derive(Clone)]
pub struct Config {
    pub server: Server,
    pub admin: Admin,
    /// cookie 签名密钥
    pub secret: String,
    /// PostgreSQL 连接串（必填）
    pub database_url: String,
    pub wechat_pay: WechatPay,
}

#[derive(Clone)]
pub struct Server {
    pub host: String,
    pub port: u16,
}

#[derive(Clone)]
pub struct Admin {
    pub username: String,
    pub password: String,
}

#[derive(Clone)]
pub struct WechatPay {
    pub enabled: bool,
    pub mchid: String,
    pub appid: String,
    pub mch_serial_no: String,
    pub mch_private_key_path: String,
    pub apiv3_key: String,
    pub notify_url: String,
}

pub fn load() -> Config {
    let database_url = env_str("DATABASE_URL").unwrap_or_else(|| {
        panic!("未配置数据库连接：请在 .env 文件中设置 DATABASE_URL=postgres://...")
    });

    let secret = env_str("SECRET").unwrap_or_else(|| {
        tracing::warn!("未设置 SECRET，使用默认值——上线前请在 .env 中改为随机长字符串");
        "change-me-to-a-random-string".into()
    });

    let admin_password = env_str("ADMIN_PASSWORD").unwrap_or_else(|| {
        tracing::warn!("未设置 ADMIN_PASSWORD，使用默认值 admin123——上线前请修改");
        "admin123".into()
    });

    Config {
        server: Server {
            host: env_str("HOST").unwrap_or_else(|| "127.0.0.1".into()),
            port: env_str("PORT")
                .and_then(|p| p.parse().ok())
                .unwrap_or(8000),
        },
        admin: Admin {
            username: env_str("ADMIN_USERNAME").unwrap_or_else(|| "admin".into()),
            password: admin_password,
        },
        secret,
        database_url,
        wechat_pay: WechatPay {
            enabled: env_str("WECHAT_PAY_ENABLED")
                .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
                .unwrap_or(false),
            mchid: env_str("WECHAT_PAY_MCHID").unwrap_or_default(),
            appid: env_str("WECHAT_PAY_APPID").unwrap_or_default(),
            mch_serial_no: env_str("WECHAT_PAY_MCH_SERIAL_NO").unwrap_or_default(),
            mch_private_key_path: env_str("WECHAT_PAY_MCH_PRIVATE_KEY_PATH").unwrap_or_default(),
            apiv3_key: env_str("WECHAT_PAY_APIV3_KEY").unwrap_or_default(),
            notify_url: env_str("WECHAT_PAY_NOTIFY_URL").unwrap_or_default(),
        },
    }
}
