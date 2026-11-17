# LegalRust — Copy Bahasa Indonesia (ID)

> Copy ini ditulis agar persis sama dengan scope yang TERIMPLEMENTASI.
> Status: uji internal 28/32 (ACCEPTANCE.md) — bukan production-ready.
> Endpoint dirujuk dari `apps/server/src/lib.rs`.

## Hero (/)

**H1:** Kelola Perkara, Dokumen, dan Akta dalam Satu Platform Aman

**Sub-copy:**
LegalRust membantu notaris, PPAT, dan firma hukum mengelola perkara (`POST /api/v1/matters`),
dokumen berversi dengan pin SHA-256 (`POST /api/v1/documents` + `PUT /api/v1/documents/{id}/content`),
akta dari template dengan alur ber-guard (`POST /api/v1/deeds/from-template`),
verifikasi identitas via OCR job + review manusia (`POST /api/v1/ocr/jobs`, `POST /api/v1/ocr/jobs/{id}/review`),
keuangan invoice anti-overpay dan trust ledger double-entry (`POST /api/v1/trust/transfer`),
serta ekspor CSV (`GET /api/v1/reports/matters.csv`) — semua di belakang RBAC per-endpoint dan jejak audit.

**CTA:** [Mulai Gratis](/login) · [Lihat Demo](/app)

**Catatan jujur 501 (tampilkan di bawah hero / halaman fitur):**
- `POST /api/v1/ocr/jobs/{id}/process` → 501: engine OCR live butuh libtesseract / cloud fallback.
- `POST /api/v1/ai/chat` → 501: belum ada AI provider; yang live: RAG keyword (`POST /api/v1/rag/index`, `GET /api/v1/rag/search`) + pencatatan token (`GET /api/v1/ai/usage`).
- `POST /api/v1/deeds/{id}/esign` → 501: provider PSrE belum dikonfigurasi.
- `POST /api/v1/invoices/{id}/pay-gateway` → 501: billing gateway belum dikonfigurasi (pembayaran manual tersedia).
- `POST /api/v1/webhooks/incoming/{provider}` → 501 tanpa integration secret.

## Fitur (6 kartu — sesuai implementasi)

1. **Manajemen Perkara Terintegrasi** — Klien (`/api/v1/clients`), perkara (`/api/v1/matters`), tugas bertransisi aman (`POST /api/v1/tasks/{tid}`), notifikasi in-app (`GET /api/v1/notifications`).
2. **Dokumen Berversi + Share Kedaluwarsa** — Upload SHA-pin, versi, tautan share 1–720 jam (`POST /api/v1/documents/{id}/shares`), unduh ber-RBAC.
3. **OCR + Review Manusia** — Job OCR per tipe dokumen (KTP/KK/NPWP/paspor/sertifikat/akta/kontrak/umum), daftar tipe (`GET /api/v1/ocr/supported`), keputusan approve/reject butuh permission Verify.
4. **Akta dari Template** — Template berversi (`/api/v1/deed-templates`), render `{{variabel}}`, state machine dengan penolakan transisi ilegal (409).
5. **Keuangan & Trust Ledger** — Invoice + pembayaran manual anti-overpay, akun trust per klien, transfer double-entry anti-negatif, SSE event (`GET /api/v1/events`).
6. **Keamanan & Audit** — Argon2, JWT + sesi revocable + API token, lockout 5x/5 mnt, rate-limit, header CSP/HSTS, audit trail role-gated (`GET /api/v1/audit`).

## Solusi per segmen (tanpa testimoni palsu)

- **Notaris:** draft akta → review → approval → signing dalam satu alur teraudit.
- **PPAT:** cek dokumen tanah via pipeline OCR + review sebelum penomoran akta.
- **Firma hukum / Advokat:** perkara + tugas + invoice per matter + ekspor CSV untuk pelaporan.
- **Korporat:** multi-org, role (Owner/Admin/Auditor), token API untuk integrasi internal.

## Harga (ringkas, TBD)

Free Trial · Professional · Business · Enterprise. Harga final TBD oleh owner; detail di `PRICING-DRAFT.md`. Struktur bulanan + tahunan (diskon tahunan TBD).

## FAQ (ringkas — penuh di `seed-faqs.json`)

Lihat `docs/MARKETING/seed-faqs.json` (8 FAQ). Intinya: data tersegregasi per-org, password Argon2, fitur 501 dijelaskan terbuka, status bukan production-ready.
