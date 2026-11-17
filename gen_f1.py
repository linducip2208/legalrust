#!/usr/bin/env python3
"""Final round 1: migration 0008 + store traits + memory impls."""
from pathlib import Path
R = Path(r"D:\project laravel\legalrust")
def w(rel, text):
    p = R/rel; p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text.strip()+"\n", encoding="utf-8", newline="\n")
def edit(rel, old, new):
    p = R/rel; t = p.read_text(encoding="utf-8")
    assert old in t, f"anchor missing in {rel}: {old[:60]}"
    p.write_text(t.replace(old, new, 1), encoding="utf-8", newline="\n")

w("migrations/0008_ops.sql", """-- 0008 notifications, shares, durable jobs
CREATE TABLE notifications (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, user_id CHAR(36) NOT NULL,
  kind VARCHAR(40) NOT NULL, title VARCHAR(255) NOT NULL, body TEXT NULL,
  read_at TIMESTAMP NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_notif_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
  KEY ix_notif_user (user_id, read_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE document_shares (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, document_id BIGINT UNSIGNED NOT NULL,
  token CHAR(36) NOT NULL UNIQUE, expires_at TIMESTAMP NOT NULL,
  revoked TINYINT(1) NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_share_doc FOREIGN KEY (document_id) REFERENCES documents (id) ON DELETE CASCADE,
  KEY ix_share_token (token)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE jobs (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, kind VARCHAR(60) NOT NULL, payload JSON NULL,
  status VARCHAR(20) NOT NULL DEFAULT 'queued', attempts INT NOT NULL DEFAULT 0,
  note TEXT NULL, run_at TIMESTAMP NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  KEY ix_jobs_status (status, run_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
""")

# --- UserStore additions ---
edit("crates/database/src/store.rs", """    async fn revoke_session(&self, sid: &str) -> Result<(), StoreError>;
    async fn session_live(&self, sid: &str) -> Result<bool, StoreError>;""",
"""    async fn revoke_session(&self, sid: &str) -> Result<(), StoreError>;
    async fn session_live(&self, sid: &str) -> Result<bool, StoreError>;
    async fn get_user(&self, id: &str) -> Result<Option<UserRow>, StoreError>;
    async fn set_password(&self, id: &str, hash: &str) -> Result<(), StoreError>;
    async fn user_roles(&self, id: &str) -> Result<Vec<String>, StoreError>;
    async fn assign_role(&self, org: &str, user_id: &str, role: &str) -> Result<(), StoreError>;
    async fn create_token(&self, user_id: &str, name: &str, hash: &str) -> Result<String, StoreError>;
    async fn list_tokens(&self, user_id: &str) -> Result<Vec<(String, String)>, StoreError>;
    async fn revoke_token(&self, user_id: &str, id: &str) -> Result<(), StoreError>;
    async fn token_owner(&self, hash: &str) -> Result<Option<(String, String)>, StoreError>;""")

# --- OpsStore additions ---
edit("crates/database/src/ops.rs", """    async fn pay_add(&self, org: &str, inv: i64, amount: i64, method: &str) -> Result<PaymentRow, StoreError>;""",
"""    async fn pay_add(&self, org: &str, inv: i64, amount: i64, method: &str) -> Result<PaymentRow, StoreError>;
    async fn notify(&self, org: &str, user: &str, kind: &str, title: &str, body: Option<&str>) -> Result<i64, StoreError>;
    async fn notify_list(&self, user: &str) -> Result<Vec<NotifRow>, StoreError>;
    async fn notify_read(&self, user: &str, id: i64) -> Result<(), StoreError>;
    async fn share_create(&self, org: &str, doc: i64, token: &str, hours: i64) -> Result<i64, StoreError>;
    async fn share_list(&self, org: &str, doc: i64) -> Result<Vec<ShareRow>, StoreError>;
    async fn share_get(&self, token: &str) -> Result<Option<ShareRow>, StoreError>;
    async fn share_revoke(&self, org: &str, id: i64) -> Result<(), StoreError>;
    async fn job_enqueue(&self, org: &str, kind: &str, payload: &str) -> Result<i64, StoreError>;
    async fn job_list(&self, org: &str) -> Result<Vec<JobRow>, StoreError>;
    async fn job_claim(&self) -> Result<Option<JobRow>, StoreError>;
    async fn job_done(&self, id: i64, ok: bool, note: &str) -> Result<(), StoreError>;
    async fn audit_list(&self, org: &str, entity: Option<&str>, limit: i64) -> Result<Vec<AuditRow>, StoreError>;
    async fn audit_add(&self, tenant: &str, actor: Option<&str>, action: &str, entity: &str, entity_id: i64) -> Result<(), StoreError>;""")
edit("crates/database/src/ops.rs", """#[derive(Debug, Clone)]
pub struct RagHit { pub doc_id: i64, pub idx: i64, pub text: String }""",
"""#[derive(Debug, Clone)]
pub struct RagHit { pub doc_id: i64, pub idx: i64, pub text: String }
#[derive(Debug, Clone)]
pub struct NotifRow { pub id: i64, pub kind: String, pub title: String, pub read: bool }
#[derive(Debug, Clone)]
pub struct ShareRow { pub id: i64, pub org: String, pub doc: i64, pub token: String, pub expires: String, pub revoked: bool }
#[derive(Debug, Clone)]
pub struct JobRow { pub id: i64, pub org: String, pub kind: String, pub payload: String, pub status: String, pub attempts: i64 }
#[derive(Debug, Clone)]
pub struct AuditRow { pub id: i64, pub actor: Option<String>, pub action: String, pub entity: String, pub entity_id: i64, pub created: String }""")
print("F1 OK")
