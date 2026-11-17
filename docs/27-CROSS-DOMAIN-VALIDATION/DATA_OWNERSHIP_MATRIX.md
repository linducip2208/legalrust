# DATA OWNERSHIP MATRIX (owner → consumers)
- clients (D02) → D01 KYC, D03 matters, D13 PPAT, D19 portal, D20 reports. - matters/tasks (D03) → D04 docs, D08 workflows, D14 costs, D15 billing, D19 progress, D20 KPIs.
- documents/versions (D04) → D05 OCR, D06 deeds, D07 RAG, D09 review, D10 sign, D23 search. - ocr_results (D05) → D01 prefill, D04 metadata, D20 metrics.
- deed_records (D06) → D12 protocol, D19 copies, D23 search. - approvals (D09) → D08 engine, D10 binding, D20 bottlenecks.
- invoices/ledger (D14/D15) → D20 finance, D21 e-faktur. - audit_events (D22) → D25 viewer, D11 evidence.
Rule: single writer domain per entity; others consume via API/events; tenant_id enforced on every row.
