# OCR BENCHMARK PROTOCOL (specification only — no scores fabricated, execution is blocker B-002)
## Corpus (CORPUS_SPEC.md)
Stratified Indonesian samples per doc type (KTP/KK/NPWP/passport/sertifikat/akta/kontrak/mixed-PDF):
each stratum n≥200 pages across capture modes (flatbed scan 300dpi, phone photo good/glare/rotated/low-light),
ground truth double-keyed + adjudicated, versioned (corpus v1.0 pins image hashes). Re-run gate: any engine/profile change.
## Metrics
CER = edit-distance/ground-truth chars; WER word-level; field-level F1 on NIK/names/dates/amounts/NOP;
layout/table F1 (cell adjacency); classification accuracy; p95 latency/page; human-correction rate; cost/page.
Report per stratum + macro average; confidence intervals (bootstrap, 95%).
## Procedure
1) freeze engine+profile+preprocessing config hash; 2) run harness (spec: container image, GPU/CPU pin, batch=1 and batch=8);
3) score with reference scorer (spec in repo-planning, implemented at B-002); 4) publish report card per engine×profile;
5) promotion gate: KTP printed field-acc ≥97% AND CER ≤2% clean-scan, else HITL-band only.
## Engines under test
ocrs (pure Rust + ONNX dep), oar-ocr pipeline parts, Tesseract bindings (native libtesseract declared), AI-vision fallback.
Winner chosen PER DOC TYPE on measured numbers only.
