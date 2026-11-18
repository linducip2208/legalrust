# 08 — Troubleshooting (Kode Error & Request-ID)

Semua error API memakai envelope `ApiResponse` dengan `code` string dan HTTP
status yang dipetakan di `crates/application/src/lib.rs`. Setiap respons —
sukses maupun gagal — membawa header **`x-request-id`**: selalu catat nilainya
saat melapor.

## 1. Tabel kode (lengkap, sesuai kode)

| HTTP | `code` | Varian `AppError` | Artinya & kapan muncul |
|---|---|---|---|
| 401 | `UNAUTHORIZED` | `Unauthorized` | Tanpa/m-token salah di `Authorization: Bearer`. Juga saat `JWT_SECRET` kosong (auth 501-terkait, lihat bawah). |
| 403 | `FORBIDDEN` | `Forbidden` | Token valid tetapi role/izin tidak cukup (`authorize` gagal; butuh `View`/`Create`/`Approve` sesuai endpoint). Minta role via `POST /api/v1/users/{uid}/roles`. |
| 404 | `NOT_FOUND` | `NotFound(...)` | Entitas tidak ada: `"route"` (path salah), `"matter"`, `"task"`, `"document"`, `"content"`, `"deed"`, `"share"`, `"user"`, `"record"`. |
| 409 | `CONFLICT` | `Conflict(...)` | Konflik logika bisnis: `"overpayment"` (bayar melebihi sisa), `"insufficient trust balance"` (overdraft trust), `"illegal transition"` (transisi akta tak terdaftar). Tidak ada data yang berubah saat 409. |
| 422 | `VALIDATION` | `Validation(...)` | Input gagal validasi: password < 8, `name`/`body` template, `sha256`/`size mismatch`, `mime`, `doc_type`, `decision` kosong, `messages` kosong, `q` kosong, `binary parsing not supported`, `not utf8`, `no content`, `to`, `role`, dsb. |
| 423 | `LOCKED` | `Locked(...)` | Terkunci (dipetakan, jarang ditemui dari endpoint saat ini). |
| 429 | `RATE_LIMITED` | `RateLimited` | Rate limiter auth (`register`/`login`, guard per email). Tunggu jendela waktu lalu coba lagi. |
| 501 | `EXTERNAL_BLOCKED` | `BlockedExternal(...)` | **Bukan bug** — dependensi eksternal belum live. Pesan selalu menyebut buktinya: `database unreachable` (`GET /readyz`), `tesseract ...` (`POST /api/v1/ocr/jobs/{id}/process` tanpa engine di host), `PSrE provider not configured` (`POST /api/v1/deeds/{id}/esign`), `tripay not configured` (`POST /api/v1/invoices/{id}/pay-gateway` tanpa env Tripay), `no AI provider ... at <base>` (`POST /api/v1/ai/chat` bila endpoint tidak menjawab). |
| 500 | `INTERNAL` | `Internal(...)` | Error server tak terduga — sertakan `x-request-id` saat melapor. |

## 2. Cara melapor yang baik

```powershell
# Tangkap header request-id dari respons gagal
curl -i http://localhost:3000/api/v1/documents/999 -H "Authorization: Bearer TOKEN"
# HTTP/1.1 404 ... x-request-id: <uuid> ...
# {"code":"NOT_FOUND","message":"document",...}
```

Sertakan: (1) `x-request-id`, (2) method + path persis, (3) `code` + message,
(4) waktu kejadian. Jejak audit terkait bisa dicek via `GET /api/v1/audit`.

## 3. Kasus umum

- **Semua request 401** → token kedaluwarsa/salah; login ulang (`POST /api/v1/auth/login`) atau cek `JWT_SECRET` server.
- **403 di endpoint tulis** → endpoint butuh izin lebih tinggi (mis. transisi akta butuh `Approve`, bukan `Create`); minta admin menetapkan role yang sesuai.
- **`GET /readyz` 501** → database belum terhubung; server berjalan dengan dev store. Bukan error instalasi, tetapi jangan simpan data penting di mode ini.
- **Share link 404 `"share"`** → token salah atau sudah dicabut (`DELETE /api/v1/shares/{id}`).
- **Upload 422 `sha256 mismatch`** → hitung ulang SHA-256 dari file yang persis diunggah (bukan hasil edit) dan samakan `size`.
- **SSE sepi** → `GET /api/v1/events` selalu mengirim `hello` pertama; jika tidak ada `hello`, masalahnya di koneksi, bukan di data.
- **OCR 501 `tesseract ...`** → install tesseract + traineddata di host itu
  (lihat 04-OCR-GUIDE §0); prod image sudah termasuk paketnya.
- **AI 501 `no AI provider ... at <base>`** → jalankan `ollama serve` +
  `ollama pull qwen2.5:1.5b`, atau isi `AI_BASE_URL/AI_MODEL` ke provider
  HTTPS (lihat 07-AI-RAG-GUIDE §2).
- **Tripay 501 `tripay not configured`** → isi `TRIPAY_API_KEY`,
  `TRIPAY_PRIVATE_KEY`, `TRIPAY_MERCHANT_CODE` (sandbox dulu), dan daftarkan
  callback URL di panel Tripay (lihat 06-FINANCE-GUIDE §3).
- **CORS ditolak browser** → API kini same-origin-only secara default; isi
  `CORS_ORIGINS` (daftar origin dipisah koma) bila klien browser eksternal
  dibutuhkan.
