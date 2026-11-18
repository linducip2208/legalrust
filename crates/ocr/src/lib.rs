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
/// Result of probing the host for a usable Tesseract OCR engine.
///
/// Pure-`std` check: looks for a `tesseract` binary on `PATH` (via a
/// best-effort `tesseract --version` spawn) and records `TESSDATA_PREFIX`.
/// No new system dependencies; no image rendering or fake extraction.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EngineProbe {
    pub tesseract_found: bool,
    pub detail: String,
}

/// Probe the host for Tesseract availability (pure `std`, no new deps).
///
/// `detail` is always non-empty and records the evidence, e.g.
/// `tesseract_on_path=false; TESSDATA_PREFIX=unset`. Callers surface it in
/// `BlockedExternal` messages so BLOCKED reports carry host evidence.
pub fn probe() -> EngineProbe {
    let tessdata = std::env::var("TESSDATA_PREFIX").ok();
    let (found, version_note) = match std::process::Command::new("tesseract")
        .arg("--version")
        .output()
    {
        Ok(out) if out.status.success() => {
            let first = String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .unwrap_or("tesseract")
                .trim()
                .to_owned();
            let v = if first.is_empty() {
                "tesseract".to_owned()
            } else {
                first
            };
            (true, v)
        }
        Ok(out) => (
            false,
            format!("tesseract exited with status {}", out.status),
        ),
        Err(e) => (false, format!("tesseract spawn failed: {e}")),
    };
    let detail = format!(
        "tesseract_on_path={found}; version_note={version_note}; TESSDATA_PREFIX={}",
        tessdata.as_deref().unwrap_or("unset")
    );
    EngineProbe {
        tesseract_found: found,
        detail,
    }
}

/// Human-readable blocked reason including live host probe evidence.
///
/// Zero API breakage: same `BlockedExternal(String)` shape, only the
/// message gains evidence. Kept separate from any constant so the detail
/// stays fresh per call.
pub fn blocked_reason() -> String {
    let p = probe();
    format!(
        "native libtesseract not installed on this host ({})",
        p.detail
    )
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
        "tesseract-cli"
    }
    fn supports(&self, _: DocType) -> bool {
        true
    }
    fn recognize(
        &self,
        image: &[u8],
        _doc: DocType,
    ) -> Result<Vec<OcrField>, legalrust_application::AppError> {
        recognize_image(image)
    }
}

/// Languages reported by `tesseract --list-langs` (stdout+stderr combined;
/// first header line skipped). Empty when the binary is missing/failing.
pub fn available_langs() -> Vec<String> {
    let out = match std::process::Command::new("tesseract")
        .arg("--list-langs")
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    combined
        .lines()
        .skip(1)
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Pick OCR languages: `ind+eng` when both traineddata exist, else `eng`,
/// else the first available language. `None` when no traineddata found.
fn pick_langs() -> Option<String> {
    let langs = available_langs();
    if langs.iter().any(|l| l == "ind") && langs.iter().any(|l| l == "eng") {
        Some("ind+eng".into())
    } else if langs.iter().any(|l| l == "eng") {
        Some("eng".into())
    } else {
        langs.into_iter().next()
    }
}

/// Sniff a supported raster-image extension from magic bytes.
/// Tesseract CLI handles PNG/JPEG/WEBP/BMP/TIFF; anything else (incl. PDF)
/// is rejected honestly by the caller.
pub fn image_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else if bytes.starts_with(b"BM") {
        Some("bmp")
    } else if bytes.starts_with(&[0x49, 0x49, 0x2A, 0x00])
        || bytes.starts_with(&[0x4D, 0x4D, 0x00, 0x2A])
    {
        Some("tif")
    } else {
        None
    }
}

