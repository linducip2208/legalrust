#!/usr/bin/env python3
"""Part 2: crates, apps, migrations, deploy, new docs structure."""
from pathlib import Path
import json
R = Path(r"D:\project laravel\legalrust")
def w(rel, text):
    p = R/rel; p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text.strip()+"\n", encoding="utf-8", newline="\n")
def crate(name, deps, lib):
    w(f"crates/{name}/Cargo.toml", f"""[package]
name = "legalrust-{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
""" + "\n".join(f"{k} = {v}" for k, v in deps.items()) + "\n")
    w(f"crates/{name}/src/lib.rs", lib)

SERDE = {"serde": '{ version = "1", features = ["derive"] }', "serde_json": '"1"', "uuid": '{ version = "1", features = ["v4", "serde"] }', "chrono": '{ version = "0.4", features = ["serde"] }'}

crate("domain", SERDE, r'''
use serde::{Deserialize, Serialize};
pub type TenantId = uuid::Uuid;
pub type UserId = uuid::Uuid;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role { SuperAdmin, OrgOwner, OfficeAdmin, Notary, Ppat, Lawyer, LegalStaff, DocumentOfficer, FinanceOfficer, Auditor, Client, Reviewer }
impl Role {
    pub fn all() -> [Role; 12] { use Role::*; [SuperAdmin, OrgOwner, OfficeAdmin, Notary, Ppat, Lawyer, LegalStaff, DocumentOfficer, FinanceOfficer, Auditor, Client, Reviewer] }
    pub fn as_str(self) -> &'static str { match self { Role::SuperAdmin => "super_admin", Role::OrgOwner => "org_owner", Role::OfficeAdmin => "office_admin", Role::Notary => "notary", Role::Ppat => "ppat", Role::Lawyer => "lawyer", Role::LegalStaff => "legal_staff", Role::DocumentOfficer => "document_officer", Role::FinanceOfficer => "finance_officer", Role::Auditor => "auditor", Role::Client => "client", Role::Reviewer => "reviewer" } }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission { View, Create, Update, Delete, Approve, Reject, Export, Download, Share, Sign, Verify, Assign }
impl Permission {
    pub fn all() -> [Permission; 12] { use Permission::*; [View, Create, Update, Delete, Approve, Reject, Export, Download, Share, Sign, Verify, Assign] }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn twelve_roles_and_permissions() { assert_eq!(Role::all().len(), 12); assert_eq!(Permission::all().len(), 12); }
}
''')

crate("application", {**SERDE, "thiserror": '"2"', "axum": '"0.8"',
    "validator": '{ version = "0.20", features = ["derive"] }'}, r'''
use axum::{Json, http::StatusCode, response::{IntoResponse, Response}};
use serde::{Deserialize, Serialize};
use serde_json::json;
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found: {0}")] NotFound(String),
    #[error("unauthorized")] Unauthorized,
    #[error("forbidden")] Forbidden,
    #[error("validation failed: {0}")] Validation(String),
    #[error("conflict: {0}")] Conflict(String),
    #[error("external dependency blocked: {0}")] BlockedExternal(String),
    #[error("internal error")] Internal(#[from] anyhow::Error),
}
impl AppError {
    pub fn code(&self) -> &'static str { match self { AppError::NotFound(_) => "NOT_FOUND", AppError::Unauthorized => "UNAUTHORIZED", AppError::Forbidden => "FORBIDDEN", AppError::Validation(_) => "VALIDATION", AppError::Conflict(_) => "CONFLICT", AppError::BlockedExternal(_) => "EXTERNAL_BLOCKED", AppError::Internal(_) => "INTERNAL" } }
    pub fn status(&self) -> StatusCode { match self { AppError::NotFound(_) => StatusCode::NOT_FOUND, AppError::Unauthorized => StatusCode::UNAUTHORIZED, AppError::Forbidden => StatusCode::FORBIDDEN, AppError::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY, AppError::Conflict(_) => StatusCode::CONFLICT, AppError::BlockedExternal(_) => StatusCode::NOT_IMPLEMENTED, AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR } }
}
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = Json(json!({"error": {"code": self.code(), "message": self.to_string()}}));
        (self.status(), body).into_response()
    }
}
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> { pub data: T, pub meta: Meta }
#[derive(Debug, Serialize)]
pub struct Meta { pub version: &'static str, pub request_id: String }
impl<T: Serialize> ApiResponse<T> {
    pub fn new(data: T, request_id: String) -> Self { Self { data, meta: Meta { version: "v1", request_id } } }
}
#[derive(Debug, Deserialize, validator::Validate)]
pub struct Pagination { #[validate(range(min = 1))] pub page: Option<u32>, #[validate(range(min = 1, max = 200))] pub per_page: Option<u32> }
impl Pagination {
    pub fn normalized(self) -> (u32, u32) { (self.page.unwrap_or(1).max(1), self.per_page.unwrap_or(20).min(200)) }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn error_codes_map_to_status() { assert_eq!(AppError::Unauthorized.status(), StatusCode::UNAUTHORIZED); assert_eq!(AppError::BlockedExternal("x".into()).status(), StatusCode::NOT_IMPLEMENTED); }
    #[test]
    fn pagination_clamps() { let (p, n) = Pagination { page: Some(0), per_page: Some(999) }.normalized(); assert_eq!((p, n), (1, 200)); }
}
''')

