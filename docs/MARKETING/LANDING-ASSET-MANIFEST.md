# Landing Asset Manifest — every `/static` asset referenced by pages

Source of truth: `apps/server/static/` on disk + live DOM/HTTP verification 2026-11-18 against `http://127.0.0.1:8080` (all 11 pages @1600x900, headless chromium). All `<img>` returned **HTTP 200**; all carry `alt`.

## Images used on `/` (only page with `<img>`)

| File | Size | Where used | Loading | Alt |
|---|---|---|---|---|
| `hero-office.png` | 2,079,027 | `/` hero `<img class="hero-photo">` + `site.css` `.hero` background layer | eager (`fetchpriority="high"`) | YES — "Ruang kerja kantor hukum modern yang memakai LegalRust" |
| `office-interior.png` | 1,566,229 | `/` section image | lazy | YES |
| `doc-scan.png` | 860,120 | `/` section image | lazy | YES |
| `ocr-scan.png` | 1,049,915 | `/` section image | lazy | YES |
| `signature.png` | 829,532 | `/` section image | lazy | YES |
| `courthouse.png` | 1,156,361 | `/` section image | lazy | YES |
| `security.png` | 1,294,591 | `/` section image | lazy | YES |
| `ai-assist.png` | 1,350,286 | `/` section image | lazy | YES |
| `cta-building.png` | 1,440,488 | `site.css` `.cta` background only (no `<img>` anywhere) | CSS bg (lazy by nature, loads on paint) | n/a (background) |

`/fitur`, `/fitur/ocr`, `/fitur/ai-legal-assistant`, `/harga`, `/faq`, `/solusi/notaris`, `/login`, `/app`, `/kontak`, `/demo`: **zero `<img>`** (verified in DOM). No missing images site-wide (no 404; every referenced file exists).

## Shipped but unreferenced (bloat, ~3.1 MB)

| File | Size | Status |
|---|---|---|
| `library.png` | 1,064,823 | NOT referenced in any page HTML or `site.css` |
| `meeting-room.png` | 965,369 | NOT referenced in any page HTML or `site.css` |
| `server-room.png` | 1,067,783 | NOT referenced in any page HTML or `site.css` |

## Shared CSS/JS/fonts (referenced by ALL pages via `page_shell`)

| File | Size | Where used |
|---|---|---|
| `site.css` | 11,899 | all pages `<link>` |
| `site.js` | 8,189 | all pages `<script>` (login/register/demo-fill, `/app` auth guard + redirect, reveal-on-scroll) |
| `vendor/tabler.min.css` | 683,730 | all pages |
| `vendor/tabler.min.js` | 151,907 | all pages |
| `vendor/tabler-fonts/tabler-icons.min.css` | 253,439 | all pages |
| `vendor/tabler-fonts/tabler-icons.*.woff2` (54 files, ~0.9 MB total) | 1,676–41,488 each | loaded on demand via icons CSS |
| `fonts/PlusJakartaSans.ttf` | 176,288 | `site.css` `@font-face` |

## Notes

- Total image payload on `/`: ~12.6 MB PNG (8 `<img>` + CTA bg). Recommend compress/AVIF conversion — performance note, not a QA fail.
- `natW=0` observed for 4 below-fold lazy images at `networkidle` without scrolling: expected lazy-load behavior, not breakage (HTTP 200 confirmed for all).
