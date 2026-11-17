#!/bin/sh
# Encrypted MySQL backup to S3-compatible storage. Configure: DB_*, S3_*, GPG_RECIPIENT.
set -eu
TS=$(date -u +%Y%m%dT%H%M%SZ)
mysqldump -h "${DB_HOST:-127.0.0.1}" -u "${DB_USER:-root}" -p"${DB_PASSWORD:?set DB_PASSWORD}" --single-transaction --routines legalrust   | gpg --encrypt --recipient "${GPG_RECIPIENT:?set GPG_RECIPIENT}" --trust-model always   | aws --endpoint-url "${S3_ENDPOINT:?set S3_ENDPOINT}" s3 cp - "s3://${S3_BUCKET:?set S3_BUCKET}/backups/legalrust-$TS.sql.gpg"
echo "backup uploaded: legalrust-$TS.sql.gpg (restore drill required monthly)"
