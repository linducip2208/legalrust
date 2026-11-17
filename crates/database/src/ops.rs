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
