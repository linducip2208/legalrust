# ID Mapping: brief F001–F1000 ranges → canonical catalog

Canonical registry: `docs/AUTONOMOUS/FEATURE-REGISTRY.json` (IDs `F-Dxx-xxx`).
Scores: `docs/AUTONOMOUS/AUDIT-1000/FEATURE-SCORES.json`.
Queue: `docs/AUTONOMOUS/TASK-QUEUE.json`.

No duplicate IDs are invented. The brief's F001–F1000 ranges map to
canonical domains as follows:

| Brief range | Canonical domain | Registry prefix |
|---|---|---|
| F001–F040 | D01 Identity & KYC | F-D01-xxx |
| F041–F080 | D02 Client & CRM | F-D02-xxx |
| F081–F120 | D03 Project & Case | F-D03-xxx |
| F121–F160 | D04 Documents | F-D04-xxx |
| F161–F200 | D05 OCR & Intel | F-D05-xxx |
| F201–F240 | D06 Deed generation | F-D06-xxx |
| F241–F280 | D07 AI assistant | F-D07-xxx |
| F281–F320 | D08 Workflow | F-D08-xxx |
| F321–F360 | D09 Approvals | F-D09-xxx |
| F361–F400 | D10 E-sign | F-D10-xxx |
| F401–F440 | D11 Compliance | F-D11-xxx |
| F441–F480 | D12 Notary office | F-D12-xxx |
| F481–F520 | D13 PPAT & property | F-D13-xxx |
| F521–F560 | D14 Finance | F-D14-xxx |
| F561–F600 | D15 Billing | F-D15-xxx |
| F601–F640 | D16 Users & roles | F-D16-xxx |
| F641–F680 | D17 Multi-office | F-D17-xxx |
| F681–F720 | D18 Comms & notif | F-D18-xxx |
| F721–F760 | D19 Portal | F-D19-xxx |
| F761–F800 | D20 Reporting | F-D20-xxx |
| F801–F840 | D21 API & integration | F-D21-xxx |
| F841–F880 | D22 Security | F-D22-xxx |
| F881–F920 | D23 Search | F-D23-xxx |
| F921–F960 | D24 Mobile & field | F-D24-xxx |
| F961–F1000 | D25 Enterprise admin | F-D25-xxx |

All scoring/evidence writes go to the canonical files. This directory
holds only orchestration checkpoints.
