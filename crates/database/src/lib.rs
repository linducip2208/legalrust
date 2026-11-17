pub mod entities;
pub mod memory;
pub mod mysql;
pub mod ops;
pub mod ops_memory;
pub mod ops_mysql;
pub mod store;
pub use entities::*;
pub use store::{
    CoreStore, DocRow, DocVersionRow, MatterRow, StoreError, TaskRow, UserRow, UserStore,
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
    marketing_allowed_transition, AuditRow, ClientTagRow, CrmStore, DeedRow, InvoiceRow, JobRow,
    MarketingArticleRow, MarketingContactRow, MarketingDemoRow, MarketingFaqRow,
    MarketingFollowUpRow, MarketingLeadRow, MarketingNoteRow, MarketingPlanRow, MarketingStore,
    MarketingSubscriberRow, MeetingRow, NotifRow, OcrFieldRow, OcrJobRow, OpsStore, PaymentRow,
    RagHit, ShareRow, TemplateRow, TrustAccountRow, TrustEntryRow,
};
