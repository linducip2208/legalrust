pub mod entities;
pub mod memory;
pub mod mysql;
pub mod ops;
pub mod ops_memory;
pub mod ops_mysql;
pub mod store;
pub use entities::*;
pub use store::{
    CoreStore, DocRow, DocVersionRow, MatterRow, SavedSearchRow, StoreError, TaskRow, UserRow,
    UserStore,
};
pub async fn connect(url: &str) -> Result<sqlx::MySqlPool, DbError> {
    sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(url)
        .await
        .map_err(|e| DbError::Connect(e.to_string()))
}
pub async fn migrate(pool: &sqlx::MySqlPool) -> Result<(), DbError> {
    sqlx::migrate!("../../migrations")
        .run(pool)
        .await
        .map_err(|e| DbError::Migrate(e.to_string()))
}
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("connect failed: {0}")]
    Connect(String),
    #[error("migrate failed: {0}")]
    Migrate(String),
}
pub use ops::{
    appr_check, marketing_allowed_transition, AdvanceRow, AnnouncementRow, AppointmentRow,
    ApprovalReqRow, ApprovalStepRow, ArOpenRow, ArchiveBoxRow, AuditFindingRow, AuditPlanRow,
    AuditRow, BackupCheckRow, BankFacilityRow, BreachRow, CapaRow, CashEntryRow, CertHandoverRow,
    ChecklistRow, ClauseRow, ClientTagRow, ConsentRow, ControlRow, ControlTestRow, CrmStore,
    DeedChargeRow, DeedRow, DelegationRow, DocCheckRow, DownPaymentRow, DpiaRow, DsarRow,
    EthicsRow, EvidenceRow, FeedbackRow, GiftRow, IncidentRow, InventoryRow, InviteRow, InvoiceRow,
    JobRow, LandMergeRow, LandSplitRow, LeaveRow, MarketingArticleRow, MarketingContactRow,
    MarketingDemoRow, MarketingFaqRow, MarketingFollowUpRow, MarketingLeadRow, MarketingNoteRow,
    MarketingPlanRow, MarketingStore, MarketingSubscriberRow, MeetingRow, MpdTransferRow, NotifRow,
    OcrFieldRow, OcrJobRow, OfficeHoursRow, OfficeIncidentRow, OpsStore, PaymentRow, PbbRow,
    PbookRow, PdpObligationRow, PhandoverRow, PolicyAckRow, PolicyRow, PpatPartyRow, PpatRow,
    PriceRow, QuotationItemRow, QuotationRow, RagHit, RecheckRow, RegChangeRow, RiskRow, RopaRow,
    RosterRow, SelfcheckRow, ShareRow, SopRow, StorageBoxRow, TemplateRow, TrainingEvidenceRow,
    TrainingRow, TrustAccountRow, TrustEntryRow, UsageRow, VendorRow, VisitorRow, WaiverRow,
    WebhookDeliveryRow, WebhookEndpointRow, WhistleRow, WitnessRow, YearSealRow, ZoneRow,
};
