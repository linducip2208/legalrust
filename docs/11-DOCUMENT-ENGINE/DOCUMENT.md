# Document Engine (planning)
Lifecycle: ingest (upload/scan/email/mobile) -> AV-scan -> hash (SHA-256) -> classify ->
OCR-queue -> metadata -> version -> workflow/approval -> sign (guardrailed) -> seal ->
retain -> legal-hold aware purge. Versioning major/minor with diff; check-in/out locks;
watermark/stamp/redaction; matter-bundle export with manifest; S3/MinIO lifecycle tiers;
trash-with-restore; integrity re-hash jobs; protocol segregation for notarial originals.
Metadata schema + custom fields; retention engine vs holds; storage quotas; large-file chunking.
