# Deploy (Linux)
1. Provision MySQL 8.4, Redis, S3-compatible storage. 2. Set env (see .env.example; JWT_SECRET from vault).
3. Run migrations (sqlx). 4. Deploy server + worker binaries (systemd/k8s). 5. TLS terminate at LB, AES-256 at rest, encrypted backups with restore drills.
