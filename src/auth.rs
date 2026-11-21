use axum::http::header::COOKIE;
use axum::http::HeaderMap;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// 从请求头里取 cookie 值
pub fn get_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(COOKIE)?.to_str().ok()?;
    for pair in raw.split(';') {
        let mut it = pair.trim().splitn(2, '=');
        if it.next()? == name {
            return it.next().map(|v| v.trim().to_string());
        }
    }
    None
}

/// 生成 Set-Cookie 头的值
pub fn cookie_header(name: &str, value: &str, max_age_secs: i64) -> String {
    format!(
        "{name}={value}; Path=/; Max-Age={max_age_secs}; HttpOnly; SameSite=Lax"
    )
}

pub fn sign_str(secret: &str, data: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("hmac key");
    mac.update(data.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// 生成"已购提示词"cookie：内容为 image_id 列表 + HMAC 签名，格式 "1,2,3.<sig>"
pub fn make_purchased_cookie(secret: &str, existing: Option<&str>, image_id: i64) -> String {
    let mut ids = existing
        .and_then(|v| decode_purchased(secret, v))
        .unwrap_or_default();
    if !ids.contains(&image_id) {
        ids.push(image_id);
    }
    let ids_str = ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    let sig = sign_str(secret, &ids_str);
    format!("{ids_str}.{sig}")
}

/// 校验并解析"已购提示词"cookie
pub fn decode_purchased(secret: &str, value: &str) -> Option<Vec<i64>> {
    let (ids, sig) = value.rsplit_once('.')?;
    if sign_str(secret, ids) != sig {
        return None;
    }
    Some(ids.split(',').filter_map(|s| s.parse().ok()).collect())
}
