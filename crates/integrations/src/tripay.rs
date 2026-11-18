//! Tripay payment gateway adapter (no hardcoded secrets).
//!
//! All credentials come from the environment at call time:
//! - `TRIPAY_API_KEY` — Bearer key from the Tripay merchant panel.
//! - `TRIPAY_PRIVATE_KEY` — used for request signatures and callback verification.
//! - `TRIPAY_MERCHANT_CODE` — merchant code from the Tripay merchant panel.
//! - `TRIPAY_MODE` — `sandbox` (default) or `production`; selects the base URL.
//! - `TRIPAY_BASE_URL` — optional full override (tests / proxies).
//!
//! Without these, every call fails with [`IntegrationError::Blocked`] so the
//! HTTP layer surfaces 501 instead of fabricating a checkout.

use super::IntegrationError;

/// Tripay base URL for `TRIPAY_MODE=sandbox`.
pub const TRIPAY_SANDBOX_BASE: &str = "https://tripay.co.id/api-sandbox";
/// Tripay base URL for `TRIPAY_MODE=production`.
pub const TRIPAY_PROD_BASE: &str = "https://tripay.co.id/api";

/// Env-var names + non-secret callback URLs. Secrets are never stored here.
#[derive(Debug, Clone)]
pub struct TripayConfig {
    pub mode_env: String,
    pub base_url_env: String,
    pub api_key_env: String,
    pub private_key_env: String,
    pub merchant_code_env: String,
    pub callback_url_env: String,
    pub return_url_env: String,
    pub callback_url: String,
    pub return_url: String,
}

impl TripayConfig {
    /// Standard `TRIPAY_*` env names with empty callback/return URLs (the
    /// server fills them from its own public base URL).
    pub fn default_env() -> Self {
        Self {
            mode_env: "TRIPAY_MODE".into(),
            base_url_env: "TRIPAY_BASE_URL".into(),
            api_key_env: "TRIPAY_API_KEY".into(),
            private_key_env: "TRIPAY_PRIVATE_KEY".into(),
            merchant_code_env: "TRIPAY_MERCHANT_CODE".into(),
            callback_url_env: "TRIPAY_CALLBACK_URL".into(),
            return_url_env: "TRIPAY_RETURN_URL".into(),
            callback_url: String::new(),
            return_url: String::new(),
        }
    }

    /// Callback/return URLs: explicit value wins, else the matching env var,
    /// else empty (Tripay then skips callbacks for the transaction).
    pub fn callback_urls(&self) -> (String, String) {
        let cb = if !self.callback_url.trim().is_empty() {
            self.callback_url.clone()
        } else {
            std::env::var(&self.callback_url_env).unwrap_or_default()
        };
        let rt = if !self.return_url.trim().is_empty() {
            self.return_url.clone()
        } else {
            std::env::var(&self.return_url_env).unwrap_or_default()
        };
        (cb, rt)
    }
}

impl Default for TripayConfig {
    fn default() -> Self {
        Self::default_env()
    }
}

/// Resolved credentials (kept out of `Debug`).
pub struct ResolvedTripay {
    pub base_url: String,
    pub api_key: String,
    pub private_key: String,
    pub merchant_code: String,
}

impl std::fmt::Debug for ResolvedTripay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedTripay")
            .field("base_url", &self.base_url)
            .field("api_key", &"[redacted]")
            .field("private_key", &"[redacted]")
            .field("merchant_code", &self.merchant_code)
            .finish()
    }
}

fn required_env(name: &str, what: &str) -> Result<String, IntegrationError> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Ok(v),
        _ => Err(IntegrationError::Blocked(format!(
            "tripay not configured: set {name} ({what}) to enable gateway payments"
        ))),
    }
}

impl TripayConfig {
    /// Read and validate env credentials. Missing/empty values yield
    /// [`IntegrationError::Blocked`]; `TRIPAY_MODE` defaults to `sandbox`
    /// and any other value is a [`IntegrationError::Validation`].
    pub fn resolve(&self) -> Result<ResolvedTripay, IntegrationError> {
        let api_key = required_env(&self.api_key_env, "API key")?;
        let private_key = required_env(&self.private_key_env, "private key")?;
        let merchant_code = required_env(&self.merchant_code_env, "merchant code")?;
        let mode = std::env::var(&self.mode_env).unwrap_or_else(|_| "sandbox".into());
        let mode = mode.trim().to_lowercase();
        let base_url = match std::env::var(&self.base_url_env) {
            Ok(v) if !v.trim().is_empty() => v.trim().trim_end_matches('/').to_owned(),
            _ => match mode.as_str() {
                "sandbox" => TRIPAY_SANDBOX_BASE.into(),
                "production" => TRIPAY_PROD_BASE.into(),
                other => {
                    return Err(IntegrationError::Validation(format!(
                        "TRIPAY_MODE must be sandbox or production, got {other:?}"
                    )));
                }
            },
        };
        Ok(ResolvedTripay {
            base_url,
            api_key,
            private_key,
            merchant_code,
        })
    }
}

