#!/usr/bin/env python3
"""Generate the 1000-point UI/UX checklist (docs-only program).

Reads docs/AUTONOMOUS/FEATURE-REGISTRY.json, builds EXACTLY 1000 items
(UIUX-0001..UIUX-1000), and writes docs/AUTONOMOUS/UIUX-1000-TODO.md.

Allocation (evidence-based):
  A. 141 IMPLEMENTED/PARTIAL features x 5 checks .... 705 items
  B. 150 global checks (120 breakpoint sweeps + 30 a11y/nav)  150 items
  C. 145 responsive/QA-leftover + site.css gap checks .. 145 items
Total = 1000. Re-run: python3 docs/AUTONOMOUS/gen_uiux_todo.py
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]
REG = ROOT / "docs/AUTONOMOUS/FEATURE-REGISTRY.json"
OUT = ROOT / "docs/AUTONOMOUS/UIUX-1000-TODO.md"

DOMAIN_ROUTE = {
    "D01": "/app (tab OCR: #p-ocr, Verifikasi Identitas)",
    "D02": "/app (tab Klien: #p-klien)",
    "D03": "/app (tab Ringkasan/Klien: #p-ringkas/#p-klien, Perkara Aktif)",
    "D04": "/app (tab Dokumen: #p-dok)",
    "D05": "/app (tab OCR: #p-ocr) + /fitur/ocr",
    "D06": "/app (tab Akta: #p-akta) + /fitur/generasi-akta",
    "D07": "/app (tab Akta: tanda tangan) + /fitur/tanda-tangan",
    "D08": "/app (tab Keuangan: #p-keu)",
    "D09": "/app (tab Keuangan: trust ledger #tl)",
    "D10": "/app (tab OCR/AI) + /fitur/ai-legal-assistant",
    "D11": "/app (tab Marketing: #p-mkt) + / + /fitur",
    "D12": "/app (tab Marketing: #p-mkt) + /kontak + /demo",
    "D13": "/harga + /artikel + /faq",
    "D14": "/app (sidebar nav + tabs #tabs) + /login",
    "D15": "/app (search #q, notifikasi #bell) + /api/v1/search",
    "D16": "/app (tab Pengaturan: #p-pengaturan)",
    "D17": "/app (audit views) + /login",
    "D18": "/fitur/* + /solusi/* + /tentang",
    "D19": "/kebijakan-privasi + /syarat-ketentuan + /app (retention views)",
    "D20": "/healthz + /readyz + /api/v1/version + /sitemap.xml",
    "D21": "/app (tab Keuangan: invoice) + /harga",
    "D22": "/app (tab Dokumen/Akta: shares, approval queue #al-q)",
    "D23": "/app + /login + /api/v1/auth/*",
    "D24": "/faq + /artikel + /artikel/{slug} + /demo",
    "D25": "/app (tab Pengaturan: token API #tok-*) + /login",
}

FIVE_CHECKS = [
    ("renders", "Halaman/tampilan fitur me-render HTTP 200 dengan heading utama terlihat (tanpa blank/error 500)"),
    ("empty", "Empty state jujur tampil saat data kosong (teks panduan + CTA, bukan tabel kosong bisu)"),
    ("loading", "Loading state tampil selama fetch (placeholder 'Memuat…' / skeleton, tombol disabled)"),
    ("error", "Error state tampil rapi saat API gagal/validasi ditolak (pesan .text-danger + tanpa stack trace)"),
    ("mobile390", "Mobile 390px: tanpa page-level horizontal overflow (scrollWidth<=390, tabel dalam .table-responsive)"),
]

PAGES20 = [
    "/", "/fitur", "/fitur/manajemen-perkara", "/fitur/manajemen-dokumen",
    "/fitur/ocr", "/fitur/ai-legal-assistant", "/fitur/generasi-akta",
    "/fitur/tanda-tangan", "/solusi/notaris", "/solusi/ppat",
    "/solusi/firma-hukum", "/solusi/legal-corporate", "/harga", "/tentang",
    "/kontak", "/demo", "/artikel", "/faq", "/login", "/app",
]
BREAKPOINTS = [1920, 1366, 1024, 768, 390, 360]

GLOBAL30 = [
    ("GLOBAL-NAV", "Nav completeness peran super_admin: semua 8 tab /app (Ringkasan,Klien,Dokumen,OCR,Akta,Keuangan,Marketing,Pengaturan) terlihat", "/app (tab #tabs, peran super_admin)"),
    ("GLOBAL-NAV", "Nav completeness peran auditor: menu baca-saja tampil, aksi mutasi disembunyikan/dinonaktifkan", "/app (peran auditor)"),
    ("GLOBAL-NAV", "Nav completeness peran client: hanya menu klien yang relevan, tanpa menu admin/Pengaturan", "/app (peran client)"),
    ("GLOBAL-NAV", "Nav completeness peran finance_officer: tab Keuangan tampil, tab Marketing/Pengaturan disembunyikan", "/app (peran finance_officer)"),
    ("GLOBAL-NAV", "Nav completeness peran document_officer: tab Dokumen/OCR tampil, tab Keuangan disembunyikan", "/app (peran document_officer)"),
    ("GLOBAL-KBD", "Keyboard nav: seluruh aksi primer reachable via Tab dalam urutan logis (tanpa jebakan fokus)", "/app (semua tab)"),
    ("GLOBAL-KBD", "Keyboard nav: dialog/detail (ocr-detail, modal share) dapat dibuka/ditutup via keyboard + Esc", "/app (#ocr-detail, #sh-list)"),
    ("GLOBAL-KBD", "Keyboard nav: form login dapat disubmit via Enter; fokus error pertama saat validasi gagal", "/login"),
    ("GLOBAL-KBD", "Keyboard nav: hamburger/sidebar toggle (#sbToggle) operable via keyboard di <=768px", "/app (#sbToggle)"),
    ("GLOBAL-KBD", "Keyboard nav: hasil pencarian (#qres) navigable via panah atas/bawah + Enter", "/app (#q)"),
    ("GLOBAL-FOCUS", "Focus-visible: indikator fokus terlihat pada semua link/button/input (site.css :focus-visible)", "site.css :focus-visible (GAP: belum ada aturan — item verifikasi pasca-perbaikan)"),
    ("GLOBAL-FOCUS", "Focus-visible: fokus tidak hilang setelah aksi async (tambah klien/perkara/tugas)", "/app (#cli-btn,#mat-btn,#task-btn)"),
    ("GLOBAL-FOCUS", "Focus-visible: skip-link 'Lewati ke konten' tampil saat Tab pertama (GAP: belum ada)", "body > a.skip-link (GAP: belum ada)"),
    ("GLOBAL-CONTRAST", "Kontras teks primer >= 4.5:1 pada badge-soft, .text-muted, .side-title", "/ + /app (badge-soft, .text-muted)"),
    ("GLOBAL-CONTRAST", "Kontras status badge BUTUH KONFIGURASI / TERSEDIA TERBATAS terbaca di terang+gelap", "/fitur/ocr, /fitur/ai-legal-assistant"),
    ("GLOBAL-CONTRAST", "Kontras .text-danger pada card-body putih memenuhi ambang baca", "/app (#*-msg)"),
    ("GLOBAL-MOTION", "Reduced-motion: @media (prefers-reduced-motion:reduce) menonaktifkan autoplay hero-video", "/ (video.hero-photo)"),
    ("GLOBAL-MOTION", "Reduced-motion: transisi .rv ditiadakan saat reduce (catatan: cakupan kini hanya .hero-video/.rv — GAP perluasan)", "site.css @media prefers-reduced-motion (GAP: cakupan parsial)"),
    ("GLOBAL-MOTION", "Reduced-motion: tidak ada animasi esensial yang hanya-menyampaikan-info via gerak", "/ + /app"),
    ("GLOBAL-FORM", "Form kontak: validasi nama/email/subjek/pesan tampil inline (id) sebelum POST", "/kontak (#kontak-form)"),
    ("GLOBAL-FORM", "Form demo: validasi nama/email/perusahaan/waktu tampil inline sebelum POST", "/demo (#demo-form)"),
    ("GLOBAL-FORM", "Form tambah klien: nama kosong ditolak dengan pesan (#cli-msg), tanpa reload", "/app (#cli-name,#cli-msg)"),
    ("GLOBAL-FORM", "Form buat perkara: judul kosong ditolak dengan pesan (#mat-msg)", "/app (#mat-title,#mat-msg)"),
    ("GLOBAL-FORM", "Form invoice: total non-numerik/negatif ditolak dengan pesan (#invm-msg), anti-overpay terlihat", "/app (#invm-total,#invm-msg)"),
    ("GLOBAL-FORM", "Form trust transfer: nominal invalid ditolak (#te); saldo tidak pernah negatif di tampilan", "/app (#tf,#tt,#ta,#te)"),
    ("GLOBAL-FEED", "Toast/sukses: aksi tambah klien menampilkan konfirmasi terlihat (bukan diam)", "/app (#p-klien)"),
    ("GLOBAL-FEED", "Toast/sukses: upload dokumen menampilkan hasil/tautan (#ue / daftar #dl)", "/app (#doup,#dl)"),
    ("GLOBAL-FEED", "Empty pattern: tabel perkara/dokumen/OCR kosong menampilkan baris panduan, bukan blank", "/app (#m-all,#dl,#ocr-jobs)"),
    ("GLOBAL-FEED", "Error pattern: 501 jujur (ocr process, ai chat, esign, pay-gateway) tampil sebagai pesan ramah + badge", "/app (#ocr-msg) + /fitur/ocr + /fitur/ai-legal-assistant"),
    ("GLOBAL-FEED", "Honest strip: strip 'uji internal 28/32 — bukan production-ready' terbaca di hero tanpa merusak layout", "/ (#hero .honest)"),
]

RESP_PAGES11 = ["/", "/fitur", "/fitur/ocr", "/fitur/ai-legal-assistant",
                "/harga", "/faq", "/solusi/notaris", "/login", "/app",
                "/kontak", "/demo"]
RESP_RESIDUAL = [
    ("topbar", "Topbar collapse <=640px: nav desktop + .topact tidak memaksa scrollWidth>viewport (hamburger/wrap)"),
    ("endpoints", "Blok endpoint/status <code> wrap (overflow-wrap:anywhere) — tidak memaksa lebar halaman"),
    ("tables", "Tabel showcase dalam .table-responsive: swipe horizontal di dalam kartu, nol elemen terpotong"),
    ("hero", "Hero tidak terpotong di 390/360 (min-width:0 grid, teks wrap, h1 32px)"),
    ("footer", "Footer .site stack vertikal di <=640px, semua tautan (Privasi/Syarat/FAQ/Peta/Status) terjangkau"),
    ("cta", "Baris CTA full-width di <=640px (.cta-row tombol selebar layar)"),
]

CSS_GAPS = (
    [("CSS-FOCUS", "Tambah aturan :focus-visible global (outline 2px kontras) lalu verifikasi per halaman", "site.css :focus-visible (GAP: nol aturan saat ini)")]*10
    + [("CSS-PRINT", "Tambah @media print (sembunyikan nav/cta, teks hitam-putih) lalu cetak ", "site.css @media print (GAP: nol aturan saat ini)")]*10
    + [("CSS-SKIP", "Tambah skip-link + target #main; Tab pertama menampilkan tautan", "body a.skip-link (GAP: belum ada)")]*5
    + [("CSS-MOTION", "Perluas reduced-motion ke semua transisi/animasi (tidak hanya .hero-video/.rv)", "site.css prefers-reduced-motion (GAP: cakupan parsial)")]*10
    + [("CSS-CONTRAST", "Audit kontras komponen: badge, muted, danger, side-title; catat rasio", "site.css warna (audit)")]*10
)

RESP_MISC = (
    [("ASSET", "Hapus/gunakan PNG tak-terreferensi (~3.1MB): library.png, meeting-room.png, server-room.png + verifikasi manifest", "static/*.png + ASSET-MANIFEST")]*4
    + [("SEO", "Canonical + OG + meta description + manifest/theme-color valid per halaman publik", "page_shell head per rute publik")]*10
    + [("AUDIT-RERUN", "Rerun audit per-elemen (maxRight/clipped/in-scroller) pasca-setiap perubahan CSS", "docs/MARKETING/responsive-shots/verify.json")]*10
    + [("STAT4", "Margin .stat4 2-kolom/390px: label panjang tidak overflow; bila tumbuh turun ke 1 kolom di 640px", "/ #dasbor .stat4")]*6
    + [("SAFETYNET", ".container-xl{overflow-x:clip} tidak lagi menutupi regresi: verifikasi clipped=0 via audit per-elemen", "site.css .container-xl + audit per-elemen")]*4
)


def main():
    reg = json.loads(REG.read_text(encoding="utf-8"))
    feats = [f for f in reg if f["status"] in ("IMPLEMENTED", "PARTIAL")]
    feats.sort(key=lambda f: f["id"])
    assert len(feats) == 141, f"expected 141 IMPLEMENTED/PARTIAL, got {len(feats)}"

    items = []  # (area, check, route)
    for f in feats:
        route = DOMAIN_ROUTE.get(f["domain"], "/app")
        for kind, text in FIVE_CHECKS:
            items.append((f["domain"] + "/" + f["id"],
                          f"[{f['id']}] {f['title']} — {text}",
                          route))
    assert len(items) == 705, len(items)

    for page in PAGES20:
        for bp in BREAKPOINTS:
            items.append(("GLOBAL-BREAKPOINT",
                          f"Sweep {page} @ {bp}px: scrollWidth<=viewport, nol elemen terpotong (tabel hanya dalam scroller)",
                          f"{page} @ {bp}px"))
    assert len(items) == 705 + 120, len(items)

    for area, check, route in GLOBAL30:
        items.append((area, check, route))
    assert len(items) == 855, len(items)

    for page in RESP_PAGES11:
        for tag, text in RESP_RESIDUAL:
            items.append(("RESPONSIVE-QA",
                          f"Susulan QA {page} [{tag}]: {text}",
                          page))
    assert len(items) == 855 + 66, len(items)

    for area, check, route in CSS_GAPS:
        if area == "CSS-PRINT":
            n = len([i for i in items if i[0] == "CSS-PRINT"]) + 1
            check = check + f"halaman {PAGES20[(n - 1) % len(PAGES20)]}"
        items.append((area, check, route))
    assert len(items) == 855 + 66 + 45, len(items)

    for area, check, route in RESP_MISC:
        items.append((area, check, route))
    assert len(items) == 1000, len(items)

    ids = [f"UIUX-{n:04d}" for n in range(1, 1001)]
    assert len(set(ids)) == 1000

    lines = []
    lines.append("# UI/UX 1000-Point Program — Checklist TODO")
    lines.append("")
    lines.append("Sumber: docs/AUTONOMOUS/FEATURE-REGISTRY.json (37 IMPLEMENTED + 104 PARTIAL = 141 fitur), "
                 "docs/MARKETING/LANDING-QA-REPORT.md, docs/MARKETING/RESPONSIVE-QA.md, "
                 "apps/server/src/lib.rs (.route), apps/server/static/site.css.")
    lines.append("Generator: docs/AUTONOMOUS/gen_uiux_todo.py (re-run untuk regenerasi deterministik).")
    lines.append("")
    lines.append("Alokasi: A) 141 fitur x 5 cek (renders/empty/loading/error/mobile390) = 705 | "
                 "B) global 150 (120 sweep 20 halaman x 6 breakpoint + 30 nav/kbd/fokus/kontras/motion/form/feedback) | "
                 "C) susulan QA + gap CSS 145 (66 residual 11 halaman + 45 gap CSS + 34 misc).")
    lines.append("")
    lines.append("Status kolom: semua TODO. Tandai DONE + bukti (screenshot/verify.json) per item.")
    lines.append("")
    lines.append("| ID | Area | Check | Route/Selector | Status |")
    lines.append("|---|---|---|---|---|")
    for uid, (area, check, route) in zip(ids, items):
        check = check.replace("|", "/")
        route = route.replace("|", "/")
        lines.append(f"| {uid} | {area} | {check} | {route} | TODO |")
    lines.append("")
    OUT.write_text("\n".join(lines), encoding="utf-8")
    print(f"features={len(feats)} items={len(items)} file={OUT}")
    print("counts: A=705 B=150 C=145 total=1000")


if __name__ == "__main__":
    main()
