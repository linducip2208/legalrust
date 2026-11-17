#!/usr/bin/env python3
"""Final round 2: memory impls for new methods + lib wiring."""
from pathlib import Path
R = Path(r"D:\project laravel\legalrust")
def edit(rel, old, new):
    p = R/rel; t = p.read_text(encoding="utf-8")
    assert old in t, f"anchor missing in {rel}: {old[:60]}"
    p.write_text(t.replace(old, new, 1), encoding="utf-8", newline="\n")

# memory Inner additions
edit("crates/database/src/memory.rs",
"""    docs: HashMap<i64, DocRow>, versions: HashMap<i64, Vec<DocVersionRow>>,
    seq: i64, org_seq: u64,""",
"""    docs: HashMap<i64, DocRow>, versions: HashMap<i64, Vec<DocVersionRow>>,
    roles: HashMap<String, Vec<String>>, tokens: HashMap<String, (String, String, String)>,
    notifs: HashMap<i64, (String, String, String, String, bool)>, shares: HashMap<i64, crate::ops::ShareRow>,
    jobs: HashMap<i64, crate::ops::JobRow>, audits: Vec<crate::ops::AuditRow>,
    seq: i64, org_seq: u64,""")
edit("crates/database/src/memory.rs",
"""    async fn session_live(&self, sid: &str) -> Result<bool, StoreError> {""",
"""    async fn get_user(&self, id: &str) -> Result<Option<UserRow>, StoreError> {
        Ok(self.inner.lock().unwrap().users.get(id).cloned())
    }
    async fn set_password(&self, id: &str, hash: &str) -> Result<(), StoreError> {
        self.inner.lock().unwrap().users.get_mut(id).map(|u| u.pw_hash = hash.into()).ok_or(StoreError::NotFound)
    }
    async fn user_roles(&self, id: &str) -> Result<Vec<String>, StoreError> {
        Ok(self.inner.lock().unwrap().roles.get(id).cloned().unwrap_or_default())
    }
    async fn assign_role(&self, _org: &str, user_id: &str, role: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        if !m.users.contains_key(user_id) { return Err(StoreError::NotFound); }
        m.roles.entry(user_id.into()).or_default().push(role.into());
        Ok(())
    }
    async fn create_token(&self, user_id: &str, name: &str, hash: &str) -> Result<String, StoreError> {
        let mut m = self.inner.lock().unwrap();
        if !m.users.contains_key(user_id) { return Err(StoreError::NotFound); }
        let id = uuid::Uuid::new_v4().to_string();
        m.tokens.insert(hash.into(), (id, user_id.into(), name.into()));
        Ok(id)
    }
    async fn list_tokens(&self, user_id: &str) -> Result<Vec<(String, String)>, StoreError> {
        Ok(self.inner.lock().unwrap().tokens.iter().filter(|(_, (_, u, _))| u == user_id).map(|(_, (id, _, n))| (id.clone(), n.clone())).collect())
    }
    async fn revoke_token(&self, user_id: &str, id: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let k = m.tokens.iter().find(|(_, (i, u, _))| i == id && u == user_id).map(|(k, _)| k.clone()).ok_or(StoreError::NotFound)?;
        m.tokens.remove(&k); Ok(())
    }
    async fn token_owner(&self, hash: &str) -> Result<Option<(String, String)>, StoreError> {
        let m = self.inner.lock().unwrap();
        let hit = m.tokens.get(hash).map(|(_, u, _)| u.clone());
        Ok(hit.and_then(|u| m.users.get(&u).map(|x| (u.clone(), x.org_id.clone()))))
    }
    async fn session_live(&self, sid: &str) -> Result<bool, StoreError> {""")
# seed org_owner role on register (memory)
edit("crates/database/src/memory.rs",
"""        m.users.insert(uid.clone(), UserRow { id: uid.clone(), org_id: org.clone(), email: email.into(), name: name.into(), pw_hash: pw_hash.into(), status: "active".into() });
        Ok((org, uid))""",
"""        m.users.insert(uid.clone(), UserRow { id: uid.clone(), org_id: org.clone(), email: email.into(), name: name.into(), pw_hash: pw_hash.into(), status: "active".into() });
        m.roles.insert(uid.clone(), vec!["org_owner".into()]);
        Ok((org, uid))""")
