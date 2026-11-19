use super::store::StoreError;
/// Strict `HH:MM` (24h) check shared by the office-hours impls.
pub(crate) fn valid_hhmm(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 5 || b[2] != b':' {
        return false;
    }
    let digit = |i: usize| b[i].is_ascii_digit();
    if !(digit(0) && digit(1) && digit(3) && digit(4)) {
        return false;
    }
    let h: u32 = s[..2].parse().unwrap_or(99);
    let mi: u32 = s[3..].parse().unwrap_or(99);
    h < 24 && mi < 60
}
#[derive(Debug, Clone)]
pub struct OcrJobRow {
    pub id: i64,
    pub org: String,
    pub doc: i64,
    pub doc_type: String,
    pub status: String,
}
#[derive(Debug, Clone)]
pub struct OcrFieldRow {
    pub name: String,
    pub value: String,
    pub confidence: f32,
    pub page: i64,
}
#[derive(Debug, Clone)]
pub struct ApprovalReqRow {
    pub id: i64,
    pub org: String,
    pub entity: String,
    pub entity_id: i64,
    pub title: String,
    pub status: String,
    pub created_by: String,
    /// Creation timestamp `YYYY-MM-DD HH:MM:SS` (age/overdue math).
    pub created: String,
}
#[derive(Debug, Clone)]
pub struct ApprovalStepRow {
    pub id: i64,
    pub req: i64,
    pub step_no: i64,
    pub role: String,
    pub user: Option<String>,
    pub status: String,
    pub comment: Option<String>,
}
/// Approval digest + escalation preferences (per user).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprPrefRow {
    pub user: String,
    pub digest_mode: String,
    pub escalate_to: String,
}
/// Approval audit-freeze flag (per org).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreezeRow {
    pub org: String,
    pub frozen: bool,
    pub reason: String,
    pub set_by: String,
}
/// Approval routing rule (amount threshold or AML-risk floor -> role).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteRuleRow {
    pub id: i64,
    pub org: String,
    pub kind: String,
    pub entity: String,
    pub min_value: i64,
    pub role: String,
    pub priority: i64,
    pub note: String,
}
/// Quorum round (M-of-N parallel sign-off).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuorumRoundRow {
    pub id: i64,
    pub org: String,
    pub entity: String,
    pub entity_id: i64,
    pub title: String,
    pub required: i64,
    pub members: String,
    pub status: String,
    pub created_by: String,
}
/// Quorum vote (one per voter per round).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuorumVoteRow {
    pub id: i64,
    pub org: String,
    pub round_id: i64,
    pub voter: String,
    pub decision: String,
}
/// Approval decision receipt (hash-chained per request).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptRow {
    pub id: i64,
    pub org: String,
    pub req_id: i64,
    pub step_no: i64,
    pub decision: String,
    pub actor: String,
    pub prev_hash: String,
    pub hash: String,
}
/// Dual-control quorum override.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuorumOverrideRow {
    pub id: i64,
    pub org: String,
    pub round_id: i64,
    pub requested_by: String,
    pub approved_by: String,
    pub status: String,
}
/// Approval policy snapshot (versioned routing rules).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicySnapRow {
    pub id: i64,
    pub org: String,
    pub version: i64,
    pub payload: String,
    pub created_by: String,
}
/// Document lifecycle flags (retention hold / archive / trash).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocLifecycleRow {
    pub id: i64,
    pub org: String,
    pub doc_id: i64,
    pub on_hold: bool,
    pub archived: bool,
    pub trashed: bool,
    pub retain_until: String,
    pub note: String,
}
/// Metadata field definition (schema).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaFieldRow {
    pub id: i64,
    pub org: String,
    pub key: String,
    pub kind: String,
    pub required: bool,
    pub note: String,
}
/// Document metadata value (tags = repeated rows on tag-kind fields).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocMetaRow {
    pub id: i64,
    pub org: String,
    pub doc_id: i64,
    pub key: String,
    pub value: String,
}
/// Document check-in/out lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocLockRow {
    pub doc_id: i64,
    pub org: String,
    pub holder: String,
    pub note: String,
}

