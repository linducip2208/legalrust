#!/usr/bin/env python3
"""Impl part 1b: mysql store impl + auth service (Claims+sid, register/login/logout)."""
from pathlib import Path
R = Path(r"D:\project laravel\legalrust")
def w(rel, text):
    p = R/rel; p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text.strip()+"\n", encoding="utf-8", newline="\n")

w("crates/database/src/mysql.rs", """use super::entities::ClientRow;
use super::store::*;
use sqlx::{MySqlPool, Row};
#[derive(Debug, Clone)]
pub struct MysqlStores { pub pool: MySqlPool }
impl MysqlStores { pub fn new(pool: MySqlPool) -> Self { Self { pool } } }
fn db_err(e: sqlx::Error) -> StoreError {
    if let Some(dbe) = e.as_database_error() {
        if dbe.code().as_deref() == Some("23000") { return StoreError::Conflict(dbe.message().into()); }
    }
    StoreError::Db(e.to_string())
}
#[async_trait::async_trait]
impl UserStore for MysqlStores {
    async fn create_org_owner(&self, org_name: &str, email: &str, name: &str, pw_hash: &str) -> Result<(String, String), StoreError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let org = uuid::Uuid::new_v4().to_string();
        let uid = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO organizations (id, name) VALUES (?,?)").bind(&org).bind(org_name).execute(&mut *tx).await.map_err(db_err)?;
        sqlx::query("INSERT INTO users (id, org_id, email, password_hash, full_name) VALUES (?,?,?,?,?)")
            .bind(&uid).bind(&org).bind(email).bind(pw_hash).bind(name).execute(&mut *tx).await.map_err(db_err)?;
        sqlx::query("INSERT INTO roles (org_id, name) VALUES (?,?)").bind(&org).bind("org_owner").execute(&mut *tx).await.map_err(db_err)?;
        let rid: u64 = sqlx::query("SELECT LAST_INSERT_ID() AS id").fetch_one(&mut *tx).await.map_err(db_err)?.try_get("id").map_err(|e| StoreError::Db(e.to_string()))?;
        sqlx::query("INSERT IGNORE INTO permissions (name) VALUES ('view'),('create'),('update'),('delete'),('approve'),('reject'),('export'),('download'),('share'),('sign'),('verify'),('assign')").execute(&mut *tx).await.map_err(db_err)?;
        sqlx::query("INSERT INTO role_permissions (role_id, permission_id) SELECT ?, id FROM permissions").bind(rid).execute(&mut *tx).await.map_err(db_err)?;
        sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES (?,?)").bind(&uid).bind(rid).execute(&mut *tx).await.map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok((org, uid))
    }
    async fn find_by_email(&self, email: &str) -> Result<Option<UserRow>, StoreError> {
        let r = sqlx::query("SELECT id, org_id, email, password_hash, full_name, status FROM users WHERE email=? LIMIT 1")
            .bind(email).fetch_optional(&self.pool).await.map_err(db_err)?;
        Ok(r.map(|x| UserRow {
            id: x.get::<String, _>("id"), org_id: x.get::<String, _>("org_id"), email: x.get::<String, _>("email"),
            name: x.get::<String, _>("full_name"), pw_hash: x.get::<String, _>("password_hash"), status: x.get::<String, _>("status"),
        }))
    }
    async fn create_session(&self, user_id: &str, exp: chrono::NaiveDateTime) -> Result<String, StoreError> {
        let sid = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO sessions (id, user_id, expires_at) VALUES (?,?,?)").bind(&sid).bind(user_id).bind(exp).execute(&self.pool).await.map_err(db_err)?;
        Ok(sid)
    }
    async fn revoke_session(&self, sid: &str) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE sessions SET revoked=1 WHERE id=?").bind(sid).execute(&self.pool).await.map_err(db_err)?.rows_affected();
        if n == 0 { return Err(StoreError::NotFound); }
        Ok(())
    }
    async fn session_live(&self, sid: &str) -> Result<bool, StoreError> {
        let r = sqlx::query("SELECT revoked, expires_at FROM sessions WHERE id=?").bind(sid).fetch_optional(&self.pool).await.map_err(db_err)?;
        Ok(matches!(r, Some(x) if !x.get::<bool, _>("revoked") && x.get::<chrono::NaiveDateTime, _>("expires_at") > chrono::Utc::now().naive_utc()))
    }
}
#[async_trait::async_trait]
impl CoreStore for MysqlStores {
    async fn create_client(&self, org: &str, kind: &str, name: &str) -> Result<ClientRow, StoreError> {
        let r = sqlx::query("INSERT INTO clients (org_id, kind, display_name) VALUES (?,?,?)").bind(org).bind(kind).bind(name).execute(&self.pool).await.map_err(db_err)?;
        Ok(ClientRow { id: r.last_insert_id() as i64, org_id: org.into(), kind: kind.into(), display_name: name.into(), status: "active".into() })
    }
    async fn list_clients(&self, org: &str) -> Result<Vec<ClientRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, kind, display_name, status FROM clients WHERE org_id=? ORDER BY id DESC LIMIT 200").bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows.into_iter().map(|x| ClientRow { id: x.get::<u64, _>("id") as i64, org_id: x.get::<String, _>("org_id"), kind: x.get::<String, _>("kind"), display_name: x.get::<String, _>("display_name"), status: x.get::<String, _>("status") }).collect())
    }
    async fn create_matter(&self, org: &str, client: Option<i64>, number: &str, title: &str) -> Result<MatterRow, StoreError> {
        let r = sqlx::query("INSERT INTO matters (org_id, client_id, number, title) VALUES (?,?,?,?)").bind(org).bind(client).bind(number).bind(title).execute(&self.pool).await.map_err(db_err)?;
        Ok(MatterRow { id: r.last_insert_id() as i64, org_id: org.into(), client_id: client, number: number.into(), title: title.into(), status: "open".into() })
    }
    async fn list_matters(&self, org: &str) -> Result<Vec<MatterRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, client_id, number, title, status FROM matters WHERE org_id=? ORDER BY id DESC LIMIT 200").bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows.into_iter().map(|x| MatterRow { id: x.get::<u64,_>("id") as i64, org_id: x.get::<String,_>("org_id"), client_id: x.get::<Option<u64>,_>("client_id").map(|v| v as i64), number: x.get::<String,_>("number"), title: x.get::<String,_>("title"), status: x.get::<String,_>("status") }).collect())
    }
    async fn get_matter(&self, org: &str, id: i64) -> Result<MatterRow, StoreError> {
        let x = sqlx::query("SELECT id, org_id, client_id, number, title, status FROM matters WHERE id=? AND org_id=?").bind(id as u64).bind(org).fetch_optional(&self.pool).await.map_err(db_err)?.ok_or(StoreError::NotFound)?;
        Ok(MatterRow { id: x.get::<u64,_>("id") as i64, org_id: x.get::<String,_>("org_id"), client_id: x.get::<Option<u64>,_>("client_id").map(|v| v as i64), number: x.get::<String,_>("number"), title: x.get::<String,_>("title"), status: x.get::<String,_>("status") })
    }
    async fn create_task(&self, org: &str, matter: i64, title: &str) -> Result<TaskRow, StoreError> {
        self.get_matter(org, matter).await?;
        let r = sqlx::query("INSERT INTO tasks (org_id, matter_id, title) VALUES (?,?,?)").bind(org).bind(matter as u64).bind(title).execute(&self.pool).await.map_err(db_err)?;
        Ok(TaskRow { id: r.last_insert_id() as i64, org_id: org.into(), matter_id: matter, title: title.into(), status: "open".into() })
    }
    async fn list_tasks(&self, org: &str, matter: i64) -> Result<Vec<TaskRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, matter_id, title, status FROM tasks WHERE org_id=? AND matter_id=? ORDER BY id").bind(org).bind(matter as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows.into_iter().map(|x| TaskRow { id: x.get::<u64,_>("id") as i64, org_id: x.get::<String,_>("org_id"), matter_id: x.get::<u64,_>("matter_id") as i64, title: x.get::<String,_>("title"), status: x.get::<String,_>("status") }).collect())
    }
    async fn set_task_status(&self, org: &str, id: i64, status: &str) -> Result<TaskRow, StoreError> {
        const OK: &[(&str, &str)] = &[("open","in_progress"),("open","blocked"),("in_progress","done"),("in_progress","blocked"),("blocked","open")];
        let cur: String = sqlx::query("SELECT status FROM tasks WHERE id=? AND org_id=?").bind(id as u64).bind(org).fetch_optional(&self.pool).await.map_err(db_err)?.ok_or(StoreError::NotFound)?.try_get("status").map_err(|e| StoreError::Db(e.to_string()))?;
        if !OK.contains(&(cur.as_str(), status)) { return Err(StoreError::Conflict("illegal transition".into())); }
        sqlx::query("UPDATE tasks SET status=? WHERE id=?").bind(status).bind(id as u64).execute(&self.pool).await.map_err(db_err)?;
        let mut t = self.list_tasks(org, 0).await.unwrap_or_default();
        let _ = &mut t;
        Ok(TaskRow { id, org_id: org.into(), matter_id: 0, title: String::new(), status: status.into() })
    }
    async fn create_doc(&self, org: &str, matter: Option<i64>, name: &str, mime: &str, size: u64, sha: &str) -> Result<DocRow, StoreError> {
        let r = sqlx::query("INSERT INTO documents (org_id, matter_id, name, mime, size_bytes, sha256) VALUES (?,?,?,?,?,?)")
            .bind(org).bind(matter.map(|v| v as u64)).bind(name).bind(mime).bind(size).bind(sha).execute(&self.pool).await.map_err(db_err)?;
        Ok(DocRow { id: r.last_insert_id() as i64, org_id: org.into(), matter_id: matter, name: name.into(), mime: mime.into(), size, sha: sha.into() })
    }
    async fn get_doc(&self, org: &str, id: i64) -> Result<(DocRow, Vec<DocVersionRow>), StoreError> {
        let x = sqlx::query("SELECT id, org_id, matter_id, name, mime, size_bytes, sha256 FROM documents WHERE id=? AND org_id=?").bind(id as u64).bind(org).fetch_optional(&self.pool).await.map_err(db_err)?.ok_or(StoreError::NotFound)?;
        let d = DocRow { id: x.get::<u64,_>("id") as i64, org_id: x.get::<String,_>("org_id"), matter_id: x.get::<Option<u64>,_>("matter_id").map(|v| v as i64), name: x.get::<String,_>("name"), mime: x.get::<String,_>("mime"), size: x.get::<u64,_>("size_bytes"), sha: x.get::<String,_>("sha256") };
        let vs = sqlx::query("SELECT document_id, version_no, object_key, sha256 FROM document_versions WHERE document_id=? ORDER BY version_no").bind(id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok((d, vs.into_iter().map(|v| DocVersionRow { doc_id: v.get::<u64,_>("document_id") as i64, version_no: v.get::<u32,_>("version_no") as i64, object_key: v.get::<String,_>("object_key"), sha: v.get::<String,_>("sha256") }).collect()))
    }
    async fn list_docs(&self, org: &str, matter: Option<i64>) -> Result<Vec<DocRow>, StoreError> {
        let rows = if let Some(mt) = matter {
            sqlx::query("SELECT id, org_id, matter_id, name, mime, size_bytes, sha256 FROM documents WHERE org_id=? AND matter_id=? ORDER BY id DESC LIMIT 200").bind(org).bind(mt as u64).fetch_all(&self.pool).await.map_err(db_err)?
        } else {
            sqlx::query("SELECT id, org_id, matter_id, name, mime, size_bytes, sha256 FROM documents WHERE org_id=? ORDER BY id DESC LIMIT 200").bind(org).fetch_all(&self.pool).await.map_err(db_err)?
        };
        Ok(rows.into_iter().map(|x| DocRow { id: x.get::<u64,_>("id") as i64, org_id: x.get::<String,_>("org_id"), matter_id: x.get::<Option<u64>,_>("matter_id").map(|v| v as i64), name: x.get::<String,_>("name"), mime: x.get::<String,_>("mime"), size: x.get::<u64,_>("size_bytes"), sha: x.get::<String,_>("sha256") }).collect())
    }
    async fn add_version(&self, org: &str, doc: i64, key: &str, sha: &str) -> Result<DocVersionRow, StoreError> {
        self.get_doc(org, doc).await?;
        let n: u32 = sqlx::query("SELECT COUNT(*) AS c FROM document_versions WHERE document_id=?").bind(doc as u64).fetch_one(&self.pool).await.map_err(db_err)?.try_get("c").map_err(|e| StoreError::Db(e.to_string()))?;
        sqlx::query("INSERT INTO document_versions (document_id, version_no, object_key, sha256) VALUES (?,?,?,?)").bind(doc as u64).bind(n + 1).bind(key).bind(sha).execute(&self.pool).await.map_err(db_err)?;
        Ok(DocVersionRow { doc_id: doc, version_no: (n + 1) as i64, object_key: key.into(), sha: sha.into() })
    }
}
""")

