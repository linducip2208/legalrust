use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRow {
    pub id: i64,
    pub org_id: String,
    pub kind: String,
    pub display_name: String,
    pub status: String,
}
