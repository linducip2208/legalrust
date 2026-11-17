# 04 — Panduan OCR (OCR Guide)

Alur OCR: cek tipe didukung → buat job → (pemrosesan engine) → review manusia.
**Engine OCR belum terkonfigurasi** — panduan ini menandai dengan jelas mana
yang berfungsi dan mana yang mengembalikan `501`.

## 1. Tipe dokumen yang didukung

```powershell
curl http://localhost:3000/api/v1/ocr/supported -H "Authorization: Bearer TOKEN"
# → ["ktp","kk","npwp","passport","land_certificate","deed","contract","general"]
```

- `GET /api/v1/ocr/supported` — daftar `doc_type` yang diterima (butuh izin View).

## 2. Membuat & memantau job (berfungsi)

```powershell
# Buat job untuk dokumen 1 bertipe ktp (doc_type lain → 422 "doc_type")
curl -X POST http://localhost:3000/api/v1/ocr/jobs `
  -H "Authorization: Bearer TOKEN" -H 'Content-Type: application/json' `
  -d '{"document_id":1,"doc_type":"ktp"}'

# Daftar job & detail job
curl http://localhost:3000/api/v1/ocr/jobs -H "Authorization: Bearer TOKEN"
curl http://localhost:3000/api/v1/ocr/jobs/7 -H "Authorization: Bearer TOKEN"
```

- `POST /api/v1/ocr/jobs` — buat job (butuh izin Create; dokumen harus ada, jika tidak 404 `"document"`).
- `GET /api/v1/ocr/jobs` — daftar job.
- `GET /api/v1/ocr/jobs/{id}` — detail + kandidat hasil.

## 3. Review manusia (berfungsi)

```powershell
curl -X POST http://localhost:3000/api/v1/ocr/jobs/7/review `
  -H "Authorization: Bearer TOKEN" -H 'Content-Type: application/json' `
  -d '{"decision":"approve"}'
```

- `POST /api/v1/ocr/jobs/{id}/review` — keputusan review (`decision` wajib diisi; kosong → 422 `"decision"`).
- Di UI, `OcrReviewPage` (`apps/web/src/lib.rs`) menampilkan kandidat dengan
  kelas confidence `low (<0,7)` / `med (<0,9)` / `high`, dan zero-state
  "hubungkan database untuk data live" bila belum ada kandidat.

## 4. Pemrosesan engine — BLOCKED (`501`)

```powershell
curl -X POST http://localhost:3000/api/v1/ocr/jobs/7/process -H "Authorization: Bearer TOKEN"
# → 501 EXTERNAL_BLOCKED: "ocr engine unavailable (job 7, type ktp, ~12345 bytes not processed);
#    native libtesseract missing, cloud fallback unconfigured"
#    Job tetap berstatus queued; tidak ada yang diproses.
```

- `POST /api/v1/ocr/jobs/{id}/process` — **selalu 501** sampai engine OCR
  eksternal (native libtesseract / cloud fallback) dikonfigurasi dan
  diverifikasi live. Jangan menjanjikan hasil OCR otomatis kepada pengguna
  akhir sebelum itu.
