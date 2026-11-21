use crate::config::WechatPay;
use aes_gcm::aead::{Aead, Payload};
use aes_gcm::{Aes256Gcm, KeyInit};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rand::Rng;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs8::DecodePrivateKey;
use rsa::pkcs1v15::SigningKey;
use rsa::signature::{SignatureEncoding, Signer};
use rsa::RsaPrivateKey;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

pub const API_BASE: &str = "https://api.mch.weixin.qq.com";

pub struct WxPayClient {
    pub mchid: String,
    pub appid: String,
    pub serial_no: String,
    pub key: RsaPrivateKey,
    pub notify_url: String,
    pub http: reqwest::Client,
}

impl WxPayClient {
    pub fn from_config(cfg: &WechatPay) -> Result<Self, String> {
        if cfg.mchid.is_empty() || cfg.appid.is_empty() || cfg.apiv3_key.is_empty() {
            return Err("微信支付配置不完整（mchid / appid / apiv3_key）".into());
        }
        let pem = std::fs::read_to_string(&cfg.mch_private_key_path)
            .map_err(|e| format!("读取商户私钥失败 {}: {e}", cfg.mch_private_key_path))?;
        let key = RsaPrivateKey::from_pkcs8_pem(&pem)
            .or_else(|_| RsaPrivateKey::from_pkcs1_pem(&pem))
            .map_err(|e| format!("解析商户私钥失败（应为 PKCS#8 或 PKCS#1 PEM）: {e}"))?;
        Ok(Self {
            mchid: cfg.mchid.clone(),
            appid: cfg.appid.clone(),
            serial_no: cfg.mch_serial_no.clone(),
            key,
            notify_url: cfg.notify_url.clone(),
            http: reqwest::Client::new(),
        })
    }

    fn auth_header(&self, method: &str, path: &str, body: &str) -> String {
        let ts = chrono::Utc::now().timestamp();
        let nonce: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(|c| c as char)
            .collect();
        let message = format!("{method}\n{path}\n{ts}\n{nonce}\n{body}\n");
        let signing = SigningKey::<Sha256>::new(self.key.clone());
        let sig = B64.encode(signing.sign(message.as_bytes()).to_bytes());
        format!(
            "WECHATPAY2-SHA256-RSA2048 mchid=\"{}\",nonce_str=\"{}\",signature=\"{}\",timestamp=\"{}\",serial_no=\"{}\"",
            self.mchid, nonce, sig, ts, self.serial_no
        )
    }

    /// Native 下单，返回 code_url（用于生成二维码）
    pub async fn native_pay(
        &self,
        order_no: &str,
        description: &str,
        amount_cents: i64,
    ) -> Result<String, String> {
        let path = "/v3/pay/transactions/native";
        let body = serde_json::json!({
            "appid": self.appid,
            "mchid": self.mchid,
            "description": description,
            "out_trade_no": order_no,
            "notify_url": self.notify_url,
            "amount": { "total": amount_cents, "currency": "CNY" }
        });
        let body_str = serde_json::to_string(&body).unwrap();
        let auth = self.auth_header("POST", path, &body_str);
        let resp = self
            .http
            .post(format!("{API_BASE}{path}"))
            .header("Authorization", auth)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(body_str)
            .send()
            .await
            .map_err(|e| format!("请求微信支付失败: {e}"))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let v: serde_json::Value =
            serde_json::from_str(&text).map_err(|_| format!("微信支付响应解析失败: {text}"))?;
        if status.is_success() {
            v.get("code_url")
                .and_then(|u| u.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| format!("微信支付响应缺少 code_url: {text}"))
        } else {
            Err(format!(
                "微信下单失败: {} {}",
                status,
                v.get("message").and_then(|m| m.as_str()).unwrap_or(&text)
            ))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct NotifyBody {
    pub resource: NotifyResource,
}

#[derive(Debug, Deserialize)]
pub struct NotifyResource {
    pub ciphertext: String,
    pub nonce: String,
    #[serde(default)]
    pub associated_data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeNotify {
    #[serde(default)]
    pub out_trade_no: String,
    #[serde(default)]
    pub transaction_id: String,
    #[serde(default)]
    pub trade_state: String,
}

/// 用 APIv3 密钥 AES-256-GCM 解密回调报文（商户平台文档标准流程）
pub fn decrypt_notify(
    apiv3_key: &str,
    resource: &NotifyResource,
) -> Result<TradeNotify, String> {
    let cipher = Aes256Gcm::new_from_slice(apiv3_key.as_bytes())
        .map_err(|_| "APIv3 密钥长度错误（必须 32 字节）".to_string())?;
    let ct = B64
        .decode(&resource.ciphertext)
        .map_err(|e| format!("回调密文 base64 解码失败: {e}"))?;
    let plain = cipher
        .decrypt(
            resource.nonce.as_bytes().into(),
            Payload {
                msg: &ct,
                aad: resource.associated_data.as_bytes(),
            },
        )
        .map_err(|_| "回调报文解密失败（APIv3 密钥不匹配或数据被篡改）".to_string())?;
    serde_json::from_slice(&plain).map_err(|e| format!("回调报文 JSON 解析失败: {e}"))
}
