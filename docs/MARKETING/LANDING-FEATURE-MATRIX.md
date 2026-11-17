# Landing Feature Matrix — Truth Audit (2026-11-17)

> Audit kebenaran untuk rebuild landing. Setiap klaim landing HARUS terlacak ke kolom Bukti.
> Status jujur: tidak ada klaim "IMPLEMENTED" tanpa endpoint + tabel + bukti kode.

## Sumber & kesegaran

| Sumber | Isi / hitungan | Tanggal |
|---|---|---|
| `docs/03-FEATURE-CATALOG/feature-catalog.json` | 1.000 fitur (25 domain × 40) — **planning only**, bukan bukti implementasi | 2026-11-17 17:43 |
| `docs/03-FEATURE-CATALOG/expansions/expansion-catalog.json` | 10.000 ekspansi — **planning only** | 2026-11-17 |
| `docs/MARKETING/COPY-ID.md` | Copy ID jujur + daftar 501 | 2026-11-17 21:33 |
| `docs/13-development-progress/ACCEPTANCE.md` | 28/32 DONE+TESTED — **NOT production-ready** | 2026-11-17 20:52 |
| `docs/13-development-progress/BLOCKERS.md` | BL-EXT-01 (kredensial eksternal), BL-LIC-01 (LICENSE), BL-DB-01 CLEARED | 2026-11-17 20:44 |
| `apps/server/src/lib.rs` | 78 `.route()` (+9 blok multi-line) ≈ 80+ endpoint | 2026-11-17 22:57 |
| `git log` | Commit terbaru 2026-11-17 (`1b79105`) | 2026-11-17 |

**Keputusan kesegaran:** semua sumber bertanggal 2026-11-17, sama dengan tanggal audit — TIDAK STALE. Tidak ada peringatan stale.
Pengecualian: `feature-catalog.json` adalah output `docs-gen.py (planning only)` (lihat `docs/03-FEATURE-CATALOG/README.md:6`) — tidak boleh dipakai sebagai bukti implementasi.

## Matriks fitur (12 baris)

