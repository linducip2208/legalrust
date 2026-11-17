# PROD Runbook

Production topology: `server` + `worker` + `mysql:8.4` + `redis:7-alpine`
via `docker-compose.prod.yml` (additive overlay — `docker-compose.yml` stays
dev-only). Images: `deploy/Dockerfile.server`, `deploy/Dockerfile.worker`.
No secrets live in any file: everything secret arrives via `.env.prod`
(copied from `.env.prod.example`, values from vault).

## 1. Build / push / run

```bash
cp .env.prod.example .env.prod   # fill every CHANGE-ME from the vault; never commit .env.prod

# build (rust:1.99-bookworm builder, debian:bookworm-slim runtime, non-root appuser)
docker compose -f docker-compose.prod.yml --env-file .env.prod build

# tag + push (pin every release; IMAGE_TAG is the rollback handle)
docker tag local/legalrust-server:$IMAGE_TAG $REGISTRY/legalrust-server:$IMAGE_TAG
docker tag local/legalrust-worker:$IMAGE_TAG $REGISTRY/legalrust-worker:$IMAGE_TAG
docker push $REGISTRY/legalrust-server:$IMAGE_TAG
docker push $REGISTRY/legalrust-worker:$IMAGE_TAG

# run
docker compose -f docker-compose.prod.yml --env-file .env.prod up -d
docker compose -f docker-compose.prod.yml --env-file .env.prod ps

# health gates (server: 8080 -> /healthz liveness, /readyz readiness incl. DB check)
curl -fsS http://127.0.0.1:${HOST_PORT:-8080}/healthz
curl -fsS http://127.0.0.1:${HOST_PORT:-8080}/readyz
```

## 2. First-boot migration behavior (auto-migrate on connect)

No manual migration step on first boot. `apps/server/src/main.rs` calls
`db::connect(&config.database_url)` with a 3 s timeout, then `db::migrate`
(sqlx forward-only migrations) on success. Verify after first boot:

```bash
docker compose -f docker-compose.prod.yml --env-file .env.prod logs server | grep -i -E "migrat|database connected"
curl -fsS http://127.0.0.1:${HOST_PORT:-8080}/readyz   # {"ready":true,"db":true}
```

Failure modes (forward-only notes, never destructive):

- `migration failed: …` in server logs → migration error is logged, process
  keeps running. Fix forward (new migration), redeploy; never edit an
  applied migration.
- `database unreachable; ephemeral dev store active` → DB never connected
  (wrong `DATABASE_URL`, mysql still starting). `/readyz` returns
  `BlockedExternal`. Do NOT accept this in prod: fix connectivity, restart
  server. The ephemeral memory store is dev-only.
- Live-DB drill (pre-prod verification, env-only creds):
  `DATABASE_URL=… ./scripts/migrate-live.sh` — see `scripts/migrate-live.sh`.

## 3. Backup / restore

Backups run out-of-band (cron/systemd timer on the host), not as a compose
service. Pointer: `scripts/backup.sh` + `docs/16-disaster-recovery/DR-RUNBOOK.md`.

```bash
# one-shot encrypted backup (env-only creds; retention 7 daily, S3 lifecycle + local prune)
DB_HOST=... DB_USER=... DB_PASSWORD=... S3_ENDPOINT=... S3_BUCKET=... \
  GPG_RECIPIENT=... BACKUP_DIR=/var/backups/legalrust ./scripts/backup.sh

# restore drill (monthly, required — stale backups are treated as no backups)
./scripts/dr-drill.sh   # see docs/16-disaster-recovery/DR-RUNBOOK.md
```

## 4. Rollback (previous image tag)

Releases are immutable tags; rollback = redeploy the previous tag.
Keep the last known-good `$IMAGE_TAG` recorded per release.

```bash
IMAGE_TAG=<previous-good-tag> docker compose -f docker-compose.prod.yml \
  --env-file .env.prod up -d server worker
curl -fsS http://127.0.0.1:${HOST_PORT:-8080}/readyz
```

Migrations are forward-only: rollback never downgrades the schema. If the bad
release shipped a migration, roll the code back AND ship a compensating
forward migration — never `DROP`/rewrite applied migrations.

## 5. Log locations

```bash
# live tail
docker compose -f docker-compose.prod.yml --env-file .env.prod logs -f server worker

# last-boot diagnostics (migration + readiness)
docker compose -f docker-compose.prod.yml --env-file .env.prod logs server | grep -i -E "migrat|ready|error|warn"
docker compose -f docker-compose.prod.yml --env-file .env.prod logs worker
docker compose -f docker-compose.prod.yml --env-file .env.prod logs mysql | tail -50

# json log driver ships to the host journal when configured; app tracing honors RUST_LOG
RUST_LOG=info docker compose -f docker-compose.prod.yml --env-file .env.prod up -d server
```
