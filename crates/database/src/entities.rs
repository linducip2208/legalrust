use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRow {
    pub id: i64,
    pub org_id: String,
    pub kind: String,
    pub display_name: String,
    pub status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustAccountEntity {
    pub id: i64,
    pub org_id: String,
    pub client_id: i64,
    pub currency: String,
    pub balance_minor: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustEntryEntity {
    pub id: i64,
    pub org_id: String,
    pub account_id: i64,
    pub matter_id: Option<i64>,
    pub kind: String,
    pub amount_minor: i64,
    pub ref_type: String,
    pub ref_id: i64,
    pub created_by: String,
}
