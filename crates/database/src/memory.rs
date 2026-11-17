use super::entities::ClientRow;
use super::store::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
#[derive(Debug, Default)]
struct Inner {
    users: HashMap<String, UserRow>,
    sessions: HashMap<String, (String, chrono::NaiveDateTime, bool)>,
    clients: HashMap<i64, ClientRow>,
    matters: HashMap<i64, MatterRow>,
    tasks: HashMap<i64, TaskRow>,
    docs: HashMap<i64, DocRow>,
    versions: HashMap<i64, Vec<DocVersionRow>>,
    roles: HashMap<String, Vec<String>>,
    tokens: HashMap<String, (String, String, String)>,
    seq: i64,
    org_seq: u64,
}
#[derive(Debug, Clone, Default)]
pub struct MemoryStores {
    inner: Arc<Mutex<Inner>>,
}
#[async_trait::async_trait]
impl UserStore for MemoryStores {
    async fn create_org_owner(
        &self,
        org_name: &str,
        email: &str,
        name: &str,
        pw_hash: &str,
    ) -> Result<(String, String), StoreError> {
        let mut m = self.inner.lock().unwrap();
        if m.users.values().any(|u| u.email == email) {
            return Err(StoreError::Conflict("email taken".into()));
        }
        m.org_seq += 1;
        let org = format!("org-{}", m.org_seq);
        let uid = uuid::Uuid::new_v4().to_string();
        let _ = org_name;
        m.users.insert(
            uid.clone(),
            UserRow {
                id: uid.clone(),
                org_id: org.clone(),
                email: email.into(),
                name: name.into(),
                pw_hash: pw_hash.into(),
                status: "active".into(),
            },
        );
        m.roles.insert(uid.clone(), vec!["org_owner".into()]);
        Ok((org, uid))
    }
    async fn find_by_email(&self, email: &str) -> Result<Option<UserRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .users
            .values()
            .find(|u| u.email == email)
            .cloned())
    }
    async fn create_session(
        &self,
        user_id: &str,
        exp: chrono::NaiveDateTime,
    ) -> Result<String, StoreError> {
        let sid = uuid::Uuid::new_v4().to_string();
        self.inner
            .lock()
            .unwrap()
            .sessions
            .insert(sid.clone(), (user_id.into(), exp, false));
        Ok(sid)
    }
    async fn revoke_session(&self, sid: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.sessions.get_mut(sid) {
            Some(s) => {
                s.2 = true;
                Ok(())
            }
            None => Err(StoreError::NotFound),
        }
    }
    async fn get_user(&self, id: &str) -> Result<Option<UserRow>, StoreError> {
        Ok(self.inner.lock().unwrap().users.get(id).cloned())
    }
    async fn set_password(&self, id: &str, hash: &str) -> Result<(), StoreError> {
        self.inner
            .lock()
            .unwrap()
            .users
            .get_mut(id)
            .map(|u| u.pw_hash = hash.into())
            .ok_or(StoreError::NotFound)
    }
    async fn user_roles(&self, id: &str) -> Result<Vec<String>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .roles
            .get(id)
            .cloned()
            .unwrap_or_default())
    }
    async fn assign_role(&self, _org: &str, user_id: &str, role: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        if !m.users.contains_key(user_id) {
            return Err(StoreError::NotFound);
        }
        m.roles.entry(user_id.into()).or_default().push(role.into());
        Ok(())
    }
    async fn create_token(
        &self,
        user_id: &str,
        name: &str,
        hash: &str,
    ) -> Result<String, StoreError> {
        let mut m = self.inner.lock().unwrap();
        if !m.users.contains_key(user_id) {
            return Err(StoreError::NotFound);
        }
        let id = uuid::Uuid::new_v4().to_string();
        m.tokens
            .insert(hash.into(), (id.clone(), user_id.into(), name.into()));
        Ok(id)
    }
    async fn list_tokens(&self, user_id: &str) -> Result<Vec<(String, String)>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .tokens
            .iter()
            .filter(|(_, (_, u, _))| u == user_id)
            .map(|(_, (id, _, n))| (id.clone(), n.clone()))
            .collect())
    }
    async fn revoke_token(&self, user_id: &str, id: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let k = m
            .tokens
            .iter()
            .find(|(_, (i, u, _))| i == id && u == user_id)
            .map(|(k, _)| k.clone())
            .ok_or(StoreError::NotFound)?;
        m.tokens.remove(&k);
        Ok(())
    }
    async fn token_owner(&self, hash: &str) -> Result<Option<(String, String)>, StoreError> {
        let m = self.inner.lock().unwrap();
        let hit = m.tokens.get(hash).map(|(_, u, _)| u.clone());
        Ok(hit.and_then(|u| m.users.get(&u).map(|x| (u.clone(), x.org_id.clone()))))
    }
    async fn session_live(&self, sid: &str) -> Result<bool, StoreError> {
        let now = chrono::Utc::now().naive_utc();
        Ok(
            matches!(self.inner.lock().unwrap().sessions.get(sid), Some((_, exp, false)) if *exp > now),
        )
    }
}
#[async_trait::async_trait]
impl CoreStore for MemoryStores {
    async fn create_client(
        &self,
        org: &str,
        kind: &str,
        name: &str,
    ) -> Result<ClientRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let id = m.seq + 1;
        m.seq = id;
        let c = ClientRow {
            id,
            org_id: org.into(),
            kind: kind.into(),
            display_name: name.into(),
            status: "active".into(),
        };
        m.clients.insert(id, c.clone());
        Ok(c)
    }
    async fn list_clients(&self, org: &str) -> Result<Vec<ClientRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .clients
            .values()
            .filter(|c| c.org_id == org)
            .cloned()
            .collect())
    }
    async fn create_matter(
        &self,
        org: &str,
        client: Option<i64>,
        number: &str,
        title: &str,
    ) -> Result<MatterRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let id = m.seq + 1;
        m.seq = id;
        let r = MatterRow {
            id,
            org_id: org.into(),
            client_id: client,
            number: number.into(),
            title: title.into(),
            status: "open".into(),
        };
        m.matters.insert(id, r.clone());
        Ok(r)
    }
    async fn list_matters(&self, org: &str) -> Result<Vec<MatterRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .matters
            .values()
            .filter(|x| x.org_id == org)
            .cloned()
            .collect())
    }
    async fn get_matter(&self, org: &str, id: i64) -> Result<MatterRow, StoreError> {
        self.inner
            .lock()
            .unwrap()
            .matters
            .get(&id)
            .filter(|x| x.org_id == org)
            .cloned()
            .ok_or(StoreError::NotFound)
    }
    async fn create_task(
        &self,
        org: &str,
        matter: i64,
        title: &str,
    ) -> Result<TaskRow, StoreError> {
        self.get_matter(org, matter).await?;
        let mut m = self.inner.lock().unwrap();
        let id = m.seq + 1;
        m.seq = id;
        let r = TaskRow {
            id,
            org_id: org.into(),
            matter_id: matter,
            title: title.into(),
            status: "open".into(),
        };
        m.tasks.insert(id, r.clone());
        Ok(r)
    }
    async fn list_tasks(&self, org: &str, matter: i64) -> Result<Vec<TaskRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .tasks
            .values()
            .filter(|x| x.org_id == org && x.matter_id == matter)
            .cloned()
            .collect())
    }
    async fn set_task_status(
        &self,
        org: &str,
        id: i64,
        status: &str,
    ) -> Result<TaskRow, StoreError> {
        const OK: &[(&str, &str)] = &[
            ("open", "in_progress"),
            ("open", "blocked"),
            ("in_progress", "done"),
            ("in_progress", "blocked"),
            ("blocked", "open"),
        ];
        let mut m = self.inner.lock().unwrap();
        let t = m
            .tasks
            .get_mut(&id)
            .filter(|x| x.org_id == org)
            .ok_or(StoreError::NotFound)?;
        if !OK.contains(&(t.status.as_str(), status)) {
            return Err(StoreError::Conflict("illegal transition".into()));
        }
        t.status = status.into();
        Ok(t.clone())
    }
    async fn create_doc(
        &self,
        org: &str,
        matter: Option<i64>,
        name: &str,
        mime: &str,
        size: u64,
        sha: &str,
    ) -> Result<DocRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let id = m.seq + 1;
        m.seq = id;
        let r = DocRow {
            id,
            org_id: org.into(),
            matter_id: matter,
            name: name.into(),
            mime: mime.into(),
            size,
            sha: sha.into(),
        };
        m.docs.insert(id, r.clone());
        Ok(r)
    }
    async fn get_doc(
        &self,
        org: &str,
        id: i64,
    ) -> Result<(DocRow, Vec<DocVersionRow>), StoreError> {
        let m = self.inner.lock().unwrap();
        let d = m
            .docs
            .get(&id)
            .filter(|x| x.org_id == org)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        Ok((d, m.versions.get(&id).cloned().unwrap_or_default()))
    }
    async fn list_docs(&self, org: &str, matter: Option<i64>) -> Result<Vec<DocRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .docs
            .values()
            .filter(|x| x.org_id == org && matter.is_none_or(|mt| x.matter_id == Some(mt)))
            .cloned()
            .collect())
    }
    async fn add_version(
        &self,
        org: &str,
        doc: i64,
        key: &str,
        sha: &str,
    ) -> Result<DocVersionRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        if m.docs.get(&doc).filter(|x| x.org_id == org).is_none() {
            return Err(StoreError::NotFound);
        }
        let v = DocVersionRow {
            doc_id: doc,
            version_no: m.versions.get(&doc).map_or(1, |v| v.len() as i64 + 1),
            object_key: key.into(),
            sha: sha.into(),
        };
        m.versions.entry(doc).or_default().push(v.clone());
        Ok(v)
    }
}
