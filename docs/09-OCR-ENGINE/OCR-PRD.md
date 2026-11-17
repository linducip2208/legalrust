# OCR & Document Intelligence PRD (planning)
## Scope
KTP/KK/NPWP/passport/property-cert/deed/contract/general-PDF; multi-page; tables; layout;
classification; enhancement (deskew/denoise/perspective/blur gate); confidence; field validation;
HITL review; versioning; archival. PII masked in logs.
## Engine strategy (trait-routed; benchmark before commit)
- **ocrs** (pure Rust, ONNX runtime dep): best for line-level Latin text; needs detection tuning for ID cards.
- **oar-ocr**: archival-oriented pipeline pieces; evaluate vs build cost.
- **Tesseract bindings** (tesseral/teeseract crates): strongest ID-doc accuracy baseline but
  brings **native libtesseract + traineddata** (NOT pure Rust) — document as external dependency.
- Cloud/local vision models via AI router for low-confidence fallback only (cost/PDP controlled).
Decision: router interface `OcrEngine::recognize(image)->lines+conf`; per-doc-type engine profiles;
winner chosen per doc type on measured CER/WER/field-accuracy, not assumption.
## Accuracy methodology
CER = edit-dist/ground-truth chars; WER word-level; field-level F1 on NIK/names/dates/amounts;
classification accuracy; p95 latency; human-correction rate. Corpus: stratified Indonesian samples
(good/flat-lay/glare/rotated/low-res), ground-truth double-keyed, versioned (see 23-RESEARCH).
Gates (planning targets, to confirm): printed KTP field-acc >= 97%, CER <= 2% clean scans;
contracts clause-seg F1 >= 0.85; correction-rate dashboard tracked per engine/profile.
## Pipeline
ingest -> QC gate -> enhance -> classify -> route engine -> extract -> validate -> confidence gate
(auto-pass >= T_high, review band, reject <= T_low with guidance) -> HITL -> version -> archive.
Retry/DLQ; cost metering per job.
