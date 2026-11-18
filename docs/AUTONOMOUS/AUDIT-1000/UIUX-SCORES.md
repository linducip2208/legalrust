# UIUX-SCORES (verified 2026-11-18, headless Chrome 1440px + 390px + CDP)

Surfaces audited with screenshots: landing, login, /app shell, /fitur,
mobile 390px (CDP-measured docW=390, zero page overflow; tables scroll
inside cards by design).

| Surface | Score | Notes |
|---|---|---|
| Landing desktop | 88 | Tabler + icons verified; honest-status box; mock labeled illustrasi |
| Login | 90 | tabs, demo button (outline), roles matrix, no dup links |
| /app shell | 82 | 10 tabs, search palette, notif bell; needs login (redirect verified) |
| Marketing pages | 85 | consistent shell, meta/OG/canonical, no fake social proof |
| Mobile 390px | 80 | stacked CTAs, 2-col stats, drawer nav; tables scroll in cards |
| Accessibility | 84 | skip-link + landmarks, focus-visible, Esc/Ctrl-K, reduced-motion |

Weights per brief (visual 15, IA 15, nav 15, usability 15, responsive 10,
a11y 10, states 10, forms/tables 5, perf 5).

Fixed this wave: fabricated "Ribuan Profesional" claim → honest copy;
letter icons → real Tabler icons; duplicate login link removed;
`Rp 248 jt` nowrap; skip-link + `id=main` on 3 shells.

Open (queued, RESUME.md): /app approvals tab UI, report center UI,
per-domain workspaces (notary/PPAT/finance), empty/loading/error states
per module, PWA icon wiring verification on device.
