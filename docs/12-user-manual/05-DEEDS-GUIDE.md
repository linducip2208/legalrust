# 05 — Panduan Akta (Deeds Guide)

Alur akta: kelola template → terbitkan akta dari template → majukan status
melewati workflow → (tanda tangan elektronik). **E-sign belum terkonfigurasi**
(`501`).

## 1. Template akta

```powershell
# Buat template (name min. 2 karakter, body wajib; validasi gagal → 422)
curl -X POST http://localhost:3000/api/v1/deed-templates `
  -H "Authorization: Bearer TOKEN" -H 'Content-Type: application/json' `
  -d '{"name":"AJB Standar","body":"...isi template..."}'

# Daftar template
curl http://localhost:3000/api/v1/deed-templates -H "Authorization: Bearer TOKEN"

# Ubah template (PUT penuh pada id)
curl -X PUT http://localhost:3000/api/v1/deed-templates/3 `
  -H "Authorization: Bearer TOKEN" -H 'Content-Type: application/json' `
  -d '{"name":"AJB Standar v2","body":"...revisi..."}'
```

- `POST /api/v1/deed-templates` — buat (izin Create).
- `GET /api/v1/deed-templates` — daftar.
- `PUT /api/v1/deed-templates/{id}` — ubah. **Tidak ada** `GET satu template`
  atau `DELETE template` — jangan mengasumsikannya.

## 2. Terbitkan akta dari template

```powershell
curl -X POST http://localhost:3000/api/v1/deeds/from-template `
  -H "Authorization: Bearer TOKEN" -H 'Content-Type: application/json' `
  -d '{"template_id":3,"matter_id":1,...}'
# matter_id yang tidak ada → 404 "matter"
```

- `POST /api/v1/deeds/from-template` — buat akta dari template (izin Create).
- `GET /api/v1/deeds` — daftar akta org.
- `GET /api/v1/deeds/{id}` — detail (`id, matter_id, number, deed_type, state`).

## 3. Transisi workflow (state machine ketat)

Status akta mengikuti mesin status di `crates/workflow` — hanya transisi yang
terdaftar yang diterima, selain itu `409 "illegal transition"`:

```text
draft → collection → verification → legal_review → approval → signing
                                              ↘ revision → legal_review (putaran revisi)
signing → registration → completion → archive
```

```powershell
curl -X POST http://localhost:3000/api/v1/deeds/9/transition `
  -H "Authorization: Bearer TOKEN" -H 'Content-Type: application/json' `
  -d '{"to":"collection"}'
# → {"id":9,"state":"collection"}
# Lompat misalnya draft → signing langsung → 409 illegal transition.
# "to" yang tidak dikenal → 422 "to".
```

- `POST /api/v1/deeds/{id}/transition` — butuh izin **Approve** (bukan Create).
  Akta yang tidak ada → 404 `"deed"`.
- Di UI, `DeedsPage` + `deed_state_badge` (`apps/web/src/lib.rs`) me-render
  badge status akta.

## 4. Tanda tangan elektronik — BLOCKED (`501`)

```powershell
curl -X POST http://localhost:3000/api/v1/deeds/9/esign -H "Authorization: Bearer TOKEN"
# → 501 EXTERNAL_BLOCKED: "PSrE provider not configured"
```

- `POST /api/v1/deeds/{id}/esign` — **selalu 501** sampai provider PSrE
  dikonfigurasi dan diverifikasi live. Alur akta yang membutuhkan e-sign
  berhenti di status `signing` dan tidak boleh diklaim selesai.
