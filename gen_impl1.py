#!/usr/bin/env python3
"""Impl part 1: Role::from_str, database stores (trait+memory+mysql), auth service."""
from pathlib import Path
R = Path(r"D:\project laravel\legalrust")
def w(rel, text):
    p = R/rel; p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text.strip()+"\n", encoding="utf-8", newline="\n")

# --- domain: add from_str ---
p = R/"crates/domain/src/lib.rs"; t = p.read_text(encoding="utf-8")
anchor = "    pub fn as_str(self) -> &'static str"
add = """    pub fn from_str(s: &str) -> Option<Role> {
        Some(match s {
            "super_admin" => Role::SuperAdmin, "org_owner" => Role::OrgOwner, "office_admin" => Role::OfficeAdmin,
            "notary" => Role::Notary, "ppat" => Role::Ppat, "lawyer" => Role::Lawyer,
            "legal_staff" => Role::LegalStaff, "document_officer" => Role::DocumentOfficer,
            "finance_officer" => Role::FinanceOfficer, "auditor" => Role::Auditor,
            "client" => Role::Client, "reviewer" => Role::Reviewer, _ => return None,
        })
    }
"""
assert anchor in t
t = t.replace(anchor, add + anchor)
w("crates/domain/src/lib.rs", t)

# --- database crate ---
w("crates/database/Cargo.toml", """[package]
name = "legalrust-database"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.9", features = ["runtime-tokio", "mysql", "migrate", "chrono", "uuid"] }
serde = { version = "1", features = ["derive"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
thiserror = "2"
async-trait = "0.1"
""")
w("crates/database/src/lib.rs", """pub mod entities;
pub mod memory;
pub mod mysql;
pub mod store;
pub use entities::*;
pub use store::{DocRow, DocVersionRow, DocumentStore, MatterRow, StoreError, TaskRow, UserRow, UserStore};
pub async fn connect(url: &str) -> Result<sqlx::MySqlPool, DbError> {
    sqlx::mysql::MySqlPoolOptions::new().max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(url).await.map_err(|e| DbError::Connect(e.to_string()))
}
pub async fn migrate(pool: &sqlx::MySqlPool) -> Result<(), DbError> {
    sqlx::migrate!("../../migrations").run(pool).await.map_err(|e| DbError::Migrate(e.to_string()))
}
#[derive(Debug, thiserror::Error)]
pub enum DbError { #[error("connect failed: {0}")] Connect(String), #[error("migrate failed: {0}")] Migrate(String) }
""")
w("crates/database/src/entities.rs", """use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRow { pub id: i64, pub org_id: String, pub kind: String, pub display_name: String, pub status: String }
""")
w("crates/database/src/store.rs", """use super::entities::ClientRow;
#[derive(Debug, thiserror::Error)]
pub enum StoreError { #[error("not found")] NotFound, #[error("conflict: {0}")] Conflict(String), #[error("db: {0}")] Db(String) }
#[derive(Debug, Clone)]
pub struct UserRow { pub id: String, pub org_id: String, pub email: String, pub name: String, pub pw_hash: String, pub status: String }
#[derive(Debug, Clone)]
pub struct DocRow { pub id: i64, pub org_id: String, pub matter_id: Option<i64>, pub name: String, pub mime: String, pub size: u64, pub sha: String }
#[derive(Debug, Clone)]
pub struct DocVersionRow { pub doc_id: i64, pub version_no: i64, pub object_key: String, pub sha: String }
#[derive(Debug, Clone)]
pub struct MatterRow { pub id: i64, pub org_id: String, pub client_id: Option<i64>, pub number: String, pub title: String, pub status: String }
#[derive(Debug, Clone)]
pub struct TaskRow { pub id: i64, pub org_id: String, pub matter_id: i64, pub title: String, pub status: String }
#[async_trait::async_trait]
pub trait UserStore: Send + Sync {
    async fn create_org_owner(&self, org_name: &str, email: &str, name: &str, pw_hash: &str) -> Result<(String, String), StoreError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<UserRow>, StoreError>;
    async fn create_session(&self, user_id: &str, exp: chrono::NaiveDateTime) -> Result<String, StoreError>;
    async fn revoke_session(&self, sid: &str) -> Result<(), StoreError>;
    async fn session_live(&self, sid: &str) -> Result<bool, StoreError>;
}
#[async_trait::async_trait]
pub trait CoreStore: Send + Sync {
    async fn create_client(&self, org: &str, kind: &str, name: &str) -> Result<ClientRow, StoreError>;
    async fn list_clients(&self, org: &str) -> Result<Vec<ClientRow>, StoreError>;
    async fn create_matter(&self, org: &str, client: Option<i64>, number: &str, title: &str) -> Result<MatterRow, StoreError>;
    async fn list_matters(&self, org: &str) -> Result<Vec<MatterRow>, StoreError>;
    async fn get_matter(&self, org: &str, id: i64) -> Result<MatterRow, StoreError>;
    async fn create_task(&self, org: &str, matter: i64, title: &str) -> Result<TaskRow, StoreError>;
    async fn list_tasks(&self, org: &str, matter: i64) -> Result<Vec<TaskRow>, StoreError>;
    async fn set_task_status(&self, org: &str, id: i64, status: &str) -> Result<TaskRow, StoreError>;
    async fn create_doc(&self, org: &str, matter: Option<i64>, name: &str, mime: &str, size: u64, sha: &str) -> Result<DocRow, StoreError>;
    async fn get_doc(&self, org: &str, id: i64) -> Result<(DocRow, Vec<DocVersionRow>), StoreError>;
    async fn list_docs(&self, org: &str, matter: Option<i64>) -> Result<Vec<DocRow>, StoreError>;
    async fn add_version(&self, org: &str, doc: i64, key: &str, sha: &str) -> Result<DocVersionRow, StoreError>;
}
""")
w("crates/database/src/memory.rs", """use super::entities::ClientRow;
use super::store::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
#[derive(Debug, Default)]
struct Inner {
    users: HashMap<String, UserRow>, sessions: HashMap<String, (String, chrono::NaiveDateTime, bool)>,
    clients: HashMap<i64, ClientRow>, matters: HashMap<i64, MatterRow>, tasks: HashMap<i64, TaskRow>,
    docs: HashMap<i64, DocRow>, versions: HashMap<i64, Vec<DocVersionRow>>,
    seq: i64, org_seq: u64,
}
#[derive(Debug, Clone, Default)]
pub struct MemoryStores { inner: Arc<Mutex<Inner>> }
impl MemoryStores {
    fn next(&self, m: &mut Inner) -> i64 { m.seq += 1; m.seq }
}
#[async_trait::async_trait]
impl UserStore for MemoryStores {
    async fn create_org_owner(&self, org_name: &str, email: &str, name: &str, pw_hash: &str) -> Result<(String, String), StoreError> {
        let mut m = self.inner.lock().unwrap();
        if m.users.values().any(|u| u.email == email) { return Err(StoreError::Conflict("email taken".into())); }
        m.org_seq += 1;
        let org = format!("org-{}", m.org_seq);
        let uid = uuid::Uuid::new_v4().to_string();
        let _ = org_name;
        m.users.insert(uid.clone(), UserRow { id: uid.clone(), org_id: org.clone(), email: email.into(), name: name.into(), pw_hash: pw_hash.into(), status: "active".into() });
        Ok((org, uid))
    }
    async fn find_by_email(&self, email: &str) -> Result<Option<UserRow>, StoreError> {
        Ok(self.inner.lock().unwrap().users.values().find(|u| u.email == email).cloned())
    }
    async fn create_session(&self, user_id: &str, exp: chrono::NaiveDateTime) -> Result<String, StoreError> {
        let sid = uuid::Uuid::new_v4().to_string();
        self.inner.lock().unwrap().sessions.insert(sid.clone(), (user_id.into(), exp, false));
        Ok(sid)
    }
    async fn revoke_session(&self, sid: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.sessions.get_mut(sid) { Some(s) => { s.2 = true; Ok(()) } None => Err(StoreError::NotFound) }
    }
    async fn session_live(&self, sid: &str) -> Result<bool, StoreError> {
        let now = chrono::Utc::now().naive_utc();
        Ok(matches!(self.inner.lock().unwrap().sessions.get(sid), Some((_, exp, false)) if *exp > now))
    }
}
#[async_trait::async_trait]
impl CoreStore for MemoryStores {
    async fn create_client(&self, org: &str, kind: &str, name: &str) -> Result<ClientRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let id = m.seq + 1; m.seq = id;
        let c = ClientRow { id, org_id: org.into(), kind: kind.into(), display_name: name.into(), status: "active".into() };
        m.clients.insert(id, c.clone()); Ok(c)
    }
    async fn list_clients(&self, org: &str) -> Result<Vec<ClientRow>, StoreError> {
        Ok(self.inner.lock().unwrap().clients.values().filter(|c| c.org_id == org).cloned().collect())
    }
    async fn create_matter(&self, org: &str, client: Option<i64>, number: &str, title: &str) -> Result<MatterRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let id = m.seq + 1; m.seq = id;
        let r = MatterRow { id, org_id: org.into(), client_id: client, number: number.into(), title: title.into(), status: "open".into() };
        m.matters.insert(id, r.clone()); Ok(r)
    }
    async fn list_matters(&self, org: &str) -> Result<Vec<MatterRow>, StoreError> {
        Ok(self.inner.lock().unwrap().matters.values().filter(|x| x.org_id == org).cloned().collect())
    }
    async fn get_matter(&self, org: &str, id: i64) -> Result<MatterRow, StoreError> {
        self.inner.lock().unwrap().matters.get(&id).filter(|x| x.org_id == org).cloned().ok_or(StoreError::NotFound)
    }
    async fn create_task(&self, org: &str, matter: i64, title: &str) -> Result<TaskRow, StoreError> {
        self.get_matter(org, matter).await?;
        let mut m = self.inner.lock().unwrap();
        let id = m.seq + 1; m.seq = id;
        let r = TaskRow { id, org_id: org.into(), matter_id: matter, title: title.into(), status: "open".into() };
        m.tasks.insert(id, r.clone()); Ok(r)
    }
    async fn list_tasks(&self, org: &str, matter: i64) -> Result<Vec<TaskRow>, StoreError> {
        Ok(self.inner.lock().unwrap().tasks.values().filter(|x| x.org_id == org && x.matter_id == matter).cloned().collect())
    }
    async fn set_task_status(&self, org: &str, id: i64, status: &str) -> Result<TaskRow, StoreError> {
        const OK: &[(&str, &str)] = &[("open", "in_progress"), ("open", "blocked"), ("in_progress", "done"), ("in_progress", "blocked"), ("blocked", "open")];
        let mut m = self.inner.lock().unwrap();
        let t = m.tasks.get_mut(&id).filter(|x| x.org_id == org).ok_or(StoreError::NotFound)?;
        if !OK.contains(&(t.status.as_str(), status)) { return Err(StoreError::Conflict("illegal transition".into())); }
        t.status = status.into(); Ok(t.clone())
    }
    async fn create_doc(&self, org: &str, matter: Option<i64>, name: &str, mime: &str, size: u64, sha: &str) -> Result<DocRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let id = m.seq + 1; m.seq = id;
        let r = DocRow { id, org_id: org.into(), matter_id: matter, name: name.into(), mime: mime.into(), size, sha: sha.into() };
        m.docs.insert(id, r.clone()); Ok(r)
    }
    async fn get_doc(&self, org: &str, id: i64) -> Result<(DocRow, Vec<DocVersionRow>), StoreError> {
        let m = self.inner.lock().unwrap();
        let d = m.docs.get(&id).filter(|x| x.org_id == org).cloned().ok_or(StoreError::NotFound)?;
        Ok((d, m.versions.get(&id).cloned().unwrap_or_default()))
    }
    async fn list_docs(&self, org: &str, matter: Option<i64>) -> Result<Vec<DocRow>, StoreError> {
        Ok(self.inner.lock().unwrap().docs.values().filter(|x| x.org_id == org && matter.map_or(true, |mt| x.matter_id == Some(mt))).cloned().collect())
    }
    async fn add_version(&self, org: &str, doc: i64, key: &str, sha: &str) -> Result<DocVersionRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        if m.docs.get(&doc).filter(|x| x.org_id == org).is_none() { return Err(StoreError::NotFound); }
        let v = DocVersionRow { doc_id: doc, version_no: m.versions.get(&doc).map_or(1, |v| v.len() as i64 + 1), object_key: key.into(), sha: sha.into() };
        m.versions.entry(doc).or_default().push(v.clone()); Ok(v)
    }
}
""")
print("IMPL1A OK")
