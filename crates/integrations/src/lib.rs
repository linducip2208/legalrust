use legalrust_application::AppError;
use std::fmt;

pub mod tripay;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrationError {
    Blocked(String),
    Skew(String),
    InvalidSignature,
    Validation(String),
}

impl fmt::Display for IntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntegrationError::Blocked(m) => write!(f, "external dependency blocked: {m}"),
            IntegrationError::Skew(m) => write!(f, "timestamp skew rejected: {m}"),
            IntegrationError::InvalidSignature => write!(f, "invalid signature"),
            IntegrationError::Validation(m) => write!(f, "validation failed: {m}"),
        }
    }
}

impl std::error::Error for IntegrationError {}

impl From<IntegrationError> for AppError {
    fn from(e: IntegrationError) -> Self {
        match e {
            IntegrationError::Blocked(m) => AppError::BlockedExternal(m),
            IntegrationError::Skew(m) => AppError::Validation(m),
            IntegrationError::InvalidSignature => AppError::Unauthorized,
            IntegrationError::Validation(m) => AppError::Validation(m),
        }
    }
}

pub trait EsignProvider {
    fn request_signature(&self, document_id: &str) -> Result<String, IntegrationError>;
}

#[derive(Debug, Clone)]
pub struct StubEsign {
    pub disabled_reason: String,
}

impl StubEsign {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            disabled_reason: reason.into(),
        }
    }
}

impl EsignProvider for StubEsign {
    fn request_signature(&self, _document_id: &str) -> Result<String, IntegrationError> {
        Err(IntegrationError::Blocked(self.disabled_reason.clone()))
    }
}

pub trait BillingGateway {
    fn charge(&self, customer: &str, amount_cents: u64) -> Result<String, IntegrationError>;
}

#[derive(Debug, Clone)]
pub struct StubBilling {
    pub disabled_reason: String,
}

impl StubBilling {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            disabled_reason: reason.into(),
        }
    }
}

impl BillingGateway for StubBilling {
    fn charge(&self, _customer: &str, _amount_cents: u64) -> Result<String, IntegrationError> {
        Err(IntegrationError::Blocked(self.disabled_reason.clone()))
    }
}

pub const DEFAULT_MAX_SKEW_SECS: i64 = 300;

/// Pure timestamp skew check for webhook replay protection.
/// Returns `Ok` when `|now - ts| <= max_skew_secs`.
pub fn timestamp_skew_check(ts: i64, now: i64, max_skew_secs: i64) -> Result<(), IntegrationError> {
    let skew = (i128::from(now) - i128::from(ts)).abs();
    if skew <= i128::from(max_skew_secs.max(0)) {
        Ok(())
    } else {
        Err(IntegrationError::Skew(format!(
            "skew {skew}s exceeds max {max_skew_secs}s"
        )))
    }
}

pub fn sign_webhook(secret: &[u8], body: &[u8], ts: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("hmac key");
    mac.update(ts.as_bytes());
    mac.update(b".");
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}
pub fn verify_webhook(secret: &[u8], body: &[u8], ts: &str, sig: &str) -> Result<(), AppError> {
    if sign_webhook(secret, body, ts) == sig {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}
// ---------------------------------------------------------------------------
// Provider adapter layer (no live network calls).
//
// Adapters validate inputs locally and only build request URLs. Anything that
// would need credentials or network I/O returns `IntegrationError::Blocked`
// with setup instructions instead, so unconfigured externals are reported as
// BLOCKED (501 upstream) rather than faked.
// ---------------------------------------------------------------------------

/// E-signature request (e.g. PSrE-style signing of a deed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EsignRequest {
    pub deed_id: String,
    pub signer_name: String,
    pub signer_id_number: String,
    pub reason: String,
}

impl EsignRequest {
    pub fn new(
        deed_id: impl Into<String>,
        signer_name: impl Into<String>,
        signer_id_number: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            deed_id: deed_id.into(),
            signer_name: signer_name.into(),
            signer_id_number: signer_id_number.into(),
            reason: reason.into(),
        }
    }
}

