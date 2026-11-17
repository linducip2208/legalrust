# LegalRust — Pricing Draft (CONFIGURABLE, harga TBD oleh owner)

> DRAF — bukan harga final. Semua angka kuota di bawah adalah USULAN configurable,
> bukan limit yang di-enforce di kode hari ini. Harga: **TBD — ditentukan owner**.
> Struktur: **bulanan + tahunan** (diskon tahunan TBD oleh owner).

## Prinsip

- Tidak ada klaim "gratis selamanya" tanpa persetujuan owner. Paket gratis = "Free Trial".
- Setiap batas (pengguna, perkara, storage) harus diimplementasikan sebagai konfigurasi
  sebelum diklaim sebagai enforced. Sampai saat itu tulis "usulan batas".
- Fitur 501 (e-sign PSrE, AI provider live, billing gateway, OCR engine live)
  TIDAK BOLEH dijual sebagai "termasuk aktif" — tulis "terencana / butuh konfigurasi".

## Tier (4)

### 1. Free Trial — Rp 0 (TBD durasi oleh owner, usulan: 14 hari)

- Usulan batas: 1 organisasi · 3 pengguna · 10 perkara · 100 dokumen · 1 GB storage.
- Termasuk: perkara, dokumen berversi + share, template akta, invoice manual, CSV, audit viewer.
- Tidak termasuk: e-sign PSrE, AI provider live, billing gateway, OCR engine live (semua 501/terencana).

### 2. Professional — harga TBD/bulan & TBD/tahun (per org)

- Usulan batas: 10 pengguna · 200 perkara · 5.000 dokumen · 50 GB storage.
- Termasuk semua Free Trial + trust ledger multi-akun, API token, RAG keyword search, SSE events, jobs queue.
- 501 notes berlaku (lihat bawah).

### 3. Business — harga TBD/bulan & TBD/tahun (per org)

- Usulan batas: 50 pengguna · 2.000 perkara · 50.000 dokumen · 500 GB storage.
- Tambahan usulan: SSO (belum diimplementasikan — tulis TERENCANA), retensi audit configurable, prioritas support.
- 501 notes berlaku.

### 4. Enterprise — harga TBD (hubungi owner / custom)

- Batas custom, deployment opsi (self-host / VPC — tunduk hasil counsel & security audit B-001).
- SLA, audit log export, onboarding, review kepatuhan — semua TERENCANA, bukan klaim aktif.

## Catatan 501 (wajib tampil di halaman /harga)

| Fitur | Status |
|---|---|
| E-sign akta (`POST /api/v1/deeds/{id}/esign`) | 501 — PSrE belum dikonfigurasi |
| AI chat (`POST /api/v1/ai/chat`) | 501 — tanpa provider; token usage dicatat |
| Billing gateway (`POST /api/v1/invoices/{id}/pay-gateway`) | 501 — pembayaran manual tersedia |
| OCR process (`POST /api/v1/ocr/jobs/{id}/process`) | 501 — tanpa engine; job + review tersedia |
| Webhook provider | 501 tanpa integration secret |

## Struktur penagihan (draf)

- Siklus: bulanan (bayar tiap bulan) + tahunan (bayar di muka 12 bulan, diskon TBD).
- Mata uang: IDR. Pajak/PPN: TBD oleh owner + counsel.
- Over-quota: TBD (usulan: blokir lembut + upsell, bukan hapus data).