/// Tripay closed-transaction signature:
/// `HMAC_SHA256(merchant_code + merchant_ref + amount, private_key)` as hex.
pub fn tripay_signature(
    private_key: &str,
    merchant_code: &str,
    merchant_ref: &str,
    amount_minor: i64,
) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(private_key.as_bytes()).expect("hmac key");
    mac.update(merchant_code.as_bytes());
    mac.update(merchant_ref.as_bytes());
    mac.update(amount_minor.to_string().as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Verify a Tripay callback: `X-Callback-Signature` must equal
/// `HMAC_SHA256(raw_body, private_key)` hex. Comparison is exact-match on
/// lowercase hex (Tripay sends lowercase).
pub fn verify_tripay_callback(
    private_key: &str,
    raw_body: &[u8],
    signature_header: &str,
) -> Result<(), IntegrationError> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(private_key.as_bytes()).expect("hmac key");
    mac.update(raw_body);
    let expect = hex::encode(mac.finalize().into_bytes());
    if expect == signature_header.trim().to_lowercase() {
        Ok(())
    } else {
        Err(IntegrationError::InvalidSignature)
    }
}

/// Build a unique merchant reference embedding the invoice id for callback
/// mapping: `LRINV{invoice_id}-{6 hex}`. Parse with [`parse_merchant_ref`].
pub fn build_merchant_ref(invoice_id: i64) -> String {
    let stamp = format!("{:06x}", rand_suffix());
    format!("LRINV{invoice_id}-{stamp}")
}

fn rand_suffix() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    nanos ^ (std::process::id().rotate_left(7))
}

/// Inverse of [`build_merchant_ref`]: returns the invoice id, or `None` when
/// the reference is not ours.
pub fn parse_merchant_ref(merchant_ref: &str) -> Option<i64> {
    let rest = merchant_ref.strip_prefix("LRINV")?;
    let (id_part, _) = rest.split_once('-')?;
    id_part.parse::<i64>().ok()
}

/// One order line for the Tripay `order_items` array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TripayItem {
    pub sku: String,
    pub name: String,
    pub price_minor: i64,
    pub quantity: u32,
}

impl TripayItem {
    pub fn new(
        sku: impl Into<String>,
        name: impl Into<String>,
        price_minor: i64,
        quantity: u32,
    ) -> Self {
        Self {
            sku: sku.into(),
            name: name.into(),
            price_minor,
            quantity,
        }
    }
}

/// Validated checkout creation request.
#[derive(Debug, Clone)]
pub struct TripayCreateReq {
    pub channel: String,
    pub merchant_ref: String,
    pub amount_minor: i64,
    pub customer_name: String,
    pub customer_email: Option<String>,
    pub customer_phone: Option<String>,
    pub items: Vec<TripayItem>,
    /// Lifetime in whole hours, clamped to 1..=168 by the caller.
    pub expired_hours: i64,
    pub callback_url: String,
    pub return_url: String,
}

impl TripayCreateReq {
    pub fn validate(&self) -> Result<(), IntegrationError> {
        if self.channel.trim().is_empty() {
            return Err(IntegrationError::Validation(
                "channel must not be empty".into(),
            ));
        }
        if self.merchant_ref.trim().is_empty() {
            return Err(IntegrationError::Validation(
                "merchant_ref must not be empty".into(),
            ));
        }
        if self.amount_minor <= 0 {
            return Err(IntegrationError::Validation(
                "amount must be greater than zero".into(),
            ));
        }
        if self.customer_name.trim().is_empty() {
            return Err(IntegrationError::Validation(
                "customer_name must not be empty".into(),
            ));
        }
        if !(1..=168).contains(&self.expired_hours) {
            return Err(IntegrationError::Validation(
                "expired_hours must be 1..=168".into(),
            ));
        }
        let items_total: i64 = self
            .items
            .iter()
            .map(|i| i.price_minor * i64::from(i.quantity))
            .sum();
        if !self.items.is_empty() && items_total != self.amount_minor {
            return Err(IntegrationError::Validation(format!(
                "order_items total {items_total} != amount {}",
                self.amount_minor
            )));
        }
        Ok(())
    }
}

