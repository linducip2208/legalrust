#!/usr/bin/env python3
"""Final round 3 (reworked): inject new methods into existing impl blocks."""
from pathlib import Path
R = Path(r"D:\project laravel\legalrust")

NEW_OPS = '''
    async fn notify(&self, org: &str, user: &str, kind: &str, title: &str, body: Option<&str>) -> Result<i64, StoreError> {
        let _ = org;
        let r = sqlx::query("INSERT INTO notifications (org_id, user_id, kind, title, body) VALUES (?,?,?,?,?)").bind(org).bind(user).bind(kind).bind(title).bind(body).execute(&self.pool).await.map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn notify_list(&self, user: &str) -> Result<Vec<NotifRow>, StoreError> {
        let rows = sqlx::query("SELECT id, kind, title, read_at FROM notifications WHERE user_id=? ORDER BY id DESC LIMIT 100").bind(user).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows.into_iter().map(|x| NotifRow { id: x.get::<u64,_>("id") as i64, kind: x.get::<String,_>("kind"), title: x.get::<String,_>("title"), read: x.get::<Option<chrono::NaiveDateTime>,_>("read_at").is_some() }).collect())
    }
    async fn notify_read(&self, user: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE notifications SET read_at=NOW() WHERE id=? AND user_id=?").bind(id as u64).bind(user).execute(&self.pool).await.map_err(db_err)?.rows_affected();
        if n == 0 { return Err(StoreError::NotFound); }
        Ok(())
    }
    async fn share_create(&self, org: &str, doc: i64, token: &str, hours: i64) -> Result<i64, StoreError> {
        let r = sqlx::query("INSERT INTO document_shares (org_id, document_id, token, expires_at) VALUES (?,?,?,DATE_ADD(NOW(), INTERVAL ? HOUR))").bind(org).bind(doc as u64).bind(token).bind(hours).execute(&self.pool).await.map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn share_list(&self, org: &str, doc: i64) -> Result<Vec<ShareRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, document_id, token, expires_at, revoked FROM document_shares WHERE org_id=? AND document_id=? ORDER BY id DESC").bind(org).bind(doc as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows.into_iter().map(|x| ShareRow { id: x.get::<u64,_>("id") as i64, org: x.get::<String,_>("org_id"), doc: x.get::<u64,_>("document_id") as i64, token: x.get::<String,_>("token"), expires: x.get::<chrono::NaiveDateTime,_>("expires_at").to_string(), revoked: x.get::<bool,_>("revoked") }).collect())
    }
    async fn share_get(&self, token: &str) -> Result<Option<ShareRow>, StoreError> {
        let x = sqlx::query("SELECT id, org_id, document_id, token, expires_at, revoked FROM document_shares WHERE token=?").bind(token).fetch_optional(&self.pool).await.map_err(db_err)?;
        Ok(x.map(|x| ShareRow { id: x.get::<u64,_>("id") as i64, org: x.get::<String,_>("org_id"), doc: x.get::<u64,_>("document_id") as i64, token: x.get::<String,_>("token"), expires: x.get::<chrono::NaiveDateTime,_>("expires_at").to_string(), revoked: x.get::<bool,_>("revoked") }))
    }
    async fn share_revoke(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE document_shares SET revoked=1 WHERE id=? AND org_id=?").bind(id as u64).bind(org).execute(&self.pool).await.map_err(db_err)?.rows_affected();
        if n == 0 { return Err(StoreError::NotFound); }
        Ok(())
    }
    async fn job_enqueue(&self, org: &str, kind: &str, payload: &str) -> Result<i64, StoreError> {
        let r = sqlx::query("INSERT INTO jobs (org_id, kind, payload) VALUES (?,?,?)").bind(org).bind(kind).bind(payload).execute(&self.pool).await.map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn job_list(&self, org: &str) -> Result<Vec<JobRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, kind, payload, status, attempts FROM jobs WHERE org_id=? ORDER BY id DESC LIMIT 200").bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows.into_iter().map(|x| JobRow { id: x.get::<u64,_>("id") as i64, org: x.get::<String,_>("org_id"), kind: x.get::<String,_>("kind"), payload: x.get::<Option<String>,_>("payload").unwrap_or_default(), status: x.get::<String,_>("status"), attempts: x.get::<i32,_>("attempts") as i64 }).collect())
    }
    async fn job_claim(&self) -> Result<Option<JobRow>, StoreError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let x = sqlx::query("SELECT id, org_id, kind, payload, status, attempts FROM jobs WHERE status='queued' AND (run_at IS NULL OR run_at <= NOW()) ORDER BY id LIMIT 1 FOR UPDATE SKIP LOCKED").fetch_optional(&mut *tx).await.map_err(db_err)?;
        match x {
            None => Ok(None),
            Some(r) => {
                let id: u64 = r.get("id");
                sqlx::query("UPDATE jobs SET status='running', attempts=attempts+1 WHERE id=?").bind(id).execute(&mut *tx).await.map_err(db_err)?;
                tx.commit().await.map_err(db_err)?;
                Ok(Some(JobRow { id: id as i64, org: r.get::<String,_>("org_id"), kind: r.get::<String,_>("kind"), payload: r.get::<Option<String>,_>("payload").unwrap_or_default(), status: "running".into(), attempts: r.get::<i32,_>("attempts") as i64 + 1 }))
            }
        }
    }
    async fn job_done(&self, id: i64, ok: bool, note: &str) -> Result<(), StoreError> {
        sqlx::query("UPDATE jobs SET status=?, note=? WHERE id=?").bind(if ok { "done" } else { "failed" }).bind(note).bind(id as u64).execute(&self.pool).await.map_err(db_err)?; Ok(())
    }
    async fn audit_list(&self, org: &str, entity: Option<&str>, limit: i64) -> Result<Vec<AuditRow>, StoreError> {
        let rows = if let Some(e) = entity {
            sqlx::query("SELECT id, actor_id, action, entity, entity_id, created_at FROM audit_events WHERE tenant_id=? AND entity=? ORDER BY id DESC LIMIT ?").bind(org).bind(e).bind(limit).fetch_all(&self.pool).await.map_err(db_err)?
        } else {
            sqlx::query("SELECT id, actor_id, action, entity, entity_id, created_at FROM audit_events WHERE tenant_id=? ORDER BY id DESC LIMIT ?").bind(org).bind(limit).fetch_all(&self.pool).await.map_err(db_err)?
        };
        Ok(rows.into_iter().map(|x| AuditRow { id: x.get::<u64,_>("id") as i64, actor: x.get::<Option<String>,_>("actor_id"), action: x.get::<String,_>("action"), entity: x.get::<String,_>("entity"), entity_id: x.get::<i64,_>("entity_id"), created: x.get::<chrono::NaiveDateTime,_>("created_at").to_string() }).collect())
    }
    async fn audit_add(&self, tenant: &str, actor: Option<&str>, action: &str, entity: &str, entity_id: i64) -> Result<(), StoreError> {
        sqlx::query("INSERT INTO audit_events (tenant_id, actor_id, action, entity, entity_id) VALUES (?,?,?,?,?)").bind(tenant).bind(actor).bind(action).bind(entity).bind(entity_id).execute(&self.pool).await.map_err(db_err)?; Ok(())
    }
'''

