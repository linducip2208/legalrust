# Counsel Sign-off Pack — Acceptance Readiness (Indonesia)

Status: DRAFT — BELUM ada sign-off counsel. Dokumen ini BUKAN nasihat hukum dan TIDAK memuat kesimpulan hukum. Semua kotak di bawah TIDAK dicentang sampai counsel menandatangani.

Tujuan pack ini: memberi counsel satu bundel fakta "sistem apa adanya" + daftar pertanyaan spesifik yang jawabannya menentukan boleh/tidaknya launch produksi dengan data klien nyata.

Dokumen checklist yang berlaku (jangan diubah): `docs/33-LEGAL-REVIEW/COUNSEL-CHECKLIST.md` — semua 8 item di sana masih `[ ]` (unchecked).

## 1. Tujuan

1. Meminta opini/putusan tertulis counsel atas 6 pertanyaan spesifik di Bagian 4 (konteks Indonesia: UU ITE beserta perubahannya, UU PDP No. 27/2022, UUJN/jabatan notaris/PPAT, PSrE/TTE tersertifikasi).
2. Mendokumentasikan 7 blocker terbuka yang menghalangi acceptance produksi (Bagian 3), masing-masing dengan bukti file agar counsel dapat memverifikasi sendiri.
3. Menegaskan batas: tidak ada data klien nyata yang boleh masuk produksi sebelum putusan counsel + penutupan blocker terkait.

## 2. Ruang lingkup sistem — apa adanya (tidak diinflasi)

Sumber: `docs/13-development-progress/ACCEPTANCE.md`.

- Skor acceptance: 25/32 item DONE + tested = 78/100 — dinyatakan eksplisit "NOT production-ready" di file tersebut.
- DONE + tested (25): PRD, arsitektur, skema, migrasi 0001–0009, auth (JWT+API token+sesi+revokasi), manajemen org, penegakan RBAC, klien, pipeline identity-verify (OCR job+review), document engine (DMS+versi+share+sha-pinning), projects/matters, tasks (transisi terjaga), workflow notariil (state machine), deeds (template/render/transisi), AI accounting+RAG baseline kata kunci, notifikasi (inbox in-app), finance (invoice/pembayaran/overpay guard + trust ledger double-entry), jobs queue, SSE events, API docs (`openapi.yaml`, 60+ routes), audit trail (writer + viewer berbasis peran), kontrol keamanan (header/rate-limit/RBAC/Argon2/JWT), pelaporan (CSV), pengujian otomatis (89 green), CI/CD, docs deployment, portal UI + design system (komponen Leptos + 12 tests).
- PARTIAL (5): OCR engine live, AI provider live, backup restore DRILL (skrip ada), user manual (ikut dengan UI), final security audit (self-audit saja).
- MISSING (2): audit performa dengan baseline, deployment produksi + migrasi live-DB.
- Guardrail konservatif yang saat ini diberlakukan (per `docs/33-LEGAL-REVIEW/C1.md` s.d. `C7.md`): kehadiran fisik (physical-presence) untuk akta; e-sign diblokir untuk akta notariil/PPAT; notaris diperlakukan sebagai controller. Guardrail ini adalah asumsi desain internal, BUKAN opini hukum — counsel diminta mengonfirmasi/mengoreksi.
- Dossier pertanyaan terbuka C1–C7 (masing-masing status "PERTANYAAN TERBUKA, bukan nasihat hukum"):
  - `docs/33-LEGAL-REVIEW/C1.md` — validitas akta elektronik / kehadiran virtual.
  - `docs/33-LEGAL-REVIEW/C2.md` — ruang lingkup pengecualian Pasal 5(4) UU ITE.
  - `docs/33-LEGAL-REVIEW/C3.md` — kedudukan notaris sebagai pengendali data (UU PDP).
  - `docs/33-LEGAL-REVIEW/C4.md` — protokol 25 tahun vs hak penghapusan.
  - `docs/33-LEGAL-REVIEW/C5.md` — bobot pembuktian TTE PSrE.
  - `docs/33-LEGAL-REVIEW/C6.md` — validitas pengarsipan/penyerahan elektronik BPN/PPAT.
  - `docs/33-LEGAL-REVIEW/C7.md` — transfer data lintas negara.

## 3. Daftar 7 blocker terbuka + bukti file

Setiap baris dapat diverifikasi di file yang dikutip. Tidak ada yang dinyatakan tertutup.

