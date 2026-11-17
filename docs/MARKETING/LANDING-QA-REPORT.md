# Landing QA Report — LegalRust marketing pages

Date: 2026-11-18 (UTC+7). QA scope: docs + screenshots ONLY. No code, static, or doc edits besides these two new files.
Server under test: `http://127.0.0.1:8080` (fresh `cargo build -p legalrust-server`, current `apps/server/src/lib.rs`).
Note: `http://legalrust.test/` (proxy) returned **503 Service Unavailable** for the whole QA window, so all checks ran direct. The first prebuilt binary (`target/debug`, 23:21) was STALE vs source (23:47, lean 7 KB home); it was rebuilt and QA re-ran on the fresh build (23 KB home, 15 sections).
Screenshots: `C:/Users/lindu cipta/Documents/legalrust-qa/` (12 files). Every screenshot below was visually inspected via image read.

## Method

- Playwright chromium `screenshot --full-page`: `/` @1600x900 and @390x844, plus `/fitur`, `/fitur/ocr`, `/fitur/ai-legal-assistant`, `/harga`, `/faq`, `/solusi/notaris`, `/login`, `/app`, `/kontak`, `/demo` @1600x900.
- Programmatic (playwright-core + headless shell): all 11 pages @1600 — every `<img>` HTTP status, `alt` presence, status-badge sniff; all 11 pages @390 — `document.scrollingElement.scrollWidth <= 390` + offending-element identification; `/app` final-URL check.

## Viewport matrix

| Page | Desktop 1600 (screenshot) | Mobile 390 | Notes |
|---|---|---|---|
| `/` | PASS | FAIL | 15 `<section>`, 16 anchors (see below); 8/8 imgs 200 + alt; badges present. Mobile: scrollWidth **760** (topbar + `#status` block). |
| `/fitur` | PASS | FAIL | 6 feature cards + Pelajari links. Mobile scrollWidth 447 (topbar). |
| `/fitur/ocr` | PASS | FAIL | `BUTUH KONFIGURASI` badge + endpoint table + honest 501 note. Mobile 447. |
| `/fitur/ai-legal-assistant` | PASS | FAIL | `BUTUH KONFIGURASI` badge + endpoint table + honest note. Mobile 447. |
| `/harga` | PASS | FAIL | No fake prices; honest "harga final ditentukan owner". Mobile 447. |
| `/faq` | PASS | FAIL | Honest empty state ("Belum ada FAQ yang diterbitkan") on ephemeral store. Mobile 447. |
| `/solusi/notaris` | PASS | FAIL | Honest "BUTUH KONFIGURASI, tidak diklaim live" + anti-fake-testimonial copy. Mobile 447. |
| `/login` | PASS | FAIL | Login card + demo-fill + role table (JS-driven, no `<form>` tag — by design, wired in `site.js`). Mobile 447. |
| `/app` (unauth) | PASS | FAIL | Shows login gate; final URL after JS guard = `/login`. No 302 (HTTP 200 + `location.href`, accepted). Mobile 447. |
| `/kontak` | PASS | FAIL | Real `<form id="kontak-form" action="/api/v1/public/contact">` (nama/email/subjek/pesan). Mobile 447. |
| `/demo` | PASS | FAIL | Real `<form id="demo-form" action="/api/v1/public/demo-requests">` (nama/email/perusahaan/waktu). Mobile 447. |

Totals: **22 checks → 11 PASS / 11 FAIL** (desktop 11/11 pass; mobile 0/11 pass, all same root cause).

## Home anchor spot-check (task asked 18 sections)

Found **15 `<section>`** and **16 id anchors**: `hero, dasbor, fitur, perkara, dokumen, ocr, akta, keuangan, keamanan, pelaporan, alur, solusi, status, kepercayaan, faq, cta`. All render in order desktop + stacked mobile. "18" does not match source; coverage of every promised area is complete, so recorded as INFO, not fail.

## No-fake-content verification — PASS

- Trust bar lists profession *categories* (NOTARIS / PPAT / FIRMA HUKUM / KORPORAT / ADVOKAT), no fake company logos.
- Dashboard mock labeled "pratinjau dasbor (ilustrasi)"; copy states "Angka pada mockup adalah ilustrasi tampilan, bukan data produksi".
- Home hero carries honest strip: "uji internal 28/32 — bukan production-ready" + explicit 501 endpoint list.
- Status badges verified in DOM: `BUTUH KONFIGURASI` (ocr, ai, solusi/notaris) + `TERSEDIA TERBATAS` (home map). Feature pages without badges (`/fitur`, `/harga`, `/faq`) make no live-ness claims needing one — accepted.

## Issues

1. [HIGH] Horizontal overflow at 390px, ALL pages: topbar desktop nav + `div.topact` (Masuk/Coba Gratis) do not collapse — scrollWidth 447 (nav links reach x=575). FIX REQUIRED: hamburger/wrap for `.topbar` under ~640px.
2. [MEDIUM] Home `#status` endpoint/status block overflows to **760px** (long unbroken endpoint `<code>` strings). FIX REQUIRED: `overflow-wrap:anywhere` / responsive table wrapper.
3. [LOW, accepted] `/app` unauth returns HTTP 200 + client-side `location.href='/login'` (site.js) instead of 302. Functionally gates correctly (verified final URL `/login`); no data leaks (dashboard shell is empty until authed API calls). Accept or harden server-side later.
4. [INFO, accepted] "18 sections" vs actual 15 sections/16 anchors — descriptive mismatch only.
5. [INFO, accepted] `/faq` empty state is data-driven (ephemeral memory store, no DB seeded). Page handles it honestly.
6. [INFO] 3 shipped-but-unreferenced PNGs (~3.1 MB): `library.png`, `meeting-room.png`, `server-room.png`. See ASSET-MANIFEST. Recommend removal or use; accepted as-is for QA.
7. [ENV, accepted] `legalrust.test` proxy 503 during QA + stale prebuilt binary at start; methodology above documents the rebuild. No product impact.
