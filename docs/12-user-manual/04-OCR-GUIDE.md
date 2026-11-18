# 04 — Panduan OCR (OCR Guide)

Alur OCR: cek tipe didukung → buat job → **proses via engine native**
→ review manusia. Engine **live bila tesseract terinstall di host**,
501 bila tidak (tidak ada fabrikasi hasil).

## 0. Prasyarat engine (per host!)

- Windows dev: `winget install UB-Mannheim.TesseractOCR` + `ind.traineddata`
  (tessdata_fast) + `configs`/`tessconfigs` ke `%LOCALAPPDATA%\tessdata`;
  tambah `C:\Program Files\Tesseract-OCR` ke PATH (user) dan
  `TESSDATA_PREFIX=%LOCALAPPDATA%\tessdata`.
- Prod (image Debian): paket `tesseract-ocr tesseract-ocr-ind tesseract-ocr-eng`
  sudah termasuk di `deploy/Dockerfile.server`.
- Verifikasi: `tesseract --list-langs` harus memuat `eng`, `ind`, `osd`.

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

## 4. Pemrosesan engine — LIVE bila tesseract terinstall

```powershell
# Dokumen WAJIB punya konten gambar (PUT /content dulu; tanpa konten → 422).
# Hanya gambar (png/jpg/webp/bmp/tif); PDF → 422. Tanpa engine di host → 501.
curl -X POST http://localhost:3000/api/v1/ocr/jobs/7/process -H "Authorization: Bearer TOKEN"
# → {"status":"processed","lines":N,"fields":[{"name":"page1_line1","value":"...","confidence":0.95,"page":1}]}
#    Job berstatus processed; fields tersimpan dan terlihat di GET job.
#    Confidence = rata-rata kata per baris (TSV tesseract); re-process menimpa
#    fields lama (idempoten).
```

- `POST /api/v1/ocr/jobs/{id}/process` — butuh izin Create. Urutan cek:
  konten dulu (422 bila kosong), mime gambar (422 bila bukan gambar),
  lalu engine (501 bila tesseract tidak ditemukan di host).
- Review (`approve`/`reject`, izin Verify) tetap wajib oleh manusia setelah
  proses — engine tidak pernah auto-approve.