NEW_USER_METHODS = '''
    async fn get_user(&self, id: &str) -> Result<Option<UserRow>, StoreError> {
        let r = sqlx::query("SELECT id, org_id, email, password_hash, full_name, status FROM users WHERE id=?").bind(id).fetch_optional(&self.pool).await.map_err(db_err)?;
        Ok(r.map(|x| UserRow { id: x.get::<String,_>("id"), org_id: x.get::<String,_>("org_id"), email: x.get::<String,_>("email"), name: x.get::<String,_>("full_name"), pw_hash: x.get::<String,_>("password_hash"), status: x.get::<String,_>("status") }))
    }
    async fn set_password(&self, id: &str, hash: &str) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE users SET password_hash=? WHERE id=?").bind(hash).bind(id).execute(&self.pool).await.map_err(db_err)?.rows_affected();
        if n == 0 { return Err(StoreError::NotFound); }
        Ok(())
    }
    async fn user_roles(&self, id: &str) -> Result<Vec<String>, StoreError> {
        let rows = sqlx::query("SELECT r.name FROM roles r JOIN user_roles ur ON ur.role_id=r.id WHERE ur.user_id=?").bind(id).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows.into_iter().map(|x| x.get::<String,_>("name")).collect())
    }
    async fn assign_role(&self, org: &str, user_id: &str, role: &str) -> Result<(), StoreError> {
        let r = sqlx::query("SELECT id FROM roles WHERE org_id=? AND name=?").bind(org).bind(role).fetch_optional(&self.pool).await.map_err(db_err)?.ok_or_else(|| StoreError::Conflict("unknown role".into()))?;
        let rid: u64 = r.try_get("id").map_err(|e| StoreError::Db(e.to_string()))?;
        sqlx::query("INSERT IGNORE INTO user_roles (user_id, role_id) VALUES (?,?)").bind(user_id).bind(rid).execute(&self.pool).await.map_err(db_err)?;
        Ok(())
    }
    async fn create_token(&self, user_id: &str, name: &str, hash: &str) -> Result<String, StoreError> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO api_tokens (id, user_id, name, token_hash) VALUES (?,?,?,?)").bind(&id).bind(user_id).bind(name).bind(hash).execute(&self.pool).await.map_err(db_err)?;
        Ok(id)
    }
    async fn list_tokens(&self, user_id: &str) -> Result<Vec<(String, String)>, StoreError> {
        let rows = sqlx::query("SELECT id, name FROM api_tokens WHERE user_id=? AND revoked=0 ORDER BY created_at DESC").bind(user_id).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows.into_iter().map(|x| (x.get::<String,_>("id"), x.get::<String,_>("name"))).collect())
    }
    async fn revoke_token(&self, user_id: &str, id: &str) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE api_tokens SET revoked=1 WHERE id=? AND user_id=?").bind(id).bind(user_id).execute(&self.pool).await.map_err(db_err)?.rows_affected();
        if n == 0 { return Err(StoreError::NotFound); }
        Ok(())
    }
    async fn token_owner(&self, hash: &str) -> Result<Option<(String, String)>, StoreError> {
        let r = sqlx::query("SELECT u.id, u.org_id FROM api_tokens t JOIN users u ON u.id=t.user_id WHERE t.token_hash=? AND t.revoked=0 AND (t.expires_at IS NULL OR t.expires_at > NOW())").bind(hash).fetch_optional(&self.pool).await.map_err(db_err)?;
        Ok(r.map(|x| (x.get::<String,_>("id"), x.get::<String,_>("org_id"))))
    }
'''