/// Validate an [`EsignRequest`]: every field must be non-empty after trimming,
/// and the signer ID number must be at least 6 characters.
pub fn validate_esign_request(req: &EsignRequest) -> Result<(), IntegrationError> {
    if req.deed_id.trim().is_empty() {
        return Err(IntegrationError::Validation(
            "deed_id must not be empty".to_string(),
        ));
    }
    if req.signer_name.trim().is_empty() {
        return Err(IntegrationError::Validation(
            "signer_name must not be empty".to_string(),
        ));
    }
    if req.signer_id_number.trim().is_empty() {
        return Err(IntegrationError::Validation(
            "signer_id_number must not be empty".to_string(),
        ));
    }
    if req.signer_id_number.trim().len() < 6 {
        return Err(IntegrationError::Validation(
            "signer_id_number must be at least 6 characters".to_string(),
        ));
    }
    if req.reason.trim().is_empty() {
        return Err(IntegrationError::Validation(
            "reason must not be empty".to_string(),
        ));
    }
    Ok(())
}

/// PSrE-style e-signature adapter. Holds only the endpoint base URL and the
/// *name* of the env var carrying the API key. The key itself is read from the
/// environment at call time and is never stored, logged, or exposed via `Debug`.
pub struct PsreAdapter {
    pub base_url: String,
    pub api_key_env: String,
}

impl PsreAdapter {
    pub fn new(base_url: impl Into<String>, api_key_env: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key_env: api_key_env.into(),
        }
    }

    fn api_key_present(&self) -> Result<(), IntegrationError> {
        match std::env::var(&self.api_key_env) {
            Ok(v) if !v.trim().is_empty() => Ok(()),
            _ => Err(IntegrationError::Blocked(format!(
                "psre not configured: set {} to enable signing",
                self.api_key_env
            ))),
        }
    }

    /// Build the signing endpoint URL for `req` without performing any I/O.
    /// Returns [`IntegrationError::Blocked`] when the API key env var is unset.
    pub fn sign_url(&self, req: &EsignRequest) -> Result<String, IntegrationError> {
        validate_esign_request(req)?;
        // Presence check only: the secret value is never retained or embedded.
        self.api_key_present()?;
        let base = self.base_url.trim_end_matches('/');
        Ok(format!("{base}/sign/{}", req.deed_id.trim()))
    }
}

impl fmt::Debug for PsreAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PsreAdapter")
            .field("base_url", &self.base_url)
            .field("api_key_env", &self.api_key_env)
            .field("api_key", &"[redacted]")
            .finish()
    }
}

/// Billing invoice in minor currency units (e.g. cents).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillingInvoice {
    pub external_id: String,
    pub amount_minor: u64,
    pub currency: String,
}

impl BillingInvoice {
    pub fn new(
        external_id: impl Into<String>,
        amount_minor: u64,
        currency: impl Into<String>,
    ) -> Self {
        Self {
            external_id: external_id.into(),
            amount_minor,
            currency: currency.into(),
        }
    }
}

/// Validate a [`BillingInvoice`]: `external_id` and `currency` must be
/// non-empty after trimming and `amount_minor` must be greater than zero.
pub fn validate_billing_invoice(inv: &BillingInvoice) -> Result<(), IntegrationError> {
    if inv.external_id.trim().is_empty() {
        return Err(IntegrationError::Validation(
            "external_id must not be empty".to_string(),
        ));
    }
    if inv.amount_minor == 0 {
        return Err(IntegrationError::Validation(
            "amount_minor must be greater than zero".to_string(),
        ));
    }
    if inv.currency.trim().is_empty() {
        return Err(IntegrationError::Validation(
            "currency must not be empty".to_string(),
        ));
    }
    Ok(())
}

/// Midtrans-style billing adapter. Holds only the endpoint base URL and the
/// *name* of the env var carrying the server key; the key itself is read at
/// call time and never stored, logged, or exposed via `Debug`.
pub struct MidtransStyleAdapter {
    pub base_url: String,
    pub server_key_env: String,
}

impl MidtransStyleAdapter {
    pub fn new(base_url: impl Into<String>, server_key_env: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            server_key_env: server_key_env.into(),
        }
    }

    fn server_key_present(&self) -> Result<(), IntegrationError> {
        match std::env::var(&self.server_key_env) {
            Ok(v) if !v.trim().is_empty() => Ok(()),
            _ => Err(IntegrationError::Blocked(format!(
                "billing not configured: set {} to enable charging",
                self.server_key_env
            ))),
        }
    }

    /// Build the charge endpoint URL for `inv` without performing any I/O.
    /// Returns [`IntegrationError::Blocked`] when the server key env var is unset.
    pub fn charge_url(&self, inv: &BillingInvoice) -> Result<String, IntegrationError> {
        validate_billing_invoice(inv)?;
        // Presence check only: the secret value is never retained or embedded.
        self.server_key_present()?;
        let base = self.base_url.trim_end_matches('/');
        Ok(format!("{base}/charge/{}", inv.external_id.trim()))
    }
}

