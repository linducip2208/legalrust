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
    appr_check, marketing_allowed_transition, AdvanceRow, AjbDraftRow, AnnouncementRow,
    AppointmentRow, ApprPrefRow, ApprovalReqRow, ApprovalStepRow, ArOpenRow, ArchiveBoxRow,
    AuditFindingRow, AuditPlanRow, AuditRow, BackupCheckRow, BankFacilityRow, BoundaryRow,
    BpnTrackRow, BreachRow, CampaignPartRow, CapaRow, CashEntryRow, CertHandoverRow, CertLinkRow,
    ChecklistRow, ClauseRow, ClientCheckItemRow, ClientCheckRow, ClientConsentRow,
    ClientCopyReqRow, ClientCorrectionRow, ClientCreditRow, ClientDocReqRow, ClientEmployeeRow,
    ClientEngLetterRow, ClientEventRow, ClientFeeRow, ClientFeedbackRow, ClientLinkRow, ClientLtv,
    ClientMessageRow, ClientNotifRouteRow, ClientOppRow, ClientPortalInviteRow, ClientReferralRow,
    ClientReminderRow, ClientRiskRow, ClientSegRow, ClientSurveyRow, ClientTagRow, ClientVipRow,
    ClientWinbackRow, ConflictCheckRow, ConsentRow, ControlRow, ControlTestRow, CrmStore,
    DeedChargeRow, DeedConsentRow, DeedRow, DelegationRow, DevBatchRow, DevUnitRow, DocCheckRow,
    DocLifecycleRow, DocLockRow, DocMetaRow, DownPaymentRow, DpiaRow, DsarRow, EthicsRow,
    EventAttendRow, EvidenceRow, FeedbackRow, FreezeRow, GiftRow, GirikRow, IncidentRow,
    InventoryRow, InviteRow, InvoiceRow, JobRow, KprPackRow, LandDisputeRow, LandHistoryRow,
    LandMergeRow, LandSplitRow, LeaseTermRow, LeaveRow, MarketingArticleRow, MarketingCampaignRow,
    MarketingContactRow, MarketingDemoRow, MarketingFaqRow, MarketingFollowUpRow, MarketingLeadRow,
    MarketingNoteRow, MarketingPlanRow, MarketingStore, MarketingSubscriberRow, MatterLinkRow,
    MeetingRow, MetaFieldRow, MpdTransferRow, NotaryHandoffRow, NotifRow, OcrFieldRow, OcrJobRow,
    OfficeHoursRow, OfficeIncidentRow, OpsStore, OrgQuotaRow, PaymentRow, PbbRow, PbookRow,
    PdpObligationRow, PhandoverRow, PolicyAckRow, PolicyRow, PolicySnapRow, PortalEventRow,
    PortalRatingRow, PpatDashboard, PpatMonthlyRow, PpatPartyRow, PpatRow, PregRow, PriceRow,
    PurgeRequestRow, QuorumOverrideRow, QuorumRoundRow, QuorumVoteRow, QuotationItemRow,
    QuotationRow, RagHit, ReceiptRow, RecheckRow, RegChangeRow, RetentionRuleRow, RiskRow, RopaRow,
    RosterRow, RouteRuleRow, SelfcheckRow, ShareRow, SopRow, StorageBoxRow, TemplateRow,
    TrainingEvidenceRow, TrainingRow, TrustAccountRow, TrustEntryRow, UnitHandoverRow, UsageRow,
    VendorRow, VisitorRow, WaiverRow, WebhookDeliveryRow, WebhookEndpointRow, WhistleRow,
    WitnessRow, YearSealRow, ZoneRow,
};
