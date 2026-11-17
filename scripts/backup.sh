#!/bin/sh
# Encrypted MySQL backup to S3-compatible storage.
# Configure via env: DB_HOST, DB_USER, DB_PASSWORD, S3_ENDPOINT, S3_BUCKET, GPG_RECIPIENT.
# Retention: 7 daily backups (enforced by S3 lifecycle rule + local prune below). No secrets in this file.
set -eu
TS=$(date -u +%Y%m%dT%H%M%SZ)
OUTDIR="${BACKUP_DIR:-/var/backups/legalrust}"
mkdir -p "$OUTDIR"
FNAME="legalrust-$TS.sql"
SQL_DUMP="$OUTDIR/$FNAME"
mysqldump -h "${DB_HOST:-127.0.0.1}" -u "${DB_USER:-root}" -p"${DB_PASSWORD:?set DB_PASSWORD}" \
  --single-transaction --routines --triggers legalrust > "$SQL_DUMP"
sha256sum "$SQL_DUMP" > "$SQL_DUMP.sha256"
sha256sum -c "$SQL_DUMP.sha256"
gpg --encrypt --recipient "${GPG_RECIPIENT:?set GPG_RECIPIENT}" --trust-model always \
  -o "$SQL_DUMP.gpg" "$SQL_DUMP"
sha256sum "$SQL_DUMP.gpg" > "$SQL_DUMP.gpg.sha256"
aws --endpoint-url "${S3_ENDPOINT:?set S3_ENDPOINT}" s3 cp "$SQL_DUMP.gpg" \
  "s3://${S3_BUCKET:?set S3_BUCKET}/backups/$FNAME.gpg"
aws --endpoint-url "${S3_ENDPOINT}" s3 cp "$SQL_DUMP.gpg.sha256" \
  "s3://${S3_BUCKET}/backups/$FNAME.gpg.sha256"
# Local retention: keep last 7 dumps (forward-only; remote retention via S3 lifecycle = 7 days).
ls -1t "$OUTDIR"/legalrust-*.sql.gpg 2>/dev/null | tail -n +8 | xargs -r rm -f
ls -1t "$OUTDIR"/legalrust-*.sql 2>/dev/null | tail -n +8 | xargs -r rm -f
ls -1t "$OUTDIR"/legalrust-*.sha256 2>/dev/null | tail -n +8 | xargs -r rm -f
echo "backup uploaded: $FNAME.gpg (+ .sha256) retention=7d (restore drill required monthly)"
