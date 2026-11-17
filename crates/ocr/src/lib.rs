#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DocType {
    Ktp,
    Kk,
    Npwp,
    Passport,
    LandCertificate,
    Deed,
    Contract,
    GeneralPdf,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewBand {
    AutoPass,
    HumanReview,
    Reject,
}
pub const AUTO_PASS_THRESHOLD: f32 = 0.97;
pub const REJECT_THRESHOLD: f32 = 0.60;
pub fn confidence_band(score: f32) -> ReviewBand {
    if score >= AUTO_PASS_THRESHOLD {
        ReviewBand::AutoPass
    } else if score >= REJECT_THRESHOLD {
        ReviewBand::HumanReview
    } else {
        ReviewBand::Reject
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OcrField {
    pub name: String,
    pub value: String,
    pub confidence: f32,
    pub page: u32,
}
pub trait OcrEngine: Send + Sync {
    fn name(&self) -> &'static str;
    fn supports(&self, doc: DocType) -> bool;
    fn recognize(
        &self,
        _image: &[u8],
        _doc: DocType,
    ) -> Result<Vec<OcrField>, legalrust_application::AppError>;
}
pub struct TesseractAdapter;
impl OcrEngine for TesseractAdapter {
    fn name(&self) -> &'static str {
        "tesseract-bindings"
    }
    fn supports(&self, _: DocType) -> bool {
        true
    }
    fn recognize(
        &self,
        _image: &[u8],
        _doc: DocType,
    ) -> Result<Vec<OcrField>, legalrust_application::AppError> {
        Err(legalrust_application::AppError::BlockedExternal(
            "native libtesseract not installed on this host".into(),
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bands() {
        assert_eq!(confidence_band(0.98), ReviewBand::AutoPass);
        assert_eq!(confidence_band(0.80), ReviewBand::HumanReview);
        assert_eq!(confidence_band(0.10), ReviewBand::Reject);
    }
    #[test]
    fn adapter_is_blocked_without_native_lib() {
        assert!(TesseractAdapter.recognize(b"x", DocType::Ktp).is_err());
    }
}
