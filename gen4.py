#!/usr/bin/env python3
"""Phase 4 planning: counsel dossiers, swimlanes, effort rollup, ops plans."""
import json
from pathlib import Path
from collections import defaultdict
R = Path(r"D:\project laravel\legalrust\docs")
cat = json.loads((R/"03-FEATURE-CATALOG"/"feature-catalog.json").read_text(encoding="utf-8"))
for d in ["33-LEGAL-REVIEW","34-PLANNING-EXT"]:
    (R/d).mkdir(exist_ok=True)
def w(rel, text):
    p = R/rel; p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text.strip()+"\n", encoding="utf-8")
def find(dom, kw):
    for f in cat:
        if f["domain"]==dom and kw.lower() in f["title"].lower(): return f
    return None
def fid(dom, kw):
    f = find(dom, kw); return f["id"] if f else "MISS:"+kw

# ---- counsel dossiers ----
Qs = [
 ("C1","Validitas akta elektronik / virtual presence","UUJN 15(3) vs KUHPerdata 1868 vs UU ITE 5(4)",
  [("D06","Deed reading"),("D06","signing ceremony"),("D12","Repertorium")],
  "Apakah kehadiran virtual + TTE tersertifikasi dapat memenuhi unsur 'di hadapan' untuk akta otentik? Jika tidak, ruang lingkup cyber-notary yang sah apa?"),
 ("C2","Ruang lingkup pengecualian Pasal 5(4) UU ITE","UU ITE 1/2024 + PP PSTE 71/2019",
  [("D10","PSrE provider"),("D10","exclusion warning"),("D06","Deed approval")],
  "Dokumen legal non-akta apa saja yang sah dengan TTE tersertifikasi/tidak tersertifikasi? Konfirmasi daftar yang wajib tetap konvensional."),
 ("C3","Kedudukan notaris sebagai pengendali data","UU PDP 27/2022 (berlaku penuh Okt 2024)",
  [("D01","Consent capture"),("D11","ROPA"),("D11","DPIA")],
  "Apakah notaris = data controller penuh (wajib ROPA/DPIA/DPO) atau ada kekhususan pejabat umum? Implikasi sumpah jabatan 16(1)f?"),
 ("C4","Protokol 25 tahun vs hak penghapusan","UUJN vs UU PDP hak DSAR/deletion",
  [("D11","25-year"),("D04","legal-hold"),("D11","deletion request")],
  "Hierarki yang benar saat subjek data meminta penghapusan atas data dalam minuta/protokol? Mekanisme penolakan sah + dokumentasinya?"),
 ("C5","Bobot pembuktian TTE PSrE","UU ITE Pasal 11-12 + sertifikasi PSrE",
  [("D10","Long-term validation"),("D10","Timestamp authority")],
  "Syarat TTE tersertifikasi agar setara tanda basah di pengadilan; kebutuhan TSA/LTV untuk arsip 10-25 tahun?"),
 ("C6","Validitas pengarsipan/penyerahan elektronik BPN","Regulasi pertanahan + PPAT",
  [("D13","BPN verification"),("D13","BPN submission"),("D21","BPN integration")],
  "Tahap mana yang boleh elektronik penuh vs wajib fisik? Format bukti serah-terima yang diakui?"),
 ("C7","Transfer data lintas negara","UU PDP bab transfer + pihak asing",
  [("D01","Foreigner"),("D25","Cross-border"),("D22","Data-residency")],
  "Syarat sah pemrosesan data WNA/pihak asing dan penyimpanan lintas-yurisdiksi untuk ekspansi?"),
]
for code, title, basis, feats, q in Qs:
    ids = " ".join(fid(d,k) for d,k in feats)
    w(f"33-LEGAL-REVIEW/{code}.md", f"""# COUNSEL DOSSIER {code} — {title}
Status: PERTANYAAN TERBUKA (bukan nasihat hukum; membutuhkan opini counsel).
Legal basis: {basis}. Affected features: {ids}.
## Pertanyaan
{q}
## Konteks desain saat ini
Guardrail konservatif diberlakukan (physical-presence untuk akta; e-sign diblokir untuk akta notariil/PPAT;
notaris diperlakukan sebagai controller). Lihat 15-COMPLIANCE + WORKFLOW_GAP_REPORT.
## Opsi yang perlu dinilai counsel
O1 Pertahankan guardrail (aman, lambat). O2 Relaksasi bersyarat (ruang yang sah + bukti yang dibutuhkan).
O3 Jalur regulasi (harmonisasi/aturan pelaksana yang ditunggu).
## Keluaran yang diminta
Opini tertulis + dasar pasal + dokumen bukti yang harus disimpan sistem + daftar fitur yang boleh dibuka/ditutup.
""")

# ---- effort rollup (computed, midpoints stated) ----
MID = {"S":4, "M":7.5, "L":20, "XL":40}
tot = defaultdict(lambda: [0,0.0]); grand=0.0; nc=defaultdict(int)
for f in cat:
    d = MID[f["complexity"]]; tot[f["domain"]][0]+=1; tot[f["domain"]][1]+=d; grand+=d; nc[f["complexity"]]+=1
lines = ["# EFFORT ROLLUP (computed from catalog complexity; planning estimate, NOT a commitment)",
 "Assumptions: midpoint person-days S=4, M=7.5, L=20, XL=40; 20 working-days/month; excludes counsel wait, benchmarks, PM/QA overhead (+35% loaded est.).",
 "", f"Mix: {dict(nc)}. Raw total: {grand:,.0f} person-days = {grand/20:,.0f} person-months (~{grand/20/12:,.1f} person-years unloaded; ~{grand*1.35/20:,.0f} PM loaded).",
 "", "| Domain | Features | Person-days (mid) | Person-months |", "|---|---|---|---|"]