/// Shared validation for approval creation: entity allowlist + contiguous
/// step numbers starting at 1 with non-empty roles.
pub fn appr_check(entity: &str, steps: &[(i64, String)]) -> Result<(), super::store::StoreError> {
    use super::store::StoreError;
    if !["deed", "invoice", "matter", "document"].contains(&entity) {
        return Err(StoreError::Conflict("entity".into()));
    }
    if steps.is_empty() {
        return Err(StoreError::Conflict("steps".into()));
    }
    let mut nos: Vec<i64> = steps.iter().map(|(n, _)| *n).collect();
    nos.sort_unstable();
    if nos.iter().enumerate().any(|(i, n)| *n != i as i64 + 1) {
        return Err(StoreError::Conflict("steps".into()));
    }
    if steps.iter().any(|(_, r)| r.trim().is_empty()) {
        return Err(StoreError::Conflict("role".into()));
    }
    Ok(())
}
#[derive(Debug, Clone)]
pub struct TemplateRow {
    pub id: i64,
    pub org: String,
    pub name: String,
    pub body: String,
    pub version: i64,
}
/// Reusable deed clause (versioned).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClauseRow {
    pub id: i64,
    pub org: String,
    pub name: String,
    pub category: String,
    pub body: String,
    pub version: i64,
}
#[derive(Debug, Clone)]
pub struct DeedRow {
    pub id: i64,
    pub org: String,
    pub matter_id: i64,
    pub number: String,
    pub deed_type: String,
    pub state: String,
    /// Creation timestamp as `YYYY-MM-DD HH:MM:SS` (repertorium ordering).
    pub created: String,
}
#[derive(Debug, Clone)]
pub struct InvoiceRow {
    pub id: i64,
    pub org: String,
    pub matter_id: i64,
    pub number: String,
    pub total: i64,
    pub currency: String,
    pub status: String,
}
#[derive(Debug, Clone)]
pub struct PaymentRow {
    pub id: i64,
    pub invoice_id: i64,
    pub amount: i64,
    pub method: String,
}
/// Open invoice with derived paid/balance (AR aging input row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArOpenRow {
    pub id: i64,
    pub number: String,
    pub matter_id: i64,
    pub total: i64,
    pub paid: i64,
    pub balance: i64,
}
/// PPAT deed register entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PpatRow {
    pub id: i64,
    pub org: String,
    pub number: String,
    pub deed_kind: String,
    pub party_seller: String,
    pub party_buyer: String,
    pub object_desc: String,
    pub status: String,
}
/// Monthly PPAT aggregate bucket (kind x status count).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PpatMonthlyRow {
    pub kind: String,
    pub status: String,
    pub n: i64,
}
/// PPAT office dashboard snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PpatDashboard {
    pub total_deeds: i64,
    pub by_kind: Vec<PpatMonthlyRow>,
    pub unverified_consents: i64,
    pub charges_due_30: i64,
    pub open_disputes: i64,
    pub pending_handovers: i64,
}
/// UU PDP DPIA assessment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DpiaRow {
    pub id: i64,
    pub org: String,
    pub title: String,
    pub data_types: String,
    pub risk_level: String,
    pub status: String,
}
/// Vendor/processor with DPA status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorRow {
    pub id: i64,
    pub org: String,
    pub name: String,
    pub service: String,
    pub dpa_status: String,
}
/// UU PDP breach log entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreachRow {
    pub id: i64,
    pub org: String,
    pub title: String,
    pub discovered_at: String,
    pub affected_count: i64,
    pub status: String,
}
/// Zone (ZNT/NJOP) reference entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneRow {
    pub id: i64,
    pub org: String,
    pub area_code: String,
    pub area_name: String,
    pub njop_minor: i64,
}
/// Manual PBB status verification log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PbbRow {
    pub id: i64,
    pub org: String,
    pub nop: String,
    pub tax_year: String,
    pub result: String,
    pub checked_on: String,
    pub note: String,
}
/// Land split (pemecahan) tracker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LandSplitRow {
    pub id: i64,
    pub org: String,
    pub source_parcel: String,
    pub new_parcels: String,
    pub split_date: String,
    pub status: String,
}
/// Land merge (penggabungan) tracker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LandMergeRow {
    pub id: i64,
    pub org: String,
    pub source_parcels: String,
    pub result_parcel: String,
    pub merge_date: String,
    pub status: String,
}
/// Land dispute flag & hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LandDisputeRow {
    pub id: i64,
    pub org: String,
    pub parcel_ref: String,
    pub kind: String,
    pub description: String,
    pub status: String,
}
/// Boundary & area verification (result auto-derived from measurements).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryRow {
    pub id: i64,
    pub org: String,
    pub parcel_ref: String,
    pub declared_area: i64,
    pub measured_area: i64,
    pub result: String,
    pub checked_on: String,
}
/// Deed consent check (marital / heir / board / power-to-sell).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeedConsentRow {
    pub id: i64,
    pub org: String,
    pub ppat_id: i64,
    pub kind: String,
    pub party: String,
    pub verified: bool,
    pub note: String,
}
/// Notary-to-PPAT handoff checklist item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotaryHandoffRow {
    pub id: i64,
    pub org: String,
    pub ppat_id: i64,
    pub doc_name: String,
    pub received: bool,
    pub note: String,
}
/// Girik/adat land intake record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GirikRow {
    pub id: i64,
    pub org: String,
    pub letter_c_ref: String,
    pub holder: String,
    pub area_minor: i64,
    pub village: String,
    pub status: String,
    pub note: String,
}
/// Lease-deed terms (one row per deed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseTermRow {
    pub id: i64,
    pub org: String,
    pub ppat_id: i64,
    pub monthly_minor: i64,
    pub duration_months: i64,
    pub start_date: String,
    pub deposit_minor: i64,
    pub note: String,
}
/// Developer bulk-transaction batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevBatchRow {
    pub id: i64,
    pub org: String,
    pub developer: String,
    pub batch_no: String,
    pub total_units: i64,
    pub status: String,
    pub note: String,
}
/// Developer unit stock entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevUnitRow {
    pub id: i64,
    pub org: String,
    pub batch_id: Option<i64>,
    pub unit_code: String,
    pub unit_type: String,
    pub price_minor: i64,
    pub status: String,
}
/// KPR bank coordination pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KprPackRow {
    pub id: i64,
    pub org: String,
    pub ppat_id: i64,
    pub bank: String,
    pub amount_minor: i64,
    pub status: String,
    pub note: String,
}
/// Stored AJB draft (template-based composition artifact).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AjbDraftRow {
    pub id: i64,
    pub org: String,
    pub ppat_id: i64,
    pub template_name: String,
    pub body: String,
    pub price_minor: i64,
}
/// Certificate vault link (deed <-> document scan).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertLinkRow {
    pub id: i64,
    pub org: String,
    pub ppat_id: i64,
    pub doc_id: i64,
    pub doc_name: String,
    pub label: String,
}
/// BPN coordination track (manual mode: verification / queue / response).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BpnTrackRow {
    pub id: i64,
    pub org: String,
    pub kind: String,
    pub ref_no: String,
    pub office: String,
    pub status: String,
    pub note: String,
}
/// Data-retention rule (entity -> years + end-of-life action).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionRuleRow {
    pub id: i64,
    pub org: String,
    pub entity: String,
    pub retain_years: i64,
    pub action: String,
    pub active: bool,
    pub note: String,
}
/// Approval-gated purge request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurgeRequestRow {
    pub id: i64,
    pub org: String,
    pub entity: String,
    pub entity_id: i64,
    pub reason: String,
    pub requested_by: String,
    pub status: String,
    pub decided_by: String,
}
/// Protocol register (in/out day-book of the protocol archive).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PregRow {
    pub id: i64,
    pub org: String,
    pub entry_date: String,
    pub book_ref: String,
    pub deed_no: String,
    pub party: String,
    pub direction: String,
}
/// Land-history (riwayat tanah) record entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LandHistoryRow {
    pub id: i64,
    pub org: String,
    pub parcel_ref: String,
    pub event: String,
    pub detail: String,
    pub event_date: String,
}
/// Unit handover (BAST) record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitHandoverRow {
    pub id: i64,
    pub org: String,
    pub unit_ref: String,
    pub handed_from: String,
    pub handed_to: String,
    pub bast_no: String,
    pub handed_on: String,
    pub status: String,
}
/// Operational incident (non-PDP incidents, complaints, near-misses).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncidentRow {
    pub id: i64,
    pub org: String,
    pub title: String,
    pub severity: String,
    pub status: String,
}
/// Corrective Action / Preventive Action item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapaRow {
    pub id: i64,
    pub org: String,
    pub title: String,
    pub cause: String,
    pub action: String,
    pub status: String,
}
/// Tracked regulatory change (peraturan baru / revisi).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegChangeRow {
    pub id: i64,
    pub org: String,
    pub title: String,
    pub source: String,
    pub effective_date: String,
    pub status: String,
}
/// Compliance training assignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainingRow {
    pub id: i64,
    pub org: String,
    pub title: String,
    pub assignee: String,
    pub due_date: String,
    pub status: String,
}
/// Enterprise risk entry; score = likelihood x impact (1-25).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskRow {
    pub id: i64,
    pub org: String,
    pub title: String,
    pub category: String,
    pub likelihood: i64,
    pub impact: i64,
    pub owner: String,
    pub status: String,
}
/// Data-subject rights request (access / deletion / rectification).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DsarRow {
    pub id: i64,
    pub org: String,
    pub subject: String,
    pub kind: String,
    pub detail: String,
    pub due_date: String,
    pub status: String,
}
/// Ethics & independence declaration (annual affirmation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthicsRow {
    pub id: i64,
    pub org: String,
    pub declarant: String,
    pub year: String,
    pub statement: String,
    pub status: String,
}
/// Gift & hospitality entry (gratifikasi control).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GiftRow {
    pub id: i64,
    pub org: String,
    pub direction: String,
    pub description: String,
    pub counterpart: String,
    pub given_at: String,
    pub status: String,
}
/// Training completion evidence note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainingEvidenceRow {
    pub id: i64,
    pub org: String,
    pub training_id: i64,
    pub note: String,
}
/// Audit finding (temuan audit internal/eksternal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditFindingRow {
    pub id: i64,
    pub org: String,
    pub title: String,
    pub area: String,
    pub severity: String,
    pub status: String,
}
/// Whistleblowing report (intake channel; contact empty = anonymous).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhistleRow {
    pub id: i64,
    pub org: String,
    pub channel: String,
    pub category: String,
    pub summary: String,
    pub contact: String,
    pub status: String,
}
/// Exception / waiver request (policy deviation with expiry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaiverRow {
    pub id: i64,
    pub org: String,
    pub title: String,
    pub justification: String,
    pub requested_by: String,
    pub expires_at: String,
    pub status: String,
}
/// Policy library document (draft -> published -> archived).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyRow {
    pub id: i64,
    pub org: String,
    pub title: String,
    pub version: String,
    pub body: String,
    pub status: String,
}
/// Policy acknowledgement (one per user per policy).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyAckRow {
    pub id: i64,
    pub org: String,
    pub policy_id: i64,
    pub user_id: String,
}
/// Control library entry (UUJN/PDP/ITE).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRow {
    pub id: i64,
    pub org: String,
    pub code: String,
    pub title: String,
    pub area: String,
    pub frequency: String,
    pub active: bool,
}
/// Control test log entry (scheduled operation evidence).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlTestRow {
    pub id: i64,
    pub org: String,
    pub control_id: i64,
    pub tested_at: String,
    pub result: String,
    pub note: String,
}
/// Internal audit plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditPlanRow {
    pub id: i64,
    pub org: String,
    pub title: String,
    pub period: String,
    pub scope: String,
    pub status: String,
}
/// Matter-level compliance checklist item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChecklistRow {
    pub id: i64,
    pub org: String,
    pub matter_id: i64,
    pub item: String,
    pub done: bool,
}
/// Compliance evidence vault link (register entry -> evidence note).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRow {
    pub id: i64,
    pub org: String,
    pub title: String,
    pub register_name: String,
    pub register_id: i64,
    pub note: String,
}
/// Protocol book (buku protokol notaris/PPAT).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PbookRow {
    pub id: i64,
    pub org: String,
    pub book_no: String,
    pub book_year: String,
    pub status: String,
}
/// Protocol custody handover log entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhandoverRow {
    pub id: i64,
    pub org: String,
    pub book_id: i64,
    pub from_holder: String,
    pub to_holder: String,
    pub handed_at: String,
    pub note: String,
}
/// Penghadap appointment (scheduler + queue board source).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppointmentRow {
    pub id: i64,
    pub org: String,
    pub client_name: String,
    pub purpose: String,
    pub scheduled_at: String,
    pub status: String,
}
/// Service price-list item (tarif layanan).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceRow {
    pub id: i64,
    pub org: String,
    pub code: String,
    pub name: String,
    pub amount_minor: i64,
    pub active: bool,
}
/// Deed fee quotation header (total = sum of snapshotted items).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotationRow {
    pub id: i64,
    pub org: String,
    pub client_name: String,
    pub deed_kind: String,
    pub total_minor: i64,
    pub status: String,
}
/// Office cash-book entry (immutable log; balance is derived).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashEntryRow {
    pub id: i64,
    pub org: String,
    pub kind: String,
    pub amount_minor: i64,
    pub note: String,
    pub occurred_on: String,
}
/// Cash advance / IOU (panjar).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvanceRow {
    pub id: i64,
    pub org: String,
    pub holder: String,
    pub amount_minor: i64,
    pub purpose: String,
    pub status: String,
}
/// Office inventory item (blanko, meterai, ATK).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryRow {
    pub id: i64,
    pub org: String,
    pub kind: String,
    pub code: String,
    pub name: String,
    pub qty: i64,
    pub unit: String,
    pub min_stock: i64,
}
/// Archive storage box (room/shelf location map).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveBoxRow {
    pub id: i64,
    pub org: String,
    pub box_code: String,
    pub location: String,
    pub description: String,
}
/// Meterai/blanko usage log (decrements stock atomically).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageRow {
    pub id: i64,
    pub org: String,
    pub item_id: i64,
    pub qty: i64,
    pub need: String,
    pub used_on: String,
}
/// Office visitor log entry (check-in -> check-out).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisitorRow {
    pub id: i64,
    pub org: String,
    pub name: String,
    pub need: String,
    pub check_in: String,
    pub check_out: String,
    pub status: String,
}
/// Office announcement (pinned board).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnouncementRow {
    pub id: i64,
    pub org: String,
    pub title: String,
    pub body: String,
    pub pinned: bool,
    pub active: bool,
}
/// Down-payment (uang muka) receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownPaymentRow {
    pub id: i64,
    pub org: String,
    pub receipt_no: String,
    pub client_name: String,
    pub matter_id: Option<i64>,
    pub amount_minor: i64,
    pub issued_on: String,
    pub status: String,
}
/// Witness pool entry (saksi akta).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessRow {
    pub id: i64,
    pub org: String,
    pub name: String,
    pub id_number: String,
    pub phone: String,
    pub active: bool,
}
/// PPAT party (seller/buyer/witness) with KYC verification flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PpatPartyRow {
    pub id: i64,
    pub org: String,
    pub ppat_id: i64,
    pub role: String,
    pub name: String,
    pub id_number: String,
    pub verified: bool,
}
/// Certificate handover receipt (berita acara penyerahan sertifikat).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertHandoverRow {
    pub id: i64,
    pub org: String,
    pub ppat_id: i64,
    pub handed_to: String,
    pub handed_at: String,
    pub receipt_no: String,
    pub status: String,
}
/// Manual identity re-verification log (external Dukcapil check BLOCKED).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecheckRow {
    pub id: i64,
    pub org: String,
    pub subject: String,
    pub method: String,
    pub result: String,
    pub verified_on: String,
    pub note: String,
}
/// Deed storage box (capacity-tracked).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageBoxRow {
    pub id: i64,
    pub org: String,
    pub box_code: String,
    pub location: String,
    pub capacity: i64,
    pub status: String,
}
/// MPD protocol transfer (retired-notary custody handover).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MpdTransferRow {
    pub id: i64,
    pub org: String,
    pub book_id: Option<i64>,
    pub recipient: String,
    pub transfer_date: String,
    pub receipt_no: String,
    pub status: String,
}
/// Office document checklist item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocCheckRow {
    pub id: i64,
    pub org: String,
    pub doc_id: i64,
    pub item: String,
    pub done: bool,
}
/// Office incident log entry (non-compliance ops incidents).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficeIncidentRow {
    pub id: i64,
    pub org: String,
    pub title: String,
    pub severity: String,
    pub status: String,
}
/// Bank-partner facility & rate sheet entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankFacilityRow {
    pub id: i64,
    pub org: String,
    pub bank_name: String,
    pub facility: String,
    pub rate_text: String,
    pub tenor: String,
    pub active: bool,
}
/// Office feedback capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackRow {
    pub id: i64,
    pub org: String,
    pub name: String,
    pub message: String,
    pub rating: i64,
    pub status: String,
}
/// SOP library document (draft -> published -> archived).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SopRow {
    pub id: i64,
    pub org: String,
    pub code: String,
    pub title: String,
    pub version: String,
    pub body: String,
    pub status: String,
}
/// Backup verification log entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupCheckRow {
    pub id: i64,
    pub org: String,
    pub checked_at: String,
    pub method: String,
    pub result: String,
    pub note: String,
}
/// Compliance self-check run (manual assessment scores).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfcheckRow {
    pub id: i64,
    pub org: String,
    pub period: String,
    pub passed_items: i64,
    pub total_items: i64,
    pub note: String,
}
/// Protocol year-end seal (book-count snapshot + seal chain).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YearSealRow {
    pub id: i64,
    pub org: String,
    pub year: String,
    pub book_count: i64,
    pub note: String,
    pub status: String,
}
/// Mortgage-deed detail (APHT/SKMHT support + expiry monitor).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeedChargeRow {
    pub id: i64,
    pub org: String,
    pub ppat_id: i64,
    pub kind: String,
    pub creditor: String,
    pub ceiling_minor: i64,
    pub valid_until: String,
    pub note: String,
}
/// Weekly office-hours row (weekday 0=Senin..6=Ahad).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficeHoursRow {
    pub id: i64,
    pub org: String,
    pub weekday: i64,
    pub open_time: String,
    pub close_time: String,
    pub active: bool,
}
/// Roster member (staff shift assignment).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterRow {
    pub id: i64,
    pub org: String,
    pub name: String,
    pub role: String,
    pub shift: String,
    pub active: bool,
}
/// Leave / substitute request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaveRow {
    pub id: i64,
    pub org: String,
    pub holder: String,
    pub kind: String,
    pub from_date: String,
    pub to_date: String,
    pub substitute: String,
    pub status: String,
}
/// Quotation line item (price snapshot at issue time).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotationItemRow {
    pub id: i64,
    pub org: String,
    pub quotation_id: i64,
    pub price_code: String,
    pub qty: i64,
    pub amount_minor: i64,
}
/// Approval delegation (delegator -> delegate).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegationRow {
    pub id: i64,
    pub org: String,
    pub delegator: String,
    pub delegate: String,
    pub active: bool,
}
/// Pending organization invite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InviteRow {
    pub id: i64,
    pub org: String,
    pub email: String,
    pub role: String,
    pub accepted: bool,
}
#[derive(Debug, Clone)]
pub struct TrustAccountRow {
    pub id: i64,
    pub org: String,
    pub client_id: i64,
    pub currency: String,
    pub balance: i64,
}
#[derive(Debug, Clone)]
pub struct TrustEntryRow {
    pub id: i64,
    pub org: String,
    pub account_id: i64,
    pub matter_id: Option<i64>,
    pub kind: String,
    pub amount: i64,
    pub ref_type: String,
    pub ref_id: i64,
    pub created_by: String,
}
#[derive(Debug, Clone)]
pub struct RagHit {
    pub doc_id: i64,
    pub idx: i64,
    pub text: String,
}
#[derive(Debug, Clone)]
pub struct WebhookDeliveryRow {
    pub id: i64,
    pub org: Option<String>,
    pub provider: String,
    pub event: String,
    pub status: String,
    pub detail: String,
    pub direction: String,
}
#[derive(Debug, Clone)]
pub struct WebhookEndpointRow {
    pub id: i64,
    pub org: String,
    pub url: String,
    pub events: String,
    pub active: bool,
}
#[derive(Debug, Clone)]
pub struct PdpObligationRow {
    pub id: i64,
    pub org: String,
    pub title: String,
    pub category: String,
    pub status: String,
    pub owner: String,
}
#[derive(Debug, Clone)]
pub struct ConsentRow {
    pub id: i64,
    pub org: String,
    pub subject: String,
    pub purpose: String,
    pub status: String,
}
#[derive(Debug, Clone)]
pub struct RopaRow {
    pub id: i64,
    pub org: String,
    pub activity: String,
    pub purpose: String,
    pub data_categories: String,
    pub legal_basis: String,
    pub retention: String,
}
#[derive(Debug, Clone)]
pub struct NotifRow {
    pub id: i64,
    pub kind: String,
    pub title: String,
    pub read: bool,
}
#[derive(Debug, Clone)]
pub struct ShareRow {
    pub id: i64,
    pub org: String,
    pub doc: i64,
    pub token: String,
    pub expires: String,
    pub revoked: bool,
}
#[derive(Debug, Clone)]
pub struct JobRow {
    pub id: i64,
    pub org: String,
    pub kind: String,
    pub payload: String,
    pub status: String,
    pub attempts: i64,
}
#[derive(Debug, Clone)]
pub struct AuditRow {
    pub id: i64,
    pub actor: Option<String>,
    pub action: String,
    pub entity: String,
    pub entity_id: i64,
    pub created: String,
}
#[derive(Debug, Clone)]
pub struct MarketingLeadRow {
    pub id: i64,
    pub org: Option<String>,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub company: Option<String>,
    pub source: String,
    pub status: String,
    pub assigned_to: Option<String>,
}
#[derive(Debug, Clone)]
pub struct MarketingDemoRow {
    pub id: i64,
    pub lead_id: Option<i64>,
    pub name: String,
    pub email: String,
    pub company: String,
    pub preferred_at: Option<String>,
    pub status: String,
}
#[derive(Debug, Clone)]
pub struct MarketingContactRow {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub subject: String,
    pub message: String,
    pub status: String,
}
#[derive(Debug, Clone)]
pub struct MarketingSubscriberRow {
    pub email: String,
    pub confirmed: bool,
}
#[derive(Debug, Clone)]
pub struct MarketingNoteRow {
    pub id: i64,
    pub lead_id: i64,
    pub author: String,
    pub note: String,
}
#[derive(Debug, Clone)]
pub struct MarketingFollowUpRow {
    pub id: i64,
    pub lead_id: i64,
    pub due_at: String,
    pub done: bool,
    pub note: String,
}
#[derive(Debug, Clone)]
pub struct MarketingPlanRow {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub monthly_minor: i64,
    pub annual_minor: i64,
    pub features_json: String,
    pub active: bool,
}
#[derive(Debug, Clone)]
pub struct MarketingFaqRow {
    pub id: i64,
    pub slug: String,
    pub question: String,
    pub answer: String,
    pub sort: i64,
}
#[derive(Debug, Clone)]
pub struct MarketingArticleRow {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub body_md: String,
    pub published: bool,
}
#[derive(Debug, Clone)]
pub struct MeetingRow {
    pub id: i64,
    pub org: String,
    pub client_id: i64,
    pub title: String,
    pub notes: Option<String>,
    pub happened_at: String,
    pub created_by: String,
}
#[derive(Debug, Clone)]
pub struct ClientTagRow {
    pub id: i64,
    pub org: String,
    pub name: String,
}

