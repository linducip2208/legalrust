# 02 — Panduan Portal /app (Dashboard Guide)

Portal utama adalah aplikasi server-rendered di route `/app` (bukan shell
Leptos di `apps/web` — itu prototipe komponen yang teruji unit, bukan UI
produksi). Semua data dimuat via `fetch` ke API dengan token JWT dari
`localStorage`; tanpa token → redirect ke `/login`. Tidak ada logika bisnis
di JavaScript — hanya fetch + render.

Login: tab **Masuk** / **Daftar organisasi**, tombol **Gunakan akun demo**
mengisi kredensial demo, proteksi brute-force 5x/5 menit, link
**Lupa kata sandi?** → `/faq`.

## 1. Tab aplikasi (`/app`, sidebar + tab atas)

| Tab | Panel | Isi |
|---|---|---|
| Dashboard | `p-ringkas` | KPI + tabel Perkara Aktif + kalender |
| Klien & Perkara | `p-klien` | Klien, perkara, tugas (transisi aman), proyek |
| Dokumen | `p-dok` | Pustaka, versi, share 1–720 jam, ekspor CSV |
| Verifikasi Identitas / OCR | `p-ocr` | Job OCR (KTP/KK/NPWP), review KYC approve/reject, ekstraksi |
| Akta | `p-akta` | Template berversi, proses akta, e-sign (PSrE-gated) |
| Keuangan | `p-keu` | Invoice, pembayaran manual, checkout Tripay, trust ledger |
| Marketing | `p-mkt` | Pipeline leads (filter/advance/assign/notes), demo, kontak, newsletter, pricing toggle, FAQ add, articles publish, CSV export |
| Pengaturan | `p-pengaturan` | Token API, ganti kata sandi, peran, ekspor CSV |
| Audit | `p-audit` | Jejak audit (role-gated: Owner/Admin/Auditor) |
| Jobs | `p-jobs` | Antrean job tahan lama + worker |

Navigasi keyboard: `Ctrl/⌘+K` fokus ke pencarian global (`/api/v1/search`,
min. 2 huruf, hasil per klien/matter/dokumen); `Esc` menutup hasil/notifikasi.
Sidebar responsif (hamburger di ≤960px). Header menampilkan organisasi aktif,
lonceng notifikasi in-app, dan menu akun (Keluar mencabut sesi).

## 2. API di balik portal (route persis, `apps/server/src/lib.rs`)

- `GET /api/v1/orgs/me` — profil org sesi berjalan.
- `POST /api/v1/clients` / `GET /api/v1/clients` — buat & daftar klien.
- `POST /api/v1/matters` / `GET /api/v1/matters` — buat & daftar matter.
- `POST /api/v1/matters/{mid}/tasks` / `GET /api/v1/matters/{mid}/tasks` — tugas per matter.
- `POST /api/v1/tasks/{tid}` — patch satu tugas.
- `GET /api/v1/search?q=` — pencarian global tenant-scoped.
- `GET /api/v1/notifications` — daftar notifikasi; `POST /api/v1/notifications/{id}/read` — tandai dibaca.
- `GET /api/v1/audit` — jejak audit (lihat aktivitas portal).
- `GET /api/v1/events` — Server-Sent Events; event pertama selalu `hello`, lalu event live.
- `GET /api/v1/reports/matters-csv` dan `GET /api/v1/reports/matters.csv` — ekspor CSV matter (`id,number,title,status`).

## 3. Yang BELUM ada di portal (jujur)

- AI chat, RAG, webhook, integrasi belum punya panel UI — hanya via API
  (lihat bab panduan masing-masing).
- Statistik dashboard memakai data live dari API; angka ilustrasi hanya ada
  di halaman marketing (diberi label "ilustrasi tampilan").