crate("infrastructure", {**SERDE, "tokio": '{ version = "1", features = ["full"] }',
    "tracing": '"0.1"', "tracing-subscriber": '{ version = "0.3", features = ["env-filter", "fmt"] }',
    "sqlx": '{ version = "0.9", features = ["runtime-tokio", "mysql"] }'}, r'''
use std::env;
#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16, pub database_url: String, pub redis_url: String,
    pub s3_endpoint: String, pub s3_bucket: String, pub jwt_secret_set: bool,
}
impl Config {
    pub fn from_env() -> Self {
        Self {
            port: env::var("PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(8080),
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| "mysql://root:legalrust@127.0.0.1:3306/legalrust".into()),
            redis_url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into()),
            s3_endpoint: env::var("S3_ENDPOINT").unwrap_or_else(|_| "http://127.0.0.1:9000".into()),
            s3_bucket: env::var("S3_BUCKET").unwrap_or_else(|_| "legalrust".into()),
            jwt_secret_set: env::var("JWT_SECRET").map(|s| !s.is_empty()).unwrap_or(false),
        }
    }
}
pub fn init_tracing() {
    let _ = tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).try_init();
}
#[derive(Debug, Clone)]
pub struct AppState { pub version: &'static str, pub pool: Option<sqlx::MySqlPool> }
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_apply() { env::remove_var("PORT"); assert_eq!(Config::from_env().port, 8080); }
}
''')

crate("database", {"tokio": '{ version = "1", features = ["full"] }',
    "sqlx": '{ version = "0.9", features = ["runtime-tokio", "mysql", "migrate"] }',
    "thiserror": '"2"'}, r'''
#[derive(Debug, thiserror::Error)]
pub enum DbError { #[error("connect failed: {0}")] Connect(String), #[error("migrate failed: {0}")] Migrate(String) }
pub async fn connect(url: &str) -> Result<sqlx::MySqlPool, DbError> {
    sqlx::mysql::MySqlPoolOptions::new().max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(url).await.map_err(|e| DbError::Connect(e.to_string()))
}
pub async fn migrate(pool: &sqlx::MySqlPool) -> Result<(), DbError> {
    sqlx::migrate!("../../migrations").run(pool).await.map_err(|e| DbError::Migrate(e.to_string()))
}
''')

crate("auth", {**SERDE, "hex": '"0.4"', "argon2": '"0.5"', "rand_core": '{ version = "0.6", features = ["getrandom"] }',
    "jsonwebtoken": '"9"', "chrono": '"0.4"', "thiserror": '"2"', "uuid": '"1"'}, r'''
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::SaltString;
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
pub struct Claims { pub sub: String, pub org: String, pub roles: Vec<String>, pub exp: usize }
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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn password_roundtrip() { let h = hash_password("s3cret-pass").unwrap(); assert!(verify_password(&h, "s3cret-pass").is_ok()); assert!(verify_password(&h, "wrong").is_err()); }
    #[test]
    fn jwt_roundtrip() {
        let c = Claims { sub: "u1".into(), org: "o1".into(), roles: vec!["notary".into()], exp: 9_999_999_999 };
        let t = issue_token(b"test-secret", &c).unwrap();
        let back = validate_token(b"test-secret", &t).unwrap();
        assert_eq!(back.sub, "u1"); assert!(validate_token(b"other", &t).is_err());
    }
}
''')

