use legalrust_application::AppError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrationError {
    Blocked(String),
    Skew(String),
    InvalidSignature,
}

impl fmt::Display for IntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntegrationError::Blocked(m) => write!(f, "external dependency blocked: {m}"),
            IntegrationError::Skew(m) => write!(f, "timestamp skew rejected: {m}"),
            IntegrationError::InvalidSignature => write!(f, "invalid signature"),
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
}