| # | Feature | Domain | Endpoint(s) | UI di /app | Bukti (file:line) | Status | Layak di landing |
|---|---|---|---|---|---|---|---|
| 1 | Perkara & CRM (clients, matters, tasks per-matter) | matter/CRM | `POST/GET /api/v1/clients`, `POST/GET /api/v1/matters`, `POST/GET /api/v1/matters/{mid}/tasks`, `POST /api/v1/tasks/{tid}` | Tab Ringkasan (tabel Perkara Aktif) | `apps/server/src/lib.rs:2739-2745`; tabel `clients/matters/tasks` di `migrations/0002_core.sql` | IMPLEMENTED | YA — inti hero + kartu 1 |
| 2 | Dokumen berversi + share kedaluwarsa | documents | `POST/GET /api/v1/documents`, `GET /api/v1/documents/{id}`, `PUT/GET /api/v1/documents/{id}/content` (SHA-pin), `POST/GET /api/v1/documents/{id}/shares` (1–720 jam), `DELETE /api/v1/shares/{id}`, `GET /api/v1/shared/{token}[ /content]` | Tab Dokumen | `apps/server/src/lib.rs:2746-2751,2784-2790`; tabel `documents/document_versions/document_shares` di `migrations/0001-0002` | IMPLEMENTED | YA — kartu 2 |
| 3 | OCR job + review manusia | OCR | `POST/GET /api/v1/ocr/jobs`, `GET /api/v1/ocr/jobs/{id}`, `POST /api/v1/ocr/jobs/{id}/review` (perm Verify), `GET /api/v1/ocr/supported`; `POST /api/v1/ocr/jobs/{id}/process` → **501** | Tidak ada tab khusus (via API) | `apps/server/src/lib.rs:747-778` (process = `BlockedExternal`), `:2752-2756`; tabel `ocr_jobs/ocr_fields/ocr_reviews` di `migrations/0005_ocr.sql` | CONFIGURATION_REQUIRED (engine; pipeline job+review IMPLEMENTED) | YA BERSYARAT — wajib catatan 501, tanpa kata "otomatis AI" |
| 4 | Akta dari template, state machine ber-guard | deeds | `POST/GET /api/v1/deed-templates`, `PUT /api/v1/deed-templates/{id}`, `POST /api/v1/deeds/from-template`, `GET /api/v1/deeds`, `GET /api/v1/deeds/{id}`, `POST /api/v1/deeds/{id}/transition` (409 ilegal); `POST /api/v1/deeds/{id}/esign` → **501** | Tab Akta | `apps/server/src/lib.rs:970-981` (esign 501), `:2757-2763`; `crates/workflow/src/lib.rs:2-28` (10 state, FLOW guard); tabel `deeds/deed_templates/deed_parties` di `migrations/0003,0006` | PARTIAL (render+transisi IMPLEMENTED; esign CONFIGURATION_REQUIRED) | YA BERSYARAT — klaim esign hanya dengan kualifikasi "aktivasi PSrE bertahap" |
| 5 | Invoice anti-overpay + trust ledger double-entry | finance/trust | `POST/GET /api/v1/matters/{mid}/invoices`, `GET /api/v1/invoices/{id}`, `POST /api/v1/invoices/{id}/payments` (manual), `POST /api/v1/trust/accounts` + `GET`, `POST /api/v1/trust/accounts/{id}/entries`, `POST /api/v1/trust/transfer` (anti-negatif); `POST /api/v1/invoices/{id}/pay-gateway` → **501**; `GET /api/v1/events` (SSE) | Tab Keuangan (termasuk tabel Rekening trust) | `apps/server/src/lib.rs:1209-1220` (gateway 501), `:2768-2775,2794-2801`; tabel `invoices/payments/trust_accounts/trust_entries` di `migrations/0003,0007,0009` | PARTIAL (manual+trust IMPLEMENTED; gateway CONFIGURATION_REQUIRED) | YA BERSYARAT — hanya pembayaran manual + trust; gateway wajib 501 |
| 6 | Keamanan: Argon2, JWT+sesi+token, lockout 5x/5 mnt, rate-limit, header CSP/HSTS, RBAC, audit | security/RBAC/audit | `POST /api/v1/auth/register\|login\|logout`, `GET /api/v1/orgs/me`, `POST/GET/DELETE /api/v1/auth/tokens[/{id}]`, `POST /api/v1/auth/password`, `POST /api/v1/users/{uid}/roles`, `GET /api/v1/audit` (role-gated) | Halaman /login (matriks peran); tidak ada tab audit di /app | `apps/server/src/lib.rs:2735-2738,2778-2781,2792` + konstanta `LOGIN_MAX_ATTEMPTS/WINDOW` (~`:47-48`); `crates/authorization/src/lib.rs:2-24` (9 peran × 11 permission); `crates/application/src/lib.rs` (`BlockedExternal → 501 NOT_IMPLEMENTED`); tabel `users/sessions/api_tokens/roles/audit_events` di `migrations/0001-0002,0008` | IMPLEMENTED (audit final eksternal masih PARTIAL per ACCEPTANCE) | YA — kartu 6, tanpa klaim "production-ready" |
| 7 | Laporan CSV + ekspor | reporting | `GET /api/v1/reports/matters-csv` dan alias `GET /api/v1/reports/matters.csv` | Tidak ada tab khusus (unduh via API) | `apps/server/src/lib.rs:1228-1256` (`matters_csv`), `:2775,2791`; `crates/reporting` | IMPLEMENTED | YA — sebut sebagai ekspor CSV, bukan "analitik" |
| 8 | RAG keyword + pencatatan token AI | RAG/search | `POST /api/v1/rag/index`, `GET /api/v1/rag/search`, `GET /api/v1/ai/usage` (meteran); `POST /api/v1/ai/chat` → **501** (catat estimasi token, nol output) | Tidak ada tab khusus | `apps/server/src/lib.rs:995-1040` (chat 501 + `usage_add`), `:2764-2767`; tabel `ai_conversations/ai_messages/ai_usage/rag_chunks` di `migrations/0007_ai_finance.sql` | CONFIGURATION_REQUIRED (chat; RAG keyword + usage IMPLEMENTED) | YA BERSYARAT — hanya "pencarian keyword + meteran token", tanpa klaim chatbot |
| 9 | Tugas bertransisi aman | tasks | `POST/GET /api/v1/matters/{mid}/tasks`, `POST /api/v1/tasks/{tid}` (guarded patch) | Tab Ringkasan (implisit; tidak ada tab Tugas) | `apps/server/src/lib.rs:2741-2745`; tabel `tasks` di `migrations/0002_core.sql` | IMPLEMENTED | YA — gabung kartu 1, jangan klaim tab Tugas |
| 10 | Notifikasi in-app + SSE | notifications | `GET /api/v1/notifications`, `POST /api/v1/notifications/{id}/read`, `GET /api/v1/events` (SSE), `POST/GET /api/v1/jobs` (+claim/complete worker) | Tidak ada tab khusus | `apps/server/src/lib.rs:2782-2783,2793,2801`; tabel `notifications/jobs` di `migrations/0008_ops.sql`; `apps/worker` | IMPLEMENTED | YA — sebagai "notifikasi in-app", bukan push/email |
| 11 | Organisasi, pengguna, peran, token API | org/users | `GET /api/v1/orgs/me`, `POST/GET/DELETE /api/v1/auth/tokens[/{id}]`, `POST /api/v1/auth/password`, `POST /api/v1/users/{uid}/roles`, `GET /api/v1/integrations` | Halaman /login (matriks peran); /app tersegregasi per-org | `apps/server/src/lib.rs:2738,2778-2781,2797`; tabel `organizations/users/roles/user_roles/permissions/role_permissions` di `migrations/0001-0002` | IMPLEMENTED | YA — segmen korporat (multi-org, peran, token API) |
| 12 | API publik marketing + pipeline admin | public/marketing | Publik: `POST /api/v1/public/contact\|demo-requests\|sales-inquiries\|newsletter/subscribe`, `GET /api/v1/public/pricing\|features\|articles[/{slug}]\|faqs`; Admin (perm Approve): `GET/POST /api/v1/admin/leads`, `GET/PUT/DELETE /api/v1/admin/leads/{id}`, `POST .../notes\|assign`, `POST/GET .../follow-ups`, `GET /api/v1/admin/contacts\|demo-requests`, `POST /api/v1/admin/pricing\|faqs\|articles` | Landing `/`, `/login`, `/app` (Tabler self-host) | `apps/server/src/lib.rs:1923-2034` (LANDING/LOGIN/APP_HTML), `:2802-2834`; tabel `marketing_*` (9 tabel) di `migrations/0010_marketing.sql` | IMPLEMENTED | YA — fondasi landing itu sendiri |

