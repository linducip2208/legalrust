# 03 — Panduan DMS (Document Management)

Alur dokumen: buat metadata → unggah konten (dengan SHA-256) → baca/unduh →
versi → share link. Semua route persis seperti di `apps/server/src/lib.rs`.

## 1. Buat dokumen & unggah konten

```powershell
$H = @{ Authorization = "Bearer TOKEN" }

# 1. Buat metadata dokumen (mime + size direncanakan di depan)
curl -X POST http://localhost:3000/api/v1/documents `
  -H "Authorization: Bearer TOKEN" -H 'Content-Type: application/json' `
  -d '{"name":"kontrak.pdf","mime":"application/pdf","size":12345,"sha":"<hex-sha256-isi>"}'
# → {"id": <doc_id>, ...}

# 2. Unggah konten (PUT). Server menolak bila:
#    - ukuran aktual ≠ size (422 "size mismatch")
#    - sha256 aktual ≠ sha (422 "sha256 mismatch")
curl -X PUT http://localhost:3000/api/v1/documents/1/content `
  -H "Authorization: Bearer TOKEN" --data-binary '@kontrak.pdf'

# 3. Daftar & baca metadata (metadata memuat daftar versi: v + object key)
curl http://localhost:3000/api/v1/documents -H "Authorization: Bearer TOKEN"
curl http://localhost:3000/api/v1/documents/1 -H "Authorization: Bearer TOKEN"

# 4. Unduh konten
curl http://localhost:3000/api/v1/documents/1/content -H "Authorization: Bearer TOKEN" -o kontrak.pdf
```

Route DMS:

- `POST /api/v1/documents` — buat metadata (butuh `name`, `mime`, `size`, `sha`; maks 50 MiB).
- `GET /api/v1/documents` — daftar dokumen org.
- `GET /api/v1/documents/{id}` — metadata + daftar versi (`versions: [{v, key}]`).
- `PUT /api/v1/documents/{id}/content` — unggah/timpa konten berversi.
- `GET /api/v1/documents/{id}/content` — unduh konten (404 `"content"` bila belum ada isi).

## 2. Content-addressing & versi

- Setiap unggahan memverifikasi **ukuran dan SHA-256** terhadap metadata (422 bila tidak cocok).
- Mengunggah ulang via `PUT .../content` menambah **versi baru** (`version_no` bertambah, object key baru); versi lama tetap tersimpan dan terlihat di `GET /api/v1/documents/{id}`.
- **Tidak ada endpoint hapus versi / hapus dokumen / daftar versi terpisah.** Jangan mengasumsikan `DELETE /api/v1/documents/{id}` atau `GET .../versions` — keduanya tidak ada di router.

## 3. Share link (berbagi eksternal)

```powershell
# Buat share untuk dokumen 1
curl -X POST http://localhost:3000/api/v1/documents/1/shares `
  -H "Authorization: Bearer TOKEN" -H 'Content-Type: application/json' -d '{}'
# → {"token":"...","id":...}

# Daftar share aktif sebuah dokumen
curl http://localhost:3000/api/v1/documents/1/shares -H "Authorization: Bearer TOKEN"

# Cabut share
curl -X DELETE http://localhost:3000/api/v1/shares/5 -H "Authorization: Bearer TOKEN"

# Akses publik via token (TANPA Authorization header)
curl http://localhost:3000/api/v1/shared/TOKEN
curl http://localhost:3000/api/v1/shared/TOKEN/content -o shared.pdf
```

Route share:

- `POST /api/v1/documents/{id}/shares` — buat share.
- `GET /api/v1/documents/{id}/shares` — daftar share.
- `DELETE /api/v1/shares/{id}` — cabut share.
- `GET /api/v1/shared/{token}` — metadata publik (404 `"share"` bila token salah/dicabut).
- `GET /api/v1/shared/{token}/content` — konten publik (404 `"share"`/`"content"`).

## 4. Batasan jujur

- Pencarian full-text dokumen hanya tersedia untuk dokumen `text/*` via RAG (`POST /api/v1/rag/index`, `GET /api/v1/rag/search`) — lihat `07-AI-RAG-GUIDE.md`. Binary (`application/pdf`, dst.) ditolak dengan 422 `binary parsing not supported`.
- Tidak ada endpoint rename/move/folder — metadata dokumen apa adanya dari `POST /api/v1/documents`.
