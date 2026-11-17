# DECISIONS (keputusan arsitektur — append-only)

## 2026-11-17/18
1. Server-rendered pages (Axum) untuk publik, bukan Leptos SSR — alasan: pola yang sudah jalan + SEO tanpa hidrasi; Leptos shell tetap ada.
2. Tabler self-host (CSS+JS+49 woff2+font lokal), nol CDN — alasan: aturan no-CDN + CSP ketat.
3. CSP `default-src 'self'` dipertahankan; semua inline CSS/JS diekstrak ke site.css/site.js.
4. TIMESTAMP MySQL dibaca via DATE_FORMAT/UNIX_TIMESTAMP (bukan NaiveDateTime) — alasan: panic decode sqlx.
5. Password min 12, sesi 8 jam, JWT secret min 32B — temuan self-audit.
6. Klaim jujur: mockup dasbor berlabel ilustrasi; statistik 128/842/256 ilustrasi; logo partner asli tidak dipakai.
7. Foto ComfyUI/SDXL primer; video WAN opsional enhancement (dibatalkan bila user pilih foto saja).
8. Nomor versi migrasi 0001–0010 sekuensial; duplikat 0009 yang tak-teraplikasi dihapus sebelum live-run.
