# Benchmarks — protocol (no measured numbers yet)

Status: protocol only. No production-like measurements taken; any numbers claimed elsewhere must be treated as BLOCKED/unverified.

## Method: `scripts/bench.sh`
- k6-less: `BASE_URL=... REQUESTS=100 ./scripts/bench.sh` curl latency loop for a path (default `/healthz`), prints min/avg/p50/p95/p99/max via awk.
- Heavy loads: `wrk -t4 -c50 -d30s $BASE_URL/healthz` (hint printed by script).

## Endpoint matrix
| Endpoint | Method | Auth | Notes |
|----------|--------|------|-------|
| /healthz | GET | no | liveness |
| /readyz | GET | no | readiness (DB ping) |
| /api/cases | GET | yes | list (paged) |
| /api/documents | GET | yes | list (paged) |
| /api/ocr/jobs | GET | yes | list |
| /api/ai/chat | POST | yes | expected 501 (blocked adapter) — measure error-path latency only |

## Concurrency levels
- 1 (baseline), 10 (normal), 50 (stress). REQUESTS=100 minimum per cell; wrk 30s for stress.

## Acceptance thresholds (targets, not results)
- `GET /healthz` p95 < 500ms at c=10.
- `GET /api/*` list p95 < 800ms at c=10, seeded DB 1k rows.
- Error rate < 1% at c=10; no OOM (container within 1Gi limit).

## Reporting template
- Date, git sha, env (CPU/RAM/MySQL), seed size, per-cell p50/p95/max + verdict PASS/FAIL.
- File reports in `docs/13-development-progress/TEST-RESULTS.md`. Until then: NOT MEASURED.
