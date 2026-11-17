# DUPLICATION REPORT
Initial scan (token-Jaccard ≥0.55): 27 pairs. After remediation: 7 residual, ALL justified:
1. F-D07-015 Per-matter AI cost allocation vs F-D14-014 Matter cost allocation — different cost objects (AI spend vs matter economics); linked, not merged.
2-4. KTP/KK/NPWP OCR pipelines (D05-001/002/003) — distinct document types, engines profiles, field sets; shared pre-processing noted.
5. D08-018/020/021 step connectors (e-sign/OCR/AI) — distinct step types behind one connector interface; intentional.
6. D02-001 vs D02-002 individual vs corporate master — distinct record shapes/validations; linked via household/group model.
Fixed (24 renames, regenerated): exact dups Mobile approval inbox (D24→Offline approval queue), OCR accuracy dashboard (D20→trend & benchmark report);
near-dups deed search, payment-gateway connector, breach runbook, bank-partner sheet, tenant console, client-user lifecycle, cash-advance tracker,
risk posture report, legal-hold application, OCR artifact redaction, protocol-box scheduler, portal announcements, approval digest prefs,
AI spend analytics, protocol sealing, API quota assignment, board-resolution verification, co-signer flow, quorum override, doc-expiry monitoring, template A/B testing.
Scoped-pattern justification: per-domain audit trails/dashboards (D04/D05/D09/D11/D14/D15/D19/D20/D22/D24/D25) are bounded-context views, not duplication.