| # | Blocker | Status jujur | Bukti file |
|---|---------|--------------|------------|
| B-001 | Sign-off counsel atas item UUJN/ITE/PDP | OPEN — belum ada opini/ttd counsel | `docs/33-LEGAL-REVIEW/COUNSEL-CHECKLIST.md` (semua `[ ]`); `docs/25-GAP-ANALYSIS/GAP_REGISTER.md` (G-011 OPEN → B-001); `docs/24-AUDIT/MASTER_AUDIT_REPORT.md` (F6 OPEN, B-001); `docs/28-QUALITY-REPORT/FINAL_SCORECARD.md` (B-001 pending) |
| B-002 | Benchmark OCR terukur pada korpus ID | OPEN — protokol/spesifikasi ada, eksekusi + skor BELUM ada (tidak difabrikasi) | `docs/30-BENCHMARK-PROTOCOLS/OCR_BENCHMARK_PROTOCOL.md` (baris 1: eksekusi = B-002); `docs/24-AUDIT/MASTER_AUDIT_REPORT.md` (F5 OPEN, B-002/B-003); `docs/25-GAP-ANALYSIS/GAP_REGISTER.md` (G-009 OPEN → B-002) |
| B-003 | Evaluasi AI terukur (groundedness/refusal) | OPEN — protokol ada, eksekusi BELUM ada | `docs/30-BENCHMARK-PROTOCOLS/AI_EVAL_PROTOCOL.md` (baris 1: eksekusi = B-003); `docs/24-AUDIT/MASTER_AUDIT_REPORT.md` (F5 OPEN, B-002/B-003); `docs/25-GAP-ANALYSIS/GAP_REGISTER.md` (G-010 OPEN → B-003) |
| B-004 | Persetujuan implementasi eksplisit (chief architect) | OPEN — belum ada persetujuan | `docs/24-AUDIT/MASTER_AUDIT_REPORT.md` (F7 OPEN, B-004); `docs/29-IMPLEMENTATION-READINESS/NEXT_STEPS.md` (re-score ≥85 → minta B-004); `docs/29-IMPLEMENTATION-READINESS/IMPLEMENTATION_BACKLOG.md` (entry rule: tidak mulai tanpa B-004); `docs/20-PROGRESS/TRACKER.md` (B-001–B-004 masih open, STOP upheld) |
| BL-DB-01 | Live-run migrasi + login berbasis MySQL live | BLOCKED — tidak ada MySQL live di environment ini; kode selesai, belum terverifikasi live | `docs/13-development-progress/BLOCKERS.md` (baris 2); `docs/13-development-progress/ACCEPTANCE.md` (MISSING: production deployment + live-DB migration run) |
| BL-EXT-01 | Kredensial provider eksternal (Dukcapil/BPN/PSrE/AI) | BLOCKED — adapter interface-complete, aktivasi menunggu kredensial | `docs/13-development-progress/BLOCKERS.md` (baris 3); `docs/13-development-progress/ACCEPTANCE.md` (PARTIAL: OCR engine live, AI provider live) |
| BL-LIC-01 | Keputusan lisensi (LICENSE file ditahan) | OPEN — menunggu intent pemilik (MIT vs proprietary) | `docs/13-development-progress/BLOCKERS.md` (baris 4) |

Catatan tambahan: `docs/13-development-progress/ACCEPTANCE.md` (baris 10) merangkum "Blockers unchanged: B-001 counsel, B-002/B-003 measurements, live DB run, DR drill, LICENSE decision"; `docs/29-IMPLEMENTATION-READINESS/DEVELOPMENT_READINESS.md` mengonfirmasi missing yang sama; `docs/13-development-progress/ROADMAP.md` mencatat B-004 authorized-track dan B-001..B-003 paralel.

## 4. Yang diminta dari counsel — 6 pertanyaan spesifik

Counsel diminta menjawab tertulis dengan dasar pasal/regulasi + bukti/dokumen yang harus disimpan sistem + daftar fitur yang boleh dibuka/ditutup. Kolom Putusan/Tanggal/Tanda WAJIB diisi counsel — tim tidak boleh mengisi.

