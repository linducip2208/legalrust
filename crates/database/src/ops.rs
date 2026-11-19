use super::store::StoreError;
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

/// Shared validation for approval creation: entity allowlist + contiguous
/// step numbers starting at 1 with non-empty roles.
pub fn appr_check(entity: &str, steps: &[(i64, String)]) -> Result<(), super::store::StoreError> {
    use super::store::StoreError;
    if !["deed", "invoice", "matter"].contains(&entity) {
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
    /// Revoke a non-terminal request (`pending`/`needs_rework` only).
    async fn appr_revoke(&self, org: &str, id: i64, user: &str) -> Result<(), StoreError>;
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