/// Parse Tesseract TSV output (word rows, `level == 5`) into line fields.
///
/// Groups words by `(page, block, par, line)`, joins text with spaces, and
/// averages word confidences (`conf` is 0-100, `-1` for non-text → 0).
/// Empty output yields an empty vec — never fabricated.
pub fn parse_tsv(tsv: &str) -> Vec<OcrField> {
    /// Per-line accumulator: grouped words, joined text, confidence sum.
    struct LineAcc {
        page: u32,
        block: u32,
        par: u32,
        line: u32,
        text: String,
        conf_sum: f32,
        n: u32,
    }
    let mut out: Vec<LineAcc> = Vec::new();
    for line in tsv.lines().skip(1) {
        let c: Vec<&str> = line.split('\t').collect();
        if c.len() < 12 || c[0] != "5" {
            continue;
        }
        let num = |i: usize| c.get(i).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
        let conf: f32 = c
            .get(10)
            .and_then(|v| v.parse::<f32>().ok())
            .map(|v| if v < 0.0 { 0.0 } else { v / 100.0 })
            .unwrap_or(0.0);
        let text = c.get(11).unwrap_or(&"").trim();
        if text.is_empty() {
            continue;
        }
        let (page, block, par, lineno) = (num(1).max(1), num(2), num(3), num(4));
        match out
            .iter_mut()
            .find(|e: &&mut LineAcc| (e.page, e.block, e.par, e.line) == (page, block, par, lineno))
        {
            Some(e) => {
                e.text.push(' ');
                e.text.push_str(text);
                e.conf_sum += conf;
                e.n += 1;
            }
            None => out.push(LineAcc {
                page,
                block,
                par,
                line: lineno,
                text: text.to_owned(),
                conf_sum: conf,
                n: 1,
            }),
        }
    }
    out.into_iter()
        .map(|e| OcrField {
            name: format!("page{}_line{}", e.page, e.line),
            value: e.text,
            confidence: (e.conf_sum / e.n as f32).clamp(0.0, 1.0),
            page: e.page,
        })
        .collect()
}

/// Run the native `tesseract` CLI on image bytes and return line fields.
///
/// Pure-`std`: temp file + `tesseract <tmp> stdout -l <langs> tsv` with a
/// 120s watchdog. Missing binary → `BlockedExternal` (501); unsupported
/// bytes → `Validation` (422); engine failure → `Internal` with truncated
/// stderr. Never fabricates text.
pub fn recognize_image(image: &[u8]) -> Result<Vec<OcrField>, legalrust_application::AppError> {
    use legalrust_application::AppError;
    if image.is_empty() {
        return Err(AppError::Validation("empty image".into()));
    }
    if !probe().tesseract_found {
        return Err(AppError::BlockedExternal(blocked_reason()));
    }
    let ext = image_extension(image).ok_or_else(|| {
        AppError::Validation(
            "unsupported image format (png/jpg/webp/bmp/tif only; pdf pending)".into(),
        )
    })?;
    let langs = pick_langs().ok_or_else(|| {
        AppError::BlockedExternal("tesseract has no traineddata (install ind+eng)".into())
    })?;
    let tmp = std::env::temp_dir().join(format!(
        "lr_ocr_{}_{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
        ext
    ));
    std::fs::write(&tmp, image)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("temp write: {e}")))?;
    let run = || -> Result<String, AppError> {
        let (tx, rx) = std::sync::mpsc::channel();
        let path = tmp.clone();
        let langs = langs.clone();
        std::thread::spawn(move || {
            let r = std::process::Command::new("tesseract")
                .arg(&path)
                .arg("stdout")
                .arg("-l")
                .arg(&langs)
                .arg("tsv")
                .output();
            let _ = tx.send(r);
        });
        let child = rx
            .recv_timeout(std::time::Duration::from_secs(120))
            .map_err(|_| AppError::Internal(anyhow::anyhow!("tesseract timed out after 120s")))?
            .map_err(|e| AppError::Internal(anyhow::anyhow!("tesseract spawn: {e}")))?;
        if !child.status.success() {
            let err = String::from_utf8_lossy(&child.stderr);
            let short: String = err.chars().take(300).collect();
            return Err(AppError::Internal(anyhow::anyhow!(
                "tesseract failed: {short}"
            )));
        }
        Ok(String::from_utf8_lossy(&child.stdout).into_owned())
    };
    let tsv = run()?;
    let _ = std::fs::remove_file(&tmp);
    Ok(parse_tsv(&tsv))
}
/// Cloud OCR fallback client (plumbing only, no extraction logic).
///
/// Raw HTTP/1.1 over `std::net::TcpStream` with zero new dependencies. This
/// adapter never fabricates field values: [`CloudOcrAdapter::submit`]
/// uploads bytes and returns the provider's job id verbatim, and
/// [`CloudOcrAdapter::fetch_text`] returns the provider's text verbatim.
/// Field mapping stays with the caller via
/// [`required_fields_for`]/[`validate_extracted`].
///
/// Without endpoint + API-key configuration (env vars named by
/// [`OcrProviderConfig`]) every call fails honestly with
/// [`OcrError::Blocked`]: engine activation stays BLOCKED pending creds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcrProviderConfig {
    /// Name of the env var holding the provider base URL, e.g.
    /// `http://127.0.0.1:8080` (scheme optional, trailing path optional).
    pub endpoint_env: String,
    /// Name of the env var holding the provider API key.
    pub api_key_env: String,
    /// Connect + read/write timeout in milliseconds.
    pub timeout_ms: u64,
}