crate("authorization", {**SERDE, "legalrust-domain": '{ path = "../domain" }'}, r'''
use legalrust_domain::{Permission, Role};
pub fn permissions(role: Role) -> &'static [Permission] {
    use Permission::*;
    match role {
        Role::SuperAdmin | Role::OrgOwner => &[View, Create, Update, Delete, Approve, Reject, Export, Download, Share, Sign, Verify, Assign],
        Role::OfficeAdmin => &[View, Create, Update, Approve, Export, Download, Share, Assign],
        Role::Notary | Role::Ppat => &[View, Create, Update, Approve, Sign, Verify, Export, Download, Share],
        Role::Lawyer => &[View, Create, Update, Export, Download, Share],
        Role::LegalStaff | Role::DocumentOfficer => &[View, Create, Update, Download, Share, Verify],
        Role::FinanceOfficer => &[View, Create, Update, Approve, Export, Download],
        Role::Auditor | Role::Reviewer => &[View, Export, Download],
        Role::Client => &[View, Create, Download, Share],
    }
}
pub fn can(role: Role, perm: Permission) -> bool { permissions(role).contains(&perm) }
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn least_privilege_holds() {
        assert!(!can(Role::Client, Permission::Approve));
        assert!(!can(Role::Auditor, Permission::Delete));
        assert!(can(Role::Notary, Permission::Sign));
        assert!(!can(Role::Notary, Permission::Delete));
        assert!(can(Role::SuperAdmin, Permission::Assign));
    }
}
''')

crate("audit", {**SERDE, "tokio": '{ version = "1", features = ["full"] }',
    "sqlx": '{ version = "0.9", features = ["runtime-tokio", "mysql"] }'}, r'''
use serde::Serialize;
#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent { pub tenant_id: String, pub actor_id: Option<String>, pub action: String, pub entity: String, pub entity_id: i64, pub ip: Option<String> }
pub async fn record(pool: &sqlx::MySqlPool, e: &AuditEvent) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO audit_events (tenant_id, actor_id, action, entity, entity_id, ip) VALUES (?,?,?,?,?,?)")
        .bind(&e.tenant_id).bind(&e.actor_id).bind(&e.action).bind(&e.entity).bind(e.entity_id).bind(&e.ip)
        .execute(pool).await.map(|_| ())
}
''')

crate("document", {**SERDE, "sha2": '"0.10"', "thiserror": '"2"'}, r'''
const ALLOWED_MIME: &[&str] = &["application/pdf", "application/vnd.openxmlformats-officedocument.wordprocessingml.document", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "text/csv", "text/plain", "image/jpeg", "image/png", "image/tiff"];
#[derive(Debug, thiserror::Error)]
pub enum DocError { #[error("mime not allowed: {0}")] Mime(String), #[error("file too large: {0} bytes")] TooLarge(u64) }
pub fn validate_upload(mime: &str, size: u64, max: u64) -> Result<(), DocError> {
    if !ALLOWED_MIME.contains(&mime) { return Err(DocError::Mime(mime.into())); }
    if size > max { return Err(DocError::TooLarge(size)); }
    Ok(())
}
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))
}
pub fn hex(bytes: &[u8]) -> String { bytes.iter().map(|b| format!("{:02x}", b)).collect() }
pub fn hex_encode(b: &[u8]) -> String { hex(b) }
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_exe_and_oversize() { assert!(validate_upload("application/x-msdownload", 10, 100).is_err()); assert!(validate_upload("application/pdf", 101, 100).is_err()); assert!(validate_upload("application/pdf", 100, 100).is_ok()); }
    #[test]
    fn checksum_stable() { assert_eq!(sha256_hex(b"akta").len(), 64); }
}
''')
print("PART2A OK")
