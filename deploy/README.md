# Deploy (Linux)
1. Provision MySQL 8.4, Redis, S3-compatible storage. 2. Set env (see .env.example; JWT_SECRET from vault).
3. Run migrations (sqlx). 4. Deploy server + worker binaries (systemd/k8s). 5. TLS terminate at LB, AES-256 at rest, encrypted backups with restore drills.
6. OCR: `deploy/Dockerfile.server` runtime already bundles `tesseract-ocr{,-ind,-eng}` — for non-Docker hosts, install the same packages or the process endpoint returns 501. 7. AI: run Ollama (`ollama serve` + model pull) on the box or set `AI_BASE_URL/AI_MODEL(/AI_API_KEY)`; otherwise chat returns 501. 8. Tripay: fill `TRIPAY_*` (sandbox first) and register the callback URL in the Tripay panel.
