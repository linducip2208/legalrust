# LegalRust — SEO Plan

> Aturan data: hanya klaim terverifikasi (ACCEPTANCE.md + `lib.rs` route list).
> Tanpa angka pengguna/pendapatan. Tanpa testimoni fiktif.
> Bahasa: id-ID. Domain contoh di bawah pakai placeholder `https://legalrust.example`
> — ganti saat domain produksi diputuskan. Jangan klaim domain yang belum dimiliki.

## 1. Peta route terencana (docs, bukan janji deploy)

| Route | Title (≤60 char) | Meta description (≤160 char) | OG |
|---|---|---|---|
| `/` | LegalRust — Kelola Perkara, Dokumen & Akta | Platform manajemen hukum: perkara, dokumen berversi SHA-256, akta template, OCR+review, trust ledger, ekspor CSV. Uji internal 118 tes hijau. | `og:title` = title; `og:description` = description; `og:type`=website |
| `/fitur/perkara` | Manajemen Perkara Hukum Terintegrasi | Kelola klien, perkara, dan tugas bertransisi aman dengan notifikasi dan audit trail per organisasi. | article |
| `/fitur/dokumen` | DMS Hukum: Versi + Share Kedaluwarsa | Dokumen berversi SHA-pin, share 1–720 jam, unduh ber-RBAC. Aman untuk kantor hukum. | article |
| `/fitur/ocr-verifikasi` | OCR + Verifikasi Dokumen Identitas | Job OCR KTP/KK/NPWP/sertifikat dengan skor kepercayaan dan review manusia approve/reject. | article |
| `/fitur/akta` | Akta dari Template + Alur Ber-guard | Template berversi, render variabel, state machine anti-transisi ilegal. E-sign PSrE terencana. | article |
| `/fitur/keuangan` | Invoice & Trust Ledger Dana Klien | Invoice anti-overpay, trust double-entry anti-negatif, ekspor CSV. Gateway terencana. | article |
| `/fitur/keamanan` | Keamanan: Argon2, RBAC, Audit Trail | JWT revocable, lockout brute-force, RBAC per endpoint, header CSP/HSTS, audit role-gated. | article |
| `/solusi/notaris` | Software Notaris: Akta Teraudit | Draf akta → review → approval → signing dalam satu alur teraudit untuk kantor notaris. | article |
| `/solusi/ppat` | Software PPAT: Verifikasi Sertifikat | Pipeline OCR + review dokumen tanah sebelum penomoran akta. | article |
| `/solusi/firma-hukum` | Manajemen Firma Hukum & Advokat | Perkara + tugas + invoice per matter + ekspor CSV pelaporan. | article |
| `/harga` | Harga LegalRust — Trial s.d. Enterprise | 4 tier: Free Trial, Professional, Business, Enterprise. Harga TBD oleh owner. Struktur bulanan+tahunan. | website |
| `/faq` | FAQ: Fitur, Keamanan & Status 501 | 8 jawaban jujur: scope live, 501 e-sign/AI/gateway/OCR-engine, keamanan, status produksi. | FAQPage companion |

Canonical: satu URL per halaman, self-canonical absolut (`<link rel="canonical" href="https://legalrust.example/...">`).
Bahasa: `<html lang="id">`, `hreflang="id"` (tambah `en` hanya setelah copy EN ditulis).

## 2. Sitemap + robots.txt (rencana)

- `sitemap.xml`: 13 URL di atas, `changefreq`: `/` weekly, `/harga` monthly, fitur/solusi monthly, `/faq` monthly. `priority`: `/` 1.0, fitur 0.8, solusi 0.8, `/harga` 0.9, `/faq` 0.7. `lastmod` = tanggal publish aktual.
- `robots.txt` rencana:
  ```
  User-agent: *
  Allow: /
  Disallow: /app
  Disallow: /api/
  Sitemap: https://legalrust.example/sitemap.xml
  ```
  `/app` dan `/api/` tidak diindeks (area login + API).

## 3. Schema.org (hanya data terverifikasi)

- `Organization`: nama "LegalRust", deskripsi generik. JANGAN isi `founder`, `revenue`, `employee count` tanpa sumber.
- `SoftwareApplication`: `applicationCategory: "BusinessApplication"`, `operatingSystem: "Web"`, `offers` hanya jika harga final diputuskan owner (saat ini OMIT `price`, atau `price: "0"` khusus Free Trial bila disetujui). Jangan klaim `aggregateRating`/`reviewCount`.
- `FAQPage`: generate dari `docs/MARKETING/seed-faqs.json` (8 Q&A, verbatim, tanpa tambahan).
- `BreadcrumbList` per halaman fitur/solusi.

## 4. Tujuh halaman topikal (outline, jujur)

1. **Software notaris** (`/solusi/notaris`): problem → alur akta template → guard 409 → audit → CTA trial. Catatan: e-sign 501.
2. **Manajemen notaris** (`/fitur/perkara` sudut notaris): antrean akta, peran Owner/Admin/Auditor, notifikasi.
3. **PPAT** (`/solusi/ppat`): checklist sertifikat, OCR+review, penomoran, arsip berversi.
4. **Perkara hukum** (`/fitur/perkara`): matter → task guarded → invoice per matter → CSV.
5. **OCR dokumen hukum** (`/fitur/ocr-verifikasi`): tipe didukung (`GET /api/v1/ocr/supported`), confidence + review, 501 engine.
6. **AI legal assistant** (`/fitur/ai-terencana` ATAU bagian dari FAQ — disarankan TIDAK jadi halaman klaim): RAG keyword live, chat 501, token usage. Hanya tulis sebagai "terencana".
7. **Manajemen dokumen legal** (`/fitur/dokumen`): SHA-pin upload, versi, share kedaluwarsa, RBAC download.

Setiap halaman: H1 satu, intro 50 kata, 3–5 H2, tabel endpoint jujur, blok "Status implementasi" (live vs 501), FAQ mini 3 item, CTA `/login`.