impl OcrProviderConfig {
    /// Name the env vars that carry the endpoint URL and API key, plus the
    /// per-operation timeout. No I/O happens here; env vars are read per
    /// call so missing config surfaces as [`OcrError::Blocked`].
    pub fn new(
        endpoint_env: impl Into<String>,
        api_key_env: impl Into<String>,
        timeout_ms: u64,
    ) -> Self {
        Self {
            endpoint_env: endpoint_env.into(),
            api_key_env: api_key_env.into(),
            timeout_ms,
        }
    }
}

/// Provider job handle returned verbatim by the OCR service on submit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudJobId(pub String);

impl CloudJobId {
    /// Borrow the raw provider-issued job id.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CloudJobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Errors from the cloud OCR fallback. `Blocked` is the honest
/// unconfigured path (never fake extraction); `Api` covers non-2xx
/// provider responses; `Timeout` covers connect/read/write deadlines.
#[derive(Debug)]
pub enum OcrError {
    /// No usable configuration (endpoint/API key unset, empty, or invalid).
    Blocked(String),
    /// TCP/DNS/protocol failure below HTTP semantics.
    Transport(String),
    /// Provider answered with non-2xx status or malformed envelope.
    Api(String),
    /// Connect, write, or read exceeded `timeout_ms`.
    Timeout,
    /// `doc_type` label rejected by the [`DocType::parse`] allowlist.
    UnsupportedType(String),
}

impl std::fmt::Display for OcrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Blocked(msg) => write!(f, "cloud OCR blocked: {msg}"),
            Self::Transport(msg) => write!(f, "cloud OCR transport error: {msg}"),
            Self::Api(msg) => write!(f, "cloud OCR provider error: {msg}"),
            Self::Timeout => write!(f, "cloud OCR request timed out"),
            Self::UnsupportedType(label) => {
                write!(f, "unsupported doc_type for cloud OCR: {label}")
            }
        }
    }
}

impl std::error::Error for OcrError {}

/// Thin async wrapper over the blocking std HTTP exchange (no runtime dep:
/// futures complete without yielding, callers `await` them directly).
#[derive(Debug, Clone)]
pub struct CloudOcrAdapter {
    config: OcrProviderConfig,
}

impl CloudOcrAdapter {
    /// Build an adapter over the given provider configuration.
    pub fn new(config: OcrProviderConfig) -> Self {
        Self { config }
    }

    /// Borrow the provider configuration this adapter was built with.
    pub fn config(&self) -> &OcrProviderConfig {
        &self.config
    }

    /// Allowlist check reusing the existing [`DocType::parse`] vocabulary.
    fn check_doc_type(doc_type: &str) -> Result<DocType, OcrError> {
        DocType::parse(doc_type).ok_or_else(|| {
            OcrError::UnsupportedType(format!("unsupported doc_type label: {doc_type}"))
        })
    }

    /// Upload document bytes for `doc_type` and return the provider's job
    /// id verbatim. Fails with `UnsupportedType` before any I/O when the
    /// label is outside the [`DocType::parse`] vocabulary, and with
    /// `Blocked` when endpoint/API-key env vars are unset.
    pub async fn submit(&self, bytes: &[u8], doc_type: &str) -> Result<CloudJobId, OcrError> {
        let doc = Self::check_doc_type(doc_type)?;
        let resolved = resolve_config(&self.config)?;
        let path = format!("{}/submit", resolved.base_path);
        let (_, text) = http_exchange(
            &resolved,
            "POST",
            &path,
            &[
                ("Content-Type", "application/octet-stream"),
                ("X-Doc-Type", canonical_label(doc)),
            ],
            bytes,
        )?;
        let id = text.trim().to_owned();
        if id.is_empty() {
            return Err(OcrError::Api("provider returned empty job id".into()));
        }
        Ok(CloudJobId(id))
    }