/// Parsed subset of Tripay's create-transaction response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TripayCreateRes {
    pub reference: String,
    pub merchant_ref: String,
    pub checkout_url: String,
    pub amount_minor: i64,
}

/// Tripay callback payload subset.
#[derive(Debug, Clone)]
pub struct TripayCallback {
    pub reference: String,
    pub merchant_ref: String,
    pub status: String,
    pub amount_received: i64,
}

impl TripayCallback {
    pub fn is_paid(&self) -> bool {
        self.status.eq_ignore_ascii_case("paid")
    }

    pub fn parse(body: &[u8]) -> Result<Self, IntegrationError> {
        let v: serde_json::Value =
            serde_json::from_slice(body).map_err(|_| IntegrationError::InvalidSignature)?;
        let s = |k: &str| {
            v.get(k)
                .and_then(|x| x.as_str())
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .map(str::to_owned)
        };
        let n = |k: &str| v.get(k).and_then(serde_json::Value::as_i64).unwrap_or(0);
        Ok(Self {
            reference: s("reference").ok_or(IntegrationError::InvalidSignature)?,
            merchant_ref: s("merchant_ref").ok_or(IntegrationError::InvalidSignature)?,
            status: s("status").ok_or(IntegrationError::InvalidSignature)?,
            amount_received: n("amount_received"),
        })
    }
}

fn map_reqwest(e: reqwest::Error) -> IntegrationError {
    if e.is_timeout() || e.is_connect() {
        IntegrationError::Blocked(format!("tripay unreachable: {e}"))
    } else {
        IntegrationError::Blocked(format!("tripay transport error: {e}"))
    }
}

/// Build the HTTPS client for Tripay (rustls, OS cert store, no OpenSSL).
pub fn http_client(timeout_secs: u64) -> Result<reqwest::Client, IntegrationError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs.max(1)))
        .build()
        .map_err(|e| IntegrationError::Blocked(format!("tripay client build: {e}")))
}

