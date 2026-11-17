# Responsive QA — Marketing Pages

Date: 2026-11-18. Scope: CSS-only fix in `apps/server/static/site.css`
(appended "responsive QA pass" block). No Rust/JS/HTML/template edits.
Server: `http://127.0.0.1:8080`. Tool: Playwright (Chromium) viewport
sweeps + `scrollWidth <= innerWidth` assert + per-element right-edge audit
(elements extending past the viewport are classified as inside a legitimate
horizontal scroller vs. clipped/unreachable).

## Result matrix (post-fix verification, 60/60 PASS)

| Page | 1920 | 1366 | 1024 | 768 | 390 | 360 |
|---|---|---|---|---|---|---|
| `/` | PASS | PASS | PASS | PASS | PASS* | PASS* |
| `/fitur` | PASS | PASS | PASS | PASS | PASS | PASS |
| `/fitur/ocr` | PASS | PASS | PASS | PASS | PASS | PASS |
| `/harga` | PASS | PASS | PASS | PASS | PASS | PASS |
| `/faq` | PASS | PASS | PASS | PASS | PASS | PASS |
| `/solusi/notaris` | PASS | PASS | PASS | PASS | PASS | PASS |
| `/kontak` | PASS | PASS | PASS | PASS | PASS | PASS |
| `/demo` | PASS | PASS | PASS | PASS | PASS | PASS |
| `/login` | PASS | PASS | PASS | PASS | PASS | PASS |
| `/artikel` | PASS | PASS | PASS | PASS | PASS | PASS |

`*` `/` at 390/360: zero page-level overflow and zero clipped elements.
9 (390) / 17 (360) table elements intentionally live inside Tabler
`.table-responsive` horizontal scrollers in the showcase `.mini` cards
(reachable via swipe/scroll — the prescribed pattern for data tables).

Pre-fix baseline: `/` FAILED at 1366 (`scrollWidth` 1454) and 1024
(`scrollWidth` 1341); all other pages passed `scrollWidth` at all widths,
but `/` content was visibly cut off at 390/360 (hero paragraph clipped
mid-word) because the pre-existing `.container-xl{overflow-x:clip}`
rule (max-width:960px) masked inner overflow instead of fixing it.

## Root causes found

1. `section#status .legend` grid (`repeat(4,1fr)`) on `/`: cell 3 contains
   the unbreakable `<code>CONFIGURATION_REQUIRED</code>`; `1fr` tracks use
   `minmax(auto,1fr)` so the min-content word forced the grid to
   ~1341–1454px at 1024–1366px viewports (page-level horizontal scroll).
2. Grid items missing `min-width:0`: `.hero-in` single-column track at
   <=960px could not shrink below content min (video/poster + lead
   `max-width:510px`), so hero text was clipped by the `overflow-x:clip`
   safety net at 390/360 instead of wrapping.
3. 4-across grids (`.legend`, `.steps4`) never collapsed between
   961–1200px; hero stayed 2-col until 960px (cramped at 1024).

## Fixes applied (site.css only)

- Global: `img,video,svg{max-width:100%}`; `min-width:0` on all children
  of `.hero-in/.split/.dgrid/.stat4/.fgrid/.steps4/.legend/.cta/.mock-body/.trust`.
- `.legend code{word-break:break-all}` + `.legend>div,.honest{overflow-wrap:anywhere}`
  (long status labels / endpoint URLs can no longer force width).
- `.mini{overflow-x:auto}` wrapper hardening; `.mini table{min-width:560px}`
  at <=960px so showcase tables scroll inside their card instead of crushing.
- Breakpoint 1200px: `.hero-in` stacks to 1fr; `.legend` and `.steps4` to
  2 cols; `.cta` to 1 col; `.topbar` wraps; `.nav` gaps tighten.
- Breakpoint 960px: additive (existing rules kept — `.hero-in/.split/.steps4/.fgrid/.dgrid`
  to 1fr, `.stat4` to 2 cols, `.mock-body` to 1fr with `.side` hidden,
  global `table{display:block;overflow-x:auto}` already covered tables).
- Breakpoint 640px: `.legend/.steps4` to 1 col; `.trust` and `footer.site`
  stack vertically; `.cta-row` buttons full-width; tighter section padding.
- Breakpoint 400px: `.stat4` to 1fr; hero `h1` 32px; `.cta` padding/margin
  tighten; `.logo` 20px; `.topact` full-width split buttons.
- Nav: hamburger (`.mnav`, Tabler `d-md-none`) already covers <768px and the
  desktop `.nav` (Tabler `d-md-flex !important`) covers >=768px — verified no
  768–960px nav gap; no change needed. No new `!important` overrides added.

## Evidence

- `docs/MARKETING/responsive-shots/verify.json` — 60-row machine assert
  (`sw`, `maxRight`, clipped/in-scroller counts per page × viewport).
- `docs/MARKETING/responsive-shots/_390.png … _360.png` — post-fix mobile
  screenshots for all 10 pages (above-the-fold).
- Pre-fix full-viewport set retained in `target-ui5/resp-shots/` (60 shots +
  `overflow.json` baseline) for before/after comparison.

## Remaining risks / notes

- `.table-responsive` tables on `/` showcase require horizontal swipe at
  <=~640px by design; consider card-list layouts later (needs HTML work,
  out of scope for this CSS-only pass).
- The pre-existing `.container-xl{overflow-x:clip}` (<=960px) safety net was
  kept; post-fix element audit confirms it no longer masks clipped content
  (clipped count = 0 everywhere), but it would hide future regressions from
  naive `scrollWidth` checks — keep running the per-element audit.
- LongVOC: `.stat4` numbers (`font-size:19px`) fit at 2-col/390px but have
  little margin; if stat labels grow, drop to 1 col at 640px.
- Test server (`target/debug/legalrust-server.exe`) does not survive between
  sessions unsupervised; re-run `verify` with Playwright after any CSS change.
