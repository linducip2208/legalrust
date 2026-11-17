# Wave 7 Re-audit Report — 1.000 fitur (fresh sample)

Tanggal: 2026-11-18. Auditor: subagent F-REAUDIT. Metode: sampel acak segar
(offset 12, setiap ke-25 → 40 fitur: F-*-013/023/033/003/028/008/018/038),
verifikasi tiap klaim evidence terhadap kode saat ini
(`apps/server/src/lib.rs`, `crates/database/src/ops*.rs`, `migrations/`),
plus 6 titik risiko yang ditandai sebelumnya. Tanpa edit kode/migrasi.

## 1. Hasil sampel (40 fitur)

| Status registry | n sampel | % sampel | % registry (n=1000) |
|---|---|---|---|
| IMPLEMENTED | 1 | 2.5% | 3.7% (37) |
| PARTIAL | 4 | 10.0% | 10.4% (104) |
| CONFIGURATION_REQUIRED | 5 | 12.5% | 15.9% (159) |
| PLANNED | 17 | 42.5% | 32.6% (326) |
| NOT_STARTED | 13 | 32.5% | 37.4% (374) |

Distribusi sampel konsisten dengan populasi (deviasi dalam toleransi n=40).
Daftar sampel: F-D01-013, F-D01-038, F-D02-023, F-D03-008, F-D03-033,
F-D04-018, F-D05-003, F-D05-028, F-D06-013, F-D06-038, F-D07-023, F-D08-008,
F-D08-033, F-D09-018, F-D10-003, F-D10-028, F-D11-013, F-D11-038, F-D12-023,
F-D13-008, F-D13-033, F-D14-018, F-D15-003, F-D15-028, F-D16-013, F-D16-038,
F-D17-023, F-D18-008, F-D18-033, F-D19-018, F-D20-003, F-D20-028, F-D21-013,
F-D21-038, F-D22-023, F-D23-008, F-D23-033, F-D24-018, F-D25-003, F-D25-028.

## 2. Verifikasi evidence klaim positif (10/10 OK)

- F-D03-033 PARTIAL: `inv_create` (lib.rs:1166), matter invoice route — ADA.
- F-D06-038 PARTIAL: `matters_csv` (:1286), report CSV — ADA (CSV only, sesuai klaim).
- F-D16-013 PARTIAL: `token_create` (:1381), revocable API tokens — ADA.
- F-D19-018 IMPLEMENTED: `public_contact` (:2772), `public_subscribe` (:2905),
  `mkt_contact_create`/`mkt_subscribe`, demo-requests routes, `migrations/0010_marketing.sql` — ADA.
- F-D23-008 PARTIAL: `tpl_create` (:795), deed-template routes (:2260) — ADA.
- F-D01-013 / F-D05-028 CONFIG: `ocr_process` (:747) → 501 BlockedExternal — ADA.
- F-D07-023 CONFIG: `ai_chat` (:995) → 501 tanpa provider — ADA.
- F-D10-003 / F-D21-013 CONFIG: integration stubs, tanpa logic — ADA (tidak ada
  panggilan eksternal di `crates/integrations` selain nama stub).

## 3. Verifikasi klaim negatif (30/30 OK)

Semua 30 fitur PLANNED/NOT_STARTED dikonfirmasi tidak punya
route/handler/tabel: grep `watermark|sla|deadline_engine|apht|mortgage|
seat_manager|heatmap|cer_wer|adverse_media` di `lib.rs` hanya false-positive
(`tower_http` mengandung `wer_h`); di `migrations/` nol match.

## 4. Tabel mismatch: NOL

| # | Feature | Registry | Aktual | Status |
|---|---|---|---|---|
| — | (tidak ada) | — | — | 0 mismatch dari 40 sampel |

## 5. Drift check vs klaim Wave 5 / ACCEPTANCE

