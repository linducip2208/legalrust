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
        sqlx::query("INSERT INTO roles (org_id, name) VALUES (?,?)")
            .bind(&org)
            .bind("org_owner")
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        let rid: u64 = sqlx::query("SELECT LAST_INSERT_ID() AS id")
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
        let r = sqlx::query("SELECT revoked, expires_at FROM sessions WHERE id=?")
            .bind(sid)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(
            matches!(r, Some(x) if !x.get::<bool, _>("revoked") && x.get::<chrono::NaiveDateTime, _>("expires_at") > chrono::Utc::now().naive_utc()),
        )
    }
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
        })
    }
    async fn list_tasks(&self, org: &str, matter: i64) -> Result<Vec<TaskRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, matter_id, title, status FROM tasks WHERE org_id=? AND matter_id=? ORDER BY id").bind(org).bind(matter as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| TaskRow {
                id: x.get::<u64, _>("id") as i64,
                org_id: x.get::<String, _>("org_id"),
                matter_id: x.get::<u64, _>("matter_id") as i64,
                title: x.get::<String, _>("title"),
                status: x.get::<String, _>("status"),
            })
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
}
