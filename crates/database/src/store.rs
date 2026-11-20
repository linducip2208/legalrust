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
pub struct SessionRow {
    pub id: String,
    pub expires_at: String,
    pub revoked: bool,
}
#[derive(Debug, Clone)]
pub struct TokenRow {
    pub id: String,
    pub name: String,
    pub expires_at: Option<String>,
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
    pub practice_area: String,
}
#[derive(Debug, Clone)]
pub struct TaskRow {
    pub id: i64,
    pub org_id: String,
    pub matter_id: i64,
    pub title: String,
    pub status: String,
    pub due: Option<String>,
    pub assignee: String,
    pub desk: String,
}
#[derive(Debug, Clone)]
pub struct SavedSearchRow {
    pub id: i64,
    pub org_id: String,
    pub user_id: String,
    pub name: String,
    pub query: String,
}

/// Shared guard for `CoreStore::task_set_due`: accepts DB-format datetimes
/// (`YYYY-MM-DD HH:MM:SS`) only, so the memory and MySQL backends stay
/// consistent. `None` clears the due date.
pub fn check_due_format(due: Option<&str>) -> Result<Option<String>, StoreError> {
    match due {
        None => Ok(None),
        Some(s) => chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
            .map(|_| Some(s.to_string()))
            .map_err(|_| StoreError::Conflict("due must be YYYY-MM-DD HH:MM:SS".into())),
    }
}

/// Shared guard for `CoreStore::saved_search_create`: trims and enforces the
/// `saved_searches` column widths so both backends reject the same inputs.
pub fn check_saved_search(name: &str, query: &str) -> Result<(String, String), StoreError> {
    let name = name.trim();
    let query = query.trim();
    if name.is_empty() {
        return Err(StoreError::Conflict("name required".into()));
    }
    if name.chars().count() > 128 {
        return Err(StoreError::Conflict(
            "name must be at most 128 chars".into(),
        ));
    }
    if query.is_empty() {
        return Err(StoreError::Conflict("query required".into()));
    }
    if query.chars().count() > 512 {
        return Err(StoreError::Conflict(
            "query must be at most 512 chars".into(),
        ));
    }
    Ok((name.to_string(), query.to_string()))
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
    /// Create a member inside an EXISTING org (invite flow).
    async fn create_org_member(
        &self,
        org: &str,
        email: &str,
        name: &str,
        pw_hash: &str,
    ) -> Result<String, StoreError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<UserRow>, StoreError>;
    async fn create_session(
        &self,
        user_id: &str,
        exp: chrono::NaiveDateTime,
    ) -> Result<String, StoreError>;
    async fn revoke_session(&self, sid: &str) -> Result<(), StoreError>;

    /// Caller's own sessions (id, expires_at display, revoked flag).
    async fn list_sessions(&self, user: &str) -> Result<Vec<SessionRow>, StoreError>;
    async fn session_live(&self, sid: &str) -> Result<bool, StoreError>;
    async fn get_user(&self, id: &str) -> Result<Option<UserRow>, StoreError>;

    /// Org-scoped user directory (never leaks other tenants).
    async fn list_users(&self, org: &str) -> Result<Vec<UserRow>, StoreError>;
    async fn set_password(&self, id: &str, hash: &str) -> Result<(), StoreError>;
    async fn user_roles(&self, id: &str) -> Result<Vec<String>, StoreError>;
    async fn assign_role(&self, org: &str, user_id: &str, role: &str) -> Result<(), StoreError>;
    async fn create_token(
        &self,
        user_id: &str,
        name: &str,
        hash: &str,
        expires_at: Option<chrono::NaiveDateTime>,
    ) -> Result<String, StoreError>;
    async fn list_tokens(&self, user_id: &str) -> Result<Vec<TokenRow>, StoreError>;
    async fn revoke_token(&self, user_id: &str, id: &str) -> Result<(), StoreError>;
    async fn token_owner(&self, hash: &str) -> Result<Option<(String, String)>, StoreError>;
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
    /// Partial client update (only `Some` fields change; validates values).
    /// Practice-area classification (allowlist).
    async fn set_matter_practice(
        &self,
        org: &str,
        id: i64,
        area: &str,
    ) -> Result<MatterRow, StoreError>;
    /// Status lifecycle: open->on_hold->open|closed, open->closed. Closed is terminal.
    async fn set_matter_status(
        &self,
        org: &str,
        id: i64,
        to: &str,
    ) -> Result<MatterRow, StoreError>;
    async fn update_client(
        &self,
        org: &str,
        id: i64,
        name: Option<&str>,
        kind: Option<&str>,
        status: Option<&str>,
    ) -> Result<ClientRow, StoreError>;
    /// Move all matters from one client to another (same org). Returns count.
    async fn repoint_matters(
        &self,
        org: &str,
        from_client: i64,
        to_client: i64,
    ) -> Result<u64, StoreError>;
    /// Delete a client only when no matters reference it.
    async fn delete_client_if_empty(&self, org: &str, id: i64) -> Result<(), StoreError>;
    async fn create_task(&self, org: &str, matter: i64, title: &str)
        -> Result<TaskRow, StoreError>;
    async fn list_tasks(&self, org: &str, matter: i64) -> Result<Vec<TaskRow>, StoreError>;
    async fn task_get(&self, org: &str, id: i64) -> Result<TaskRow, StoreError>;
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
    /// Secure deletion: removes versions + metadata, returning the storage
    /// object keys so the caller can wipe bytes. Org-scoped.
    async fn delete_doc(&self, org: &str, id: i64) -> Result<Vec<String>, StoreError>;
    async fn add_version(
        &self,
        org: &str,
        doc: i64,
        key: &str,
        sha: &str,
    ) -> Result<DocVersionRow, StoreError>;
    async fn task_set_due(
        &self,
        org: &str,
        id: i64,
        due: Option<&str>,
    ) -> Result<TaskRow, StoreError>;
    async fn task_due_list(&self, org: &str, limit: i64) -> Result<Vec<TaskRow>, StoreError>;
    async fn task_assign(
        &self,
        org: &str,
        id: i64,
        assignee: &str,
        desk: &str,
    ) -> Result<TaskRow, StoreError>;
    async fn task_board(&self, org: &str, limit: i64) -> Result<Vec<TaskRow>, StoreError>;
    async fn saved_search_create(
        &self,
        org: &str,
        user: &str,
        name: &str,
        query: &str,
    ) -> Result<SavedSearchRow, StoreError>;
    async fn saved_search_list(
        &self,
        org: &str,
        user: &str,
    ) -> Result<Vec<SavedSearchRow>, StoreError>;
    async fn saved_search_delete(&self, org: &str, user: &str, id: i64) -> Result<(), StoreError>;
}
