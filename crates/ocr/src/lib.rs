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
impl DocType {
    /// Parse a user-supplied doc-type label (case-insensitive,
    /// `-`/`_` tolerant). Returns `None` for unknown labels.
    pub fn parse(s: &str) -> Option<Self> {
        let n: String = s
            .trim()
            .to_lowercase()
            .chars()
            .map(|c| if c == '-' { '_' } else { c })
            .collect();
        match n.as_str() {
            "ktp" => Some(Self::Ktp),
            "kk" | "kartu_keluarga" => Some(Self::Kk),
            "npwp" => Some(Self::Npwp),
            "passport" | "paspor" => Some(Self::Passport),
            "land_certificate" | "sertifikat" | "shm" | "hgb" => Some(Self::LandCertificate),
            "deed" | "akta" => Some(Self::Deed),
            "contract" | "kontrak" => Some(Self::Contract),
            "general" | "general_pdf" | "generalpdf" | "other" => Some(Self::GeneralPdf),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewBand {
    AutoPass,
    HumanReview,
    Reject,
}
pub const AUTO_PASS_THRESHOLD: f32 = 0.97;
pub const REJECT_THRESHOLD: f32 = 0.60;
/// Minimum per-field confidence for an extracted field to be accepted
/// without human review. External OCR engines remain BLOCKED until a
/// native lib is configured; this threshold is pure validation logic.
pub const CONFIDENCE_THRESHOLD: f32 = 0.70;
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
/// Static specification for one extractable field.
///
/// `regex` holds an optional documented pattern for the field value
/// (e.g. `"\\d{16}"` for NIK). It is descriptive only — no regex engine
/// is linked, keeping this crate offline-capable with zero new deps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FieldSpec {
    pub name: &'static str,
    pub required: bool,
    pub regex: Option<&'static str>,
}

const KTP_FIELDS: &[FieldSpec] = &[
    FieldSpec {
        name: "nik",
        required: true,
        regex: Some("\\d{16}"),
    },
    FieldSpec {
        name: "nama",
        required: true,
        regex: None,
    },
    FieldSpec {
        name: "tanggal_lahir",
        required: true,
        regex: None,
    },
    FieldSpec {
        name: "alamat",
        required: true,
        regex: None,
    },
];
const KK_FIELDS: &[FieldSpec] = &[
    FieldSpec {
        name: "no_kk",
        required: true,
        regex: Some("\\d{16}"),
    },
    FieldSpec {
        name: "nama_kepala_keluarga",
        required: true,
        regex: None,
    },
    FieldSpec {
        name: "alamat",
        required: true,
        regex: None,
    },
];
const NPWP_FIELDS: &[FieldSpec] = &[
    FieldSpec {
        name: "npwp",
        required: true,
        regex: Some("[0-9.\\-]{15,}"),
    },
    FieldSpec {
        name: "nama",
        required: true,
        regex: None,
    },
];
const PASSPORT_FIELDS: &[FieldSpec] = &[
    FieldSpec {
        name: "passport_no",
        required: true,
        regex: Some("[A-Z][0-9]{7,}"),
    },
    FieldSpec {
        name: "nama",
        required: true,
        regex: None,
    },
    FieldSpec {
        name: "kewarganegaraan",
        required: true,
        regex: None,
    },
];
const LAND_CERT_FIELDS: &[FieldSpec] = &[
    FieldSpec {
        name: "hak_no",
        required: true,
        regex: None,
    },
    FieldSpec {
        name: "nama_pemegang",
        required: true,
        regex: None,
    },
    FieldSpec {
        name: "luas",
        required: true,
        regex: None,
    },
];
const DEED_FIELDS: &[FieldSpec] = &[
    FieldSpec {
        name: "nomor_akta",
        required: true,
        regex: None,
    },
    FieldSpec {
        name: "notaris",
        required: true,
        regex: None,
    },
    FieldSpec {
        name: "tanggal",
        required: true,
        regex: None,
    },
];
const CONTRACT_FIELDS: &[FieldSpec] = &[
    FieldSpec {
        name: "nomor_kontrak",
        required: true,
        regex: None,
    },
    FieldSpec {
        name: "para_pihak",
        required: true,
        regex: None,
    },
    FieldSpec {
        name: "tanggal",
        required: true,
        regex: None,
    },
];
const GENERAL_FIELDS: &[FieldSpec] = &[FieldSpec {
    name: "text",
    required: false,
    regex: None,
}];

/// Required field specs for a document type (compat: `GeneralPdf`
/// covers the `general` label).
pub fn required_fields_for(doc: DocType) -> &'static [FieldSpec] {
    match doc {
        DocType::Ktp => KTP_FIELDS,
        DocType::Kk => KK_FIELDS,
        DocType::Npwp => NPWP_FIELDS,
        DocType::Passport => PASSPORT_FIELDS,
        DocType::LandCertificate => LAND_CERT_FIELDS,
        DocType::Deed => DEED_FIELDS,
        DocType::Contract => CONTRACT_FIELDS,
        DocType::GeneralPdf => GENERAL_FIELDS,
    }
}

/// Validate extracted fields against the required specs.
///
/// Returns problem entries: `missing:<name>` when a required field is
/// absent (case-insensitive match, empty value counts as missing) and
/// `low_confidence:<name>` when present but below
/// [`CONFIDENCE_THRESHOLD`].
pub fn validate_extracted(doc: DocType, fields: &[OcrField]) -> Vec<String> {
    let mut out = Vec::new();
    for spec in required_fields_for(doc) {
        if !spec.required {
            continue;
        }
        match fields
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(spec.name))
        {
            None => out.push(format!("missing:{}", spec.name)),
            Some(f) if f.value.trim().is_empty() || f.confidence < CONFIDENCE_THRESHOLD => {
                if f.value.trim().is_empty() {
                    out.push(format!("missing:{}", spec.name));
                } else {
                    out.push(format!("low_confidence:{}", spec.name));
                }
            }
            Some(_) => {}
        }
    }
    out
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
    #[test]
    fn parse_doc_types() {
        assert_eq!(DocType::parse("KTP"), Some(DocType::Ktp));
        assert_eq!(
            DocType::parse("land-certificate"),
            Some(DocType::LandCertificate)
        );
        assert_eq!(DocType::parse("general"), Some(DocType::GeneralPdf));
        assert_eq!(DocType::parse("nope"), None);
    }
    #[test]
    fn required_fields_nonempty_for_known_types() {
        assert!(!required_fields_for(DocType::Ktp).is_empty());
        assert!(required_fields_for(DocType::Ktp)
            .iter()
            .any(|f| f.name == "nik"));
        assert!(!required_fields_for(DocType::Contract).is_empty());
    }
    #[test]
    fn validate_reports_missing() {
        let problems = validate_extracted(DocType::Ktp, &[]);
        assert!(problems.iter().any(|p| p == "missing:nik"));
    }
    #[test]
    fn validate_reports_low_confidence_and_accepts_good() {
        let fields = vec![
            OcrField {
                name: "nik".into(),
                value: "123".into(),
                confidence: 0.5,
                page: 1,
            },
            OcrField {
                name: "nama".into(),
                value: "A".into(),
                confidence: 0.99,
                page: 1,
            },
            OcrField {
                name: "tanggal_lahir".into(),
                value: "B".into(),
                confidence: 0.99,
                page: 1,
            },
            OcrField {
                name: "alamat".into(),
                value: "C".into(),
                confidence: 0.99,
                page: 1,
            },
        ];
        let problems = validate_extracted(DocType::Ktp, &fields);
        assert_eq!(problems, vec!["low_confidence:nik".to_string()]);
        let good: Vec<OcrField> = fields
            .into_iter()
            .map(|mut f| {
                f.confidence = 0.99;
                f
            })
            .collect();
        assert!(validate_extracted(DocType::Ktp, &good).is_empty());
    }
}
