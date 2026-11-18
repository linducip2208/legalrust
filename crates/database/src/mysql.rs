use super::entities::ClientRow;
use super::store::*;
use sqlx::{MySqlPool, Row};
#[derive(Debug, Clone)]
pub struct MysqlStores {
    pub pool: MySqlPool,
}
impl MysqlStores {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}
fn db_err(e: sqlx::Error) -> StoreError {
    if let Some(dbe) = e.as_database_error() {
        if dbe.code().as_deref() == Some("23000") {
            return StoreError::Conflict(dbe.message().into());
        }
    }
    StoreError::Db(e.to_string())
}
#[async_trait::async_trait]
impl UserStore for MysqlStores {
    async fn create_org_owner(
        &self,
        org_name: &str,
        email: &str,
        name: &str,
        pw_hash: &str,
    ) -> Result<(String, String), StoreError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let org = uuid::Uuid::new_v4().to_string();
        let uid = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO organizations (id, name) VALUES (?,?)")
            .bind(&org)
            .bind(org_name)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        sqlx::query(
            "INSERT INTO users (id, org_id, email, password_hash, full_name) VALUES (?,?,?,?,?)",
        )
        .bind(&uid)
        .bind(&org)
        .bind(email)
        .bind(pw_hash)
        .bind(name)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        for rn in [
            "super_admin",
            "org_owner",
            "office_admin",
            "notary",
            "ppat",
            "lawyer",
            "legal_staff",
            "document_officer",
            "finance_officer",
            "auditor",
            "client",
            "reviewer",
        ] {
            sqlx::query("INSERT INTO roles (org_id, name) VALUES (?,?)")
                .bind(&org)
                .bind(rn)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
        }
        let rid: u64 = sqlx::query("SELECT id FROM roles WHERE org_id=? AND name='org_owner'")
            .bind(&org)
            .fetch_one(&mut *tx)
            .await
            .map_err(db_err)?
            .try_get("id")
            .map_err(|e| StoreError::Db(e.to_string()))?;
        sqlx::query("INSERT IGNORE INTO permissions (name) VALUES ('view'),('create'),('update'),('delete'),('approve'),('reject'),('export'),('download'),('share'),('sign'),('verify'),('assign')").execute(&mut *tx).await.map_err(db_err)?;
        sqlx::query(
            "INSERT INTO role_permissions (role_id, permission_id) SELECT ?, id FROM permissions",
        )
        .bind(rid)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES (?,?)")
            .bind(&uid)
            .bind(rid)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok((org, uid))
    }
    async fn find_by_email(&self, email: &str) -> Result<Option<UserRow>, StoreError> {
        let r = sqlx::query("SELECT id, org_id, email, password_hash, full_name, status FROM users WHERE email=? LIMIT 1")
            .bind(email).fetch_optional(&self.pool).await.map_err(db_err)?;
        Ok(r.map(|x| UserRow {
            id: x.get::<String, _>("id"),
            org_id: x.get::<String, _>("org_id"),
            email: x.get::<String, _>("email"),
            name: x.get::<String, _>("full_name"),
            pw_hash: x.get::<String, _>("password_hash"),
            status: x.get::<String, _>("status"),
        }))
    }
    async fn create_session(
        &self,
        user_id: &str,
        exp: chrono::NaiveDateTime,
    ) -> Result<String, StoreError> {
        let sid = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO sessions (id, user_id, expires_at) VALUES (?,?,?)")
            .bind(&sid)
            .bind(user_id)
            .bind(exp)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(sid)
    }
    async fn revoke_session(&self, sid: &str) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE sessions SET revoked=1 WHERE id=?")
            .bind(sid)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }
    async fn session_live(&self, sid: &str) -> Result<bool, StoreError> {
        let r = sqlx::query(
            "SELECT revoked, UNIX_TIMESTAMP(expires_at) AS exp_unix FROM sessions WHERE id=?",
        )
        .bind(sid)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(
            matches!(r, Some(x) if !x.get::<bool, _>("revoked") && x.get::<i64, _>("exp_unix") > chrono::Utc::now().timestamp()),
        )
    }
    async fn get_user(&self, id: &str) -> Result<Option<UserRow>, StoreError> {
        let r = sqlx::query(
            "SELECT id, org_id, email, password_hash, full_name, status FROM users WHERE id=?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.map(|x| UserRow {
            id: x.get::<String, _>("id"),
            org_id: x.get::<String, _>("org_id"),
            email: x.get::<String, _>("email"),
            name: x.get::<String, _>("full_name"),
            pw_hash: x.get::<String, _>("password_hash"),
            status: x.get::<String, _>("status"),
        }))
    }
    async fn set_password(&self, id: &str, hash: &str) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE users SET password_hash=? WHERE id=?")
            .bind(hash)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }
    async fn list_users(&self, org: &str) -> Result<Vec<UserRow>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, org_id, email, password_hash, full_name, status FROM users WHERE org_id=? ORDER BY email LIMIT 200",
        )
        .bind(org)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| UserRow {
                id: x.get::<String, _>("id"),
                org_id: x.get::<String, _>("org_id"),
                email: x.get::<String, _>("email"),
                name: x.get::<String, _>("full_name"),
                pw_hash: x.get::<String, _>("password_hash"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn user_roles(&self, id: &str) -> Result<Vec<String>, StoreError> {
        let rows = sqlx::query(
            "SELECT r.name FROM roles r JOIN user_roles ur ON ur.role_id=r.id WHERE ur.user_id=?",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| x.get::<String, _>("name"))
            .collect())
    }
    async fn assign_role(&self, org: &str, user_id: &str, role: &str) -> Result<(), StoreError> {
        let r = sqlx::query("SELECT id FROM roles WHERE org_id=? AND name=?")
            .bind(org)
            .bind(role)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .ok_or_else(|| StoreError::Conflict("unknown role".into()))?;
        let rid: u64 = r.try_get("id").map_err(|e| StoreError::Db(e.to_string()))?;
        sqlx::query("INSERT IGNORE INTO user_roles (user_id, role_id) VALUES (?,?)")
            .bind(user_id)
            .bind(rid)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn create_token(
        &self,
        user_id: &str,
        name: &str,
        hash: &str,
    ) -> Result<String, StoreError> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO api_tokens (id, user_id, name, token_hash) VALUES (?,?,?,?)")
            .bind(&id)
            .bind(user_id)
            .bind(name)
            .bind(hash)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(id)
    }
    async fn list_tokens(&self, user_id: &str) -> Result<Vec<(String, String)>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, name FROM api_tokens WHERE user_id=? AND revoked=0 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| (x.get::<String, _>("id"), x.get::<String, _>("name")))
            .collect())
    }
    async fn revoke_token(&self, user_id: &str, id: &str) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE api_tokens SET revoked=1 WHERE id=? AND user_id=?")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }
    async fn token_owner(&self, hash: &str) -> Result<Option<(String, String)>, StoreError> {
        let r = sqlx::query(
            "SELECT u.id, u.org_id FROM api_tokens t JOIN users u ON u.id=t.user_id WHERE t.token_hash=? AND t.revoked=0 AND (t.expires_at IS NULL OR t.expires_at > NOW())",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.map(|x| (x.get::<String, _>("id"), x.get::<String, _>("org_id"))))
    }
}
fn task_row(x: &sqlx::mysql::MySqlRow) -> Result<TaskRow, StoreError> {
    Ok(TaskRow {
        id: x
            .try_get::<u64, _>("id")
            .map(|v| v as i64)
            .map_err(|e| StoreError::Db(e.to_string()))?,
        org_id: x
            .try_get::<String, _>("org_id")
            .map_err(|e| StoreError::Db(e.to_string()))?,
        matter_id: x
            .try_get::<u64, _>("matter_id")
            .map(|v| v as i64)
            .map_err(|e| StoreError::Db(e.to_string()))?,
        title: x
            .try_get::<String, _>("title")
            .map_err(|e| StoreError::Db(e.to_string()))?,
        status: x
            .try_get::<String, _>("status")
            .map_err(|e| StoreError::Db(e.to_string()))?,
        due: x
            .try_get::<Option<String>, _>("due_at")
            .map_err(|e| StoreError::Db(e.to_string()))?,
    })
}
#[async_trait::async_trait]
impl CoreStore for MysqlStores {
    async fn create_client(
        &self,
        org: &str,
        kind: &str,
        name: &str,
    ) -> Result<ClientRow, StoreError> {
        let r = sqlx::query("INSERT INTO clients (org_id, kind, display_name) VALUES (?,?,?)")
            .bind(org)
            .bind(kind)
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(ClientRow {
            id: r.last_insert_id() as i64,
            org_id: org.into(),
            kind: kind.into(),
            display_name: name.into(),
            status: "active".into(),
        })
    }
    async fn list_clients(&self, org: &str) -> Result<Vec<ClientRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, kind, display_name, status FROM clients WHERE org_id=? ORDER BY id DESC LIMIT 200").bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| ClientRow {
                id: x.get::<u64, _>("id") as i64,
                org_id: x.get::<String, _>("org_id"),
                kind: x.get::<String, _>("kind"),
                display_name: x.get::<String, _>("display_name"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn create_matter(
        &self,
        org: &str,
        client: Option<i64>,
        number: &str,
        title: &str,
    ) -> Result<MatterRow, StoreError> {
        let r =
            sqlx::query("INSERT INTO matters (org_id, client_id, number, title) VALUES (?,?,?,?)")
                .bind(org)
                .bind(client)
                .bind(number)
                .bind(title)
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(MatterRow {
            id: r.last_insert_id() as i64,
            org_id: org.into(),
            client_id: client,
            number: number.into(),
            title: title.into(),
            status: "open".into(),
        })
    }
    async fn list_matters(&self, org: &str) -> Result<Vec<MatterRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, client_id, number, title, status FROM matters WHERE org_id=? ORDER BY id DESC LIMIT 200").bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| MatterRow {
                id: x.get::<u64, _>("id") as i64,
                org_id: x.get::<String, _>("org_id"),
                client_id: x.get::<Option<u64>, _>("client_id").map(|v| v as i64),
                number: x.get::<String, _>("number"),
                title: x.get::<String, _>("title"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn get_matter(&self, org: &str, id: i64) -> Result<MatterRow, StoreError> {
        let x = sqlx::query("SELECT id, org_id, client_id, number, title, status FROM matters WHERE id=? AND org_id=?").bind(id as u64).bind(org).fetch_optional(&self.pool).await.map_err(db_err)?.ok_or(StoreError::NotFound)?;
        Ok(MatterRow {
            id: x.get::<u64, _>("id") as i64,
            org_id: x.get::<String, _>("org_id"),
            client_id: x.get::<Option<u64>, _>("client_id").map(|v| v as i64),
            number: x.get::<String, _>("number"),
            title: x.get::<String, _>("title"),
            status: x.get::<String, _>("status"),
        })
    }
    async fn create_task(
        &self,
        org: &str,
        matter: i64,
        title: &str,
    ) -> Result<TaskRow, StoreError> {
        self.get_matter(org, matter).await?;
        let r = sqlx::query("INSERT INTO tasks (org_id, matter_id, title) VALUES (?,?,?)")
            .bind(org)
            .bind(matter as u64)
            .bind(title)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(TaskRow {
            id: r.last_insert_id() as i64,
            org_id: org.into(),
            matter_id: matter,
            title: title.into(),
            status: "open".into(),
            due: None,
        })
    }
    async fn list_tasks(&self, org: &str, matter: i64) -> Result<Vec<TaskRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, matter_id, title, status, DATE_FORMAT(due_at,'%Y-%m-%d %H:%i:%s') AS due_at FROM tasks WHERE org_id=? AND matter_id=? ORDER BY id").bind(org).bind(matter as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        rows.iter().map(task_row).collect()
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
        let cur: String = sqlx::query("SELECT status FROM tasks WHERE id=? AND org_id=?")
            .bind(id as u64)
            .bind(org)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .ok_or(StoreError::NotFound)?
            .try_get("status")
            .map_err(|e| StoreError::Db(e.to_string()))?;
        if !OK.contains(&(cur.as_str(), status)) {
            return Err(StoreError::Conflict("illegal transition".into()));
        }
        sqlx::query("UPDATE tasks SET status=? WHERE id=?")
            .bind(status)
            .bind(id as u64)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        let mut t = self.list_tasks(org, 0).await.unwrap_or_default();
        let _ = &mut t;
        Ok(TaskRow {
            id,
            org_id: org.into(),
            matter_id: 0,
            title: String::new(),
            status: status.into(),
            due: None,
        })
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
        let r = sqlx::query("INSERT INTO documents (org_id, matter_id, name, mime, size_bytes, sha256) VALUES (?,?,?,?,?,?)")
            .bind(org).bind(matter.map(|v| v as u64)).bind(name).bind(mime).bind(size).bind(sha).execute(&self.pool).await.map_err(db_err)?;
        Ok(DocRow {
            id: r.last_insert_id() as i64,
            org_id: org.into(),
            matter_id: matter,
            name: name.into(),
            mime: mime.into(),
            size,
            sha: sha.into(),
        })
    }
    async fn get_doc(
        &self,
        org: &str,
        id: i64,
    ) -> Result<(DocRow, Vec<DocVersionRow>), StoreError> {
        let x = sqlx::query("SELECT id, org_id, matter_id, name, mime, size_bytes, sha256 FROM documents WHERE id=? AND org_id=?").bind(id as u64).bind(org).fetch_optional(&self.pool).await.map_err(db_err)?.ok_or(StoreError::NotFound)?;
        let d = DocRow {
            id: x.get::<u64, _>("id") as i64,
            org_id: x.get::<String, _>("org_id"),
            matter_id: x.get::<Option<u64>, _>("matter_id").map(|v| v as i64),
            name: x.get::<String, _>("name"),
            mime: x.get::<String, _>("mime"),
            size: x.get::<u64, _>("size_bytes"),
            sha: x.get::<String, _>("sha256"),
        };
        let vs = sqlx::query("SELECT document_id, version_no, object_key, sha256 FROM document_versions WHERE document_id=? ORDER BY version_no").bind(id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok((
            d,
            vs.into_iter()
                .map(|v| DocVersionRow {
                    doc_id: v.get::<u64, _>("document_id") as i64,
                    version_no: v.get::<u32, _>("version_no") as i64,
                    object_key: v.get::<String, _>("object_key"),
                    sha: v.get::<String, _>("sha256"),
                })
                .collect(),
        ))
    }
    async fn list_docs(&self, org: &str, matter: Option<i64>) -> Result<Vec<DocRow>, StoreError> {
        let rows = if let Some(mt) = matter {
            sqlx::query("SELECT id, org_id, matter_id, name, mime, size_bytes, sha256 FROM documents WHERE org_id=? AND matter_id=? ORDER BY id DESC LIMIT 200").bind(org).bind(mt as u64).fetch_all(&self.pool).await.map_err(db_err)?
        } else {
            sqlx::query("SELECT id, org_id, matter_id, name, mime, size_bytes, sha256 FROM documents WHERE org_id=? ORDER BY id DESC LIMIT 200").bind(org).fetch_all(&self.pool).await.map_err(db_err)?
        };
        Ok(rows
            .into_iter()
            .map(|x| DocRow {
                id: x.get::<u64, _>("id") as i64,
                org_id: x.get::<String, _>("org_id"),
                matter_id: x.get::<Option<u64>, _>("matter_id").map(|v| v as i64),
                name: x.get::<String, _>("name"),
                mime: x.get::<String, _>("mime"),
                size: x.get::<u64, _>("size_bytes"),
                sha: x.get::<String, _>("sha256"),
            })
            .collect())
    }
    async fn add_version(
        &self,
        org: &str,
        doc: i64,
        key: &str,
        sha: &str,
    ) -> Result<DocVersionRow, StoreError> {
        self.get_doc(org, doc).await?;
        let n: u32 = sqlx::query("SELECT COUNT(*) AS c FROM document_versions WHERE document_id=?")
            .bind(doc as u64)
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?
            .try_get("c")
            .map_err(|e| StoreError::Db(e.to_string()))?;
        sqlx::query("INSERT INTO document_versions (document_id, version_no, object_key, sha256) VALUES (?,?,?,?)").bind(doc as u64).bind(n + 1).bind(key).bind(sha).execute(&self.pool).await.map_err(db_err)?;
        Ok(DocVersionRow {
            doc_id: doc,
            version_no: (n + 1) as i64,
            object_key: key.into(),
            sha: sha.into(),
        })
    }
    async fn task_set_due(
        &self,
        org: &str,
        id: i64,
        due: Option<&str>,
    ) -> Result<TaskRow, StoreError> {
        let clean = check_due_format(due)?;
        // Re-parse is infallible: `check_due_format` already validated it.
        let ts: Option<chrono::NaiveDateTime> = match clean {
            None => None,
            Some(s) => Some(
                chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                    .map_err(|e| StoreError::Db(e.to_string()))?,
            ),
        };
        let n = sqlx::query("UPDATE tasks SET due_at=? WHERE id=? AND org_id=?")
            .bind(ts)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        let x = sqlx::query("SELECT id, org_id, matter_id, title, status, DATE_FORMAT(due_at,'%Y-%m-%d %H:%i:%s') AS due_at FROM tasks WHERE id=?").bind(id as u64).fetch_one(&self.pool).await.map_err(db_err)?;
        task_row(&x)
    }
    async fn task_due_list(&self, org: &str, limit: i64) -> Result<Vec<TaskRow>, StoreError> {
        let lim = limit.max(0) as u64;
        let rows = sqlx::query("SELECT id, org_id, matter_id, title, status, DATE_FORMAT(due_at,'%Y-%m-%d %H:%i:%s') AS due_at FROM tasks WHERE org_id=? AND due_at IS NOT NULL ORDER BY due_at ASC LIMIT ?").bind(org).bind(lim).fetch_all(&self.pool).await.map_err(db_err)?;
        rows.iter().map(task_row).collect()
    }
    async fn saved_search_create(
        &self,
        org: &str,
        user: &str,
        name: &str,
        query: &str,
    ) -> Result<SavedSearchRow, StoreError> {
        let (name, query) = check_saved_search(name, query)?;
        // `query` is backticked: it is a reserved word in MySQL.
        let r = sqlx::query(
            "INSERT INTO saved_searches (org_id, user_id, name, `query`) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(user)
        .bind(&name)
        .bind(&query)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(SavedSearchRow {
            id: r.last_insert_id() as i64,
            org_id: org.into(),
            user_id: user.into(),
            name,
            query,
        })
    }
    async fn saved_search_list(
        &self,
        org: &str,
        user: &str,
    ) -> Result<Vec<SavedSearchRow>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, org_id, user_id, name, `query` FROM saved_searches WHERE org_id=? AND user_id=? ORDER BY id",
        )
        .bind(org)
        .bind(user)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        rows.iter()
            .map(|x| {
                Ok(SavedSearchRow {
                    id: x
                        .try_get::<u64, _>("id")
                        .map(|v| v as i64)
                        .map_err(|e| StoreError::Db(e.to_string()))?,
                    org_id: x
                        .try_get::<String, _>("org_id")
                        .map_err(|e| StoreError::Db(e.to_string()))?,
                    user_id: x
                        .try_get::<String, _>("user_id")
                        .map_err(|e| StoreError::Db(e.to_string()))?,
                    name: x
                        .try_get::<String, _>("name")
                        .map_err(|e| StoreError::Db(e.to_string()))?,
                    query: x
                        .try_get::<String, _>("query")
                        .map_err(|e| StoreError::Db(e.to_string()))?,
                })
            })
            .collect()
    }
    async fn saved_search_delete(&self, org: &str, user: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM saved_searches WHERE id=? AND org_id=? AND user_id=?")
            .bind(id as u64)
            .bind(org)
            .bind(user)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }
}
