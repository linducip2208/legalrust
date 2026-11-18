# BLOCKERS
- BL-DB-01: CLEARED 2026-11-17 — live MySQL 8.4.9 (Laragon) found; migrations 0001–0009 applied to `legalrust_drill`; `live_mysql_register_and_matter` PASS; DR restore drill to `legalrust_staging` verified (counts match). Follow-up: `.env` DATABASE_URL password does not match local server (root/empty) — align before shared runs.
- BL-EXT-01: PSrE/AI-remote/BPN/Dukcapil creds -> adapters interface-complete, activation BLOCKED. CLEARED dev-live: OCR native (tesseract 5.4.0 + ind/eng/osd; prod image bundles it), AI lokal (Ollama qwen2.5:1.5b + env AI_*), Tripay wired (needs merchant TRIPAY_API_KEY/PRIVATE_KEY/MERCHANT_CODE, sandbox first).
- BL-LIC-01: LICENSE file withheld pending owner licensing intent (MIT vs proprietary).