# memory OpsStore new methods
edit("crates/database/src/ops_memory.rs", """    async fn pay_add(&self, org: &str, inv: i64, amount: i64, method: &str) -> Result<PaymentRow, StoreError> {""",
"""    async fn notify(&self, org: &str, user: &str, kind: &str, title: &str, body: Option<&str>) -> Result<i64, StoreError> {
        let _ = org; let mut m = self.inner.lock().unwrap(); let id = self.next(&mut m);
        m.notifs.insert(id, (user.into(), kind.into(), title.into(), body.unwrap_or("").into(), false)); Ok(id)
    }
    async fn notify_list(&self, user: &str) -> Result<Vec<NotifRow>, StoreError> {
        Ok(self.inner.lock().unwrap().notifs.iter().filter(|(_, (u, _, _, _, _))| u == user).map(|(id, (_, k, t, _, r))| NotifRow { id: *id, kind: k.clone(), title: t.clone(), read: *r }).collect())
    }
    async fn notify_read(&self, user: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        m.notifs.get_mut(&id).filter(|(u, _, _, _, _)| u == user).map(|n| n.4 = true).ok_or(StoreError::NotFound)
    }
    async fn share_create(&self, org: &str, doc: i64, token: &str, hours: i64) -> Result<i64, StoreError> {
        use chrono::Duration;
        let mut m = self.inner.lock().unwrap(); let id = self.next(&mut m);
        let exp = (chrono::Utc::now() + Duration::hours(hours)).naive_utc().to_string();
        m.shares.insert(id, ShareRow { id, org: org.into(), doc, token: token.into(), expires: exp, revoked: false }); Ok(id)
    }
    async fn share_list(&self, org: &str, doc: i64) -> Result<Vec<ShareRow>, StoreError> {
        Ok(self.inner.lock().unwrap().shares.values().filter(|s| s.org == org && s.doc == doc).cloned().collect())
    }
    async fn share_get(&self, token: &str) -> Result<Option<ShareRow>, StoreError> {
        Ok(self.inner.lock().unwrap().shares.values().find(|s| s.token == token).cloned())
    }
    async fn share_revoke(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        m.shares.get_mut(&id).filter(|s| s.org == org).map(|s| s.revoked = true).ok_or(StoreError::NotFound)
    }
    async fn job_enqueue(&self, org: &str, kind: &str, payload: &str) -> Result<i64, StoreError> {
        let mut m = self.inner.lock().unwrap(); let id = self.next(&mut m);
        m.jobs.insert(id, JobRow { id, org: org.into(), kind: kind.into(), payload: payload.into(), status: "queued".into(), attempts: 0 }); Ok(id)
    }
    async fn job_list(&self, org: &str) -> Result<Vec<JobRow>, StoreError> {
        Ok(self.inner.lock().unwrap().jobs.values().filter(|j| j.org == org).cloned().collect())
    }
    async fn job_claim(&self) -> Result<Option<JobRow>, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let id = m.jobs.values().filter(|j| j.status == "queued").map(|j| j.id).min();
        Ok(match id { Some(i) => { let j = m.jobs.get_mut(&i).unwrap(); j.status = "running".into(); j.attempts += 1; Some(j.clone()) } None => None })
    }
    async fn job_done(&self, id: i64, ok: bool, _note: &str) -> Result<(), StoreError> {
        self.inner.lock().unwrap().jobs.get_mut(&id).map(|j| j.status = if ok { "done".into() } else { "failed".into() }).ok_or(StoreError::NotFound)
    }
    async fn audit_list(&self, org: &str, entity: Option<&str>, limit: i64) -> Result<Vec<AuditRow>, StoreError> {
        let _ = org;
        Ok(self.inner.lock().unwrap().audits.iter().rev().filter(|a| entity.map_or(true, |e| a.entity == e)).take(limit.max(0) as usize).cloned().collect())
    }
    async fn audit_add(&self, tenant: &str, actor: Option<&str>, action: &str, entity: &str, entity_id: i64) -> Result<(), StoreError> {
        let _ = tenant; let mut m = self.inner.lock().unwrap(); let id = m.seq + 1; m.seq = id;
        m.audits.push(AuditRow { id, actor: actor.map(str::to_string), action: action.into(), entity: entity.into(), entity_id, created: chrono::Utc::now().naive_utc().to_string() }); Ok(())
    }
    async fn pay_add(&self, org: &str, inv: i64, amount: i64, method: &str) -> Result<PaymentRow, StoreError> {""")
# lib.rs: export new modules/rows
edit("crates/database/src/lib.rs",
"pub use ops::{DeedRow, InvoiceRow, OcrFieldRow, OcrJobRow, OpsStore, PaymentRow, RagHit, TemplateRow};",
"pub use ops::{AuditRow, DeedRow, InvoiceRow, JobRow, NotifRow, OcrFieldRow, OcrJobRow, OpsStore, PaymentRow, RagHit, ShareRow, TemplateRow};")
print("F2 OK")
