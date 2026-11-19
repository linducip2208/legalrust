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
    saved_searches: HashMap<i64, SavedSearchRow>,
    docs: HashMap<i64, DocRow>,
    versions: HashMap<i64, Vec<DocVersionRow>>,
    roles: HashMap<String, Vec<String>>,
    tokens: HashMap<String, (String, String, String, Option<chrono::NaiveDateTime>)>,
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
    async fn create_org_member(
        &self,
        org: &str,
        email: &str,
        name: &str,
        pw_hash: &str,
    ) -> Result<String, StoreError> {
        let mut m = self.inner.lock().unwrap();
        if m.users.values().any(|u| u.email == email) {
            return Err(StoreError::Conflict("email taken".into()));
        }
        let uid = uuid::Uuid::new_v4().to_string();
        m.users.insert(
            uid.clone(),
            UserRow {
                id: uid.clone(),
                org_id: org.into(),
                email: email.into(),
                name: name.into(),
                pw_hash: pw_hash.into(),
                status: "active".into(),
            },
        );
        m.roles.insert(uid.clone(), Vec::new());
        Ok(uid)
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
    async fn list_sessions(&self, user: &str) -> Result<Vec<SessionRow>, StoreError> {
        let mut v: Vec<SessionRow> = self
            .inner
            .lock()
            .unwrap()
            .sessions
            .iter()
            .filter(|(_, (u, _, _))| u == user)
            .map(|(sid, (_, exp, revoked))| SessionRow {
                id: sid.clone(),
                expires_at: exp.format("%Y-%m-%d %H:%M:%S").to_string(),
                revoked: *revoked,
            })
            .collect();
        v.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(v)
    }
    async fn get_user(&self, id: &str) -> Result<Option<UserRow>, StoreError> {
        Ok(self.inner.lock().unwrap().users.get(id).cloned())
    }
    async fn list_users(&self, org: &str) -> Result<Vec<UserRow>, StoreError> {
        let mut v: Vec<UserRow> = self
            .inner
            .lock()
            .unwrap()
            .users
            .values()
            .filter(|u| u.org_id == org)
            .cloned()
            .collect();
        v.sort_by(|a, b| a.email.cmp(&b.email));
        Ok(v)
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
        expires_at: Option<chrono::NaiveDateTime>,
    ) -> Result<String, StoreError> {
        let mut m = self.inner.lock().unwrap();
        if !m.users.contains_key(user_id) {
            return Err(StoreError::NotFound);
        }
        let id = uuid::Uuid::new_v4().to_string();
        m.tokens.insert(
            hash.into(),
            (id.clone(), user_id.into(), name.into(), expires_at),
        );
        Ok(id)
    }
    async fn list_tokens(&self, user_id: &str) -> Result<Vec<TokenRow>, StoreError> {
        let mut v: Vec<TokenRow> = self
            .inner
            .lock()
            .unwrap()
            .tokens
            .iter()
            .filter(|(_, (_, u, _, _))| u == user_id)
            .map(|(_, (id, _, n, exp))| TokenRow {
                id: id.clone(),
                name: n.clone(),
                expires_at: exp.map(|e| e.format("%Y-%m-%d %H:%M:%S").to_string()),
            })
            .collect();
        v.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(v)
    }
    async fn revoke_token(&self, user_id: &str, id: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let k = m
            .tokens
            .iter()
            .find(|(_, (i, u, _, _))| i == id && u == user_id)
            .map(|(k, _)| k.clone())
            .ok_or(StoreError::NotFound)?;
        m.tokens.remove(&k);
        Ok(())
    }
    async fn token_owner(&self, hash: &str) -> Result<Option<(String, String)>, StoreError> {
        let now = chrono::Utc::now().naive_utc();
        let m = self.inner.lock().unwrap();
        let hit = m
            .tokens
            .get(hash)
            .filter(|(_, _, _, exp)| exp.is_none_or(|e| e > now))
            .map(|(_, u, _, _)| u.clone());
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
            practice_area: "general".into(),
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
    async fn update_client(
        &self,
        org: &str,
        id: i64,
        name: Option<&str>,
        kind: Option<&str>,
        status: Option<&str>,
    ) -> Result<ClientRow, StoreError> {
        if let Some(n) = name {
            if n.trim().len() < 2 || n.len() > 255 {
                return Err(StoreError::Conflict("name".into()));
            }
        }
        if let Some(k) = kind {
            if !["individual", "corporate"].contains(&k) {
                return Err(StoreError::Conflict("kind".into()));
            }
        }
        if let Some(st) = status {
            if !["active", "archived"].contains(&st) {
                return Err(StoreError::Conflict("status".into()));
            }
        }
        let mut m = self.inner.lock().unwrap();
        let c = m
            .clients
            .get_mut(&id)
            .filter(|x| x.org_id == org)
            .ok_or(StoreError::NotFound)?;
        if let Some(n) = name {
            c.display_name = n.trim().into();
        }
        if let Some(k) = kind {
            c.kind = k.into();
        }
        if let Some(st) = status {
            c.status = st.into();
        }
        Ok(c.clone())
    }
    async fn repoint_matters(
        &self,
        org: &str,
        from_client: i64,
        to_client: i64,
    ) -> Result<u64, StoreError> {
        if from_client == to_client {
            return Err(StoreError::Conflict("same client".into()));
        }
        let mut m = self.inner.lock().unwrap();
        for cid in [from_client, to_client] {
            if m.clients.get(&cid).filter(|x| x.org_id == org).is_none() {
                return Err(StoreError::NotFound);
            }
        }
        let mut n = 0u64;
        for mt in m
            .matters
            .values_mut()
            .filter(|x| x.org_id == org && x.client_id == Some(from_client))
        {
            mt.client_id = Some(to_client);
            n += 1;
        }
        Ok(n)
    }
    async fn delete_client_if_empty(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.clients.get(&id) {
            Some(c) if c.org_id == org => {}
            _ => return Err(StoreError::NotFound),
        }
        if m.matters
            .values()
            .any(|x| x.org_id == org && x.client_id == Some(id))
        {
            return Err(StoreError::Conflict("client has matters".into()));
        }
        m.clients.remove(&id);
        Ok(())
    }
    async fn set_matter_practice(
        &self,
        org: &str,
        id: i64,
        area: &str,
    ) -> Result<MatterRow, StoreError> {
        let area = area.trim().to_lowercase();
        if ![
            "general",
            "litigation",
            "corporate",
            "family",
            "property",
            "notary",
            "ppat",
        ]
        .contains(&area.as_str())
        {
            return Err(StoreError::Conflict("practice_area".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let r = m
            .matters
            .get_mut(&id)
            .filter(|x| x.org_id == org)
            .ok_or(StoreError::NotFound)?;
        r.practice_area = area;
        Ok(r.clone())
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
            due: None,
            assignee: String::new(),
            desk: String::new(),
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
    async fn delete_doc(&self, org: &str, id: i64) -> Result<Vec<String>, StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.docs.get(&id) {
            Some(d) if d.org_id == org => {}
            _ => return Err(StoreError::NotFound),
        }
        m.docs.remove(&id);
        Ok(m.versions
            .remove(&id)
            .unwrap_or_default()
            .into_iter()
            .map(|v| v.object_key)
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
    async fn task_set_due(
        &self,
        org: &str,
        id: i64,
        due: Option<&str>,
    ) -> Result<TaskRow, StoreError> {
        let clean = check_due_format(due)?;
        let mut m = self.inner.lock().unwrap();
        let t = m
            .tasks
            .get_mut(&id)
            .filter(|x| x.org_id == org)
            .ok_or(StoreError::NotFound)?;
        t.due = clean;
        Ok(t.clone())
    }
    async fn task_due_list(&self, org: &str, limit: i64) -> Result<Vec<TaskRow>, StoreError> {
        let mut v: Vec<TaskRow> = self
            .inner
            .lock()
            .unwrap()
            .tasks
            .values()
            .filter(|x| x.org_id == org && x.due.is_some())
            .cloned()
            .collect();
        // Stored dues are validated `%Y-%m-%d %H:%M:%S`, so lexicographic
        // order matches chronological order (overdue first).
        v.sort_by(|a, b| a.due.cmp(&b.due));
        v.truncate(limit.max(0) as usize);
        Ok(v)
    }
    async fn task_assign(
        &self,
        org: &str,
        id: i64,
        assignee: &str,
        desk: &str,
    ) -> Result<TaskRow, StoreError> {
        if assignee.len() > 120 || desk.len() > 60 {
            return Err(StoreError::Conflict("assignee/desk".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let t = m
            .tasks
            .get_mut(&id)
            .filter(|x| x.org_id == org)
            .ok_or(StoreError::NotFound)?;
        t.assignee = assignee.trim().into();
        t.desk = desk.trim().into();
        Ok(t.clone())
    }
    async fn task_board(&self, org: &str, limit: i64) -> Result<Vec<TaskRow>, StoreError> {
        let mut v: Vec<TaskRow> = self
            .inner
            .lock()
            .unwrap()
            .tasks
            .values()
            .filter(|x| x.org_id == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        v.truncate(limit.clamp(1, 500) as usize);
        Ok(v)
    }
    async fn saved_search_create(
        &self,
        org: &str,
        user: &str,
        name: &str,
        query: &str,
    ) -> Result<SavedSearchRow, StoreError> {
        let (name, query) = check_saved_search(name, query)?;
        let mut m = self.inner.lock().unwrap();
        let id = m.seq + 1;
        m.seq = id;
        let r = SavedSearchRow {
            id,
            org_id: org.into(),
            user_id: user.into(),
            name,
            query,
        };
        m.saved_searches.insert(id, r.clone());
        Ok(r)
    }
    async fn saved_search_list(
        &self,
        org: &str,
        user: &str,
    ) -> Result<Vec<SavedSearchRow>, StoreError> {
        let mut v: Vec<SavedSearchRow> = self
            .inner
            .lock()
            .unwrap()
            .saved_searches
            .values()
            .filter(|r| r.org_id == org && r.user_id == user)
            .cloned()
            .collect();
        v.sort_by_key(|r| r.id);
        Ok(v)
    }
    async fn saved_search_delete(&self, org: &str, user: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.saved_searches.get(&id) {
            Some(r) if r.org_id == org && r.user_id == user => {
                m.saved_searches.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
}

#[cfg(test)]
mod deadlines_tests {
    use super::*;

    async fn seeded() -> (MemoryStores, i64) {
        let s = MemoryStores::default();
        s.create_client("org1", "individual", "Acme").await.unwrap();
        let mt = s
            .create_matter("org1", None, "M-001", "Matter one")
            .await
            .unwrap();
        (s, mt.id)
    }

    #[tokio::test]
    async fn set_due_persists_on_row() {
        let (s, matter) = seeded().await;
        let t = s.create_task("org1", matter, "file deed").await.unwrap();
        assert_eq!(t.due, None);
        let t = s
            .task_set_due("org1", t.id, Some("2026-12-01 09:30:00"))
            .await
            .unwrap();
        assert_eq!(t.due.as_deref(), Some("2026-12-01 09:30:00"));
        let listed = s.list_tasks("org1", matter).await.unwrap();
        assert_eq!(listed[0].due.as_deref(), Some("2026-12-01 09:30:00"));
    }

    #[tokio::test]
    async fn clear_due_removes_from_due_list() {
        let (s, matter) = seeded().await;
        let t = s.create_task("org1", matter, "file deed").await.unwrap();
        s.task_set_due("org1", t.id, Some("2026-12-01 09:30:00"))
            .await
            .unwrap();
        assert_eq!(s.task_due_list("org1", 10).await.unwrap().len(), 1);
        let t = s.task_set_due("org1", t.id, None).await.unwrap();
        assert_eq!(t.due, None);
        assert!(s.task_due_list("org1", 10).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn set_due_rejects_bad_format_and_unknown_task() {
        let (s, matter) = seeded().await;
        let t = s.create_task("org1", matter, "file deed").await.unwrap();
        for bad in ["tomorrow", "2026-12-01", ""] {
            let err = s.task_set_due("org1", t.id, Some(bad)).await.unwrap_err();
            assert!(matches!(err, StoreError::Conflict(_)));
        }
        let err = s
            .task_set_due("org1", 999, Some("2026-12-01 09:30:00"))
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::NotFound));
        let err = s
            .task_set_due("org2", t.id, Some("2026-12-01 09:30:00"))
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::NotFound));
    }

    #[tokio::test]
    async fn due_list_orders_overdue_first_and_respects_limit_and_org() {
        let (s, matter) = seeded().await;
        let a = s.create_task("org1", matter, "a").await.unwrap();
        let b = s.create_task("org1", matter, "b").await.unwrap();
        let c = s.create_task("org1", matter, "c-no-due").await.unwrap();
        s.task_set_due("org1", a.id, Some("2026-12-10 00:00:00"))
            .await
            .unwrap();
        s.task_set_due("org1", b.id, Some("2026-11-01 00:00:00"))
            .await
            .unwrap();
        let all = s.task_due_list("org1", 10).await.unwrap();
        // Overdue (b) sorts before upcoming (a); undated (c) excluded.
        assert_eq!(
            vec![b.id, a.id],
            all.iter().map(|t| t.id).collect::<Vec<_>>()
        );
        assert!(all.iter().all(|t| t.id != c.id));
        let one = s.task_due_list("org1", 1).await.unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].id, b.id);
        assert!(s.task_due_list("org2", 10).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn saved_search_crud() {
        let s = MemoryStores::default();
        let a = s
            .saved_search_create("org1", "u1", " Overdue ", " status:open ")
            .await
            .unwrap();
        assert_eq!(a.name, "Overdue");
        assert_eq!(a.query, "status:open");
        let b = s
            .saved_search_create("org1", "u1", "Mine", "owner:me")
            .await
            .unwrap();
        let list = s.saved_search_list("org1", "u1").await.unwrap();
        assert_eq!(
            vec![a.id, b.id],
            list.iter().map(|r| r.id).collect::<Vec<_>>()
        );
        s.saved_search_delete("org1", "u1", a.id).await.unwrap();
        let list = s.saved_search_list("org1", "u1").await.unwrap();
        assert_eq!(vec![b.id], list.iter().map(|r| r.id).collect::<Vec<_>>());
        let err = s.saved_search_delete("org1", "u1", a.id).await.unwrap_err();
        assert!(matches!(err, StoreError::NotFound));
    }

    #[tokio::test]
    async fn saved_search_rejects_blank_and_overlong() {
        let s = MemoryStores::default();
        for (name, query) in [("", "q"), ("n", ""), ("   ", "q"), ("n", "   ")] {
            let err = s
                .saved_search_create("org1", "u1", name, query)
                .await
                .unwrap_err();
            assert!(matches!(err, StoreError::Conflict(_)));
        }
        let err = s
            .saved_search_create("org1", "u1", &"n".repeat(129), "q")
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::Conflict(_)));
        let err = s
            .saved_search_create("org1", "u1", "n", &"q".repeat(513))
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::Conflict(_)));
    }

    #[tokio::test]
    async fn saved_search_isolated_by_org_and_user() {
        let s = MemoryStores::default();
        let a = s.saved_search_create("org1", "u1", "A", "q").await.unwrap();
        s.saved_search_create("org1", "u2", "B", "q").await.unwrap();
        s.saved_search_create("org2", "u1", "C", "q").await.unwrap();
        assert_eq!(s.saved_search_list("org1", "u1").await.unwrap().len(), 1);
        assert!(s
            .saved_search_list("org1", "nobody")
            .await
            .unwrap()
            .is_empty());
        // Cross-org / cross-user deletes are rejected and leave the row intact.
        for (org, user) in [("org2", "u1"), ("org1", "u2")] {
            let err = s.saved_search_delete(org, user, a.id).await.unwrap_err();
            assert!(matches!(err, StoreError::NotFound));
        }
        assert_eq!(s.saved_search_list("org1", "u1").await.unwrap().len(), 1);
    }
}

#[cfg(test)]
mod token_tests {
    use super::*;
    use crate::store::UserStore;

    #[tokio::test]
    async fn expired_token_has_no_owner() {
        let s = MemoryStores::default();
        let (_org, uid) = s
            .create_org_owner("Acme", "a@x.id", "A", "hash")
            .await
            .unwrap();
        let past = chrono::Utc::now().naive_utc() - chrono::Duration::days(1);
        s.create_token(&uid, "old", "h-expired", Some(past))
            .await
            .unwrap();
        assert!(s.token_owner("h-expired").await.unwrap().is_none());
        let future = chrono::Utc::now().naive_utc() + chrono::Duration::days(30);
        s.create_token(&uid, "fresh", "h-fresh", Some(future))
            .await
            .unwrap();
        let own = s.token_owner("h-fresh").await.unwrap().expect("owner");
        assert_eq!(own.0, uid);
        let listed = s.list_tokens(&uid).await.unwrap();
        assert_eq!(listed.len(), 2);
        assert!(listed
            .iter()
            .any(|t| t.name == "old" && t.expires_at.is_some()));
        // Unknown user cannot mint.
        assert!(s.create_token("nobody", "x", "h-x", None).await.is_err());
    }
}