    /// Fetch the provider's text for `job` verbatim (no field fabrication).
    pub async fn fetch_text(&self, job: &CloudJobId) -> Result<String, OcrError> {
        if job.as_str().trim().is_empty() {
            return Err(OcrError::Transport("empty job id".into()));
        }
        let resolved = resolve_config(&self.config)?;
        let path = format!("{}/fetch/{}", resolved.base_path, job.as_str().trim());
        let (_, text) = http_exchange(&resolved, "GET", &path, &[], &[])?;
        Ok(text)
    }
}

/// Canonical wire label for a [`DocType`]; every value round-trips through
/// [`DocType::parse`].
fn canonical_label(doc: DocType) -> &'static str {
    match doc {
        DocType::Ktp => "ktp",
        DocType::Kk => "kk",
        DocType::Npwp => "npwp",
        DocType::Passport => "passport",
        DocType::LandCertificate => "land_certificate",
        DocType::Deed => "deed",
        DocType::Contract => "contract",
        DocType::GeneralPdf => "general",
    }
}

#[derive(Debug, Clone)]
struct ResolvedProvider {
    host: String,
    port: u16,
    base_path: String,
    api_key: String,
    timeout: std::time::Duration,
}

fn resolve_config(config: &OcrProviderConfig) -> Result<ResolvedProvider, OcrError> {
    let endpoint = std::env::var(&config.endpoint_env)
        .unwrap_or_default()
        .trim()
        .to_owned();
    if endpoint.is_empty() {
        return Err(OcrError::Blocked(format!(
            "env {} is unset or empty; engine activation BLOCKED pending creds",
            config.endpoint_env
        )));
    }
    let api_key = std::env::var(&config.api_key_env)
        .unwrap_or_default()
        .trim()
        .to_owned();
    if api_key.is_empty() {
        return Err(OcrError::Blocked(format!(
            "env {} is unset or empty; engine activation BLOCKED pending creds",
            config.api_key_env
        )));
    }
    let (host, port, base_path) = split_endpoint(&endpoint).ok_or_else(|| {
        OcrError::Blocked(format!(
            "endpoint in {} is not a valid host:port URL: {endpoint}",
            config.endpoint_env
        ))
    })?;
    Ok(ResolvedProvider {
        host,
        port,
        base_path,
        api_key,
        timeout: std::time::Duration::from_millis(config.timeout_ms.max(1)),
    })
}

/// Split `host:port[/base]` (with optional `http(s)://` scheme) into parts.
fn split_endpoint(raw: &str) -> Option<(String, u16, String)> {
    let mut rest = raw.trim();
    if let Some(s) = rest.strip_prefix("http://") {
        rest = s;
    }
    if let Some(s) = rest.strip_prefix("https://") {
        rest = s;
    }
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], rest[i..].trim_end_matches('/').to_owned()),
        None => (rest, String::new()),
    };
    if authority.is_empty() {
        return None;
    }
    let (host, port) = match authority.rfind(':') {
        Some(i) => {
            let host = authority[..i].to_owned();
            let port = authority[i + 1..].parse::<u16>().ok()?;
            (host, port)
        }
        None => (authority.to_owned(), 80),
    };
    if host.is_empty() {
        return None;
    }
    Some((host, port, path))
}

/// Offset just past `\r\n\r\n` in `buf`, if a full header block arrived.
fn header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|i| i + 4)
}

fn map_io(e: std::io::Error) -> OcrError {
    match e.kind() {
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => OcrError::Timeout,
        _ => OcrError::Transport(e.to_string()),
    }
}

