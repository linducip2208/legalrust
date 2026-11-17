use super::entities::ClientRow;
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("db: {0}")]
    Db(String),
}
#[derive(Debug, Clone)]
pub struct UserRow {
    pub id: String,
    pub org_id: String,
    pub email: String,
    pub name: String,
    pub pw_hash: String,
    pub status: String,
}
#[derive(Debug, Clone)]
pub struct DocRow {
    pub id: i64,
    pub org_id: String,
    pub matter_id: Option<i64>,
    pub name: String,
    pub mime: String,
    pub size: u64,
    pub sha: String,
}
#[derive(Debug, Clone)]
pub struct DocVersionRow {
    pub doc_id: i64,
    pub version_no: i64,
    pub object_key: String,
    pub sha: String,
}
#[derive(Debug, Clone)]
pub struct MatterRow {
    pub id: i64,
    pub org_id: String,
    pub client_id: Option<i64>,
    pub number: String,
    pub title: String,
    pub status: String,
}
#[derive(Debug, Clone)]
pub struct TaskRow {
    pub id: i64,
    pub org_id: String,
    pub matter_id: i64,
    pub title: String,
    pub status: String,
}
#[async_trait::async_trait]
pub trait UserStore: Send + Sync {
    async fn create_org_owner(
        &self,
        org_name: &str,
        email: &str,
        name: &str,
        pw_hash: &str,
    ) -> Result<(String, String), StoreError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<UserRow>, StoreError>;
    async fn create_session(
        &self,
        user_id: &str,
        exp: chrono::NaiveDateTime,
    ) -> Result<String, StoreError>;
    async fn revoke_session(&self, sid: &str) -> Result<(), StoreError>;
    async fn session_live(&self, sid: &str) -> Result<bool, StoreError>;
}
#[async_trait::async_trait]
pub trait CoreStore: Send + Sync {
    async fn create_client(
        &self,
        org: &str,
        kind: &str,
        name: &str,
    ) -> Result<ClientRow, StoreError>;
    async fn list_clients(&self, org: &str) -> Result<Vec<ClientRow>, StoreError>;
    async fn create_matter(
        &self,
        org: &str,
        client: Option<i64>,
        number: &str,
        title: &str,
    ) -> Result<MatterRow, StoreError>;
    async fn list_matters(&self, org: &str) -> Result<Vec<MatterRow>, StoreError>;
    async fn get_matter(&self, org: &str, id: i64) -> Result<MatterRow, StoreError>;
    async fn create_task(&self, org: &str, matter: i64, title: &str)
        -> Result<TaskRow, StoreError>;
    async fn list_tasks(&self, org: &str, matter: i64) -> Result<Vec<TaskRow>, StoreError>;
    async fn set_task_status(
        &self,
        org: &str,
        id: i64,
        status: &str,
    ) -> Result<TaskRow, StoreError>;
    async fn create_doc(
        &self,
        org: &str,
        matter: Option<i64>,
        name: &str,
        mime: &str,
        size: u64,
        sha: &str,
    ) -> Result<DocRow, StoreError>;
    async fn get_doc(&self, org: &str, id: i64)
        -> Result<(DocRow, Vec<DocVersionRow>), StoreError>;
    async fn list_docs(&self, org: &str, matter: Option<i64>) -> Result<Vec<DocRow>, StoreError>;
    async fn add_version(
        &self,
        org: &str,
        doc: i64,
        key: &str,
        sha: &str,
    ) -> Result<DocVersionRow, StoreError>;
}