| # | Pertanyaan untuk counsel | Dasar konteks Indonesia | Putusan counsel (diisi counsel) | Tanggal | Nama / Firma / Tanda |
|---|--------------------------|-------------------------|----------------------------------|---------|----------------------|
| Q1 | Jenis akta/dokumen mana yang WAJIB tetap akta otentik di hadapan notaris/PPAT (kehadiran fisik) dan mana yang sah elektronik? Apakah asumsi guardrail "physical-presence untuk akta; e-sign diblokir untuk akta notariil/PPAT" sudah benar atau perlu dikoreksi? | UUJN (jabatan notaris/PPAT), KUHPerdata 1868, UU ITE Pasal 5(4); dossier `docs/33-LEGAL-REVIEW/C1.md`, `C2.md`, `C6.md` | [ ] Disetujui apa adanya / [ ] Dikoreksi (tulis koreksi): ____ | ____ | ____ |
| Q2 | Untuk dokumen non-akta: mana yang sah dengan TTE tersertifikasi vs tidak tersertifikasi, dan bukti apa (hash binding, audit trail, retensi log, TSA/LTV) yang harus disimpan agar memenuhi syarat pembuktian elektronik? | UU ITE Pasal 11–12 + regulasi PSrE/PSTE (PP 71/2019); dossier `docs/33-LEGAL-REVIEW/C2.md`, `C5.md` | [ ] Disetujui / [ ] Dikoreksi: ____ | ____ | ____ |
| Q3 | Apakah notaris/PPAT pengguna sistem berkedudukan sebagai pengendali data penuh menurut UU PDP No. 27/2022 (wajib ROPA/DPIA/DPO, pemenuhan hak subjek data, notifikasi pelanggaran 72 jam) atau ada kekhususan pejabat umum/sumpah jabatan? Dasar hukum + kewajiban yang berlaku apa? | UU PDP No. 27/2022 (berlaku penuh Okt 2024); dossier `docs/33-LEGAL-REVIEW/C3.md` | [ ] Disetujui / [ ] Dikoreksi: ____ | ____ | ____ |
| Q4 | Bagaimana hierarki yang benar antara kewajiban penyimpanan protokol/minuta (UUJN ±25 tahun) dan hak penghapusan/keberatan subjek data (UU PDP)? Mekanisme penolakan/pembatasan yang sah + dokumentasinya seperti apa, dan bagaimana jadwal retensi/penghapusan aman sistem harus ditulis? | UUJN vs UU PDP (hak DSAR/penghapusan); dossier `docs/33-LEGAL-REVIEW/C4.md`; checklist `docs/33-LEGAL-REVIEW/COUNSEL-CHECKLIST.md` (retensi & deletion) | [ ] Disetujui / [ ] Dikoreksi: ____ | ____ | ____ |
| Q5 | Di mana data (MySQL/S3/minio, backup, log audit) boleh di-hosting, dan syarat apa yang berlaku untuk transfer lintas negara / pemrosesan data WNA/pihak asing (termasuk pengungkapan region backup)? | UU PDP (transfer lintas negara), kewajiban residensi/transfer; dossier `docs/33-LEGAL-REVIEW/C7.md`; checklist `docs/33-LEGAL-REVIEW/COUNSEL-CHECKLIST.md` (data residency) | [ ] Disetujui / [ ] Dikoreksi: ____ | ____ | ____ |
| Q6 | Apakah matriks RBAC, penanganan privilege advokat–klien, dan logging akses istimewa saat ini cukup untuk kewajiban kerahasiaan/sumpah jabatan dan pembuktian di pengadilan? Kontrol/logging tambahan apa yang wajib sebelum launch dengan data klien nyata? | Sumpah/kerahasiaan jabatan (UUJN), UU ITE (admissibility, consent log, retensi audit trail); checklist `docs/33-LEGAL-REVIEW/COUNSEL-CHECKLIST.md` (RBAC & privilege); `docs/13-development-progress/ACCEPTANCE.md` (RBAC enforcement + audit trail DONE-tested, bukan opini hukum) | [ ] Disetujui / [ ] Dikoreksi: ____ | ____ | ____ |

## 5. Pernyataan kejujuran (tim)

- [ ] Semua kotak counsel di `docs/33-LEGAL-REVIEW/COUNSEL-CHECKLIST.md` masih TIDAK dicentang pada saat pack ini ditulis. (Fakta — terverifikasi dari file.)
- [ ] Skor acceptance 78/100, status NOT production-ready — dikutip dari `docs/13-development-progress/ACCEPTANCE.md`, bukan penilaian baru.
- [ ] B-002/B-003 belum dieksekusi; tidak ada skor benchmark/eval yang difabrikasi — sesuai `docs/30-BENCHMARK-PROTOCOLS/OCR_BENCHMARK_PROTOCOL.md`, `docs/30-BENCHMARK-PROTOCOLS/AI_EVAL_PROTOCOL.md`, `docs/24-AUDIT/MASTER_AUDIT_REPORT.md`.
- [ ] BL-DB-01/BL-EXT-01/BL-LIC-01 masih BLOCKED/OPEN — dikutip dari `docs/13-development-progress/BLOCKERS.md`.
- [ ] Pack ini tidak memuat kesimpulan hukum; setiap interpretasi UU ITE/PDP/UUJN/PSrE menunggu putusan counsel pada tabel Q1–Q6 di atas.
