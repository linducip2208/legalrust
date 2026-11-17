# CORPUS SPEC v1.0 (planning)
Schema: corpus/{doctype}/{stratum}/{sample_id}/{image, gt.json, meta.json}. gt.json: full text + field spans + boxes.
meta.json: capture device, dpi, lighting, rotation, PII-synthetic flag (real citizen data NEVER in corpus without consent;
prefer synthetic ID layouts mirroring field geometry). Versioning: manifest.csv with SHA-256 per file + corpus semver.
Size target v1.0: 8 doctypes × 4 strata × 200 = 6,400 pages. Custodian: OCR researcher. License/consent log per sample.
