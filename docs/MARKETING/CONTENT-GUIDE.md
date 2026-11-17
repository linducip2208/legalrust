# LegalRust — Content Guide (ID)

> Sumber kebenaran: `docs/13-development-progress/ACCEPTANCE.md` (28/32, BUKAN production-ready)
> + daftar route di `apps/server/src/lib.rs` (`build_router`).
> Semua klaim pemasaran WAJIB merujuk pada endpoint yang benar-benar ada.
> Bahasa utama pemasaran: Bahasa Indonesia.

## 1. Suara merek (voice)

- Profesional, tenang, presisi hukum. Hindari hiperbola startup ("tercepat di dunia", "AI tercanggih").
- Selalu sebut status jujur: "uji internal 118 tes hijau", "belum production-ready", "butuh counsel sign-off".
- Contoh kalimat yang BOLEH: "Alur akta ber-guard: transisi ilegal ditolak 409".
- Contoh kalimat yang DILARANG: "Dipercaya ribuan notaris" (tidak ada datanya).

## 2. Kebijakan klaim (claims policy)

| Dilarang | Alasan | Pengganti yang jujur |
|---|---|---|
| Logo/testimoni klien palsu | Tidak ada data klien | "Dirancang untuk notaris, PPAT, firma hukum" (segmen, bukan klaim pengguna) |
| Statistik palsu ("128 perkara", "Rp 248 jt") | Angka di landing hanya ilustrasi mock | Beri caption "ilustrasi tampilan" pada setiap mock |
| "OCR otomatis penuh" | `POST /api/v1/ocr/jobs/{id}/process` → 501 tanpa engine | "OCR job + review manusia; engine live butuh libtesseract / cloud fallback" |
| "AI legal assistant live" | `POST /api/v1/ai/chat` → 501 tanpa provider; hanya RAG keyword + pencatatan token | "Pencarian keyword RAG + pencatatan token; jawaban AI butuh provider (501 saat ini)" |
| "Tanda tangan elektronik" | `POST /api/v1/deeds/{id}/esign` → 501 (PSrE belum dikonfigurasi) | "Alur akta sampai signing; e-sign PSrE terencana, saat ini 501" |
| "Pembayaran online otomatis" | `POST /api/v1/invoices/{id}/pay-gateway` → 501 | "Invoice + pembayaran manual anti-overpay; gateway terencana, saat ini 501" |
| "Terintegrasi Dukcapil/BPN" | `GET /api/v1/integrations` lapor `configured:false` tanpa secret; webhook butuh secret | "Adapter webhook siap; aktivasi per-provider butuh konfigurasi" |
| "100% siap produksi" | ACCEPTANCE 28/32, blocker B-001/B-002/B-003 | "Uji internal lulus; production & counsel review masih berjalan" |

## 3. Istilah baku (ID)

- Perkara (matter), Klien, Dokumen, Akta, Trust ledger / dana titipan klien, Invoice anti-overpay, OCR + review, Ekspor CSV, Jejak audit, RBAC.
- Mata uang default: IDR, nominal dalam minor (sen).

## 4. Aturan CTA & harga

- CTA utama: "Mulai Gratis" → `/login` (register organisasi). CTA sekunder: "Lihat Demo" → `/app`.
- Jangan tampilkan harga final. Semua harga "TBD — ditentukan owner". Lihat `PRICING-DRAFT.md`.
- Paket hanya boleh menyebut batas CONFIGURABLE (draft), bukan kuota yang di-enforce hari ini.

## 5. Checklist sebelum publish

1. Setiap klaim fitur punya pasangan endpoint di `openapi.yaml` / `lib.rs`.
2. Setiap 501 dicantumkan jujur di catatan kaki / FAQ.
3. Tidak ada angka pengguna/pendapatan tanpa sumber.
4. Mock UI berlabel ilustrasi.