for dom in sorted(tot): n,days = tot[dom]; lines.append(f"| {dom} | {n} | {days:,.0f} | {days/20:,.0f} |")
lines += ["", "Scenarios (loaded, dev-only): 6 devs → ~{:.0f} mo; 10 devs → ~{:.0f} mo; 15 devs → ~{:.0f} mo (excludes B-001..B-004 wait + phased scope; P0-first per roadmap).".format(grand*1.35/20/6, grand*1.35/20/10, grand*1.35/20/15),
 "Critical path (must sequence): D16/D17 → D02 → D03 → D04 → D05 → D06 → D09/D10 → D15/D14. Non-path domains parallelize (max 4-8 teams)."]
w("34-PLANNING-EXT/EFFORT_ROLLUP.md", "\n".join(lines))

# ---- swimlanes ----
def lane(title, steps):
    t = [f"# SWIMLANE — {title}", "", "| Step | Actor | Feature | Gate |", "|---|---|---|---|"]
    for s,a,d,k,g in steps: t.append(f"| {s} | {a} | {fid(d,k)} {k} | {g} |")
    return "\n".join(t)
w("34-PLANNING-EXT/W1_DEED_SWIMLANE.md", lane("W1 Penerbitan akta notariil", [
 ("1. Intake & KYC","Front-desk","D01","KTP-el","dokumen lengkap"),("2. OCR + review","Reviewer","D05","Human-in-the-loop","field-acc ≥ ambang"),
 ("3. Drafting","Drafter","D06","drafting wizard","template + klausul"),("4. Persetujuan","Supervisor","D09","request composer","disetujui"),
 ("5. Penomoran","Notaris","D06","Deed numbering","nomor repertorium"),("6. Pembacaan & penandatanganan fisik","Notaris/para pihak","D06","signing ceremony","kehadiran fisik + saksi"),
 ("7. Protokol","Notaris","D12","Repertorium","tersimpan"),("8. Salinan/kutipan","Staff","D06","copy issuance","tanda terima")]))
w("34-PLANNING-EXT/W2_PPAT_SWIMLANE.md", lane("W2 Transaksi PPAT", [
 ("1. Intake sertifikat","Officer","D13","intake","asli + cek"),("2. Verifikasi BPN","Officer","D13","BPN verification","respons BPN"),
 ("3. KYC para pihak","Front-desk","D01","KTP-el","lolos + COI"),("4. AJB/APHT","PPAT","D13","AJB drafting","draft disetujui"),
 ("5. Pajak","Finance","D13","PPh","validasi"),("6. Penandatanganan","PPAT/pihak","D13","handover receipt","fisik"),
 ("7. Penyerahan BPN","Officer","D21","BPN integration","tanda terima"),("8. Arsip protokol PPAT","Staff","D13","PPAT protocol","tersimpan")]))
w("34-PLANNING-EXT/W3_BILLING_SWIMLANE.md", lane("W3 Penagihan", [
 ("1. Catat waktu/biaya","Staff","D03","Time entry","teralokasi matter"),("2. Draf tagihan","Finance","D15","Invoice composer","fee schedule"),
 ("3. Persetujuan","Partner","D09","request composer","ambang nilai"),("4. Kirim + bayar","Klien","D19","Online payment","tercatat"),
 ("5. Rekonsiliasi","Finance","D14","Bank reconciliation","cocok"),("6. Trust ledger","Finance","D14","escrow","non-negatif")]))
w("34-PLANNING-EXT/W4_ONBOARDING_SWIMLANE.md", lane("W4 Onboarding klien", [
 ("1. Registrasi","Klien","D19","registration with KYC","identitas"),("2. Verifikasi","KYC officer","D01","Dukcapil verification","terverifikasi"),
 ("3. Master record","Staff","D02","individual","dedupe lolos"),("4. Engagement","Partner","D02","Engagement letter","ditandatangani (non-akta)"),
 ("5. Buka matter","Handler","D03","creation wizard","COI bersih")]))
w("34-PLANNING-EXT/W5_BREACH_SWIMLANE.md", lane("W5 Respons insiden/breach 72 jam", [
 ("1. Deteksi","Sistem/whistleblow","D11","Incident register","tiket"),("2. Kontainmen","Security","D22","Breach containment","forensik"),
 ("3. Notifikasi","DPO","D11","72h","tenggat 72 jam"),("4. Broadcast","Comms","D18","Emergency broadcast","terkirim"),
 ("5. CAPA","Owner","D11","CAPA","terverifikasi"),("6. Bukti audit","Auditor","D22","hash chain","utuh")]))
w("34-PLANNING-EXT/MIGRATION_TRAINING_PLAN.md", """
# MIGRATION + TRAINING PLAN (spec)
Migrasi (backfile fisik→digital, G-003): inventarisasi → pemindaian QC → OCR batch → review → indeks → serah-terima protokol;
validasi via BL-007 harness (sampling + checksum + retention tagging). Migrasi data antar-kantor (D17): export/import + validasi + jendela beku.
Training: peran (notaris/PPAT/staf/keuangan/klien) × modul (KYC, OCR review, deed, approval, e-sign, breach) + bukti kehadiran di D11;
go-live bertahap per kantor dengan hypercare 30 hari + kriteria keluar (SLA + correction-rate + zero P0).
""")
print(f"GEN4 OK domains={len(set(f['domain'] for f in cat))} total_days={sum(MID[f['complexity']] for f in cat):,.0f}")
