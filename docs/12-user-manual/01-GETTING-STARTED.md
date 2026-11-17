# 01 — Memulai (Getting Started)

Panduan instalasi dan login pertama untuk pengguna LegalRust.
Bahasa: Indonesia. Semua route di bawah merujuk pada inventaris nyata
`apps/server/src/lib.rs` (`build_router`, 55 pemanggilan `.route()`).

## 1. Prasyarat

- Rust toolchain stabil (edisi 2021; perintah standar repo: `cargo fmt --all`,
  `cargo check --workspace`, `cargo test --workspace`).
- MySQL **opsional** untuk pengembangan lokal. Jika database tidak terjangkau
  dalam 3 detik saat server start, server tetap jalan dengan *ephemeral dev
  store* in-memory (`apps/server/src/main.rs`). Data dev store hilang saat
  restart — jangan dipakai untuk data produksi.
- Objek dokumen tersimpan lokal di `./var/objects` (relative terhadap direktori
  kerja server).
- Batas upload: **50 MiB** per file (`MAX_UPLOAD` di `apps/server/src/lib.rs`).

## 2. Konfigurasi environment

Server membaca konfigurasi dari environment (`Config::from_env`,
`crates/infrastructure`). Variabel yang dipakai langsung di
`apps/server/src/main.rs`:

| Variabel | Wajib | Fungsi |
|---|---|---|
| `JWT_SECRET` | Ya (untuk auth) | Kunci penanda tangan token. Jika kosong, server menulis peringatan dan **endpoint auth mengembalikan 501** sampai dikonfigurasi. |
| `INTEGRATION_SECRET` | Ya (untuk webhook/integrasi) | Secret verifikasi integrasi eksternal. |
| Database URL | Opsional lokal | Diambil via `Config::from_env` (`config.database_url`). Jika tidak terjangkau → dev store in-memory + peringatan log. |

Contoh (PowerShell):

```powershell
$env:JWT_SECRET = "isi-dengan-rahasia-minimal-32-karakter"
$env:INTEGRATION_SECRET = "isi-dengan-rahasia-integrasi"
cargo run -p legalrust-server
```

Server listen di `0.0.0.0:{config.port}` dan log `listening on {port}`.

## 3. Menjalankan server

Tidak ada binari worker terpisah — satu-satunya binari adalah
`legalrust-server` (`apps/server/src/main.rs`; paket `legalrust-server`).
Pekerjaan latar (OCR, dsb.) dikelola lewat API job dan event stream —
`POST /api/v1/jobs` / `GET /api/v1/jobs` (antrean job) dan
`GET /api/v1/events` (Server-Sent Events, event pertama selalu `hello`):

```powershell
cargo run -p legalrust-server
# cek kesehatan:
curl http://localhost:3000/healthz        # → {"status":"ok",...}
curl http://localhost:3000/readyz         # 200 jika DB siap; 501 jika DB tidak terjangkau
curl http://localhost:3000/api/v1/version # versi server, tanpa auth
```

Route yang dipakai di langkah ini (tanpa autentikasi):

- `GET /healthz`
- `GET /readyz` (501 `database unreachable` bila tanpa DB — ini status jujur, bukan error instalasi)
- `GET /api/v1/version`

## 4. Registrasi organisasi pertama & login

Organisasi dibuat lewat registrasi (tidak ada seed admin bawaan):

```powershell
# 1. Register (buat org + user pertama, password minimal 8 karakter)
curl -X POST http://localhost:3000/api/v1/auth/register `
  -H 'Content-Type: application/json' `
  -d '{"org_name":"Kantor Contoh","email":"owner@contoh.id","name":"Owner","password":"rahasia123"}'
# → {"user_id":"...","org_id":"...","token":"..."}

# 2. Login
curl -X POST http://localhost:3000/api/v1/auth/login `
  -H 'Content-Type: application/json' `
  -d '{"email":"owner@contoh.id","password":"rahasia123"}'
# → {"token":"..."} (kurang-lebih; simpan token ini)

# 3. Verifikasi sesi
curl http://localhost:3000/api/v1/orgs/me -H "Authorization: Bearer TOKEN"
curl -X POST http://localhost:3000/api/v1/auth/logout -H "Authorization: Bearer TOKEN"
```

Route auth (persis seperti di kode):

- `POST /api/v1/auth/register`
- `POST /api/v1/auth/login` (rate-limit brute-force per email → `429` bila berlebihan)
- `POST /api/v1/auth/logout`
- `GET /api/v1/orgs/me`
- `POST /api/v1/auth/tokens` + `GET /api/v1/auth/tokens` (token API)
- `DELETE /api/v1/auth/tokens/{id}` (cabut token)
- `POST /api/v1/auth/password` (ganti password, butuh sesi)
- `POST /api/v1/users/{uid}/roles` (tetapkan role; 12 role terdaftar di `crates/domain`: `super_admin`, `org_owner`, `office_admin`, `notary`, `ppat`, `lawyer`, `legal_staff`, `document_officer`, `finance_officer`, `auditor`, `client`, `reviewer`)

Semua request terautentikasi memakai header `Authorization: Bearer <token>`.
Setiap respons membawa header `x-request-id` — catat nilainya saat melapor
masalah (lihat `08-TROUBLESHOOTING.md`).

## 5. Langkah berikut

- Portal & halaman UI → `02-PORTAL-GUIDE.md`
- Dokumen & share link → `03-DMS-GUIDE.md`
- OCR & review → `04-OCR-GUIDE.md`
- Akta → `05-DEEDS-GUIDE.md`
- Keuangan & trust → `06-FINANCE-GUIDE.md`
- AI & RAG → `07-AI-RAG-GUIDE.md`
- Kode error & request-id → `08-TROUBLESHOOTING.md`
