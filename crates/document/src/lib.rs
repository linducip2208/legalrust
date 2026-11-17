const ALLOWED_MIME: &[&str] = &["application/pdf", "application/vnd.openxmlformats-officedocument.wordprocessingml.document", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "text/csv", "text/plain", "image/jpeg", "image/png", "image/tiff"];
#[derive(Debug, thiserror::Error)]
pub enum DocError { #[error("mime not allowed: {0}")] Mime(String), #[error("file too large: {0} bytes")] TooLarge(u64) }
pub fn validate_upload(mime: &str, size: u64, max: u64) -> Result<(), DocError> {
    if !ALLOWED_MIME.contains(&mime) { return Err(DocError::Mime(mime.into())); }
    if size > max { return Err(DocError::TooLarge(size)); }
    Ok(())
}
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_exe_and_oversize() { assert!(validate_upload("application/x-msdownload", 10, 100).is_err()); assert!(validate_upload("application/pdf", 101, 100).is_err()); assert!(validate_upload("application/pdf", 100, 100).is_ok()); }
    #[test]
    fn checksum_stable() { assert_eq!(sha256_hex(b"akta").len(), 64); }
}
