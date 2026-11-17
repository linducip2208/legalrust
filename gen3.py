#!/usr/bin/env python3
"""Phase 3 planning docs: benchmark protocols, detailed design, assurance. Additive only."""
import json, csv
from pathlib import Path
R = Path(r"D:\project laravel\legalrust\docs")
cat = json.loads((R/"03-FEATURE-CATALOG"/"feature-catalog.json").read_text(encoding="utf-8"))
for d in ["30-BENCHMARK-PROTOCOLS","31-DETAILED-DESIGN","32-ASSURANCE"]:
    (R/d).mkdir(exist_ok=True)
def w(rel, text):
    p = R/rel; p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text.strip()+"\n", encoding="utf-8")

# ---------- 30 ----------
w("30-BENCHMARK-PROTOCOLS/OCR_BENCHMARK_PROTOCOL.md", """
# OCR BENCHMARK PROTOCOL (specification only — no scores fabricated, execution is blocker B-002)
## Corpus (CORPUS_SPEC.md)
Stratified Indonesian samples per doc type (KTP/KK/NPWP/passport/sertifikat/akta/kontrak/mixed-PDF):
each stratum n≥200 pages across capture modes (flatbed scan 300dpi, phone photo good/glare/rotated/low-light),
ground truth double-keyed + adjudicated, versioned (corpus v1.0 pins image hashes). Re-run gate: any engine/profile change.
## Metrics
CER = edit-distance/ground-truth chars; WER word-level; field-level F1 on NIK/names/dates/amounts/NOP;
layout/table F1 (cell adjacency); classification accuracy; p95 latency/page; human-correction rate; cost/page.
Report per stratum + macro average; confidence intervals (bootstrap, 95%).
## Procedure
1) freeze engine+profile+preprocessing config hash; 2) run harness (spec: container image, GPU/CPU pin, batch=1 and batch=8);
3) score with reference scorer (spec in repo-planning, implemented at B-002); 4) publish report card per engine×profile;
5) promotion gate: KTP printed field-acc ≥97% AND CER ≤2% clean-scan, else HITL-band only.
## Engines under test
ocrs (pure Rust + ONNX dep), oar-ocr pipeline parts, Tesseract bindings (native libtesseract declared), AI-vision fallback.
Winner chosen PER DOC TYPE on measured numbers only.
""")
w("30-BENCHMARK-PROTOCOLS/CORPUS_SPEC.md", """
# CORPUS SPEC v1.0 (planning)
Schema: corpus/{doctype}/{stratum}/{sample_id}/{image, gt.json, meta.json}. gt.json: full text + field spans + boxes.
meta.json: capture device, dpi, lighting, rotation, PII-synthetic flag (real citizen data NEVER in corpus without consent;
prefer synthetic ID layouts mirroring field geometry). Versioning: manifest.csv with SHA-256 per file + corpus semver.
Size target v1.0: 8 doctypes × 4 strata × 200 = 6,400 pages. Custodian: OCR researcher. License/consent log per sample.
""")
w("30-BENCHMARK-PROTOCOLS/AI_EVAL_PROTOCOL.md", """
# AI EVAL PROTOCOL (specification only — execution is blocker B-003)
Suites: grounded-QA accuracy (answer must cite retrieved chunks; citation precision/recall), refusal correctness
(validity-certification + disallowed requests refused with safe completion), regression pack (prompt/template change diff),
red-team set (prompt-injection: instruction override, tool-output poisoning, untrusted-doc exfiltration attempts — must not comply),
cost/latency SLOs per model route, PII-leak probe (input canary must never appear in outputs/logs).
Gates: grounded-QA ≥90%, refusal 100% on certification probes, injection-compliance 0%. Every model/prompt version scored before promotion.
Full audit (prompt hash/response/model/version/cost) retained per 10-AI-ENGINE.
""")
# ---------- 31 ----------
by_dom = {}
for f in cat: by_dom.setdefault(f["domain"], []).append(f)
screens = ["## Global shell + dashboards"]
screens += ["- SHELL-01 App shell (office switcher, command bar, notifications) | SHELL-02 Login/MFA/SSO | " +
 "DASH-EXEC Executive cockpit | DASH-NOT Notary | DASH-PPAT | DASH-PART Partner | DASH-FIN Finance | DASH-AUD Auditor | " +
 "DASH-OPS Ops/health | PORTAL-01 Client home | MOB-01 Field inbox"]
for dom in sorted(by_dom):
    dn = by_dom[dom][0]["domain_name"]
    screens.append(f"\n## {dom} — {dn} (6 screens each)")
    for s, nm in [("LST","List + filters"),("DTL","Detail workspace"),("EDT","Guided create/edit"),
                  ("REV","Review/approve"),("HST","History/audit"),("WGT","Dashboard widget")]:
        screens.append(f"- {dom}-{s}: {nm} — components: table/cards, search, SLA badges; states: loading/empty/error/permission-denied; a11y: focus order + labels; perm: read/write/approve/export scoped.")
w("31-DETAILED-DESIGN/SCREEN_INVENTORY.md",
 "# SCREEN INVENTORY (~170 screens)\n" + "\n".join(screens) +
 "\n\nTotal: 2 shell + 8 dashboards + 1 portal home + 1 mobile inbox + 150 domain screens = 162 specified screens. Prototype build is backlog (UI/UX deduction retained).")