p = R/"crates/database/src/ops_mysql.rs"; t = p.read_text(encoding="utf-8")
assert t.rstrip().endswith("}")
t = t.rstrip()
assert t.endswith("}")
t = t[:t.rfind("\n}")] + "\n" + NEW_OPS + "}\n"
p.write_text(t, encoding="utf-8", newline="\n")

p = R/"crates/database/src/mysql.rs"; t = p.read_text(encoding="utf-8")
# seed all 12 roles at registration
old_seed = '''        sqlx::query("INSERT INTO roles (org_id, name) VALUES (?,?)").bind(&org).bind("org_owner").execute(&mut *tx).await.map_err(db_err)?;
        let rid: u64 = sqlx::query("SELECT LAST_INSERT_ID() AS id").fetch_one(&mut *tx).await.map_err(db_err)?.try_get("id").map_err(|e| StoreError::Db(e.to_string()))?;'''
new_seed = '''        for rn in ["super_admin","org_owner","office_admin","notary","ppat","lawyer","legal_staff","document_officer","finance_officer","auditor","client","reviewer"] {
            sqlx::query("INSERT INTO roles (org_id, name) VALUES (?,?)").bind(&org).bind(rn).execute(&mut *tx).await.map_err(db_err)?;
        }
        let rid: u64 = sqlx::query("SELECT id FROM roles WHERE org_id=? AND name='org_owner'").bind(&org).fetch_one(&mut *tx).await.map_err(db_err)?.try_get("id").map_err(|e| StoreError::Db(e.to_string()))?;'''
assert old_seed in t
t = t.replace(old_seed, new_seed, 1)
t = t.rstrip()
assert t.endswith("}")
# insert new methods before final close of UserStore impl (last method is session_live, then CoreStore impl follows)
anchor = "    async fn session_live(&self, sid: &str) -> Result<bool, StoreError> {"
i = t.find(anchor)
assert i > 0
# find end of that method: next "\n    }\n" after i... session_live body ends; then CoreStore impl starts with "#[async_trait::async_trait]\nimpl CoreStore"
j = t.find("#[async_trait::async_trait]\nimpl CoreStore for MysqlStores", i)
assert j > 0
t = t[:j] + NEW_USER_METHODS + t[j:]
# fix imports: mysql.rs needs chrono::NaiveDateTime? uses chrono::Utc — chrono in scope via full path. Row trait import exists. uuid used. fine.
p.write_text(t, encoding="utf-8", newline="\n")
print("F3 OK")