/// Create a Tripay closed transaction. Network + credential errors surface as
/// [`IntegrationError::Blocked`]; provider-side rejections include the
/// provider message (never the API key).
pub async fn create_transaction(
    client: &reqwest::Client,
    cfg: &ResolvedTripay,
    req: &TripayCreateReq,
) -> Result<TripayCreateRes, IntegrationError> {
    req.validate()?;
    let expired_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
        + req.expired_hours * 3600;
    let signature = tripay_signature(
        &cfg.private_key,
        &cfg.merchant_code,
        &req.merchant_ref,
        req.amount_minor,
    );
    let mut body = serde_json::json!({
        "method": req.channel.trim(),
        "merchant_ref": req.merchant_ref,
        "amount": req.amount_minor,
        "customer_name": req.customer_name,
        "expired_time": expired_time,
        "signature": signature,
    });
    if let Some(email) = req
        .customer_email
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        body["customer_email"] = serde_json::json!(email);
    }
    if let Some(phone) = req
        .customer_phone
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        body["customer_phone"] = serde_json::json!(phone);
    }
    if !req.items.is_empty() {
        body["order_items"] = serde_json::json!(req
            .items
            .iter()
            .map(|i| serde_json::json!({
                "sku": i.sku,
                "name": i.name,
                "price": i.price_minor,
                "quantity": i.quantity,
            }))
            .collect::<Vec<_>>());
    }
    if !req.callback_url.trim().is_empty() {
        body["callback_url"] = serde_json::json!(req.callback_url);
    }
    if !req.return_url.trim().is_empty() {
        body["return_url"] = serde_json::json!(req.return_url);
    }
    let url = format!("{}/transaction/create", cfg.base_url.trim_end_matches('/'));
    let resp = client
        .post(&url)
        .bearer_auth(&cfg.api_key)
        .json(&body)
        .send()
        .await
        .map_err(map_reqwest)?;
    let status = resp.status();
    let value: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| IntegrationError::Blocked(format!("tripay bad response envelope: {e}")))?;
    if !status.is_success() || value.get("success").and_then(|v| v.as_bool()) != Some(true) {
        let msg = value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("rejected");
        let short: String = msg.chars().take(200).collect();
        return Err(IntegrationError::Blocked(format!(
            "tripay rejected ({status}): {short}"
        )));
    }
    let data = value
        .get("data")
        .ok_or_else(|| IntegrationError::Blocked("tripay response missing data".into()))?;
    let s = |k: &str| {
        data.get(k)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|x| !x.is_empty())
            .map(str::to_owned)
    };
    let reference = s("reference")
        .ok_or_else(|| IntegrationError::Blocked("tripay response missing reference".into()))?;
    let checkout_url = s("checkout_url")
        .or_else(|| s("pay_url"))
        .or_else(|| s("qr_url"))
        .or_else(|| s("pay_code"))
        .ok_or_else(|| IntegrationError::Blocked("tripay response missing checkout_url".into()))?;
    Ok(TripayCreateRes {
        reference,
        merchant_ref: s("merchant_ref").unwrap_or(req.merchant_ref.clone()),
        checkout_url,
        amount_minor: data
            .get("amount")
            .and_then(|v| v.as_i64())
            .unwrap_or(req.amount_minor),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clean_env() {
        for v in [
            "TRIPAY_MODE",
            "TRIPAY_BASE_URL",
            "TRIPAY_API_KEY",
            "TRIPAY_PRIVATE_KEY",
            "TRIPAY_MERCHANT_CODE",
            "TRIPAY_CALLBACK_URL",
            "TRIPAY_RETURN_URL",
        ] {
            std::env::remove_var(v);
        }
    }

    #[test]
    fn signature_known_vector() {
        // HMAC_SHA256("TCODE123INV-1-ABCDEF250000", key="test-private-key").
        let sig = tripay_signature("test-private-key", "TCODE123", "INV-1-ABCDEF", 250000);
        assert_eq!(
            sig,
            "cb2a08b54b1ffe2d9e7adfc14dee8ebb1fcba516fe7e6998baf589c11199ffe1"
        );
    }

    #[test]
    fn verify_callback_ok_and_wrong() {
        let body = br#"{"reference":"T1","status":"PAID"}"#;
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let mut mac = Hmac::<Sha256>::new_from_slice(b"k").expect("key");
        mac.update(body);
        let sig = hex::encode(mac.finalize().into_bytes());
        assert!(verify_tripay_callback("k", body, &sig).is_ok());
        assert!(verify_tripay_callback("k", body, "deadbeef").is_err());
        assert!(verify_tripay_callback("other", body, &sig).is_err());
    }

    #[test]
    fn merchant_ref_roundtrip() {
        let r = build_merchant_ref(42);
        assert_eq!(parse_merchant_ref(&r), Some(42));
        assert_eq!(parse_merchant_ref("T12345"), None);
        assert_eq!(parse_merchant_ref("LRINVabc-x"), None);
    }

    #[test]
    fn resolve_blocked_without_env() {
        clean_env();
        let err = TripayConfig::default_env().resolve().unwrap_err();
        assert!(matches!(err, IntegrationError::Blocked(_)));
    }

    #[test]
    fn resolve_mode_validation() {
        clean_env();
        std::env::set_var("TRIPAY_API_KEY", "k");
        std::env::set_var("TRIPAY_PRIVATE_KEY", "p");
        std::env::set_var("TRIPAY_MERCHANT_CODE", "M");
        std::env::set_var("TRIPAY_MODE", "bogus");
        let err = TripayConfig::default_env().resolve().unwrap_err();
        assert!(matches!(err, IntegrationError::Validation(_)));
        std::env::set_var("TRIPAY_MODE", "sandbox");
        let r = TripayConfig::default_env().resolve().expect("sandbox ok");
        assert_eq!(r.base_url, TRIPAY_SANDBOX_BASE);
        std::env::set_var("TRIPAY_MODE", "production");
        let r = TripayConfig::default_env().resolve().expect("prod ok");
        assert_eq!(r.base_url, TRIPAY_PROD_BASE);
        clean_env();
    }

    #[test]
    fn create_req_validation() {
        let base = TripayCreateReq {
            channel: "QRIS".into(),
            merchant_ref: "LRINV1-abc".into(),
            amount_minor: 100000,
            customer_name: "Budi".into(),
            customer_email: None,
            customer_phone: None,
            items: vec![],
            expired_hours: 24,
            callback_url: String::new(),
            return_url: String::new(),
        };
        assert!(base.validate().is_ok());
        let mut bad = base.clone();
        bad.channel = "  ".into();
        assert!(bad.validate().is_err());
        let mut bad = base.clone();
        bad.expired_hours = 0;
        assert!(bad.validate().is_err());
        let mut bad = base.clone();
        bad.items = vec![TripayItem::new("s", "n", 60000, 1)];
        assert!(bad.validate().is_err());
    }

    #[test]
    fn callback_parse_and_paid() {
        let body = br#"{"reference":"T1","merchant_ref":"LRINV7-x","status":"PAID","amount_received":50000}"#;
        let cb = TripayCallback::parse(body).expect("parse ok");
        assert!(cb.is_paid());
        assert_eq!(cb.amount_received, 50000);
        let body = br#"{"reference":"T2","merchant_ref":"LRINV7-x","status":"EXPIRED"}"#;
        let cb = TripayCallback::parse(body).expect("parse ok");
        assert!(!cb.is_paid());
        assert!(TripayCallback::parse(b"nope").is_err());
    }

    #[test]
    fn callback_urls_from_env() {
        clean_env();
        let cfg = TripayConfig::default_env();
        assert_eq!(cfg.callback_urls(), (String::new(), String::new()));
        std::env::set_var(
            "TRIPAY_CALLBACK_URL",
            "https://app.example.id/api/v1/webhooks/incoming/tripay",
        );
        let (cb, rt) = cfg.callback_urls();
        assert!(cb.contains("tripay"));
        assert!(rt.is_empty());
        clean_env();
    }

    #[test]
    fn debug_redacts_secrets() {
        let r = ResolvedTripay {
            base_url: "https://x".into(),
            api_key: "SECRET-A".into(),
            private_key: "SECRET-P".into(),
            merchant_code: "M".into(),
        };
        let d = format!("{r:?}");
        assert!(!d.contains("SECRET-A"));
        assert!(!d.contains("SECRET-P"));
    }

    /// Live HTTP shape test against a local stub (no Tripay network needed).
    #[tokio::test]
    async fn create_transaction_ok_against_stub() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind stub");
        let addr = listener.local_addr().expect("addr");
        let handle = tokio::task::spawn_blocking(move || {
            let (mut s, _) = listener.accept().expect("accept");
            let mut buf = vec![0u8; 8192];
            let _ = s.read(&mut buf);
            let body = br#"{"success":true,"message":"ok","data":{"reference":"T999","merchant_ref":"LRINV3-ab","amount":75000,"checkout_url":"https://tripay.co.id/checkout/T999"}}"#;
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            s.write_all(head.as_bytes()).expect("head");
            s.write_all(body).expect("body");
        });
        let cfg = ResolvedTripay {
            base_url: format!("http://{addr}"),
            api_key: "stub-key".into(),
            private_key: "stub-priv".into(),
            merchant_code: "STUB".into(),
        };
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("client");
        let req = TripayCreateReq {
            channel: "QRIS".into(),
            merchant_ref: "LRINV3-ab".into(),
            amount_minor: 75000,
            customer_name: "Ayu".into(),
            customer_email: Some("ayu@x.id".into()),
            customer_phone: None,
            items: vec![],
            expired_hours: 24,
            callback_url: String::new(),
            return_url: String::new(),
        };
        let res = create_transaction(&client, &cfg, &req)
            .await
            .expect("stub ok");
        assert_eq!(res.reference, "T999");
        assert_eq!(res.amount_minor, 75000);
        assert!(res.checkout_url.contains("T999"));
        handle.await.expect("stub done");
    }

    #[tokio::test]
    async fn create_transaction_rejection_surfaces() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind stub");
        let addr = listener.local_addr().expect("addr");
        let handle = tokio::task::spawn_blocking(move || {
            let (mut s, _) = listener.accept().expect("accept");
            let mut buf = vec![0u8; 8192];
            let _ = s.read(&mut buf);
            let body = br#"{"success":false,"message":"Invalid API Key"}"#;
            let head = format!(
                "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            s.write_all(head.as_bytes()).expect("head");
            s.write_all(body).expect("body");
        });
        let cfg = ResolvedTripay {
            base_url: format!("http://{addr}"),
            api_key: "bad".into(),
            private_key: "p".into(),
            merchant_code: "M".into(),
        };
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("client");
        let req = TripayCreateReq {
            channel: "QRIS".into(),
            merchant_ref: "LRINV3-ab".into(),
            amount_minor: 75000,
            customer_name: "Ayu".into(),
            customer_email: None,
            customer_phone: None,
            items: vec![],
            expired_hours: 24,
            callback_url: String::new(),
            return_url: String::new(),
        };
        let err = create_transaction(&client, &cfg, &req).await.unwrap_err();
        assert!(matches!(err, IntegrationError::Blocked(_)));
        let msg = err.to_string();
        assert!(msg.contains("Invalid API Key"));
        handle.await.expect("stub done");
    }
}