**Jumlah baris matriks: 12** (sesuai 12 grup kapabilitas yang diminta).

## Kesiapan non-fitur (di luar 12 baris — tidak boleh diklaim sebagai fitur)

| Item | Bukti | Status | Layak di landing |
|---|---|---|---|
| Mobile app | `apps/mobile/src/lib.rs` — sync-core in-memory saja (outbox 1 MiB guard, FIFO, backoff, LWW resolver; tanpa I/O/network/UI) | PLANNED (sync-core only) | TIDAK — nol klaim mobile |
| Counsel sign-off / production | `ACCEPTANCE.md:7-8` (MISSING 1: prod deploy + counsel), `BLOCKERS.md:2-3` (BL-EXT-01 kredensial, BL-LIC-01 LICENSE) | PLANNED/BLOCKED | TIDAK — wajib banner "uji internal 28/32, bukan production-ready" |

## Konflik & keputusan (kode menang atas dok/marketing)

1. **Katalog 1.000/10.000 fitur vs endpoint nyata (~80).** `README.md:6` menyatakan katalog dibuat `docs-gen.py (planning only)`. Keputusan: katalog TIDAK dipakai sebagai bukti; kolom Bukti hanya merujuk `lib.rs`, `crates/*`, `migrations/*`. Landing dilarang menyebut angka 1.000/10.000 sebagai fitur tersedia.
2. **Kartu landing "OCR otomatis dengan AI" (`lib.rs:1960`) vs `ocr_process` 501 (`lib.rs:747-778`).** Keputusan: kode menang. Copy wajib mengikuti `COPY-ID.md:23` (engine 501, yang live hanya job + review manusia). Kata "otomatis AI" dilarang.
3. **Kartu landing "Tanda Tangan Digital online" (`lib.rs:1961`) vs `deed_esign` 501 (`lib.rs:970-981`).** Keputusan: kode menang. Klaim esign hanya dengan kualifikasi persis "aktivasi PSrE bertahap" + catatan 501 `COPY-ID.md:24`.
4. **Duplikat endpoint laporan:** `COPY-ID.md:16` merujuk `GET /api/v1/reports/matters.csv`, kode punya DUA rute (`:2775` `matters-csv` dan `:2791` `matters.csv`, handler sama `matters_csv` `:1228`). Keputusan: kode menang — keduanya live (alias). Landing boleh sebut salah satu; kanonisasi disarankan di luar file ini (tanpa edit kode).
5. **`ai_chat` mencatat conv+token lalu 501 (`lib.rs:995-1029`) vs kesan "AI tersedia".** Keputusan: kode + `COPY-ID.md:23` menang — yang live hanya RAG keyword + `ai_usage`. Klaim chatbot dilarang; `public_features` (`lib.rs:2255`) sendiri menandai `ai-usage.available: false`.
6. **Webhook "diterima" vs secret-gating (`lib.rs:1258-1295`).** `webhook_in` 501 bila `integration_secret` kosong + wajib header `x-signature`/`x-ts`. Keputusan: status secret-gated; landing tidak boleh klaim integrasi webhook aktif.
7. **Badge state di `apps/web` vs `DeedState` backend.** `apps/web/src/lib.rs` (`deed_state_badge`) memakai slug sederhana (`draft/review/approved/signed/...`) yang TIDAK cocok dengan 10 state backend (`crates/workflow/src/lib.rs:2-28`). Keputusan: backend menang; landing tidak boleh menampilkan slug badge sebagai alur akta resmi.
8. **Tab /app hanya 4 (Ringkasan/Dokumen/Akta/Keuangan, `lib.rs:2004`) vs 12 grup.** Tasks, notifikasi, OCR, RAG, audit, laporan, admin marketing TIDAK punya tab. Keputusan: kolom "UI di /app" jujur per baris; landing memakai CTA `/app` generik tanpa klaim cakupan tab.
9. **Kartu "Laporan & Analitik" / "Keamanan enterprise" (`lib.rs:1963-1964`) vs ACCEPTANCE 28/32.** Keputusan: `ACCEPTANCE.md` + `BLOCKERS.md` menang — yang ada hanya ekspor CSV dan kontrol teruji internal; "analitik" dan "enterprise/production-ready" dilarang. Banner kejujuran wajib.
10. **Skor 28/32 vs klaim kesiapan.** `ACCEPTANCE.md:9` eksplisit `NOT production-ready`; `BLOCKERS.md` masih menahan counsel, kredensial eksternal, LICENSE. Keputusan: setiap halaman landing wajib memuat catatan jujur 501 + status uji internal, mengikuti `COPY-ID.md:21-26`.

## Aturan pakai file ini

- Setiap klaim landing baru wajib menambah/memperbarui baris di atas dengan Bukti `file:line` yang diverifikasi baca, bukan dari ingatan.
- Perubahan endpoint (`apps/server/src/lib.rs`) atau migrasi baru mewajibkan re-audit baris terdampak.
- Status `CONFIGURATION_REQUIRED` hanya boleh naik ke `IMPLEMENTED` setelah kredensial/provider live terverifikasi (BL-EXT-01 clear), bukan setelah janji.
