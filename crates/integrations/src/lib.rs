use legalrust_application::AppError;
pub fn sign_webhook(secret: &[u8], body: &[u8], ts: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("hmac key");
    mac.update(ts.as_bytes()); mac.update(b"."); mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}
pub fn verify_webhook(secret: &[u8], body: &[u8], ts: &str, sig: &str) -> Result<(), AppError> {
    if sign_webhook(secret, body, ts) == sig { Ok(()) } else { Err(AppError::Unauthorized) }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() { let s = sign_webhook(b"k", b"{}", "t"); assert!(verify_webhook(b"k", b"{}", "t", &s).is_ok()); assert!(verify_webhook(b"k", b"{}", "t", "bad").is_err()); }
}
