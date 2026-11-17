#!/bin/bash
# bench.sh — k6-less latency probe for /healthz (+ optional path).
# Usage: BASE_URL=http://127.0.0.1:8080 REQUESTS=100 ./scripts/bench.sh
# For heavier loads use wrk, e.g.: wrk -t4 -c50 -d30s "$BASE_URL/healthz"
set -euo pipefail
BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
PATH_P="${PATH_P:-/healthz}"
REQUESTS="${REQUESTS:-100}"
TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT
echo "probing $BASE_URL$PATH_P x$REQUESTS ..."
i=0
while [ "$i" -lt "$REQUESTS" ]; do
  curl -s -o /dev/null -w "%{time_total}\n" "$BASE_URL$PATH_P" >> "$TMP" || echo "ERR" >> "$TMP"
  i=$((i + 1))
done
grep -v ERR "$TMP" | sort -n | awk -v n="$REQUESTS" '
  { a[NR]=$1; sum+=$1 }
  END {
    if (NR==0) { print "no successful samples"; exit 1 }
    p50=a[int(NR*0.50)+1]; p95=a[int(NR*0.95)+1]; p99=a[int(NR*0.99)+1];
    printf "n=%d ok=%d err=%d\n", n, NR, n-NR
    printf "min=%.3fs avg=%.3fs p50=%.3fs p95=%.3fs p99=%.3fs max=%.3fs\n", a[1], sum/NR, p50, p95, p99, a[NR]
  }'
echo "hint: wrk -t4 -c50 -d30s $BASE_URL$PATH_P"
