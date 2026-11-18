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
    async fn notify(
        &self,
        org: &str,
        user: &str,
        kind: &str,
        title: &str,
        body: Option<&str>,
    ) -> Result<i64, StoreError>;
    async fn notify_list(&self, user: &str) -> Result<Vec<NotifRow>, StoreError>;
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