# --- auth crate: service with sid claims ---
w("crates/auth/Cargo.toml", """[package]
name = "legalrust-auth"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
hex = "0.4"
argon2 = "0.5"
rand_core = { version = "0.6", features = ["getrandom"] }
jsonwebtoken = "9"
chrono = "0.4"
thiserror = "2"
anyhow = "1"
uuid = "1"
legalrust-database = { path = "../database" }
legalrust-application = { path = "../application" }
""")
w("crates/auth/src/lib.rs", """use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::SaltString;
use legalrust_application::AppError;
use legalrust_database::{StoreError, UserStore};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
#[derive(Debug, thiserror::Error)]
pub enum AuthError { #[error("hash failed")] Hash, #[error("invalid credentials")] Invalid, #[error("token error: {0}")] Token(String) }
pub fn hash_password(plain: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default().hash_password(plain.as_bytes(), &salt).map(|h| h.to_string()).map_err(|_| AuthError::Hash)
}
pub fn verify_password(hash: &str, plain: &str) -> Result<(), AuthError> {
    let parsed = PasswordHash::new(hash).map_err(|_| AuthError::Invalid)?;
    Argon2::default().verify_password(plain.as_bytes(), &parsed).map_err(|_| AuthError::Invalid)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims { pub sub: String, pub org: String, pub roles: Vec<String>, pub sid: String, pub exp: usize }
pub fn issue_token(secret: &[u8], claims: &Claims) -> Result<String, AuthError> {
    jsonwebtoken::encode(&jsonwebtoken::Header::default(), claims, &jsonwebtoken::EncodingKey::from_secret(secret)).map_err(|e| AuthError::Token(e.to_string()))
}
pub fn validate_token(secret: &[u8], token: &str) -> Result<Claims, AuthError> {
    jsonwebtoken::decode::<Claims>(token, &jsonwebtoken::DecodingKey::from_secret(secret), &jsonwebtoken::Validation::default())
        .map(|d| d.claims).map_err(|e| AuthError::Token(e.to_string()))
}
pub fn recovery_codes(n: usize) -> Vec<String> {
    use rand_core::RngCore;
    (0..n).map(|_| { let mut b = [0u8; 10]; OsRng.fill_bytes(&mut b); hex::encode(b) }).collect()
}
fn map_store(e: StoreError) -> AppError {
    match e { StoreError::NotFound => AppError::NotFound("record".into()), StoreError::Conflict(m) => AppError::Conflict(m), StoreError::Db(m) => AppError::Internal(anyhow::Error::msg(m)) }
}
#[derive(Debug, Clone, Serialize)]
pub struct Session { pub user_id: String, pub org_id: String, pub token: String, pub session_id: String }
fn expiry() -> (chrono::NaiveDateTime, usize) {
    let dt = chrono::Utc::now() + chrono::Duration::hours(24);
    (dt.naive_utc(), dt.timestamp() as usize)
}
pub fn check_secret(secret: &[u8]) -> Result<(), AppError> {
    if secret.is_empty() { return Err(AppError::BlockedExternal("JWT_SECRET not configured".into())); }
    Ok(())
}
pub async fn register(store: &dyn UserStore, secret: &[u8], org: &str, email: &str, name: &str, pw: &str) -> Result<Session, AppError> {
    check_secret(secret)?;
    if !email.contains('@') || pw.len() < 8 { return Err(AppError::Validation("email/password invalid".into())); }
    let hash = hash_password(pw).map_err(|_| AppError::Internal(anyhow::Error::msg("hash")))?;
    let (org_id, uid) = store.create_org_owner(org, email, name, &hash).await.map_err(map_store)?;
    let (exp_dt, exp) = expiry();
    let sid = store.create_session(&uid, exp_dt).await.map_err(map_store)?;
    let token = issue_token(secret, &Claims { sub: uid.clone(), org: org_id.clone(), roles: vec!["org_owner".into()], sid: sid.clone(), exp }).map_err(|e| AppError::Internal(anyhow::Error::msg(e.to_string())))?;
    Ok(Session { user_id: uid, org_id, token, session_id: sid })
}
pub async fn login(store: &dyn UserStore, secret: &[u8], email: &str, pw: &str) -> Result<Session, AppError> {
    check_secret(secret)?;
    let u = store.find_by_email(email).await.map_err(map_store)?.ok_or(AppError::Unauthorized)?;
    if u.status != "active" { return Err(AppError::Unauthorized); }
    verify_password(&u.pw_hash, pw).map_err(|_| AppError::Unauthorized)?;
    let (exp_dt, exp) = expiry();
    let sid = store.create_session(&u.id, exp_dt).await.map_err(map_store)?;
    let token = issue_token(secret, &Claims { sub: u.id.clone(), org: u.org_id.clone(), roles: vec!["org_owner".into()], sid: sid.clone(), exp }).map_err(|e| AppError::Internal(anyhow::Error::msg(e.to_string())))?;
    Ok(Session { user_id: u.id, org_id: u.org_id, token, session_id: sid })
}
#[cfg(test)]
mod tests {
    use super::*;
    use legalrust_database::memory::MemoryStores;
    #[test]
    fn password_roundtrip() { let h = hash_password("s3cret-pass").unwrap(); assert!(verify_password(&h, "s3cret-pass").is_ok()); assert!(verify_password(&h, "wrong").is_err()); }
    #[test]
    fn jwt_roundtrip() {
        let c = Claims { sub: "u1".into(), org: "o1".into(), roles: vec!["notary".into()], sid: "s1".into(), exp: 9_999_999_999 };
        let t = issue_token(b"test-secret", &c).unwrap();
        let back = validate_token(b"test-secret", &t).unwrap();
        assert_eq!(back.sub, "u1"); assert!(validate_token(b"other", &t).is_err());
    }
    #[tokio::test]
    async fn register_then_login_memory() {
        let s = MemoryStores::default();
        let r = register(&s, b"secret", "Acme", "a@x.id", "Ayu", "password1").await.unwrap();
        assert_eq!(r.org_id.chars().count(), 5 + 1);
        let l = login(&s, b"secret", "a@x.id", "password1").await.unwrap();
        assert_eq!(l.org_id, r.org_id);
        assert!(login(&s, b"secret", "a@x.id", "nope-nope").await.is_err());
        assert!(register(&s, b"", "Acme", "b@x.id", "Bud", "password1").await.is_err());
    }
}
""")
print("IMPL1B OK")
