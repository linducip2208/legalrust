# DR Runbook — legalrust

Status: documented, live drill pending deployment (BLOCKED on staging env until first monthly drill passes).

## Objectives
- RPO 24h (max 24h data loss; daily encrypted backups via `scripts/backup.sh`).
- RTO 4h (service restored within 4h of disaster declaration).

## Roles
| Role | Responsibility |
|------|----------------|
| Incident Commander (IC) | Declares disaster, coordinates, communicates status. |
| DBA / Backend on-call | Runs restore, verifies integrity, replays binlog. |
| Security officer | Approves GPG key use, audits access, rotates secrets post-incident. |
| Comms | Notifies customers/regulator if PDP-relevant breach. |

## Preconditions
- Backups: daily `mysqldump --single-transaction` + GPG + S3 (`scripts/backup.sh`), sha256 sidecars, 7-day retention (S3 lifecycle + local prune). No secrets in repo; keys via vault/env.
- Migrations are forward-only: restore = replay backup then `sqlx migrate run` forward; never roll back destructively.
- Binlog enabled on MySQL for point-in-time recovery within RPO window.

## Restore steps (staging first, then prod)
1. Pick restore point: latest `legalrust-<TS>.sql.gpg` in `s3://<bucket>/backups/` + matching `.sha256`. Verify `sha256sum -c`.
2. Dry-run on staging: `./scripts/dr-drill.sh` with `STAGE_DB_*` env (decrypt → mysql import → `SELECT 1` + row counts). Drill FAIL blocks prod restore.
3. Prod restore:
   ```sh
   aws --endpoint-url "$S3_ENDPOINT" s3 cp "s3://$S3_BUCKET/backups/legalrust-<TS>.sql.gpg" /tmp/restore.sql.gpg
   sha256sum -c restore.sql.gpg.sha256
   gpg --decrypt -o /tmp/restore.sql /tmp/restore.sql.gpg
   mysql -h "$DB_HOST" -u root -p legalrust < /tmp/restore.sql
   sqlx migrate run   # forward-only
   ```
4. Verify: `/healthz` + `/readyz` return 200; `SELECT 1`; row counts vs drill report; smoke login + case list.
5. Re-enable traffic (LB), rotate DB creds/JWT_SECRET if compromise suspected, monitor error rate 60 min.

## RPO/RTO tracking
- Record backup timestamp vs incident time (RPO actual) and declare-to-healthy time (RTO actual) in drill report.
- Monthly drill required; file report under `docs/13-development-progress/DAILY-LOG.md`.

## Known limits (honest)
- No live restore verified yet → status BLOCKED until first successful `dr-drill.sh` run on staging.
- Binlog PITR replay steps depend on operator's MySQL config; validate during first drill.
