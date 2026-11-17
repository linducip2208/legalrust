# NILAI FITUR - 1000 katalog (registry FEATURE-REGISTRY.json)

Skor = IMPLEMENTED 100 / PARTIAL 55 / CONFIG_REQUIRED 30 / PLANNED 5 / NOT_STARTED 0.

## Skor keseluruhan: 15.8/100 (grade E)

| Domain | Nama | Skor | Grade |
|---|---|---|---|
| D01 | D01 | 16.8 | E |
| D02 | D02 | 14.9 | E |
| D03 | D03 | 19.4 | E |
| D04 | D04 | 29.8 | E |
| D05 | D05 | 27.5 | E |
| D06 | D06 | 21.6 | E |
| D07 | D07 | 25.4 | E |
| D08 | D08 | 8.9 | E |
| D09 | D09 | 3.8 | E |
| D10 | D10 | 23.4 | E |
| D11 | D11 | 5.2 | E |
| D12 | D12 | 6.5 | E |
| D13 | D13 | 8.5 | E |
| D14 | D14 | 11.5 | E |
| D15 | D15 | 13.5 | E |
| D16 | D16 | 18.9 | E |
| D17 | D17 | 8.2 | E |
| D18 | D18 | 17.2 | E |
| D19 | D19 | 20.8 | E |
| D20 | D20 | 8.2 | E |
| D21 | D21 | 27.2 | E |
| D22 | D22 | 24.2 | E |
| D23 | D23 | 19.2 | E |
| D24 | D24 | 4.2 | E |
| D25 | D25 | 10.6 | E |

## Cara naik grade (jujur, tanpa klaim palsu)
- CONFIGURATION_REQUIRED -> IMPLEMENTED hanya dengan kredensial/infra live + verifikasi.
- PLANNED/NOT_STARTED -> IMPLEMENTED hanya lewat implementasi nyata + tes hijau.
- Nilai dihitung ulang otomatis dari registry setiap wave (Wave 7 re-audit: 0 mismatch).

## Metode skor UI/UX 1000/100 (program UIUX-0001..UIUX-1000)

Sumber item: docs/AUTONOMOUS/UIUX-1000-TODO.md (generator: docs/AUTONOMOUS/gen_uiux_todo.py).
Alokasi: A) 141 fitur IMPLEMENTED/PARTIAL x 5 cek (renders/empty/loading/error/mobile390) = 705;
B) global 150 (120 sweep 20 halaman x 6 breakpoint 1920/1366/1024/768/390/360 + 30 nav-peran/keyboard/focus-visible/kontras/reduced-motion/form/toast);
C) susulan QA + gap site.css 145 (66 residual 11 halaman + 45 gap CSS + 34 misc). Totaltepat 1000.

Aturan skor (raw + skala):
- Setiap item bernilai 1 poin: DONE = 1, TODO/BLOCKED = 0.
- UI/UX raw = completed/1000 (mis. 250 DONE -> raw 250/1000).
- UI/UX maturity (skala 100) = completed/10 (mis. 250 DONE -> 25.0/100).
- Kedua angka dicatat berdampingan setiap wave; maturity TIDAK menggantikan raw.
- Kriteria DONE: bukti screenshot/verify.json per baris (kolom Status -> DONE + bukti).

Baseline wave-0 (2026-11-18): raw 0/1000, maturity 0.0/100 (seluruh item TODO).
Pelacakan: gelombang UIUX-01..UIUX-10 di docs/AUTONOMOUS/TASK-QUEUE.json (100 item per wave, rantai UIUX-01->...->UIUX-10).