/// String-level pipeline guard mirroring `legalrust-domain::MarketingLeadStatus`.
/// Kept here so the database crate needs no domain dependency.
pub fn marketing_allowed_transition(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("new", "contacted")
            | ("new", "lost")
            | ("contacted", "qualified")
            | ("contacted", "lost")
            | ("qualified", "demo")
            | ("qualified", "lost")
            | ("demo", "proposal")
            | ("demo", "lost")
            | ("proposal", "negotiation")
            | ("proposal", "lost")
            | ("negotiation", "won")
            | ("negotiation", "lost")
            | ("lost", "new")
    )
}

#[async_trait::async_trait]
pub trait MarketingStore: Send + Sync {
    async fn mkt_lead_create(
        &self,
        org: Option<&str>,
        name: &str,
        email: &str,
        phone: Option<&str>,
        company: Option<&str>,
        source: &str,
    ) -> Result<MarketingLeadRow, StoreError>;
    async fn mkt_lead_list(&self, org: Option<&str>) -> Result<Vec<MarketingLeadRow>, StoreError>;
    async fn mkt_lead_get(&self, id: i64) -> Result<MarketingLeadRow, StoreError>;
    async fn mkt_lead_set_status(
        &self,
        id: i64,
        status: &str,
    ) -> Result<MarketingLeadRow, StoreError>;
    async fn mkt_lead_assign(
        &self,
        id: i64,
        assignee: Option<&str>,
    ) -> Result<MarketingLeadRow, StoreError>;
    async fn mkt_note_add(
        &self,
        lead: i64,
        author: &str,
        note: &str,
    ) -> Result<MarketingNoteRow, StoreError>;
    async fn mkt_note_list(&self, lead: i64) -> Result<Vec<MarketingNoteRow>, StoreError>;
    async fn mkt_followup_add(
        &self,
        lead: i64,
        due_at: &str,
        note: &str,
    ) -> Result<MarketingFollowUpRow, StoreError>;
    async fn mkt_followup_list(&self, lead: i64) -> Result<Vec<MarketingFollowUpRow>, StoreError>;
    async fn mkt_followup_done(&self, id: i64) -> Result<MarketingFollowUpRow, StoreError>;
    async fn mkt_demo_create(
        &self,
        lead: Option<i64>,
        name: &str,
        email: &str,
        company: &str,
        preferred_at: Option<&str>,
    ) -> Result<MarketingDemoRow, StoreError>;
    async fn mkt_demo_list(&self) -> Result<Vec<MarketingDemoRow>, StoreError>;
    async fn mkt_contact_create(
        &self,
        name: &str,
        email: &str,
        subject: &str,
        message: &str,
    ) -> Result<MarketingContactRow, StoreError>;
    async fn mkt_contact_list(&self) -> Result<Vec<MarketingContactRow>, StoreError>;
    async fn mkt_subscribe(&self, email: &str) -> Result<MarketingSubscriberRow, StoreError>;
    async fn mkt_unsubscribe(&self, email: &str) -> Result<(), StoreError>;
    async fn mkt_pricing_upsert(
        &self,
        code: &str,
        name: &str,
        monthly_minor: i64,
        annual_minor: i64,
        features_json: &str,
        active: bool,
    ) -> Result<MarketingPlanRow, StoreError>;
    async fn mkt_pricing_list(&self) -> Result<Vec<MarketingPlanRow>, StoreError>;
    async fn mkt_faq_upsert(
        &self,
        slug: &str,
        question: &str,
        answer: &str,
        sort: i64,
    ) -> Result<MarketingFaqRow, StoreError>;
    async fn mkt_faq_list(&self) -> Result<Vec<MarketingFaqRow>, StoreError>;
    async fn mkt_article_upsert(
        &self,
        slug: &str,
        title: &str,
        body_md: &str,
        published: bool,
    ) -> Result<MarketingArticleRow, StoreError>;
    async fn mkt_article_get(&self, slug: &str) -> Result<MarketingArticleRow, StoreError>;
    async fn mkt_article_list(
        &self,
        published_only: bool,
    ) -> Result<Vec<MarketingArticleRow>, StoreError>;
}
#[async_trait::async_trait]
pub trait OpsStore: Send + Sync {
    async fn ocr_create(&self, org: &str, doc: i64, doc_type: &str) -> Result<i64, StoreError>;
    async fn ocr_list(&self, org: &str) -> Result<Vec<OcrJobRow>, StoreError>;
    async fn ocr_get(
        &self,
        org: &str,
        id: i64,
    ) -> Result<(OcrJobRow, Vec<OcrFieldRow>), StoreError>;
    async fn ocr_set_status(&self, org: &str, id: i64, status: &str) -> Result<(), StoreError>;
    /// Store engine-extracted fields and set the final job status atomically
    /// (single lock / single DB transaction — no half-written results).
    /// Re-processing a job replaces its previous fields (idempotent).
    async fn ocr_store_result(
        &self,
        org: &str,
        id: i64,
        status: &str,
        fields: &[OcrFieldRow],
    ) -> Result<(), StoreError>;
    async fn ocr_review(
        &self,
        org: &str,
        id: i64,
        reviewer: &str,
        decision: &str,
        note: Option<&str>,
    ) -> Result<(), StoreError>;
    async fn tpl_create(
        &self,
        org: &str,
        name: &str,
        body: &str,
    ) -> Result<TemplateRow, StoreError>;
    async fn tpl_list(&self, org: &str) -> Result<Vec<TemplateRow>, StoreError>;
    async fn tpl_get(&self, org: &str, id: i64) -> Result<TemplateRow, StoreError>;
    async fn tpl_update(&self, org: &str, id: i64, body: &str) -> Result<TemplateRow, StoreError>;
    async fn clause_create(
        &self,
        org: &str,
        name: &str,
        category: &str,
        body: &str,
    ) -> Result<ClauseRow, StoreError>;
    async fn clause_list(&self, org: &str) -> Result<Vec<ClauseRow>, StoreError>;
    async fn clause_update(&self, org: &str, id: i64, body: &str) -> Result<ClauseRow, StoreError>;
    async fn clause_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    async fn deed_create(
        &self,
        org: &str,
        matter: i64,
        number: &str,
        deed_type: &str,
        rendered: &str,
    ) -> Result<DeedRow, StoreError>;
    async fn deed_list(&self, org: &str) -> Result<Vec<DeedRow>, StoreError>;
    async fn deed_set_state(&self, org: &str, id: i64, state: &str) -> Result<DeedRow, StoreError>;
    async fn conv_create(&self, org: &str, title: &str) -> Result<i64, StoreError>;
    async fn msg_add(&self, conv: i64, role: &str, content: &str) -> Result<(), StoreError>;
    async fn usage_add(&self, org: &str, input: i64, output: i64) -> Result<(), StoreError>;
    async fn usage_sum(&self, org: &str) -> Result<(i64, i64), StoreError>;
    async fn rag_put(&self, doc: i64, chunks: &[String]) -> Result<(), StoreError>;
    async fn rag_search(&self, org: &str, q: &str, limit: i64) -> Result<Vec<RagHit>, StoreError>;
    async fn inv_create(
        &self,
        org: &str,
        matter: i64,
        number: &str,
        total: i64,
        currency: &str,
    ) -> Result<InvoiceRow, StoreError>;
    async fn inv_list(&self, org: &str, matter: i64) -> Result<Vec<InvoiceRow>, StoreError>;
    async fn inv_get(
        &self,
        org: &str,
        id: i64,
    ) -> Result<(InvoiceRow, Vec<PaymentRow>), StoreError>;
    /// Open (partially paid or unpaid) invoices with paid/balance math.
    async fn inv_list_open(&self, org: &str) -> Result<Vec<ArOpenRow>, StoreError>;
    async fn pay_add(
        &self,
        org: &str,
        inv: i64,
        amount: i64,
        method: &str,
    ) -> Result<PaymentRow, StoreError>;
    /// Owner organization of an invoice (for unauthenticated callbacks that
    /// only carry the invoice id). Returns `Ok(None)` when unknown.
    async fn inv_owner_org(&self, inv: i64) -> Result<Option<String>, StoreError>;
    /// Create an approval request with ordered steps (single transaction).
    /// `steps` is `[(step_no, approver_role)]`; step numbers must start at 1
    /// and be contiguous. Entity must be one of `deed|invoice|matter`.
    async fn appr_create(
        &self,
        org: &str,
        entity: &str,
        entity_id: i64,
        title: &str,
        created_by: &str,
        steps: &[(i64, String)],
    ) -> Result<i64, StoreError>;
    async fn appr_list(
        &self,
        org: &str,
        status: Option<&str>,
    ) -> Result<Vec<ApprovalReqRow>, StoreError>;
    async fn appr_get(
        &self,
        org: &str,
        id: i64,
    ) -> Result<(ApprovalReqRow, Vec<ApprovalStepRow>), StoreError>;
    /// Decide step `step_no` as `user`. Enforces: request open, step pending,
    /// all earlier steps approved, decision in {approve,reject,rework}.
    /// `approve` on the last step closes the request approved; `reject`
    /// closes it rejected; `rework` resets all steps to pending with the
    /// request back at `needs_rework`.
    async fn appr_decide(
        &self,
        org: &str,
        id: i64,
        step_no: i64,
        user: &str,
        decision: &str,
        comment: Option<&str>,
    ) -> Result<ApprovalStepRow, StoreError>;
    /// Controlled bulk decide: all items validated first, then applied
    /// atomically (single Tx). Any failure rolls everything back.
    /// Items are `(request_id, step_no, decision, comment)`.
    async fn appr_decide_bulk(
        &self,
        org: &str,
        user: &str,
        items: &[(i64, i64, String, Option<String>)],
    ) -> Result<Vec<ApprovalStepRow>, StoreError>;
    /// Revoke a non-terminal request (`pending`/`needs_rework` only).
    async fn appr_revoke(&self, org: &str, id: i64, user: &str) -> Result<(), StoreError>;
    /// Idempotency-key claim for approval mutations. First claim wins;
    /// replays get `Conflict("replayed")` before any mutation runs.
    async fn idem_claim(&self, org: &str, key: &str, endpoint: &str) -> Result<(), StoreError>;
    /// Approval digest + escalation preferences (per user id).
    async fn apref_get(&self, user: &str) -> Result<ApprPrefRow, StoreError>;
    async fn apref_set(
        &self,
        user: &str,
        digest_mode: &str,
        escalate_to: &str,
    ) -> Result<ApprPrefRow, StoreError>;
    /// Approval audit freeze (per org). While frozen, decide paths 409.
    async fn freeze_get(&self, org: &str) -> Result<FreezeRow, StoreError>;
    async fn freeze_set(
        &self,
        org: &str,
        frozen: bool,
        reason: &str,
        set_by: &str,
    ) -> Result<FreezeRow, StoreError>;
    /// Approval routing rules (amount thresholds + AML-risk floors).
    #[allow(clippy::too_many_arguments)]
    async fn aroute_add(
        &self,
        org: &str,
        kind: &str,
        entity: &str,
        min_value: i64,
        role: &str,
        priority: i64,
        note: &str,
    ) -> Result<i64, StoreError>;
    async fn aroute_list(&self, org: &str) -> Result<Vec<RouteRuleRow>, StoreError>;
    async fn aroute_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Approval policy snapshots (immutable, versioned per org).
    async fn pol_snapshot(
        &self,
        org: &str,
        payload: &str,
        created_by: &str,
    ) -> Result<(i64, i64), StoreError>;
    async fn pol_list(&self, org: &str) -> Result<Vec<PolicySnapRow>, StoreError>;
    async fn pol_get(&self, org: &str, version: i64) -> Result<PolicySnapRow, StoreError>;
    /// Document lifecycle flags (upsert) + retention-due listing.
    #[allow(clippy::too_many_arguments)]
    async fn dlc_set(
        &self,
        org: &str,
        doc_id: i64,
        on_hold: Option<bool>,
        archived: Option<bool>,
        trashed: Option<bool>,
        retain_until: Option<&str>,
        note: Option<&str>,
    ) -> Result<DocLifecycleRow, StoreError>;
    async fn dlc_get(&self, org: &str, doc_id: i64) -> Result<Option<DocLifecycleRow>, StoreError>;
    async fn dlc_due(&self, org: &str) -> Result<Vec<DocLifecycleRow>, StoreError>;
    async fn dlc_clear(&self, org: &str, doc_id: i64) -> Result<(), StoreError>;
    /// Metadata schema + values.
    async fn mfield_add(
        &self,
        org: &str,
        key: &str,
        kind: &str,
        required: bool,
        note: &str,
    ) -> Result<i64, StoreError>;
    async fn mfield_list(&self, org: &str) -> Result<Vec<MetaFieldRow>, StoreError>;
    async fn mfield_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    async fn dmeta_set(
        &self,
        org: &str,
        doc_id: i64,
        key: &str,
        value: &str,
    ) -> Result<i64, StoreError>;
    async fn dmeta_list(&self, org: &str, doc_id: i64) -> Result<Vec<DocMetaRow>, StoreError>;
    async fn dmeta_delete(
        &self,
        org: &str,
        doc_id: i64,
        key: &str,
        value: Option<&str>,
    ) -> Result<u64, StoreError>;
    async fn dmeta_by_tag(&self, org: &str, tag: &str) -> Result<Vec<DocMetaRow>, StoreError>;
    /// Check-in/out locking (one holder per doc).
    async fn dlock_lock(
        &self,
        org: &str,
        doc_id: i64,
        holder: &str,
        note: &str,
    ) -> Result<DocLockRow, StoreError>;
    async fn dlock_get(&self, org: &str, doc_id: i64) -> Result<Option<DocLockRow>, StoreError>;
    async fn dlock_unlock(
        &self,
        org: &str,
        doc_id: i64,
        by: &str,
        force: bool,
    ) -> Result<(), StoreError>;
    /// Favorites (per user id).
    async fn dfav_add(&self, org: &str, user: &str, doc_id: i64) -> Result<(), StoreError>;
    async fn dfav_list(&self, org: &str, user: &str) -> Result<Vec<i64>, StoreError>;
    async fn dfav_remove(&self, org: &str, user: &str, doc_id: i64) -> Result<(), StoreError>;
    /// Quorum rounds (M-of-N) + votes + dual-control overrides.
    #[allow(clippy::too_many_arguments)]
    async fn qround_add(
        &self,
        org: &str,
        entity: &str,
        entity_id: i64,
        title: &str,
        required: i64,
        members: &[String],
        created_by: &str,
    ) -> Result<i64, StoreError>;
    async fn qround_list(&self, org: &str) -> Result<Vec<QuorumRoundRow>, StoreError>;
    async fn qround_get(
        &self,
        org: &str,
        id: i64,
    ) -> Result<(QuorumRoundRow, Vec<QuorumVoteRow>), StoreError>;
    async fn qvote_add(
        &self,
        org: &str,
        round_id: i64,
        voter: &str,
        decision: &str,
    ) -> Result<String, StoreError>;
    async fn qoverride_request(
        &self,
        org: &str,
        round_id: i64,
        requested_by: &str,
    ) -> Result<i64, StoreError>;
    async fn qoverride_decide(
        &self,
        org: &str,
        id: i64,
        approve: bool,
        decided_by: &str,
    ) -> Result<String, StoreError>;
    /// Decision receipt chain (hash `GENESIS` for the first receipt).
    #[allow(clippy::too_many_arguments)]
    async fn receipt_append(
        &self,
        org: &str,
        req_id: i64,
        step_no: i64,
        decision: &str,
        actor: &str,
        hash: &str,
        prev: &str,
    ) -> Result<i64, StoreError>;
    async fn receipt_list(&self, org: &str, req_id: i64) -> Result<Vec<ReceiptRow>, StoreError>;
    /// Privacy redaction of a step comment (admin only at the API layer).
    async fn appr_redact_comment(&self, org: &str, id: i64, step_no: i64)
        -> Result<(), StoreError>;
    /// Record an inbound webhook receipt (observability; never fails the
    /// webhook itself — callers ignore the result).
    async fn webhook_log(
        &self,
        org: Option<&str>,
        provider: &str,
        event: &str,
        status: &str,
        detail: &str,
    ) -> Result<i64, StoreError>;
    /// Tenant-scoped delivery log, newest first (cap 200).
    async fn webhook_list(
        &self,
        org: &str,
        provider: Option<&str>,
    ) -> Result<Vec<WebhookDeliveryRow>, StoreError>;
    /// Register an outbound webhook endpoint (URL allowlist: http/https only).
    async fn webhook_ep_create(
        &self,
        org: &str,
        url: &str,
        secret: &str,
        events: &str,
    ) -> Result<i64, StoreError>;
    async fn webhook_ep_list(&self, org: &str) -> Result<Vec<WebhookEndpointRow>, StoreError>;
    async fn webhook_ep_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Endpoint secret for signing outbound test sends (server-side only,
    /// never serialized to API responses).
    async fn webhook_ep_secret(&self, org: &str, id: i64) -> Result<Option<String>, StoreError>;
    /// Create a UU PDP obligation (`open` initially).
    async fn pdp_obligation_create(
        &self,
        org: &str,
        title: &str,
        category: &str,
        owner: &str,
    ) -> Result<i64, StoreError>;
    async fn pdp_obligation_list(
        &self,
        org: &str,
        status: Option<&str>,
    ) -> Result<Vec<PdpObligationRow>, StoreError>;
    /// Transition an obligation: open <-> in_progress -> done (done terminal).
    async fn pdp_obligation_set_status(
        &self,
        org: &str,
        id: i64,
        status: &str,
    ) -> Result<(), StoreError>;
    /// Record consent granted (one active row per subject+purpose; re-grant
    /// after withdraw opens a new row).
    async fn consent_grant(
        &self,
        org: &str,
        subject: &str,
        purpose: &str,
    ) -> Result<i64, StoreError>;
    async fn consent_withdraw(&self, org: &str, id: i64) -> Result<(), StoreError>;
    async fn consent_list(
        &self,
        org: &str,
        subject: Option<&str>,
    ) -> Result<Vec<ConsentRow>, StoreError>;
    /// Record of Processing Activities entry.
    async fn ropa_create(
        &self,
        org: &str,
        activity: &str,
        purpose: &str,
        data_categories: &str,
        legal_basis: &str,
        retention: &str,
    ) -> Result<i64, StoreError>;
    async fn ropa_list(&self, org: &str) -> Result<Vec<RopaRow>, StoreError>;
    async fn ropa_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// PPAT deed register: create/list/status-step (draft->signed->registered).
    async fn ppat_create(
        &self,
        org: &str,
        number: &str,
        deed_kind: &str,
        seller: &str,
        buyer: &str,
        object: &str,
    ) -> Result<i64, StoreError>;
    async fn ppat_list(&self, org: &str) -> Result<Vec<PpatRow>, StoreError>;
    async fn ppat_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    /// PPAT monthly report buckets (kind x status) for `month` = YYYY-MM.
    async fn ppat_monthly(&self, org: &str, month: &str)
        -> Result<Vec<PpatMonthlyRow>, StoreError>;
    /// PPAT office dashboard snapshot (cross-table counts).
    async fn ppat_dashboard(&self, org: &str) -> Result<PpatDashboard, StoreError>;
    /// Notary-to-PPAT handoff checklist.
    async fn nhoff_add(
        &self,
        org: &str,
        ppat_id: i64,
        doc_name: &str,
        note: &str,
    ) -> Result<i64, StoreError>;
    async fn nhoff_list(
        &self,
        org: &str,
        ppat_id: i64,
    ) -> Result<Vec<NotaryHandoffRow>, StoreError>;
    async fn nhoff_set_received(
        &self,
        org: &str,
        id: i64,
        received: bool,
    ) -> Result<(), StoreError>;
    async fn nhoff_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Girik/adat land intake.
    async fn girik_add(
        &self,
        org: &str,
        letter_c_ref: &str,
        holder: &str,
        area_minor: i64,
        village: &str,
        note: &str,
    ) -> Result<i64, StoreError>;
    async fn girik_list(&self, org: &str) -> Result<Vec<GirikRow>, StoreError>;
    async fn girik_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn girik_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Lease-deed terms (one row per deed, upsert).
    #[allow(clippy::too_many_arguments)]
    async fn lease_upsert(
        &self,
        org: &str,
        ppat_id: i64,
        monthly_minor: i64,
        duration_months: i64,
        start_date: &str,
        deposit_minor: i64,
        note: &str,
    ) -> Result<i64, StoreError>;
    async fn lease_get(&self, org: &str, ppat_id: i64) -> Result<Option<LeaseTermRow>, StoreError>;
    async fn lease_delete(&self, org: &str, ppat_id: i64) -> Result<(), StoreError>;
    /// Developer bulk-transaction batches.
    async fn dbatch_add(
        &self,
        org: &str,
        developer: &str,
        batch_no: &str,
        total_units: i64,
        note: &str,
    ) -> Result<i64, StoreError>;
    async fn dbatch_list(&self, org: &str) -> Result<Vec<DevBatchRow>, StoreError>;
    async fn dbatch_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn dbatch_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Developer unit stock registry.
    async fn dunit_add(
        &self,
        org: &str,
        batch_id: Option<i64>,
        unit_code: &str,
        unit_type: &str,
        price_minor: i64,
    ) -> Result<i64, StoreError>;
    async fn dunit_list(
        &self,
        org: &str,
        batch_id: Option<i64>,
    ) -> Result<Vec<DevUnitRow>, StoreError>;
    async fn dunit_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn dunit_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// KPR bank coordination packs.
    async fn kpr_add(
        &self,
        org: &str,
        ppat_id: i64,
        bank: &str,
        amount_minor: i64,
        note: &str,
    ) -> Result<i64, StoreError>;
    async fn kpr_list(&self, org: &str, ppat_id: i64) -> Result<Vec<KprPackRow>, StoreError>;
    async fn kpr_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn kpr_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Protocol register entries.
    async fn preg_add(
        &self,
        org: &str,
        entry_date: &str,
        book_ref: &str,
        deed_no: &str,
        party: &str,
        direction: &str,
    ) -> Result<i64, StoreError>;
    async fn preg_list(&self, org: &str) -> Result<Vec<PregRow>, StoreError>;
    async fn preg_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Stored AJB drafts.
    async fn ajbd_add(
        &self,
        org: &str,
        ppat_id: i64,
        template_name: &str,
        body: &str,
        price_minor: i64,
    ) -> Result<i64, StoreError>;
    async fn ajbd_list(&self, org: &str, ppat_id: i64) -> Result<Vec<AjbDraftRow>, StoreError>;
    async fn ajbd_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Certificate vault links (deed <-> document scan).
    async fn clink_add(
        &self,
        org: &str,
        ppat_id: i64,
        doc_id: i64,
        doc_name: &str,
        label: &str,
    ) -> Result<i64, StoreError>;
    async fn clink_list(&self, org: &str, ppat_id: i64) -> Result<Vec<CertLinkRow>, StoreError>;
    async fn clink_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// BPN coordination tracks (manual mode).
    async fn bpn_add(
        &self,
        org: &str,
        kind: &str,
        ref_no: &str,
        office: &str,
        note: &str,
    ) -> Result<i64, StoreError>;
    async fn bpn_list(&self, org: &str, kind: Option<&str>)
        -> Result<Vec<BpnTrackRow>, StoreError>;
    async fn bpn_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn bpn_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Data-retention rules.
    async fn rrule_upsert(
        &self,
        org: &str,
        entity: &str,
        retain_years: i64,
        action: &str,
        note: &str,
    ) -> Result<i64, StoreError>;
    async fn rrule_list(&self, org: &str) -> Result<Vec<RetentionRuleRow>, StoreError>;
    async fn rrule_set_active(&self, org: &str, id: i64, active: bool) -> Result<(), StoreError>;
    async fn rrule_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Approval-gated purge requests.
    async fn preq_add(
        &self,
        org: &str,
        entity: &str,
        entity_id: i64,
        reason: &str,
        requested_by: &str,
    ) -> Result<i64, StoreError>;
    async fn preq_list(&self, org: &str) -> Result<Vec<PurgeRequestRow>, StoreError>;
    async fn preq_decide(
        &self,
        org: &str,
        id: i64,
        approve: bool,
        decided_by: &str,
    ) -> Result<String, StoreError>;
    async fn preq_execute(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Out-of-office approval delegation: delegate may decide steps the
    /// delegator could decide. Returns delegation id.
    async fn delegation_set(
        &self,
        org: &str,
        delegator: &str,
        delegate: &str,
        active: bool,
    ) -> Result<i64, StoreError>;
    async fn delegation_list(&self, org: &str) -> Result<Vec<DelegationRow>, StoreError>;
    /// Delegator ids that actively delegate to `user` (same org).
    async fn delegation_delegators(&self, org: &str, user: &str)
        -> Result<Vec<String>, StoreError>;
    /// Organization invite: returns (id, raw_token shown once).
    async fn invite_create(
        &self,
        org: &str,
        email: &str,
        role: &str,
        hash: &str,
        expires_at: &str,
    ) -> Result<i64, StoreError>;
    async fn invite_list(&self, org: &str) -> Result<Vec<InviteRow>, StoreError>;
    async fn invite_peek(&self, hash: &str) -> Result<Option<InviteRow>, StoreError>;
    async fn invite_redeem(&self, hash: &str) -> Result<InviteRow, StoreError>;
    async fn invite_revoke(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// UU PDP DPIA assessment register.
    async fn dpia_create(
        &self,
        org: &str,
        title: &str,
        data_types: &str,
        risk: &str,
    ) -> Result<i64, StoreError>;
    async fn dpia_list(&self, org: &str) -> Result<Vec<DpiaRow>, StoreError>;
    async fn dpia_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn vendor_create(&self, org: &str, name: &str, service: &str) -> Result<i64, StoreError>;
    async fn vendor_list(&self, org: &str) -> Result<Vec<VendorRow>, StoreError>;
    async fn vendor_set_dpa(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn vendor_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// UU PDP breach log (72h notification tracking).
    async fn breach_create(
        &self,
        org: &str,
        title: &str,
        discovered: &str,
        affected: i64,
    ) -> Result<i64, StoreError>;
    async fn breach_list(&self, org: &str) -> Result<Vec<BreachRow>, StoreError>;
    async fn breach_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    /// Operational incident register.
    async fn incident_create(
        &self,
        org: &str,
        title: &str,
        severity: &str,
    ) -> Result<i64, StoreError>;
    async fn incident_list(&self, org: &str) -> Result<Vec<IncidentRow>, StoreError>;
    async fn incident_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn incident_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// CAPA tracker.
    async fn capa_create(
        &self,
        org: &str,
        title: &str,
        cause: &str,
        action: &str,
    ) -> Result<i64, StoreError>;
    async fn capa_list(&self, org: &str) -> Result<Vec<CapaRow>, StoreError>;
    async fn capa_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn capa_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Regulatory change tracker.
    async fn regchange_create(
        &self,
        org: &str,
        title: &str,
        source: &str,
        effective: &str,
    ) -> Result<i64, StoreError>;
    async fn regchange_list(&self, org: &str) -> Result<Vec<RegChangeRow>, StoreError>;
    async fn regchange_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn regchange_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Compliance training assignments.
    async fn training_create(
        &self,
        org: &str,
        title: &str,
        assignee: &str,
        due: &str,
    ) -> Result<i64, StoreError>;
    async fn training_list(&self, org: &str) -> Result<Vec<TrainingRow>, StoreError>;
    async fn training_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn training_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Enterprise risk register (score = likelihood x impact).
    async fn risk_create(
        &self,
        org: &str,
        title: &str,
        category: &str,
        likelihood: i64,
        impact: i64,
        owner: &str,
    ) -> Result<i64, StoreError>;
    async fn risk_list(&self, org: &str) -> Result<Vec<RiskRow>, StoreError>;
    async fn risk_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn risk_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Data-subject rights requests (UU PDP Chapter on subject rights).
    async fn dsar_create(
        &self,
        org: &str,
        subject: &str,
        kind: &str,
        detail: &str,
        due: &str,
    ) -> Result<i64, StoreError>;
    async fn dsar_list(&self, org: &str) -> Result<Vec<DsarRow>, StoreError>;
    async fn dsar_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn dsar_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Ethics & independence declarations.
    async fn ethics_create(
        &self,
        org: &str,
        declarant: &str,
        year: &str,
        statement: &str,
    ) -> Result<i64, StoreError>;
    async fn ethics_list(&self, org: &str) -> Result<Vec<EthicsRow>, StoreError>;
    async fn ethics_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn ethics_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Gift & hospitality register.
    async fn gift_create(
        &self,
        org: &str,
        direction: &str,
        description: &str,
        counterpart: &str,
        given_at: &str,
    ) -> Result<i64, StoreError>;
    async fn gift_list(&self, org: &str) -> Result<Vec<GiftRow>, StoreError>;
    async fn gift_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn gift_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Training completion evidence (notes attached to an assignment).
    async fn tevidence_add(
        &self,
        org: &str,
        training_id: i64,
        note: &str,
    ) -> Result<i64, StoreError>;
    async fn tevidence_list(
        &self,
        org: &str,
        training_id: i64,
    ) -> Result<Vec<TrainingEvidenceRow>, StoreError>;
    async fn tevidence_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Audit finding tracker.
    async fn afinding_create(
        &self,
        org: &str,
        title: &str,
        area: &str,
        severity: &str,
    ) -> Result<i64, StoreError>;
    async fn afinding_list(&self, org: &str) -> Result<Vec<AuditFindingRow>, StoreError>;
    async fn afinding_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn afinding_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Whistleblowing intake channel.
    async fn whistle_create(
        &self,
        org: &str,
        channel: &str,
        category: &str,
        summary: &str,
        contact: &str,
    ) -> Result<i64, StoreError>;
    async fn whistle_list(&self, org: &str) -> Result<Vec<WhistleRow>, StoreError>;
    async fn whistle_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn whistle_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Exception / waiver workflow.
    async fn waiver_create(
        &self,
        org: &str,
        title: &str,
        justification: &str,
        requested_by: &str,
        expires_at: &str,
    ) -> Result<i64, StoreError>;
    async fn waiver_list(&self, org: &str) -> Result<Vec<WaiverRow>, StoreError>;
    async fn waiver_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn waiver_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Policy library with acknowledgement tracking.
    async fn policy_create(
        &self,
        org: &str,
        title: &str,
        version: &str,
        body: &str,
    ) -> Result<i64, StoreError>;
    async fn policy_list(&self, org: &str) -> Result<Vec<PolicyRow>, StoreError>;
    async fn policy_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn policy_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    async fn policy_ack(&self, org: &str, policy_id: i64, user: &str) -> Result<i64, StoreError>;
    async fn policy_ack_list(
        &self,
        org: &str,
        policy_id: i64,
    ) -> Result<Vec<PolicyAckRow>, StoreError>;
    /// Control library.
    async fn control_create(
        &self,
        org: &str,
        code: &str,
        title: &str,
        area: &str,
        frequency: &str,
    ) -> Result<i64, StoreError>;
    async fn control_list(&self, org: &str) -> Result<Vec<ControlRow>, StoreError>;
    async fn control_set_active(&self, org: &str, id: i64, active: bool) -> Result<(), StoreError>;
    async fn control_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Control test scheduler log.
    async fn ctest_add(
        &self,
        org: &str,
        control_id: i64,
        tested_at: &str,
        result: &str,
        note: &str,
    ) -> Result<i64, StoreError>;
    async fn ctest_list(
        &self,
        org: &str,
        control_id: i64,
    ) -> Result<Vec<ControlTestRow>, StoreError>;
    async fn ctest_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Internal audit plan register.
    async fn aplan_create(
        &self,
        org: &str,
        title: &str,
        period: &str,
        scope: &str,
    ) -> Result<i64, StoreError>;
    async fn aplan_list(&self, org: &str) -> Result<Vec<AuditPlanRow>, StoreError>;
    async fn aplan_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn aplan_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Matter-level compliance checklist.
    async fn clist_add(&self, org: &str, matter_id: i64, item: &str) -> Result<i64, StoreError>;
    async fn clist_list(&self, org: &str, matter_id: i64) -> Result<Vec<ChecklistRow>, StoreError>;
    async fn clist_toggle(&self, org: &str, id: i64, done: bool) -> Result<(), StoreError>;
    async fn clist_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Compliance evidence vault links.
    async fn evidence_add(
        &self,
        org: &str,
        title: &str,
        register_name: &str,
        register_id: i64,
        note: &str,
    ) -> Result<i64, StoreError>;
    async fn evidence_list(
        &self,
        org: &str,
        register_name: Option<&str>,
        register_id: Option<i64>,
    ) -> Result<Vec<EvidenceRow>, StoreError>;
    async fn evidence_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Protocol book register.
    async fn pbook_create(&self, org: &str, book_no: &str, year: &str) -> Result<i64, StoreError>;
    async fn pbook_list(&self, org: &str) -> Result<Vec<PbookRow>, StoreError>;
    async fn pbook_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn pbook_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Protocol custody handover log.
    async fn phandover_add(
        &self,
        org: &str,
        book_id: i64,
        from_holder: &str,
        to_holder: &str,
        handed_at: &str,
        note: &str,
    ) -> Result<i64, StoreError>;
    async fn phandover_list(
        &self,
        org: &str,
        book_id: i64,
    ) -> Result<Vec<PhandoverRow>, StoreError>;
    async fn phandover_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Penghadap appointment scheduler.
    async fn appt_create(
        &self,
        org: &str,
        client_name: &str,
        purpose: &str,
        scheduled_at: &str,
    ) -> Result<i64, StoreError>;
    async fn appt_list(&self, org: &str) -> Result<Vec<AppointmentRow>, StoreError>;
    async fn appt_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn appt_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Service price list.
    async fn price_create(
        &self,
        org: &str,
        code: &str,
        name: &str,
        amount_minor: i64,
    ) -> Result<i64, StoreError>;
    async fn price_list(&self, org: &str) -> Result<Vec<PriceRow>, StoreError>;
    async fn price_set_active(&self, org: &str, id: i64, active: bool) -> Result<(), StoreError>;
    async fn price_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Deed fee quotations (amounts snapshotted from active price codes).
    async fn quote_create(
        &self,
        org: &str,
        client_name: &str,
        deed_kind: &str,
        items: &[(String, i64)],
    ) -> Result<i64, StoreError>;
    async fn quote_list(&self, org: &str) -> Result<Vec<QuotationRow>, StoreError>;
    async fn quote_items(
        &self,
        org: &str,
        quotation_id: i64,
    ) -> Result<Vec<QuotationItemRow>, StoreError>;
    async fn quote_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn quote_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Office cash book (immutable log + derived balance).
    async fn cash_add(
        &self,
        org: &str,
        kind: &str,
        amount_minor: i64,
        note: &str,
        occurred_on: &str,
    ) -> Result<i64, StoreError>;
    async fn cash_list(&self, org: &str) -> Result<Vec<CashEntryRow>, StoreError>;
    async fn cash_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Cash advance / IOU tracker.
    async fn advance_create(
        &self,
        org: &str,
        holder: &str,
        amount_minor: i64,
        purpose: &str,
    ) -> Result<i64, StoreError>;
    async fn advance_list(&self, org: &str) -> Result<Vec<AdvanceRow>, StoreError>;
    async fn advance_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn advance_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Office inventory (blanko/meterai/ATK).
    #[allow(clippy::too_many_arguments)]
    async fn stock_create(
        &self,
        org: &str,
        code: &str,
        name: &str,
        qty: i64,
        unit: &str,
        min_stock: i64,
        kind: &str,
    ) -> Result<i64, StoreError>;
    async fn stock_list(
        &self,
        org: &str,
        kind: Option<&str>,
    ) -> Result<Vec<InventoryRow>, StoreError>;
    async fn stock_adjust(&self, org: &str, id: i64, delta: i64) -> Result<i64, StoreError>;
    async fn stock_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Meterai/blanko usage (atomic stock decrement + log).
    async fn stock_use_add(
        &self,
        org: &str,
        item_id: i64,
        qty: i64,
        need: &str,
        used_on: &str,
    ) -> Result<i64, StoreError>;
    async fn stock_use_list(&self, org: &str, item_id: i64) -> Result<Vec<UsageRow>, StoreError>;
    async fn stock_use_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Archive storage boxes (location map).
    async fn abox_create(
        &self,
        org: &str,
        box_code: &str,
        location: &str,
        description: &str,
    ) -> Result<i64, StoreError>;
    async fn abox_list(&self, org: &str) -> Result<Vec<ArchiveBoxRow>, StoreError>;
    async fn abox_move(&self, org: &str, id: i64, location: &str) -> Result<(), StoreError>;
    async fn abox_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Office visitor log.
    async fn visitor_checkin(
        &self,
        org: &str,
        name: &str,
        need: &str,
        check_in: &str,
    ) -> Result<i64, StoreError>;
    async fn visitor_list(&self, org: &str) -> Result<Vec<VisitorRow>, StoreError>;
    async fn visitor_checkout(&self, org: &str, id: i64, check_out: &str)
        -> Result<(), StoreError>;
    async fn visitor_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Office announcement board.
    async fn announce_create(
        &self,
        org: &str,
        title: &str,
        body: &str,
        pinned: bool,
    ) -> Result<i64, StoreError>;
    async fn announce_list(&self, org: &str) -> Result<Vec<AnnouncementRow>, StoreError>;
    async fn announce_set(
        &self,
        org: &str,
        id: i64,
        pinned: Option<bool>,
        active: Option<bool>,
    ) -> Result<(), StoreError>;
    async fn announce_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Down-payment (uang muka) receipts.
    async fn dp_create(
        &self,
        org: &str,
        receipt_no: &str,
        client_name: &str,
        matter_id: Option<i64>,
        amount_minor: i64,
        issued_on: &str,
    ) -> Result<i64, StoreError>;
    async fn dp_list(&self, org: &str) -> Result<Vec<DownPaymentRow>, StoreError>;
    async fn dp_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn dp_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Witness pool.
    async fn witness_create(
        &self,
        org: &str,
        name: &str,
        id_number: &str,
        phone: &str,
    ) -> Result<i64, StoreError>;
    async fn witness_list(&self, org: &str) -> Result<Vec<WitnessRow>, StoreError>;
    async fn witness_set_active(&self, org: &str, id: i64, active: bool) -> Result<(), StoreError>;
    async fn witness_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Manual identity re-verification log.
    async fn recheck_add(
        &self,
        org: &str,
        subject: &str,
        method: &str,
        result: &str,
        verified_on: &str,
        note: &str,
    ) -> Result<i64, StoreError>;
    async fn recheck_list(&self, org: &str) -> Result<Vec<RecheckRow>, StoreError>;
    async fn recheck_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Office working-hours & roster.
    async fn hours_set(
        &self,
        org: &str,
        weekday: i64,
        open_time: &str,
        close_time: &str,
        active: bool,
    ) -> Result<i64, StoreError>;
    async fn hours_list(&self, org: &str) -> Result<Vec<OfficeHoursRow>, StoreError>;
    async fn roster_add(
        &self,
        org: &str,
        name: &str,
        role: &str,
        shift: &str,
    ) -> Result<i64, StoreError>;
    async fn roster_list(&self, org: &str) -> Result<Vec<RosterRow>, StoreError>;
    async fn roster_set_active(&self, org: &str, id: i64, active: bool) -> Result<(), StoreError>;
    async fn roster_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Leave / substitute protocol.
    async fn leave_request(
        &self,
        org: &str,
        holder: &str,
        kind: &str,
        from_date: &str,
        to_date: &str,
        substitute: &str,
    ) -> Result<i64, StoreError>;
    async fn leave_list(&self, org: &str) -> Result<Vec<LeaveRow>, StoreError>;
    async fn leave_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn leave_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Deed storage boxes.
    async fn sbox_create(
        &self,
        org: &str,
        box_code: &str,
        location: &str,
        capacity: i64,
    ) -> Result<i64, StoreError>;
    async fn sbox_list(&self, org: &str) -> Result<Vec<StorageBoxRow>, StoreError>;
    async fn sbox_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn sbox_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// MPD protocol transfers.
    async fn mtransfer_create(
        &self,
        org: &str,
        book_id: Option<i64>,
        recipient: &str,
        transfer_date: &str,
        receipt_no: &str,
    ) -> Result<i64, StoreError>;
    async fn mtransfer_list(&self, org: &str) -> Result<Vec<MpdTransferRow>, StoreError>;
    async fn mtransfer_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn mtransfer_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Office document checklist.
    async fn dclist_add(&self, org: &str, doc_id: i64, item: &str) -> Result<i64, StoreError>;
    async fn dclist_list(&self, org: &str, doc_id: i64) -> Result<Vec<DocCheckRow>, StoreError>;
    async fn dclist_toggle(&self, org: &str, id: i64, done: bool) -> Result<(), StoreError>;
    async fn dclist_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Office incident log.
    async fn oincident_create(
        &self,
        org: &str,
        title: &str,
        severity: &str,
    ) -> Result<i64, StoreError>;
    async fn oincident_list(&self, org: &str) -> Result<Vec<OfficeIncidentRow>, StoreError>;
    async fn oincident_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn oincident_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Bank-partner facility & rate sheet.
    async fn bankf_create(
        &self,
        org: &str,
        bank_name: &str,
        facility: &str,
        rate_text: &str,
        tenor: &str,
    ) -> Result<i64, StoreError>;
    async fn bankf_list(&self, org: &str) -> Result<Vec<BankFacilityRow>, StoreError>;
    async fn bankf_set_active(&self, org: &str, id: i64, active: bool) -> Result<(), StoreError>;
    async fn bankf_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Office feedback capture.
    async fn feedback_add(
        &self,
        org: &str,
        name: &str,
        message: &str,
        rating: i64,
    ) -> Result<i64, StoreError>;
    async fn feedback_list(&self, org: &str) -> Result<Vec<FeedbackRow>, StoreError>;
    async fn feedback_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn feedback_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// SOP library.
    async fn sop_create(
        &self,
        org: &str,
        code: &str,
        title: &str,
        version: &str,
        body: &str,
    ) -> Result<i64, StoreError>;
    async fn sop_list(&self, org: &str) -> Result<Vec<SopRow>, StoreError>;
    async fn sop_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn sop_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Backup verification log.
    async fn bcheck_add(
        &self,
        org: &str,
        checked_at: &str,
        method: &str,
        result: &str,
        note: &str,
    ) -> Result<i64, StoreError>;
    async fn bcheck_list(&self, org: &str) -> Result<Vec<BackupCheckRow>, StoreError>;
    async fn bcheck_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Compliance self-check runs.
    async fn selfcheck_add(
        &self,
        org: &str,
        period: &str,
        passed: i64,
        total: i64,
        note: &str,
    ) -> Result<i64, StoreError>;
    async fn selfcheck_list(&self, org: &str) -> Result<Vec<SelfcheckRow>, StoreError>;
    async fn selfcheck_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Protocol year-end sealing.
    async fn sealy_prepare(&self, org: &str, year: &str, note: &str) -> Result<i64, StoreError>;
    async fn sealy_list(&self, org: &str) -> Result<Vec<YearSealRow>, StoreError>;
    async fn sealy_seal(&self, org: &str, id: i64) -> Result<(), StoreError>;
    async fn sealy_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Mortgage-deed details (APHT/SKMHT) + expiry monitor.
    #[allow(clippy::too_many_arguments)]
    async fn charge_upsert(
        &self,
        org: &str,
        ppat_id: i64,
        kind: &str,
        creditor: &str,
        ceiling_minor: i64,
        valid_until: &str,
        note: &str,
    ) -> Result<i64, StoreError>;
    async fn charge_list(&self, org: &str, ppat_id: i64) -> Result<Vec<DeedChargeRow>, StoreError>;
    async fn charge_due(&self, org: &str, days: i64) -> Result<Vec<DeedChargeRow>, StoreError>;
    async fn charge_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// PPAT party KYC bundle.
    async fn pparty_add(
        &self,
        org: &str,
        ppat_id: i64,
        role: &str,
        name: &str,
        id_number: &str,
    ) -> Result<i64, StoreError>;
    async fn pparty_list(&self, org: &str, ppat_id: i64) -> Result<Vec<PpatPartyRow>, StoreError>;
    async fn pparty_set_verified(
        &self,
        org: &str,
        id: i64,
        verified: bool,
    ) -> Result<(), StoreError>;
    async fn pparty_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Certificate handover receipts.
    async fn chandover_add(
        &self,
        org: &str,
        ppat_id: i64,
        handed_to: &str,
        handed_at: &str,
        receipt_no: &str,
    ) -> Result<i64, StoreError>;
    async fn chandover_list(
        &self,
        org: &str,
        ppat_id: i64,
    ) -> Result<Vec<CertHandoverRow>, StoreError>;
    async fn chandover_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn chandover_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Zone (ZNT/NJOP) reference table.
    async fn zone_create(
        &self,
        org: &str,
        area_code: &str,
        area_name: &str,
        njop_minor: i64,
    ) -> Result<i64, StoreError>;
    async fn zone_list(&self, org: &str) -> Result<Vec<ZoneRow>, StoreError>;
    async fn zone_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Manual PBB status verification log.
    async fn pbb_add(
        &self,
        org: &str,
        nop: &str,
        tax_year: &str,
        result: &str,
        checked_on: &str,
        note: &str,
    ) -> Result<i64, StoreError>;
    async fn pbb_list(&self, org: &str) -> Result<Vec<PbbRow>, StoreError>;
    async fn pbb_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Land split (pemecahan) tracker.
    async fn lsplit_create(
        &self,
        org: &str,
        source_parcel: &str,
        new_parcels: &str,
        split_date: &str,
    ) -> Result<i64, StoreError>;
    async fn lsplit_list(&self, org: &str) -> Result<Vec<LandSplitRow>, StoreError>;
    async fn lsplit_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn lsplit_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Land merge (penggabungan) tracker.
    async fn lmerge_create(
        &self,
        org: &str,
        source_parcels: &str,
        result_parcel: &str,
        merge_date: &str,
    ) -> Result<i64, StoreError>;
    async fn lmerge_list(&self, org: &str) -> Result<Vec<LandMergeRow>, StoreError>;
    async fn lmerge_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn lmerge_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Land dispute flag & hold.
    async fn ldispute_create(
        &self,
        org: &str,
        parcel_ref: &str,
        kind: &str,
        description: &str,
    ) -> Result<i64, StoreError>;
    async fn ldispute_list(&self, org: &str) -> Result<Vec<LandDisputeRow>, StoreError>;
    async fn ldispute_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn ldispute_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Boundary & area verification.
    async fn bndry_add(
        &self,
        org: &str,
        parcel_ref: &str,
        declared_area: i64,
        measured_area: i64,
        checked_on: &str,
    ) -> Result<i64, StoreError>;
    async fn bndry_list(&self, org: &str) -> Result<Vec<BoundaryRow>, StoreError>;
    async fn bndry_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Deed consent checks (marital / heir / board / power-to-sell).
    async fn dconsent_add(
        &self,
        org: &str,
        ppat_id: i64,
        kind: &str,
        party: &str,
        note: &str,
    ) -> Result<i64, StoreError>;
    async fn dconsent_list(
        &self,
        org: &str,
        ppat_id: i64,
    ) -> Result<Vec<DeedConsentRow>, StoreError>;
    async fn dconsent_verify(&self, org: &str, id: i64, verified: bool) -> Result<(), StoreError>;
    async fn dconsent_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Land-history (riwayat tanah) records.
    async fn lhist_add(
        &self,
        org: &str,
        parcel_ref: &str,
        event: &str,
        detail: &str,
        event_date: &str,
    ) -> Result<i64, StoreError>;
    async fn lhist_list(
        &self,
        org: &str,
        parcel_ref: &str,
    ) -> Result<Vec<LandHistoryRow>, StoreError>;
    async fn lhist_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// Unit handover (BAST) records.
    async fn uho_add(
        &self,
        org: &str,
        unit_ref: &str,
        handed_from: &str,
        handed_to: &str,
        bast_no: &str,
        handed_on: &str,
    ) -> Result<i64, StoreError>;
    async fn uho_list(&self, org: &str) -> Result<Vec<UnitHandoverRow>, StoreError>;
    async fn uho_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError>;
    async fn uho_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    /// App-layer referential check: does `register_name`/`register_id` exist?
    /// Default implementation reuses the register list methods so memory and
    /// MySQL backends behave identically; unknown names yield `false`.
    async fn evidence_register_exists(
        &self,
        org: &str,
        register_name: &str,
        register_id: i64,
    ) -> Result<bool, StoreError> {
        let hit = match register_name {
            "obligations" => self
                .pdp_obligation_list(org, None)
                .await?
                .iter()
                .any(|r| r.id == register_id),
            "consents" => self
                .consent_list(org, None)
                .await?
                .iter()
                .any(|r| r.id == register_id),
            "ropa" => self
                .ropa_list(org)
                .await?
                .iter()
                .any(|r| r.id == register_id),
            "dpia" => self
                .dpia_list(org)
                .await?
                .iter()
                .any(|r| r.id == register_id),
            "breach" => self
                .breach_list(org)
                .await?
                .iter()
                .any(|r| r.id == register_id),
            "vendors" => self
                .vendor_list(org)
                .await?
                .iter()
                .any(|r| r.id == register_id),
            "incidents" => self
                .incident_list(org)
                .await?
                .iter()
                .any(|r| r.id == register_id),
            "capas" => self
                .capa_list(org)
                .await?
                .iter()
                .any(|r| r.id == register_id),
            "regchanges" => self
                .regchange_list(org)
                .await?
                .iter()
                .any(|r| r.id == register_id),
            "trainings" => self
                .training_list(org)
                .await?
                .iter()
                .any(|r| r.id == register_id),
            "risks" => self
                .risk_list(org)
                .await?
                .iter()
                .any(|r| r.id == register_id),
            "dsar" => self
                .dsar_list(org)
                .await?
                .iter()
                .any(|r| r.id == register_id),
            "ethics" => self
                .ethics_list(org)
                .await?
                .iter()
                .any(|r| r.id == register_id),
            "gifts" => self
                .gift_list(org)
                .await?
                .iter()
                .any(|r| r.id == register_id),
            "findings" => self
                .afinding_list(org)
                .await?
                .iter()
                .any(|r| r.id == register_id),
            "whistle" => self
                .whistle_list(org)
                .await?
                .iter()
                .any(|r| r.id == register_id),
            "waivers" => self
                .waiver_list(org)
                .await?
                .iter()
                .any(|r| r.id == register_id),
            "policies" => self
                .policy_list(org)
                .await?
                .iter()
                .any(|r| r.id == register_id),
            "controls" => self
                .control_list(org)
                .await?
                .iter()
                .any(|r| r.id == register_id),
            _ => false,
        };
        Ok(hit)
    }
    /// Notification preferences: per-user kind opt-out. Disabled kinds are
    /// skipped by the notify fan-out (F-D18-006).
    async fn notif_pref_set(&self, user: &str, kind: &str, enabled: bool)
        -> Result<(), StoreError>;
    async fn notif_pref_list(&self, user: &str) -> Result<Vec<(String, bool)>, StoreError>;
    async fn notify(
        &self,
        org: &str,
        user: &str,
        kind: &str,
        title: &str,
        body: Option<&str>,
    ) -> Result<i64, StoreError>;
    async fn notify_list(
        &self,
        user: &str,
        kind: Option<&str>,
        unread_only: bool,
        limit: i64,
    ) -> Result<Vec<NotifRow>, StoreError>;
    async fn notify_read(&self, user: &str, id: i64) -> Result<(), StoreError>;
    async fn share_create(
        &self,
        org: &str,
        doc: i64,
        token: &str,
        hours: i64,
    ) -> Result<i64, StoreError>;
    async fn share_list(&self, org: &str, doc: i64) -> Result<Vec<ShareRow>, StoreError>;
    async fn share_get(&self, token: &str) -> Result<Option<ShareRow>, StoreError>;
    async fn share_revoke(&self, org: &str, id: i64) -> Result<(), StoreError>;
    async fn job_enqueue(&self, org: &str, kind: &str, payload: &str) -> Result<i64, StoreError>;
    /// Enqueue with a not-before delay (durable backoff for retries).
    async fn job_enqueue_at(
        &self,
        org: &str,
        kind: &str,
        payload: &str,
        delay_secs: u64,
    ) -> Result<i64, StoreError>;
    async fn job_list(&self, org: &str) -> Result<Vec<JobRow>, StoreError>;
    async fn job_claim(&self) -> Result<Option<JobRow>, StoreError>;
    async fn job_done(&self, id: i64, ok: bool, note: &str) -> Result<(), StoreError>;
    async fn audit_list(
        &self,
        org: &str,
        entity: Option<&str>,
        actor: Option<&str>,
        limit: i64,
    ) -> Result<Vec<AuditRow>, StoreError>;
    async fn audit_add(
        &self,
        tenant: &str,
        actor: Option<&str>,
        action: &str,
        entity: &str,
        entity_id: i64,
    ) -> Result<(), StoreError>;
    async fn trust_create_account(
        &self,
        org: &str,
        client_id: i64,
        currency: &str,
    ) -> Result<TrustAccountRow, StoreError>;
    async fn trust_get(
        &self,
        org: &str,
        id: i64,
    ) -> Result<(TrustAccountRow, Vec<TrustEntryRow>), StoreError>;
    async fn trust_list(&self, org: &str) -> Result<Vec<TrustAccountRow>, StoreError>;
    #[allow(clippy::too_many_arguments)]
    async fn trust_post(
        &self,
        org: &str,
        account: i64,
        matter: Option<i64>,
        kind: &str,
        amount: i64,
        ref_type: &str,
        ref_id: i64,
        created_by: &str,
    ) -> Result<TrustEntryRow, StoreError>;
    #[allow(clippy::too_many_arguments)]
    async fn trust_transfer(
        &self,
        org: &str,
        from: i64,
        to: i64,
        amount: i64,
        matter: Option<i64>,
        ref_type: &str,
        ref_id: i64,
        created_by: &str,
    ) -> Result<(TrustEntryRow, TrustEntryRow), StoreError>;
}

/// Entities eligible for approval-gated purge (ephemeral tracker rows only;
/// deeds, consents and vault pins are legal holds and can never be purged).
pub(crate) fn purgeable(entity: &str) -> bool {
    matches!(entity, "bpn" | "girik" | "lhist" | "uho" | "preg" | "nhoff")
}

/// Entities a retention rule may cover (purgeable set plus informational
/// archive-only subjects).
pub(crate) fn rule_entity_ok(entity: &str) -> bool {
    purgeable(entity)
        || matches!(
            entity,
            "deed"
                | "ppat"
                | "charge"
                | "consent"
                | "vault"
                | "draft"
                | "audit"
                | "register"
                | "batch"
                | "unit"
                | "kpr"
                | "dispute"
        )
}

#[async_trait::async_trait]
pub trait CrmStore: Send + Sync {
    async fn meeting_create(
        &self,
        org: &str,
        client_id: i64,
        title: &str,
        notes: Option<&str>,
        happened_at: &str,
        created_by: &str,
    ) -> Result<MeetingRow, StoreError>;
    async fn meeting_list_by_client(
        &self,
        org: &str,
        client_id: i64,
    ) -> Result<Vec<MeetingRow>, StoreError>;
    async fn meeting_list_by_org(&self, org: &str) -> Result<Vec<MeetingRow>, StoreError>;
    async fn meeting_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    async fn tag_create(&self, org: &str, name: &str) -> Result<ClientTagRow, StoreError>;
    async fn tag_list(&self, org: &str) -> Result<Vec<ClientTagRow>, StoreError>;
    async fn tag_delete(&self, org: &str, id: i64) -> Result<(), StoreError>;
    async fn tag_attach(&self, org: &str, tag_id: i64, client_id: i64) -> Result<(), StoreError>;
    async fn tag_detach(&self, org: &str, tag_id: i64, client_id: i64) -> Result<(), StoreError>;
    async fn tags_for_client(
        &self,
        org: &str,
        client_id: i64,
    ) -> Result<Vec<ClientTagRow>, StoreError>;
}
