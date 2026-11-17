# DOMAIN EVENT MATRIX (NATS `legal.<tenant>.<domain>.<event>.v1`)
- d01: kyc.verified, kyc.expired, identity.fraud_flagged → D02, D13, D19. - d02: client.created, coi.cleared, engagement.signed → D03, D11.
- d03: matter.opened, deadline.sla_set, matter.closed → D04, D08, D15, D18. - d04: document.sealed, version.published, hold.applied → D05, D06, D09, D23.
- d05: ocr.completed, ocr.needs_review, ocr.rejected → D01, D04, D20. - d06: deed.numbered, deed.signed, copy.issued → D12, D15, D19.
- d08: workflow.completed, instance.stuck → D09, D25. - d09: approval.granted, approval.overdue → D08, D10, D18.
- d10: envelope.signed, envelope.declined → D04, D09, D18. - d11: breach.declared, hold.imposed → D18, D22, D25.
- d13: bpn.submitted, certificate.received → D04, D06, D18. - d14/d15: invoice.issued, payment.received → D09, D18, D19, D20.
Delivery: at-least-once; consumer idempotency keys; DLQ + error inbox (D08-40).