/// One minimal HTTP/1.1 exchange: write request, parse status line, read
/// exactly `Content-Length` bytes (or drain to close when absent).
fn http_exchange(
    resolved: &ResolvedProvider,
    method: &str,
    path: &str,
    extra_headers: &[(&str, &str)],
    body: &[u8],
) -> Result<(u16, String), OcrError> {
    use std::io::{Read, Write};
    use std::net::ToSocketAddrs;
    let sock = format!("{}:{}", resolved.host, resolved.port)
        .to_socket_addrs()
        .map_err(|e| OcrError::Transport(format!("cannot resolve {}: {e}", resolved.host)))?
        .next()
        .ok_or_else(|| OcrError::Transport(format!("cannot resolve {}", resolved.host)))?;
    let mut stream =
        std::net::TcpStream::connect_timeout(&sock, resolved.timeout).map_err(map_io)?;
    stream
        .set_read_timeout(Some(resolved.timeout))
        .map_err(map_io)?;
    stream
        .set_write_timeout(Some(resolved.timeout))
        .map_err(map_io)?;

    let mut req = format!(
        "{method} {path} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\nConnection: close\r\n",
        resolved.host, resolved.api_key
    );
    for (k, v) in extra_headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
    stream.write_all(req.as_bytes()).map_err(map_io)?;
    stream.write_all(body).map_err(map_io)?;
    stream.flush().map_err(map_io)?;

    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        let n = stream.read(&mut tmp).map_err(map_io)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if header_end(&buf).is_some() {
            break;
        }
        if buf.len() > 1_048_576 {
            return Err(OcrError::Api("response headers too large".into()));
        }
    }
    let hend = header_end(&buf)
        .ok_or_else(|| OcrError::Api("malformed HTTP response: no header terminator".into()))?;
    let header_str = String::from_utf8_lossy(&buf[..hend]).into_owned();
    let status: u16 = header_str
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse().ok())
        .ok_or_else(|| OcrError::Api("malformed HTTP status line".into()))?;
    let content_len: usize = header_str
        .lines()
        .skip(1)
        .filter_map(|l| {
            let (k, v) = l.split_once(':')?;
            if k.trim().eq_ignore_ascii_case("content-length") {
                v.trim().parse().ok()
            } else {
                None
            }
        })
        .next()
        .unwrap_or(0);
    let mut body_bytes = buf[hend..].to_vec();
    if content_len > 0 {
        while body_bytes.len() < content_len {
            let n = stream.read(&mut tmp).map_err(map_io)?;
            if n == 0 {
                break;
            }
            body_bytes.extend_from_slice(&tmp[..n]);
        }
        body_bytes.truncate(content_len);
    } else {
        loop {
            match stream.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => body_bytes.extend_from_slice(&tmp[..n]),
                Err(e) => return Err(map_io(e)),
            }
        }
    }
    if !(200..300).contains(&status) {
        let snippet: String = String::from_utf8_lossy(&body_bytes)
            .chars()
            .take(300)
            .collect();
        return Err(OcrError::Api(format!("provider HTTP {status}: {snippet}")));
    }
    Ok((status, String::from_utf8_lossy(&body_bytes).into_owned()))
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
    fn adapter_rejects_garbage_bytes() {
        // b"x" is not image magic: Validation when the engine is present,
        // BlockedExternal when it is absent. Either way: honest error.
        assert!(TesseractAdapter.recognize(b"x", DocType::Ktp).is_err());
    }
    #[test]
    fn recognize_empty_is_validation_err() {
        assert!(recognize_image(&[]).is_err());
    }
    #[test]
    fn image_extension_sniffs_magic() {
        assert_eq!(
            image_extension(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0]),
            Some("png")
        );
        assert_eq!(image_extension(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("jpg"));
        assert_eq!(image_extension(b"RIFF\x00\x00\x00\x00WEBP"), Some("webp"));
        assert_eq!(image_extension(b"BMxx"), Some("bmp"));
        assert_eq!(image_extension(&[0x49, 0x49, 0x2A, 0x00]), Some("tif"));
        assert_eq!(image_extension(&[0x4D, 0x4D, 0x00, 0x2A]), Some("tif"));
        assert_eq!(image_extension(b"%PDF-1.7"), None);
        assert_eq!(image_extension(b""), None);
    }
    #[test]
    fn parse_tsv_groups_words_into_lines() {
        let tsv = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n\
             1\t1\t0\t0\t0\t0\t0\t0\t10\t10\t-1\t\n\
             5\t1\t0\t0\t1\t1\t0\t0\t5\t10\t95\tHello\n\
             5\t1\t0\t0\t1\t2\t5\t0\t5\t10\t90\tWorld\n\
             5\t1\t0\t0\t2\t1\t0\t0\t5\t10\t-100\tKTP\n";
        let fs = parse_tsv(tsv);
        assert_eq!(fs.len(), 2);
        assert_eq!(fs[0].name, "page1_line1");
        assert_eq!(fs[0].value, "Hello World");
        assert!((fs[0].confidence - 0.925).abs() < 1e-6);
        assert_eq!(fs[1].value, "KTP");
        assert_eq!(fs[1].confidence, 0.0);
    }
    #[test]
    fn parse_tsv_empty_yields_no_fields() {
        let tsv = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n";
        assert!(parse_tsv(tsv).is_empty());
    }
    /// Live-gated: on hosts with tesseract installed, a blank 1x1 PNG must
    /// run the full pipeline and yield zero fabricated fields. Without the
    /// binary the test passes vacuously (CI stays green).
    #[test]
    fn live_blank_png_yields_no_fabrication() {
        if !probe().tesseract_found {
            return;
        }
        let png = white_png_1x1();
        let fs = recognize_image(&png).expect("tesseract must run on blank png");
        assert!(
            fs.is_empty(),
            "blank image must yield no fields, got {fs:?}"
        );
    }
    /// Minimal 1x1 white grayscale PNG, CRC/Adler computed inline so the
    /// fixture needs no external asset.
    fn white_png_1x1() -> Vec<u8> {
        fn crc(data: &[u8]) -> u32 {
            let mut c = 0xFFFF_FFFFu32;
            for &b in data {
                c ^= b as u32;
                for _ in 0..8 {
                    c = if c & 1 == 1 {
                        0xEDB8_8320 ^ (c >> 1)
                    } else {
                        c >> 1
                    };
                }
            }
            !c
        }
        fn chunk(out: &mut Vec<u8>, tag: &[u8; 4], body: &[u8]) {
            out.extend_from_slice(&(body.len() as u32).to_be_bytes());
            out.extend_from_slice(tag);
            out.extend_from_slice(body);
            let mut mac = tag.to_vec();
            mac.extend_from_slice(body);
            out.extend_from_slice(&crc(&mac).to_be_bytes());
        }
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        chunk(&mut png, b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 0, 0, 0, 0]);
        chunk(
            &mut png,
            b"IDAT",
            &[
                0x78, 0x01, 0x01, 0x02, 0x00, 0xFD, 0xFF, 0x00, 0xFF, 0x01, 0x01, 0x01, 0x00,
            ],
        );
        chunk(&mut png, b"IEND", &[]);
        png
    }
    #[test]
    fn probe_returns_struct_with_nonempty_detail() {
        let p = probe();
        assert!(!p.detail.is_empty());
        assert!(p.detail.contains("tesseract_on_path="));
        assert!(p.detail.contains("TESSDATA_PREFIX="));
        // This host has no tesseract binary (verified via Get-Command);
        // if a future host installs one, the flag flips honestly.
        if std::env::var("TESSDATA_PREFIX").is_err() {
            assert!(p.detail.contains("unset"));
        }
    }
    #[test]
    fn blocked_reason_carries_probe_evidence() {
        let msg = blocked_reason();
        assert!(msg.contains("libtesseract"));
        assert!(msg.contains("tesseract_on_path="));
    }
    #[test]
    fn validate_extracted_still_honest_alongside_probe() {
        let _ = probe();
        let problems = validate_extracted(DocType::Ktp, &[]);
        assert!(problems.iter().any(|p| p == "missing:nik"));
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

/// Stub-tested cloud fallback client. The local TCP stub returns fixed
/// fixture strings; tests assert plumbing only (job-id passthrough, text
/// passthrough, error mapping) and never fabricate field values.
#[cfg(test)]
mod cloud_tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpListener};
    use std::time::Duration;

    /// Minimal executor: adapter futures never yield, so one poll loop
    /// without external deps is enough.
    fn block_on<F: std::future::Future>(mut fut: F) -> F::Output {
        use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
        unsafe fn clone_waker(_: *const ()) -> RawWaker {
            RawWaker::new(
                std::ptr::null(),
                &RawWakerVTable::new(clone_waker, noop, noop, noop),
            )
        }
        unsafe fn noop(_: *const ()) {}
        let waker = unsafe {
            Waker::from_raw(RawWaker::new(
                std::ptr::null(),
                &RawWakerVTable::new(clone_waker, noop, noop, noop),
            ))
        };
        let mut cx = Context::from_waker(&waker);
        let mut fut = unsafe { std::pin::Pin::new_unchecked(&mut fut) };
        loop {
            match fut.as_mut().poll(&mut cx) {
                Poll::Ready(v) => return v,
                Poll::Pending => std::thread::yield_now(),
            }
        }
    }

    type Observed = (String, Vec<(String, String)>, Vec<u8>);

    fn read_stub_request(stream: &mut std::net::TcpStream) -> Observed {
        let mut buf = Vec::new();
        let mut tmp = [0u8; 4096];
        loop {
            match stream.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => {
                    buf.extend_from_slice(&tmp[..n]);
                    if header_end(&buf).is_some() {
                        break;
                    }
                }
                Err(_) => break,
            }
            if buf.len() > 1_048_576 {
                break;
            }
        }
        let hend = header_end(&buf).unwrap_or(buf.len());
        let head = String::from_utf8_lossy(&buf[..hend]).into_owned();
        let mut lines = head.lines();
        let line = lines.next().unwrap_or("").to_owned();
        let mut headers = Vec::new();
        let mut content_len = 0usize;
        for l in lines {
            if let Some((k, v)) = l.split_once(':') {
                if k.trim().eq_ignore_ascii_case("content-length") {
                    content_len = v.trim().parse().unwrap_or(0);
                }
                headers.push((k.trim().to_lowercase(), v.trim().to_owned()));
            }
        }
        let mut body = buf[hend.min(buf.len())..].to_vec();
        while body.len() < content_len {
            match stream.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => body.extend_from_slice(&tmp[..n]),
                Err(_) => break,
            }
        }
        body.truncate(content_len);
        (line, headers, body)
    }

    fn write_stub_response(stream: &mut std::net::TcpStream, status: u16, body: &str) {
        let reason = match status {
            200 => "OK",
            500 => "Internal Server Error",
            _ => "Error",
        };
        let resp = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(resp.as_bytes()).ok();
    }

    /// Spawn a single-route stub. `handler` sees (request_line, headers,
    /// body) and returns (status, body). `delay` postpones the response to
    /// exercise client timeouts. Unique env-var names per test keep
    /// parallel tests isolated.
    fn spawn_stub<F>(handler: F, max_conn: usize, delay: Duration) -> SocketAddr
    where
        F: Fn(String, Vec<(String, String)>, Vec<u8>) -> (u16, String) + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub");
        let addr = listener.local_addr().expect("stub addr");
        std::thread::spawn(move || {
            for _ in 0..max_conn {
                let Ok((mut s, _)) = listener.accept() else {
                    return;
                };
                s.set_read_timeout(Some(Duration::from_secs(5))).ok();
                let (line, headers, body) = read_stub_request(&mut s);
                if !delay.is_zero() {
                    std::thread::sleep(delay);
                }
                let (st, text) = handler(line, headers, body);
                write_stub_response(&mut s, st, &text);
            }
        });
        addr
    }

    fn header_value(headers: &[(String, String)], name: &str) -> String {
        headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    }

    /// Point a config at `addr` (or nowhere) using test-unique env names.
    fn test_config(tag: &str, addr: Option<SocketAddr>, timeout_ms: u64) -> OcrProviderConfig {
        let ep = format!("LEGALRUST_TEST_OCR_EP_{tag}");
        let key = format!("LEGALRUST_TEST_OCR_KEY_{tag}");
        std::env::remove_var(&ep);
        std::env::remove_var(&key);
        if let Some(a) = addr {
            std::env::set_var(&ep, format!("http://{a}"));
        }
        std::env::set_var(&key, "test-key");
        OcrProviderConfig::new(ep, key, timeout_ms)
    }

    #[test]
    fn cloud_submit_ok_returns_job_id_and_uploads_bytes() {
        let (tx, rx) = std::sync::mpsc::channel::<Observed>();
        let addr = spawn_stub(
            move |_line, headers, body| {
                tx.send((String::new(), headers, body)).ok();
                (200, "job-123".to_string())
            },
            1,
            Duration::ZERO,
        );
        let adapter = CloudOcrAdapter::new(test_config("SUBMIT_OK", Some(addr), 2000));
        let job =
            block_on(adapter.submit(b"raw-image-bytes", "land-certificate")).expect("submit ok");
        assert_eq!(job.as_str(), "job-123");
        let (_, headers, body) = rx.recv_timeout(Duration::from_secs(5)).expect("observed");
        assert_eq!(body, b"raw-image-bytes");
        assert_eq!(header_value(&headers, "x-doc-type"), "land_certificate");
    }

    #[test]
    fn cloud_fetch_ok_returns_fixture_text_verbatim() {
        const FIXTURE: &str = "FIXTURE OCR TEXT - plumbing only";
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let addr = spawn_stub(
            move |line, _, _| {
                tx.send(line).ok();
                (200, FIXTURE.to_string())
            },
            1,
            Duration::ZERO,
        );
        let adapter = CloudOcrAdapter::new(test_config("FETCH_OK", Some(addr), 2000));
        let text = block_on(adapter.fetch_text(&CloudJobId("job-123".into()))).expect("fetch ok");
        assert_eq!(text, FIXTURE);
        let line = rx.recv_timeout(Duration::from_secs(5)).expect("observed");
        assert!(line.starts_with("GET /fetch/job-123"), "got {line}");
    }

    #[test]
    fn cloud_500_maps_to_api_error() {
        let addr = spawn_stub(|_, _, _| (500, "boom".to_string()), 1, Duration::ZERO);
        let adapter = CloudOcrAdapter::new(test_config("ERR500", Some(addr), 2000));
        let err = block_on(adapter.submit(b"x", "ktp")).expect_err("must fail");
        match err {
            OcrError::Api(msg) => assert!(msg.contains("500"), "got {msg}"),
            other => panic!("expected Api, got {other:?}"),
        }
    }

    #[test]
    fn cloud_slow_provider_maps_to_timeout() {
        let addr = spawn_stub(
            |_, _, _| (200, "job-1".to_string()),
            1,
            Duration::from_millis(600),
        );
        let adapter = CloudOcrAdapter::new(test_config("TIMEOUT", Some(addr), 100));
        let err = block_on(adapter.submit(b"x", "ktp")).expect_err("must time out");
        assert!(matches!(err, OcrError::Timeout), "got {err:?}");
    }

    #[test]
    fn cloud_unset_env_is_blocked() {
        let ep = "LEGALRUST_TEST_OCR_EP_UNSET_XYZ";
        let key = "LEGALRUST_TEST_OCR_KEY_UNSET_XYZ";
        std::env::remove_var(ep);
        std::env::remove_var(key);
        let adapter = CloudOcrAdapter::new(OcrProviderConfig::new(ep, key, 1000));
        let err = block_on(adapter.submit(b"x", "ktp")).expect_err("must block");
        assert!(matches!(err, OcrError::Blocked(_)), "got {err:?}");
        let err =
            block_on(adapter.fetch_text(&CloudJobId("job-1".into()))).expect_err("must block");
        assert!(matches!(err, OcrError::Blocked(_)), "got {err:?}");
    }

    #[test]
    fn cloud_unsupported_type_rejected_before_io() {
        // Unroutable endpoint proves the allowlist fires before any network.
        let adapter = CloudOcrAdapter::new(test_config("BADTYPE", None, 1000));
        std::env::set_var(&adapter.config.endpoint_env, "http://127.0.0.1:1");
        let err = block_on(adapter.submit(b"x", "nope")).expect_err("must reject");
        assert!(matches!(err, OcrError::UnsupportedType(_)), "got {err:?}");
    }

    #[test]
    fn cloud_missing_api_key_is_blocked() {
        let addr = spawn_stub(|_, _, _| (200, "job-1".to_string()), 1, Duration::ZERO);
        let cfg = test_config("NOKEY", Some(addr), 2000);
        std::env::remove_var(&cfg.api_key_env);
        let adapter = CloudOcrAdapter::new(cfg);
        let err = block_on(adapter.submit(b"x", "ktp")).expect_err("must block");
        assert!(matches!(err, OcrError::Blocked(_)), "got {err:?}");
    }
}