api = ["# API ENDPOINT CATALOG (planning — 150 endpoints, versioned /api/v1)"]
for dom in sorted(by_dom):
    dl = dom.lower()
    api.append(f"\n## {dom}")
    for m, p, sc, idem in [("GET",f"/api/v1/{dl}/items","read","no"),("POST",f"/api/v1/{dl}/items","write","KEY"),
      ("GET",f"/api/v1/{dl}/items/:id","read","no"),("PUT",f"/api/v1/{dl}/items/:id","write","KEY"),
      ("POST",f"/api/v1/{dl}/items/:id/transition","approve","KEY"),("GET",f"/api/v1/{dl}/items/:id/history","read","no")]:
        api.append(f"- {m} {p} — scope `perm:{dl}:*` + office claim; idempotency: {idem}; paging: cursor; errors: problem-details.")
w("31-DETAILED-DESIGN/API_ENDPOINT_CATALOG.md", "\n".join(api) +
 "\n\nConventions: OIDC JWT, MFA step-up header for money/deed/sign, If-Match row_version, tenant/office claim enforced server-side, OpenAPI published at implementation.")
td = ["# FULL TABLE DICTIONARY (planning — core 50 tables, 2 per domain, illustrative DDL)"]
for dom in sorted(by_dom):
    dl = dom.lower().replace(" & ","_").replace(" ","_").replace("/","_")
    t1, t2 = f"{dl}_item", f"{dl}_event"
    td.append(f"\n## {dom}")
    td.append(f"```sql\nCREATE TABLE {t1} (\n  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,\n  tenant_id BIGINT UNSIGNED NOT NULL,\n  office_id BIGINT UNSIGNED NOT NULL,\n  status VARCHAR(40) NOT NULL DEFAULT 'draft',\n  payload JSON NULL,\n  row_version BIGINT UNSIGNED NOT NULL DEFAULT 0,\n  created_by BIGINT UNSIGNED NOT NULL,\n  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,\n  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,\n  KEY ix_{dl[:20]}_tenant_office (tenant_id, office_id, status)\n) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;\n```")
    td.append(f"- `{t2}`: append-only (id, tenant_id, actor_id, action, entity_id, before/after_hash, ip, created_at). Retention: 10y (25y for deed/protocol classes); legal-hold blocks purge.")
w("31-DETAILED-DESIGN/TABLE_DICTIONARY_FULL.md", "\n".join(td))
# ---------- 32 ----------
st = ["# STRIDE REGISTER (per-domain summary; full threat models at implementation)"]
for dom in sorted(by_dom):
    dn = by_dom[dom][0]["domain_name"]
    st.append(f"- {dom} {dn}: S-spoofed actor→OIDC+MFA; T-payload tamper→hash+row_version; R-repudiation→append-only audit; "
              f"I-PII disclosure→masking+scopes; D-workflow stall→SLA+DLQ; E-scope escape→tenant-claim tests.")
w("32-ASSURANCE/STRIDE_REGISTER.md", "\n".join(st))
w("32-ASSURANCE/ASVS_MAPPING.md", """
# OWASP ASVS MAPPING (planning → verification activities at implementation)
- 2.x AuthN (OIDC/MFA/WebAuthn) → D16/D22, verify: auth flows + MFA bypass tests. - 3.x Session (rotation/anomaly) → D16/D24.
- 4.x Access (RBAC/scopes/ABAC/break-glass) → D16+D17+every domain; verify: horizontal/vertical traversal tests per endpoint (150).
- 5.x Validation (serde/DTO/SQLx params) → D08/D21; verify: fuzz + injection suites. - 6.x Crypto (AES-256/KMS/TLS1.2+) → D22/D01.
- 7.x Errors/logging (PII redaction, no stack leaks) → D22/D05. - 8.x Data protection (retention/hold/purge approvals) → D11/D04.
- 9.x Comms (TLS, webhooks HMAC) → D21/D18. - 10.x Malicious (AV-scan, upload gates) → D04. - 11.x Business logic (COI, quorum, trust-nonnegative) → D02/D09/D14.
- 12.x Files (integrity hash, lifecycle) → D04/D06. - 13.x API (rate-limit, versioning, sandbox) → D21. - 14.x Config (secrets, flags) → D25.
Each maps to QA suites in 16-TESTING-QA; pen-test + DAST at hardening phase.
""")
with open(R/"32-ASSURANCE"/"TRACEABILITY_MATRIX.csv","w",newline="",encoding="utf-8") as fh:
    wcsv = csv.writer(fh); wcsv.writerow(["feature_id","domain","ac_ids","pos_tests","neg_tests","edge_tests","status"])
    for f in cat:
        fid = f["id"]
        wcsv.writerow([fid, f["domain"], ";".join(f"{fid}-AC{i}" for i in range(1,6)),
          ";".join(f"{fid}-TCP{i}" for i in range(1,4)), ";".join(f"{fid}-TCN{i}" for i in range(1,4)),
          ";".join(f"{fid}-TCE{i}" for i in range(1,5)), "SPECIFIED"])
print("GEN3 OK")
