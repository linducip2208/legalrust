#!/bin/bash
# DR drill: backup -> restore to staging -> verify -> report.
# No secrets in this file; all credentials via env (STAGE_DB_*, DB_*).
# Usage: STAGE_DB_HOST=... STAGE_DB_USER=... STAGE_DB_PASSWORD=... ./scripts/dr-drill.sh
set -euo pipefail

STEP_OK=()
STEP_FAIL=()
mark_ok() { STEP_OK+=("$1"); echo "OK: $1"; }
mark_fail() { STEP_FAIL+=("$1"); echo "FAIL: $1"; }

echo "=== DR drill start $(date -u +%FT%TZ) ==="

# 1. Backup (delegates to backup.sh)
echo "--- step 1: backup ---"
if ./scripts/backup.sh; then mark_ok "backup"; else mark_fail "backup"; fi

# 2. Restore latest backup to staging (decrypt + mysql import)
echo "--- step 2: restore to staging ---"
: "${STAGE_DB_HOST:?set STAGE_DB_HOST}"
: "${STAGE_DB_USER:?set STAGE_DB_USER}"
: "${STAGE_DB_PASSWORD:?set STAGE_DB_PASSWORD}"
OUTDIR="${BACKUP_DIR:-/var/backups/legalrust}"
LATEST=$(ls -1t "$OUTDIR"/legalrust-*.sql.gpg 2>/dev/null | head -n 1 || true)
if [ -z "${LATEST:-}" ]; then
  mark_fail "restore (no local backup found)"
else
  TMP_RESTORE=$(mktemp /tmp/legalrust-drill-XXXX.sql)
  trap 'rm -f "$TMP_RESTORE"' EXIT
  if gpg --decrypt -o "$TMP_RESTORE" "$LATEST" \
    && mysql -h "$STAGE_DB_HOST" -u "$STAGE_DB_USER" -p"$STAGE_DB_PASSWORD" legalrust < "$TMP_RESTORE"; then
    mark_ok "restore ($LATEST -> staging)"
  else
    mark_fail "restore"
  fi
  rm -f "$TMP_RESTORE"
  trap - EXIT
fi

# 3. Verify: SELECT 1 + row counts on staging
echo "--- step 3: verify staging ---"
if mysql -h "$STAGE_DB_HOST" -u "$STAGE_DB_USER" -p"$STAGE_DB_PASSWORD" -e "SELECT 1;" legalrust; then
  mark_ok "verify SELECT 1"
else
  mark_fail "verify SELECT 1"
fi
for T in users orgs cases documents invoices; do
  if mysql -h "$STAGE_DB_HOST" -u "$STAGE_DB_USER" -p"$STAGE_DB_PASSWORD" -N -e "SELECT COUNT(*) FROM \`$T\`;" legalrust 2>/dev/null; then
    echo "rows $T: counted"
    mark_ok "verify row-count $T"
  else
    echo "rows $T: table missing/unreadable (non-fatal, recorded)"
    mark_fail "verify row-count $T"
  fi
done

# 4. Report
echo "=== DR drill report ==="
echo "passed: ${#STEP_OK[@]} failed: ${#STEP_FAIL[@]}"
printf '  PASS: %s\n' "${STEP_OK[@]:-none}"
if [ "${#STEP_FAIL[@]}" -gt 0 ]; then
  printf '  FAIL: %s\n' "${STEP_FAIL[@]}"
  echo "RESULT: DR-DRILL FAILED"
  exit 1
fi
echo "RESULT: DR-DRILL PASSED"
