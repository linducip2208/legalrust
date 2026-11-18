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

## 2026-11-17 measured (dev machine, ephemeral store, no DB)

> NON-PRODUCTION. Dev-machine loopback numbers only: debug build, ephemeral
> in-memory store (no MySQL — `DATABASE_URL` pointed at a dead port so the
> server fell back to memory), sequential clients (concurrency = 1), unique
> port 8091 (never 8080). NOT a capacity claim, NOT comparable to release or
> production. Do not quote these as SLAs.

- Machine: Windows 11 Home Single Language, Intel Core Ultra 7 155H (16 cores), 63.7 GiB RAM.
- Build: `cargo build -p legalrust-server --bin legalrust-server` with
  `CARGO_TARGET_DIR=target-bench` (isolated from `target/`), profile `dev`
  (unoptimized + debuginfo), rustc 1.99.0. Binary: `target-bench/debug/legalrust-server.exe`.
- Server env: `PORT=8091`, `JWT_SECRET=bench-secret-*`, `DATABASE_URL=mysql://root:bad@127.0.0.1:9/none`
  (unreachable by design → "ephemeral dev store active"), `INTEGRATION_SECRET=bench-int`.
- Method: 5 warmup reqs, then sequential `GET` loop via .NET HttpClient
  (proxy disabled); percentiles computed client-side over per-request wall time.
  Full flow = single pass register → login → create client → create matter (timed individually).
- Raw client: min/mean/p50/p95/p99/max in ms. No `wrk`/concurrency sweep (single client only).

| Endpoint | n | ok (<500) | min | mean | p50 | p95 | p99 | max |
|----------|---|-----------|-----|------|-----|-----|-----|-----|
| `GET /healthz` | 200 | 200 (200x200) | 0.24ms | 0.88ms | 0.45ms | 4.61ms | 5.95ms | 6.02ms |
| `GET /api/v1/version` | 20 | 20 (200x20) | 0.23ms | 0.35ms | 0.30ms | 0.49ms | 0.89ms | 0.89ms |
| `GET /nope-404-bench` (fallback 404) | 20 | 20 (404x20) | 0.24ms | 0.34ms | 0.29ms | 0.51ms | 0.68ms | 0.68ms |

| Full-flow step | status | wall time |
|----------------|--------|-----------|
| `POST /api/v1/auth/register` | 200 | 486ms (argon2 hash; expected dominant cost) |
| `POST /api/v1/auth/login` | 200 | 587ms (argon2 verify; expected dominant cost) |
| `POST /api/v1/clients` | 200 | 2ms |
| `POST /api/v1/matters` | 200 | 3ms |

Notes:
- Two prior same-setup runs gave `/healthz` p95 1.04ms / 1.69ms (vs 4.61ms here);
  sub-ms medians were stable, tail varies run to run on a shared desktop — reported run is the
  last clean pass (full flow + orderly server shutdown, port 8091 verified free afterwards).
- Vs protocol thresholds (§Acceptance thresholds): `/healthz` p95 4.61ms < 500ms target — but that
  target assumes c=10/release/seeded DB, so this is a smoke signal only, NOT a PASS verdict.
- `scripts/bench.sh` unchanged (bash/k6-less loop still valid for Linux/CI; Windows run above used
  an equivalent throwaway client, since deleted).

## 2026-11-18 - wave 8: approvals + compliance + webhooks endpoints (live Laragon MySQL, debug build)
- Method: throwaway python client (10-20 req, pauses to respect auth rate-limit), Ultra 7 155H, Laragon MySQL 8.4.9 live-DB, server :8080.
- GET /healthz: n=10 min=4.9ms p50=19.1ms p95=27.4ms max=114.5ms (run 1 incl. warmup); repeat run p50=5.9ms p95=27.9ms (tail varies, shared desktop).
- POST /api/v1/matters: n=5 min=17.6ms p50=20.4ms p95=22.5ms max=42.8ms.
- POST /api/v1/approvals: n=5 min=15.7ms p50=27.5ms p95=31.4ms max=33.7ms.
- GET /api/v1/approvals: n=5 min=8.3ms p50=8.9ms p95=9.0ms max=25.7ms.
- GET /api/v1/webhooks/deliveries: n=5 min=7.7ms p50=8.4ms p95=8.9ms max=27.9ms.
- GET /api/v1/consents: n=5 min=7.1ms p50=7.5ms p95=8.3ms max=12.6ms.
- GET /api/v1/reports/deeds.csv: n=5 min=11.0ms p50=13.2ms p95=17.1ms max=35.1ms.
- Note: auth register/login intentionally unbenchmarked per-request (Argon2 ~500ms + rate-limit); first burst hit 429 as designed.
- Verdict: smoke signals only (debug build, n small, shared desktop) — NOT a prod PASS; thresholds in protocol require c=10/release/seeded DB.
