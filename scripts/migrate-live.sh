#!/usr/bin/env bash
# migrate-live.sh — live-DB migration drill helper (BL-DB-01).
# Env-only credentials. Never hardcode passwords in this file.
#
# Required env:
#   DATABASE_URL   full MySQL URL, e.g. mysql://root@127.0.0.1:3306/legalrust_drill
#                  (empty-password root works with Laragon defaults; otherwise
#                  put the password in the URL — never commit it, never echo it)
# Optional env:
#   MYSQL_BIN      dir containing mysql.exe/mysqladmin (Laragon), default below
#   CARGO_TARGET_DIR isolated build dir, default target-dblive
#
# Steps: mysqladmin ping -> CREATE DATABASE IF NOT EXISTS <db from URL> ->
# cargo test -p legalrust-server --test api_db -- --ignored (runs sqlx migrate
# 0001-0009 + live register flow) -> row-count sanity.
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL must be set in the environment (never commit it)}"
MYSQL_BIN="${MYSQL_BIN:-/d/laragon/bin/mysql/mysql-8.4.3-winx64/bin}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target-dblive}"

HOST="$(printf '%s' "$DATABASE_URL" | sed -E 's#^mysql://([^:/?#@]+)(:([^@/?#]*))?@([^:/?#]+)(:([0-9]+))?/([^?#]+).*#\4#')"
PORT="$(printf '%s' "$DATABASE_URL" | sed -E 's#^mysql://([^:/?#@]+)(:([^@/?#]*))?@([^:/?#]+)(:([0-9]+))?/([^?#]+).*#\6#')"
USER="$(printf '%s' "$DATABASE_URL" | sed -E 's#^mysql://([^:/?#@]+)(:([^@/?#]*))?@.*#\1#')"
DB="$(printf '%s' "$DATABASE_URL" | sed -E 's#^mysql://([^:/?#@]+)(:([^@/?#]*))?@([^:/?#]+)(:([0-9]+))?/([^?#]+).*#\7#')"
PW="$(printf '%s' "$DATABASE_URL" | sed -E 's#^mysql://[^:/?#@]+:([^@/?#]*)@.*#\1#;t;d')"
PORT="${PORT:-3306}"

echo "host=$HOST port=$PORT user=$USER db=$DB password=<redacted>"
export MYSQL_PWD="$PW"

"$MYSQL_BIN/mysqladmin" --host "$HOST" --port "$PORT" --user "$USER" ping
"$MYSQL_BIN/mysql" --host "$HOST" --port "$PORT" --user "$USER" \
  -e "CREATE DATABASE IF NOT EXISTS \`$DB\` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;"

export DATABASE_URL
cargo test -p legalrust-server --test api_db -- --ignored --nocapture

"$MYSQL_BIN/mysql" --host "$HOST" --port "$PORT" --user "$USER" "$DB" \
  -e "SELECT version FROM _sqlx_migrations ORDER BY version; SELECT (SELECT COUNT(*) FROM organizations) AS organizations, (SELECT COUNT(*) FROM users) AS users, (SELECT COUNT(*) FROM roles) AS roles, (SELECT COUNT(*) FROM permissions) AS permissions, (SELECT COUNT(*) FROM sessions) AS sessions;"

MYSQL_PWD='' export MYSQL_PWD
echo "LIVE-RUN OK (password cleared from env)"