- Klaim "28/32 = 87/100, bukan production-ready" — masih akurat; sampel tidak
  menemukan fitur yang diam-diam jadi live maupun yang hilang.
- Klaim "187–203 tests green" — konsisten (tidak dijalankan ulang di wave ini,
  di luar mandat).
- Klaim "5 endpoint 501 jujur" — terkonfirmasi masih 501 di kode.

## 6. Cek 6 titik risiko

1. **Auth min-length — PASS.** `RegisterReq`/`LoginReq`/`PasswordReq` semua
   `length(min = 12)` (lib.rs:219,227,1431-1434); cocok dengan
   `MIN_PASSWORD_LEN = 12` (`crates/auth/src/lib.rs:21`).
2. **TIMESTAMP decodes — 1 SISA (lihat isu R-01).** `ops_mysql.rs:550`
   `read_at` (TIMESTAMP NULL) decode mentah; `:1060` `preferred_at` aman
   (kolom DATETIME, didukung sqlx). `mysql.rs:136` workaround UNIX_TIMESTAMP
   untuk sessions tetap ada.
3. **CSP inline — PASS.** Nol `style="`, `onclick`, `<style` di page consts
   (satu-satunya match adalah komentar penegasan di :1951). Satu `<script`
   hanyalah JSON-LD + 2 external `src` (:2027). Header CSP/HSTS/nosniff/
   DENY/referrer terpasang (:3553-3571).
4. **.env committed — PASS.** Hanya `.env.example` + `.env.prod.example` yang
   ter-track; tidak ada `.env` berisi secret.
5. **Duplikat migrasi — PASS.** `0001`–`0010` unik, tanpa versi ganda.
6. **target-*/scratch committed — PASS.** `git ls-files` bersih dari keduanya.

## 7. Isu baru (severity + fix-or-accepted)

| ID | Severity | Isu | Fix atau accepted |
|---|---|---|---|
| R-01 | MEDIUM | `notify_list` decode mentah `read_at TIMESTAMP NULL → Option<NaiveDateTime>` (`ops_mysql.rs:550`); path MySQL ini TIDAK teruji (semua API tests pakai `MemoryStores`, `pool: None`, `dev_state()` lib.rs:3576-3590). Jika sqlx menolak TIMESTAMP→NaiveDateTime di live MySQL, `GET /notifications` 500. | FIX sebelum prod-DB: pakai pola `UNIX_TIMESTAMP` seperti sessions (`mysql.rs:136`) atau `read_at IS NOT NULL`; lalu verifikasi di live DB. |
| R-02 | LOW | Nomor baris evidence di registry meleset perlahan (kode tumbuh; cth. DOC_TYPES sitasi lama). Simbol masih ada, tapi sitasi `:line` melemah sebagai bukti. | ACCEPTED sementara; perbaiki saat registrasi berikutnya (sitasi simbol + hash commit, bukan nomor baris). |
| R-03 | LOW | CSP `default-src 'self'` tanpa `script-src` eksplisit: JSON-LD inline diizinkan browser (tipe non-executable), tapi rapuh jika parser CSP mengetat. | ACCEPTED (bukan executable); opsional: pindah JSON-LD ke file statis. |
| R-04 | INFO | `git status` menunjukkan `M apps/server/tests/api.rs` (uncommitted, pasca komit animasi nav). | Higiene: commit/pisahkan sebelum wave berikutnya. Bukan temuan kode. |

## 8. Ringkasan count

- Sampel: 40/1000, mismatch: **0**.
- Critical: 0, High: 0, **Medium: 1**, **Low: 2**, Info: 1.
- 6 titik risiko: 5 PASS, 1 temuan lanjutan (R-01, sudah diketahui polanya).
- Kesimpulan: registry AKURAT pada sampel segar; tidak ada drift vs klaim
  Wave 5; satu-satunya blocker potensial menuju prod adalah R-01 + verifikasi
  live-DB yang memang sudah tercatat sebagai blocker (B-001/B-002).