impl fmt::Debug for MidtransStyleAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MidtransStyleAdapter")
            .field("base_url", &self.base_url)
            .field("server_key_env", &self.server_key_env)
            .field("server_key", &"[redacted]")
            .finish()
    }
}

/// Normalized inbound webhook event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookEvent {
    pub provider: String,
    pub event_type: String,
    pub payload: serde_json::Value,
}

impl WebhookEvent {
    pub fn new(
        provider: impl Into<String>,
        event_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            provider: provider.into(),
            event_type: event_type.into(),
            payload,
        }
    }
}

/// Parse a raw webhook body into a [`WebhookEvent`]. The body must be valid
/// JSON containing a non-empty `"event"` or `"type"` string field; otherwise
/// [`IntegrationError::InvalidSignature`] is returned so callers reject the
/// payload as untrusted.
pub fn parse_webhook_event(provider: &str, body: &[u8]) -> Result<WebhookEvent, IntegrationError> {
    if provider.trim().is_empty() {
        return Err(IntegrationError::Validation(
            "provider must not be empty".to_string(),
        ));
    }
    let value: serde_json::Value =
        serde_json::from_slice(body).map_err(|_| IntegrationError::InvalidSignature)?;
    let event_type = value
        .get("event")
        .or_else(|| value.get("type"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(IntegrationError::InvalidSignature)?;
    Ok(WebhookEvent {
        provider: provider.trim().to_string(),
        event_type: event_type.to_string(),
        payload: value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        let s = sign_webhook(b"k", b"{}", "t");
        assert!(verify_webhook(b"k", b"{}", "t", &s).is_ok());
        assert!(verify_webhook(b"k", b"{}", "t", "bad").is_err());
    }
    #[test]
    fn skew_valid_ok() {
        assert!(timestamp_skew_check(1000, 1100, 300).is_ok());
        assert!(timestamp_skew_check(1000, 1000, 300).is_ok());
    }
    #[test]
    fn skew_expired_err() {
        assert!(timestamp_skew_check(1000, 2000, 300).is_err());
    }
    #[test]
    fn skew_future_err() {
        assert!(timestamp_skew_check(2000, 1000, 300).is_err());
    }
    #[test]
    fn stubs_are_blocked() {
        let esign = StubEsign::new("esign not configured");
        let err = esign.request_signature("doc-1").unwrap_err();
        assert!(matches!(err, IntegrationError::Blocked(_)));
        let app_err: AppError = err.into();
        assert!(matches!(app_err, AppError::BlockedExternal(_)));

        let billing = StubBilling::new("billing not configured");
        let err = billing.charge("cust-1", 100).unwrap_err();
        assert!(matches!(err, IntegrationError::Blocked(_)));
    }
    #[test]
    fn esign_request_valid_ok() {
        let req = EsignRequest::new(
            "deed-1",
            "Siti Aminah",
            "3174010101900001",
            "akta jual beli",
        );
        assert!(validate_esign_request(&req).is_ok());
    }
    #[test]
    fn esign_request_empty_field_err() {
        let req = EsignRequest::new("", "Siti Aminah", "3174010101900001", "akta");
        let err = validate_esign_request(&req).unwrap_err();
        assert!(matches!(err, IntegrationError::Validation(_)));
        let app_err: AppError = err.into();
        assert!(matches!(app_err, AppError::Validation(_)));
    }
    #[test]
    fn esign_request_short_id_number_err() {
        let req = EsignRequest::new("deed-1", "Siti Aminah", "12345", "akta");
        let err = validate_esign_request(&req).unwrap_err();
        assert!(matches!(err, IntegrationError::Validation(_)));
    }
    #[test]
    fn psre_blocked_without_env() {
        let env_name = "LEGALRUST_TEST_PSRE_KEY_BLOCKED_CASE";
        std::env::remove_var(env_name);
        let adapter = PsreAdapter::new("https://psre.example.com/v1", env_name);
        let req = EsignRequest::new("deed-9", "Budi Santoso", "3174010101900002", "kuasa");
        let err = adapter.sign_url(&req).unwrap_err();
        assert!(matches!(err, IntegrationError::Blocked(_)));
        let app_err: AppError = err.into();
        assert!(matches!(app_err, AppError::BlockedExternal(_)));
    }
    #[test]
    fn psre_sign_url_ok_with_env() {
        let env_name = "LEGALRUST_TEST_PSRE_KEY_OK_CASE";
        std::env::set_var(env_name, "test-key-value");
        let adapter = PsreAdapter::new("https://psre.example.com/v1/", env_name);
        let req = EsignRequest::new("deed-7", "Budi Santoso", "3174010101900002", "kuasa");
        let url = adapter.sign_url(&req).expect("sign_url with env set");
        assert!(url.starts_with("https://psre.example.com/v1/sign/"));
        assert!(url.contains("deed-7"));
        assert!(!url.contains("test-key-value"));
        std::env::remove_var(env_name);
    }
    #[test]
    fn billing_invoice_valid_ok() {
        let inv = BillingInvoice::new("inv-1001", 150_000, "IDR");
        assert!(validate_billing_invoice(&inv).is_ok());
    }
    #[test]
    fn billing_invoice_invalid_err() {
        let zero = BillingInvoice::new("inv-1002", 0, "IDR");
        assert!(matches!(
            validate_billing_invoice(&zero).unwrap_err(),
            IntegrationError::Validation(_)
        ));
        let empty_cur = BillingInvoice::new("inv-1003", 5000, "  ");
        assert!(matches!(
            validate_billing_invoice(&empty_cur).unwrap_err(),
            IntegrationError::Validation(_)
        ));
        let empty_id = BillingInvoice::new("", 5000, "IDR");
        assert!(matches!(
            validate_billing_invoice(&empty_id).unwrap_err(),
            IntegrationError::Validation(_)
        ));
    }
    #[test]
    fn midtrans_blocked_without_env() {
        let env_name = "LEGALRUST_TEST_MIDTRANS_KEY_BLOCKED_CASE";
        std::env::remove_var(env_name);
        let adapter = MidtransStyleAdapter::new("https://api.midtrans.example/v2", env_name);
        let inv = BillingInvoice::new("inv-2001", 25_000, "IDR");
        let err = adapter.charge_url(&inv).unwrap_err();
        assert!(matches!(err, IntegrationError::Blocked(_)));
    }
    #[test]
    fn midtrans_charge_url_ok_with_env() {
        let env_name = "LEGALRUST_TEST_MIDTRANS_KEY_OK_CASE";
        std::env::set_var(env_name, "test-server-key");
        let adapter = MidtransStyleAdapter::new("https://api.midtrans.example/v2/", env_name);
        let inv = BillingInvoice::new("inv-2002", 25_000, "IDR");
        let url = adapter.charge_url(&inv).expect("charge_url with env set");
        assert!(url.contains("inv-2002"));
        assert!(!url.contains("test-server-key"));
        std::env::remove_var(env_name);
    }
    #[test]
    fn parse_webhook_event_ok() {
        let body = br#"{"event":"payment.succeeded","order_id":"inv-2002"}"#;
        let evt = parse_webhook_event("midtrans", body).expect("parse ok");
        assert_eq!(evt.provider, "midtrans");
        assert_eq!(evt.event_type, "payment.succeeded");
        assert_eq!(evt.payload["order_id"], serde_json::json!("inv-2002"));
    }
    #[test]
    fn parse_webhook_type_field_ok() {
        let body = br#"{"type":"deed.signed","deed_id":"deed-7"}"#;
        let evt = parse_webhook_event("psre", body).expect("parse ok");
        assert_eq!(evt.event_type, "deed.signed");
    }
    #[test]
    fn parse_webhook_missing_field_err() {
        let body = br#"{"order_id":"inv-2002"}"#;
        let err = parse_webhook_event("midtrans", body).unwrap_err();
        assert!(matches!(err, IntegrationError::InvalidSignature));
    }
    #[test]
    fn parse_webhook_invalid_json_err() {
        let err = parse_webhook_event("midtrans", b"not-json").unwrap_err();
        assert!(matches!(err, IntegrationError::InvalidSignature));
    }
    #[test]
    fn adapters_debug_redacts_secrets() {
        let env_name = "LEGALRUST_TEST_PSRE_KEY_REDACT_CASE";
        std::env::set_var(env_name, "super-secret-value");
        let psre = PsreAdapter::new("https://psre.example.com", env_name);
        let dbg = format!("{psre:?}");
        assert!(!dbg.contains("super-secret-value"));
        assert!(dbg.contains("[redacted]"));
        let mid = MidtransStyleAdapter::new("https://api.midtrans.example", env_name);
        let dbg = format!("{mid:?}");
        assert!(!dbg.contains("super-secret-value"));
        assert!(dbg.contains("[redacted]"));
        std::env::remove_var(env_name);
    }
}
