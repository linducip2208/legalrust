# 02 — Panduan Portal (Portal Guide)

Halaman web LegalRust adalah **workspace shell** — komponen UI yang benar-benar
ada didefinisikan di `apps/web/src/lib.rs` (`AppShell`, halaman-halaman, dan
helper murni yang teruji). Tidak ada halaman lain yang didokumentasikan di sini.

## 1. Kerangka umum (`AppShell`)

Setiap halaman dibungkus `AppShell` dengan brand `LEGALRUST` dan navigasi atas:

`Dashboard` · `Clients` · `Matters` · `Documents` · `Deeds` · `Finance`

Form login (`LoginForm`): field `email` + `password` (minimal 8 karakter,
validasi sisi klien di `validate_login`), tombol **Sign in**.

> Catatan jujur: shell me-render `DashboardPage` secara default dengan teks
> "Enterprise Legal Operations — workspace shell (Phase 5 builds full UI)".
> Daftar navigasi "pending Phase 6 data" pada komponen `App` mencerminkan status
> implementasi — data live tampil setelah backend terhubung.

## 2. Halaman-halaman dan zero-state

Semua halaman daftar memakai pola yang sama: bila data kosong, tampil
zero-state **"hubungkan database untuk data live"**. Ini bukan error — artinya
backend belum mengembalikan data (mis. dev store kosong atau DB belum
terhubung).

| Halaman (route UI) | Komponen | Data | Zero-state bila kosong |
|---|---|---|---|
| `/dashboard` | `DashboardPage` | 4 kartu statistik: Clients, Matters, Documents, Deeds (placeholder `—`) | "hubungkan database untuk data live" |
| `/clients` | `ClientsPage` | Daftar `ClientRow { id, name }` ← `GET /api/v1/clients` | sama |
| `/matters` | `MattersPage` | Daftar `MatterRow { id, title, status }` ← `GET /api/v1/matters` | sama |
| `/documents` | `DocumentsPage` | Daftar `DocumentRow { id, title }` ← `GET /api/v1/documents` | sama |
| `/deeds` | `DeedsPage` | Daftar `DeedRow { id, title, state }` + badge status ← `GET /api/v1/deeds` | sama |
| `/finance` | `FinancePage` | Daftar `InvoiceRow { number, balance_minor, currency }`, format `format_minor` (cth. `IDR 1.500.000`) ← `GET /api/v1/matters/{mid}/invoices`, `GET /api/v1/invoices/{id}` | sama |
| (review OCR) | `OcrReviewPage` | Kandidat `OcrCandidate { id, label, confidence }` dengan kelas `low (<0,7)` / `med (<0,9)` / `high` ← `GET /api/v1/ocr/jobs`, `GET /api/v1/ocr/jobs/{id}` | sama |

Badge status akta (`deed_state_badge`, diuji unit): `draft`, `review`,
`approved`, `signed`, `archived`, `rejected` → kelas CSS
`badge-*`, selain itu `badge-unknown`.

## 3. API di balik portal (route persis, `apps/server/src/lib.rs`)

- `GET /api/v1/orgs/me` — profil org sesi berjalan.
- `POST /api/v1/clients` / `GET /api/v1/clients` — buat & daftar klien.
- `POST /api/v1/matters` / `GET /api/v1/matters` — buat & daftar matter.
- `POST /api/v1/matters/{mid}/tasks` / `GET /api/v1/matters/{mid}/tasks` — tugas per matter.
- `POST /api/v1/tasks/{tid}` — patch satu tugas.
- `GET /api/v1/notifications` — daftar notifikasi; `POST /api/v1/notifications/{id}/read` — tandai dibaca.
- `GET /api/v1/audit` — jejak audit (lihat aktivitas portal).
- `GET /api/v1/events` — Server-Sent Events; event pertama selalu `hello`, lalu event live.
- `GET /api/v1/reports/matters-csv` dan `GET /api/v1/reports/matters.csv` — ekspor CSV matter (`id,number,title,status`; nama file via `csv_filename`, cth. `matters_2026-11-17.csv`).

## 4. Yang BELUM ada di portal (jujur)

- Tidak ada halaman UI untuk: OCR jobs selain `OcrReviewPage`, template akta,
  invoice detail, trust ledger, AI chat, RAG, webhook, integrasi, manajemen
  token/role. Fungsi-fungsi itu hanya tersedia via API (lihat bab panduan
  masing-masing).
- Statistik dashboard masih placeholder `—` sampai data live terhubung.
