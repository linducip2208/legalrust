# Business Rules Register (extract; full rule text per feature in catalog)
- BR-G01 Tenant/office scoping mandatory on every query. - BR-G02 Legal hold blocks deletion/purge.
- BR-G03 PII display masking by role; NIK never in logs. - BR-G04 Money/deed/sign actions need approval.
- BR-G05 Consent (UU PDP) required before processing; withdrawal honored prospectively.
- BR-ID01 NIK 16-digit + province/regency sanity; expiry tracked. - BR-DEED01 Deed numbers immutable once issued;
  corrections via renvoi/akta perbaikan. - BR-PROTO01 Protocol retention 25 years overrides standard purge.
- BR-SIGN01 Certified PSrE only for qualifying signatures; akta exclusion warning enforced.
- BR-FIN01 Trust ledger never negative; segregation of duties on journals.
- BR-WF01 Idempotent transitions; illegal transitions rejected (409).
Numbering: full BR-### per feature; conflicts resolved by G-rules first, then domain rules.
